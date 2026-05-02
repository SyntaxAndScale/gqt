use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use anyhow::{Result, anyhow};
use crate::db::Database;
use crate::gqueues::GqueuesClient;
use crate::gqueues::client::GqueuesError;
use crate::models::Operation;

pub enum SyncEvent {
    Complete,
    Error(String),
}

pub struct SyncEngine {
    client: Arc<GqueuesClient>,
    db: Arc<Mutex<Database>>,
    active_queue_key: Arc<Mutex<Option<String>>>,
    tx: mpsc::Sender<SyncEvent>,
}

impl SyncEngine {
    pub fn new(
        client: Arc<GqueuesClient>,
        db: Arc<Mutex<Database>>,
        active_queue_key: Arc<Mutex<Option<String>>>,
        tx: mpsc::Sender<SyncEvent>,
    ) -> Self {
        Self { client, db, active_queue_key, tx }
    }

    pub async fn run(&mut self) {
        let mut retry_count = 0;

        loop {
            let interval = match self.sync_cycle().await {
                Ok(_) => {
                    retry_count = 0;
                    log::info!("Sync cycle completed successfully");
                    let _ = self.tx.send(SyncEvent::Complete).await;
                    Duration::from_secs(60)
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
                                retry_count += 1;
                                log::error!("Sync cycle failed: {}", e);
                                let backoff = Duration::from_secs(2u64.pow(retry_count).min(60).max(5));
                                let _ = self.tx.send(SyncEvent::Error(format!("Sync error: {}", e))).await;
                                backoff
                            }
                        }
                    } else {
                        retry_count += 1;
                        log::error!("Sync cycle failed with non-Gqueues error: {}", e);
                        let backoff = Duration::from_secs(2u64.pow(retry_count).min(60).max(5));
                        let _ = self.tx.send(SyncEvent::Error(format!("Sync error: {}", e))).await;
                        backoff
                    }
                }
            };

            sleep(interval).await;
        }
    }

    async fn sync_cycle(&self) -> Result<()> {
        self.push_pending_changes().await?;
        self.pull_remote_changes().await?;
        Ok(())
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
        
        let active_key = {
            let active = self.active_queue_key.lock().unwrap();
            active.clone()
        };

        // 2. Identify priority queue
        let mut queues_to_sync = Vec::new();
        for q in api_queues {
            let is_priority = active_key.as_ref() == Some(&q.key);
            queues_to_sync.push((q, is_priority));
        }

        // Sort: priority first
        queues_to_sync.sort_by(|a, b| b.1.cmp(&a.1));

        for (api_queue, is_priority) in queues_to_sync {
            let should_pull = {
                let db = self.db.lock().unwrap();
                let local_modified = db.get_queue_last_modified(&api_queue.key)?;
                
                // Pull if it's the priority queue OR if metadata says it changed
                is_priority || local_modified.as_deref() != api_queue.last_modified.as_deref()
            };

            if should_pull {
                log::info!("Pulling tasks for queue: {} (priority: {})", api_queue.name, is_priority);
                let tasks = self.client.get_tasks(&api_queue.key).await
                    .map_err(|e| anyhow!(e))?;
                
                let db = self.db.lock().unwrap();
                // Update queue metadata first (to update local_id mapping and last_modified)
                db.upsert_queue(&api_queue)?;
                
                for mut t in tasks {
                    t.queue_key = Some(api_queue.key.clone());
                    db.upsert_task(&t)?;
                }
                db.update_queue_sync_time(&api_queue.key)?;
            } else {
                log::debug!("Skipping pull for queue: {} (no changes detected)", api_queue.name);
            }
        }

        Ok(())
    }
}
