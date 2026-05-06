use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use anyhow::{Result, anyhow};
use crate::db::Database;
use crate::gqueues::GqueuesClient;
use crate::gqueues::client::GqueuesError;
use crate::gqueues::Queue;
use crate::models::Operation;
use rusqlite::OptionalExtension;

pub enum SyncEvent {
    Complete { unfetched: usize, total: usize },
    Error(String),
}

pub enum SyncCommand {
    ForceSync,
}

pub struct SyncEngine {
    client: Arc<GqueuesClient>,
    db: Arc<Mutex<Database>>,
    active_queue_key: Arc<Mutex<Option<String>>>,
    tx: mpsc::Sender<SyncEvent>,
    cmd_rx: mpsc::Receiver<SyncCommand>,
}

impl SyncEngine {
    pub fn new(
        client: Arc<GqueuesClient>,
        db: Arc<Mutex<Database>>,
        active_queue_key: Arc<Mutex<Option<String>>>,
        tx: mpsc::Sender<SyncEvent>,
        cmd_rx: mpsc::Receiver<SyncCommand>,
    ) -> Self {
        Self { client, db, active_queue_key, tx, cmd_rx }
    }

    pub async fn run(&mut self) {
        let mut retry_count = 0;
        let mut interval = Duration::from_secs(60);

        loop {
            tokio::select! {
                _ = sleep(interval) => {
                    // Periodic sync
                    interval = self.handle_sync_cycle(&mut retry_count).await;
                }
                Some(cmd) = self.cmd_rx.recv() => {
                    match cmd {
                        SyncCommand::ForceSync => {
                            log::info!("Manual sync triggered via command");
                            interval = self.handle_sync_cycle(&mut retry_count).await;
                        }
                    }
                }
            }
        }
    }

    async fn handle_sync_cycle(&self, retry_count: &mut u32) -> Duration {
        match self.sync_cycle().await {
            Ok(stats) => {
                *retry_count = 0;
                log::info!("Sync cycle completed successfully: {}/{} remaining", stats.0, stats.1);
                let _ = self.tx.send(SyncEvent::Complete { unfetched: stats.0, total: stats.1 }).await;
                
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
                            let _ = self.tx.send(SyncEvent::Error(format!("Rate limited. Waiting {:?}...", dur))).await;
                            *dur
                        }
                        _ => {
                            *retry_count += 1;
                            log::error!("Sync cycle failed: {}", e);
                            let backoff = Duration::from_secs(2u64.pow(*retry_count).min(60).max(5));
                            let _ = self.tx.send(SyncEvent::Error(format!("Sync error: {}", e))).await;
                            backoff
                        }
                    }
                } else {
                    *retry_count += 1;
                    log::error!("Sync cycle failed with non-Gqueues error: {}", e);
                    let backoff = Duration::from_secs(2u64.pow(*retry_count).min(60).max(5));
                    let _ = self.tx.send(SyncEvent::Error(format!("Sync error: {}", e))).await;
                    backoff
                }
            }
        }
    }

    async fn sync_cycle(&self) -> Result<(usize, usize)> {
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

    async fn push_pending_changes(&self) -> Result<()> {
        let pending = {
            let db = self.db.lock().unwrap();
            db.get_pending_transactions()?
        };

        for (tx_id, op_json, idem_key) in pending {
            let operation: Operation = serde_json::from_str(&op_json)?;
            
            match operation {
                Operation::CreateTask(task) => {
                    match self.client.create_task_with_idempotency(
                        &task.title,
                        task.queue_key.as_deref(),
                        task.notes.as_deref(),
                        &idem_key
                    ).await {
                        Ok(remote_task) => {
                            let db = self.db.lock().unwrap();
                            db.update_task_remote_key(&task.key, &remote_task.key)?;
                            db.mark_transaction_synced(&tx_id)?;
                        }
                        Err(e) => return Err(anyhow!(e)),
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    async fn pull_remote_changes(&self) -> Result<()> {
        // 1. Sync Queues Metadata
        let api_queues = self.client.get_queues().await
            .map_err(|e| anyhow!(e))?;
        
        {
            let db = self.db.lock().unwrap();
            for q in api_queues {
                db.upsert_queue(&q)?;
            }
        }

        let active_key = {
            let active = self.active_queue_key.lock().unwrap();
            active.clone()
        };

        // 2. Decide what to pull
        let next_queue = {
            let db = self.db.lock().unwrap();
            
            // Check if active queue needs fetching
            let mut active_needs_fetch = false;
            if let Some(ref ak) = active_key {
                let mut stmt = db.conn.prepare("SELECT tasks_fetched FROM queues WHERE remote_key = ?1")?;
                let fetched: Option<i32> = stmt.query_row([ak], |row| row.get(0)).optional()?;
                active_needs_fetch = fetched == Some(0);
            }

            if active_needs_fetch {
                let ak = active_key.as_ref().unwrap();
                let mut stmt = db.conn.prepare("SELECT remote_key, name, is_inbox, last_modified, category, category_name, team_name, scope FROM queues WHERE remote_key = ?1")?;
                let q = stmt.query_row([ak], |row| Ok(Queue {
                    key: row.get(0)?,
                    name: row.get(1)?,
                    is_inbox: row.get::<_, i32>(2)? != 0,
                    last_modified: row.get(3)?,
                    category: row.get(4)?,
                    category_name: row.get(5)?,
                    team_name: row.get(6)?,
                    scope: row.get(7)?,
                }))?;
                Some(q)
            } else {
                db.get_next_unfetched_queue()?
            }
        };

        if let Some(queue) = next_queue {
            log::info!("Lazy Sync: Pulling tasks for queue: {}", queue.name);
            let tasks = self.client.get_tasks(&queue.key).await
                .map_err(|e| anyhow!(e))?;
            
            {
                let db = self.db.lock().unwrap();
                for mut t in tasks {
                    t.queue_key = Some(queue.key.clone());
                    db.upsert_task(&t)?;
                }
                db.mark_tasks_fetched(&queue.key)?;
                db.update_queue_sync_time(&queue.key)?;
            }
        } else {
            // All queues are fetched, do normal metadata check for the most out-of-date queue
            let stale_queue = {
                let db = self.db.lock().unwrap();
                let mut stmt = db.conn.prepare(
                    "SELECT remote_key, name, is_inbox, last_modified, category, category_name, team_name, scope 
                     FROM queues 
                     ORDER BY last_synced_at ASC LIMIT 1"
                )?;
                stmt.query_row([], |row| Ok(Queue {
                    key: row.get(0)?,
                    name: row.get(1)?,
                    is_inbox: row.get::<_, i32>(2)? != 0,
                    last_modified: row.get(3)?,
                    category: row.get(4)?,
                    category_name: row.get(5)?,
                    team_name: row.get(6)?,
                    scope: row.get(7)?,
                })).optional()?
            };

            if let Some(q) = stale_queue {
                log::debug!("Checking for updates on stale queue: {}", q.name);
                let tasks = self.client.get_tasks(&q.key).await
                    .map_err(|e| anyhow!(e))?;
                
                let db = self.db.lock().unwrap();
                for mut t in tasks {
                    t.queue_key = Some(q.key.clone());
                    db.upsert_task(&t)?;
                }
                db.update_queue_sync_time(&q.key)?;
            }
        }

        Ok(())
    }
}
