use crate::db::Database;
use crate::models::Operation;
use anyhow::{anyhow, Result};
use chrono::Utc;
use gqueues_api_rs::Queue;
use gqueues_api_rs::GqueuesClient;
use gqueues_api_rs::client::GqueuesError;
use rusqlite::OptionalExtension;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

/// Events emitted by the SyncEngine.
#[derive(Debug, Clone)]
pub enum SyncEvent {
    /// Indicates an operation is currently in progress with a descriptive message.
    InProgress { message: String },
    /// Indicates a full sync cycle has finished, providing progress statistics.
    Complete { unfetched: usize, total: usize },
    /// Indicates a non-fatal error occurred during synchronization.
    Error(String),
}

/// A trait for reporting synchronization progress.
pub trait ProgressReporter: Send + Sync {
    fn report(&self, event: SyncEvent);
}

/// A reporter that sends events through an mpsc channel.
pub struct ChannelReporter {
    pub tx: mpsc::Sender<SyncEvent>,
}

impl ProgressReporter for ChannelReporter {
    fn report(&self, event: SyncEvent) {
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let _ = tx.send(event).await;
        });
    }
}

/// Commands sent from the TUI to the background SyncEngine.
pub enum SyncCommand {
    /// Triggers an immediate synchronization cycle, bypassing the periodic timer.
    ForceSync,
}

/// The background task responsible for reconciling local state with the GQueues API.
pub struct SyncEngine {
    client: Arc<GqueuesClient>,
    db: Arc<Mutex<Database>>,
    active_queue_key: Arc<Mutex<Option<String>>>,
    reporter: Option<Arc<dyn ProgressReporter>>,
    cmd_rx: mpsc::Receiver<SyncCommand>,
}

impl SyncEngine {
    /// Creates a new SyncEngine instance for TUI.
    pub fn new(
        client: Arc<GqueuesClient>,
        db: Arc<Mutex<Database>>,
        active_queue_key: Arc<Mutex<Option<String>>>,
        tx: mpsc::Sender<SyncEvent>,
        cmd_rx: mpsc::Receiver<SyncCommand>,
    ) -> Self {
        Self {
            client,
            db,
            active_queue_key,
            reporter: Some(Arc::new(ChannelReporter { tx })),
            cmd_rx,
        }
    }

    /// Creates a minimal SyncEngine for one-off operations (like CLI).
    pub fn new_minimal(client: Arc<GqueuesClient>, db: Arc<Mutex<Database>>) -> Self {
        let (_, cmd_rx) = mpsc::channel(1);
        Self {
            client,
            db,
            active_queue_key: Arc::new(Mutex::new(None)),
            reporter: None,
            cmd_rx,
        }
    }

    /// Sets a progress reporter for the engine.
    pub fn with_reporter(mut self, reporter: Arc<dyn ProgressReporter>) -> Self {
        self.reporter = Some(reporter);
        self
    }

    /// The main execution loop for the background sync task.
    pub async fn run(&mut self) {
        let mut retry_count = 0;
        let mut interval = Duration::from_secs(1); // Start first cycle almost immediately

        loop {
            tokio::select! {
                _ = sleep(interval) => {
                    // Periodic sync
                    interval = self.handle_sync_cycle(&mut retry_count).await;
                }
                Some(cmd) = self.cmd_rx.recv() => {
                    match cmd {
                        SyncCommand::ForceSync => {
                            log::info!("Manual sync triggered: forcing refresh of active queue");
                            let active_key = {
                                let active = self.active_queue_key.lock().unwrap();
                                active.clone()
                            };
                            if let Some(ak) = active_key {
                                let db = self.db.lock().unwrap();
                                let _ = db.mark_queue_unfetched(&ak);
                            }
                            interval = self.handle_sync_cycle(&mut retry_count).await;
                        }
                    }
                }
            }
        }
    }

    /// Handles a single synchronization cycle, including push and pull phases.
    async fn handle_sync_cycle(&self, retry_count: &mut u32) -> Duration {
        if let Some(ref r) = self.reporter {
            r.report(SyncEvent::InProgress {
                message: "⏳ Sync in progress...".into(),
            });
        }
        match self.sync_cycle().await {
            Ok(stats) => {
                *retry_count = 0;
                log::info!(
                    "Sync cycle completed successfully: {}/{} remaining",
                    stats.0,
                    stats.1
                );
                if let Some(ref r) = self.reporter {
                    r.report(SyncEvent::Complete {
                        unfetched: stats.0,
                        total: stats.1,
                    });
                }

                // If we still have unfetched queues, run again sooner
                if stats.0 > 0 {
                    Duration::from_secs(5)
                } else {
                    Duration::from_secs(60)
                }
            }
            Err(e) => {
                if let Some(gq_err) = e.downcast_ref::<GqueuesError>() {
                    match gq_err {
                        GqueuesError::RateLimited(dur) => {
                            log::warn!("Rate limited. Waiting for {:?}", dur);
                            if let Some(ref r) = self.reporter {
                                r.report(SyncEvent::Error(format!(
                                    "Rate limited. Waiting {:?}...",
                                    dur
                                )));
                            }
                            *dur
                        }
                        _ => {
                            *retry_count += 1;
                            log::error!("Sync cycle failed: {}", e);
                            let backoff = Duration::from_secs(2u64.pow(*retry_count).clamp(5, 60));
                            if let Some(ref r) = self.reporter {
                                r.report(SyncEvent::Error(format!("Sync error: {}", e)));
                            }
                            backoff
                        }
                    }
                } else {
                    *retry_count += 1;
                    log::error!("Sync cycle failed with non-Gqueues error: {}", e);
                    let backoff = Duration::from_secs(2u64.pow(*retry_count).clamp(5, 60));
                    if let Some(ref r) = self.reporter {
                        r.report(SyncEvent::Error(format!("Sync error: {}", e)));
                    }
                    backoff
                }
            }
        }
    }

    pub async fn sync_cycle(&self) -> Result<(usize, usize)> {
        self.push_pending_changes().await?;
        self.pull_remote_changes().await?;

        let (unfetched, total) = {
            let db = self.db.lock().unwrap();
            let unfetched = db.get_unfetched_queues_count()?;
            let total = db.get_total_queues_count()?;
            (unfetched, total)
        };
        Ok((unfetched, total))
    }

    /// Pushes local transactions to the remote GQueues API.
    pub async fn push_pending_changes(&self) -> Result<()> {
        let pending = {
            let db = self.db.lock().unwrap();
            db.get_pending_transactions()?
        };

        for (tx_id, op_json, idem_key) in pending {
            let operation: Operation = serde_json::from_str(&op_json)?;

            let (task, is_quick_add) = match operation {
                Operation::Create(t) => (t, false),
                Operation::CreateQuick(t) => (t, true),
                _ => {
                    log::warn!("Sync Engine: Skipping unimplemented operation: {:?}", operation);
                    let db = self.db.lock().unwrap();
                    db.mark_transaction_synced(&tx_id)?;
                    continue;
                }
            };

            let title = task.title.as_deref().unwrap_or("");
            if let Some(ref r) = self.reporter {
                r.report(SyncEvent::InProgress {
                    message: format!("⏳ Syncing: {}", title),
                });
            }
            log::info!(
                "Sync Engine: Promoting local task '{}' (Local Key: {}, Quick Add: {}) to GQueues",
                title,
                task.key,
                is_quick_add
            );
            match self
                .client
                .create_task_with_idempotency(
                    title,
                    task.queue_key.as_deref(),
                    task.parent_key.as_deref(),
                    task.notes.as_deref(),
                    task.tags.clone(),
                    task.due_date.as_ref().and_then(|d| d.raw_date.as_deref()),
                    is_quick_add,
                    &idem_key,
                )
                .await
            {
                Ok(remote_task) => {
                    log::info!(
                        "Sync Engine: Successfully promoted '{}'. New Remote Key: {}",
                        remote_task.title.as_deref().unwrap_or(""),
                        remote_task.key
                    );
                    let db = self.db.lock().unwrap();
                    db.update_task_remote_key(&task.key, &remote_task.key)?;
                    db.mark_transaction_synced(&tx_id)?;
                }
                Err(e) => {
                    log::error!(
                        "Sync Engine: Failed to promote task '{}': {}",
                        title,
                        e
                    );
                    return Err(anyhow!(e));
                }
            }
        }
        Ok(())
    }

    /// Pulls remote changes from the API and reconciles them with the local database.
    pub async fn pull_remote_changes(&self) -> Result<()> {
        if let Some(ref r) = self.reporter {
            r.report(SyncEvent::InProgress {
                message: "⏳ Fetching queue metadata...".into(),
            });
        }
        let api_queues = self.client.get_queues().await.map_err(|e| anyhow!(e))?;

        let active_key = {
            let active = self.active_queue_key.lock().unwrap();
            active.clone()
        };

        let mut modified_queues = Vec::new();
        {
            let db = self.db.lock().unwrap();
            for api_q in api_queues {
                let local_modified = db.get_queue_last_modified(&api_q.key)?;
                let needs_fetch = {
                    let mut stmt = db
                        .conn
                        .prepare("SELECT tasks_fetched FROM queues WHERE remote_key = ?1")?;
                    let fetched: Option<i32> =
                        stmt.query_row([&api_q.key], |row| row.get(0)).optional()?;
                    fetched != Some(1)
                };

                if needs_fetch || local_modified.as_deref() != api_q.last_modified.as_deref() {
                    modified_queues.push(api_q.clone());
                }

                db.upsert_queue(&api_q)?;
            }
        }

        if let Some(ref ak) = active_key {
            modified_queues.sort_by(|a, b| {
                if &a.key == ak {
                    std::cmp::Ordering::Less
                } else if &b.key == ak {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            });
        }

        if modified_queues.is_empty() {
            let stale_queue = {
                let db = self.db.lock().unwrap();
                let mut stmt = db.conn.prepare(
                    "SELECT remote_key, name, is_inbox, last_modified, category, category_name, team_name, scope 
                     FROM queues 
                     ORDER BY last_synced_at ASC LIMIT 1"
                )?;
                stmt.query_row([], |row| {
                    Ok(Queue {
                        key: row.get(0)?,
                        name: row.get(1)?,
                        is_inbox: row.get::<_, i32>(2)? != 0,
                        last_modified: row.get(3)?,
                        category: row.get(4)?,
                        category_name: row.get(5)?,
                        team_name: row.get(6)?,
                        scope: row.get(7)?,
                    })
                })
                .optional()?
            };

            if let Some(q) = stale_queue {
                if let Some(ref r) = self.reporter {
                    r.report(SyncEvent::InProgress {
                        message: format!("⏳ Syncing: {}", q.name),
                    });
                }
                log::debug!("Background Sync: Checking stale queue: {}", q.name);
                let tasks = self
                    .client
                    .get_tasks(&q.key)
                    .await
                    .map_err(|e| anyhow!(e))?;

                let db = self.db.lock().unwrap();
                for mut t in tasks {
                    t.queue_key = Some(q.key.clone());
                    db.upsert_task(&t)?;
                }
                let now = Utc::now().to_rfc3339();
                db.update_queue_sync_point(&q.key, &now)?;
            }
        } else {
            for queue in modified_queues {
                if let Some(ref r) = self.reporter {
                    r.report(SyncEvent::InProgress {
                        message: format!("⏳ Syncing: {}", queue.name),
                    });
                }
                log::info!(
                    "Syncing tasks for queue: {} (Key: {})",
                    queue.name,
                    queue.key
                );
                let tasks = self
                    .client
                    .get_tasks(&queue.key)
                    .await
                    .map_err(|e| anyhow!(e))?;

                {
                    let db = self.db.lock().unwrap();
                    for mut t in tasks {
                        t.queue_key = Some(queue.key.clone());
                        db.upsert_task(&t)?;
                    }
                    db.update_queue_sync_point(
                        &queue.key,
                        queue.last_modified.as_deref().unwrap_or(""),
                    )?;
                }
                sleep(Duration::from_millis(500)).await;
            }
        }

        Ok(())
    }
}
