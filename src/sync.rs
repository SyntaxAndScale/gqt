use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use anyhow::Result;
use crate::db::Database;
use crate::gqueues::GqueuesClient;
use crate::models::Operation;

pub enum SyncEvent {
    Complete,
    Error(String),
}

pub struct SyncEngine {
    client: Arc<GqueuesClient>,
    db: Arc<Mutex<Database>>,
    tx: mpsc::Sender<SyncEvent>,
}

impl SyncEngine {
    pub fn new(
        client: Arc<GqueuesClient>,
        db: Arc<Mutex<Database>>,
        tx: mpsc::Sender<SyncEvent>,
    ) -> Self {
        Self { client, db, tx }
    }

    pub async fn run(&mut self) {
        let mut retry_count = 0;

        loop {
            let interval = match self.sync_cycle().await {
                Ok(_) => {
                    retry_count = 0;
                    let _ = self.tx.send(SyncEvent::Complete).await;
                    Duration::from_secs(60)
                }
                Err(e) => {
                    retry_count += 1;
                    // Exponential backoff: 5s, 10s, 20s, 40s, max 60s
                    let backoff = Duration::from_secs(2u64.pow(retry_count).min(60).max(5));
                    let _ = self.tx.send(SyncEvent::Error(format!("Sync error: {}", e))).await;
                    backoff
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
                        Err(e) => return Err(e),
                    }
                }
                _ => {
                    // Update/Delete not supported by API yet
                }
            }
        }
        Ok(())
    }

    async fn pull_remote_changes(&self) -> Result<()> {
        let queues = self.client.get_queues().await?;
        {
            let db = self.db.lock().unwrap();
            for q in &queues {
                db.upsert_queue(q)?;
            }
        }

        // For now, pull tasks for all queues we have
        // Future optimization: only active or modified
        let current_queues = {
            let db = self.db.lock().unwrap();
            db.get_queues()?
        };

        for queue in current_queues {
            let tasks = self.client.get_tasks(&queue.key).await?;
            let db = self.db.lock().unwrap();
            for mut t in tasks {
                // Ensure queue_key is set (API might omit it as it's implicit in the request)
                t.queue_key = Some(queue.key.clone());
                db.upsert_task(&t)?;
            }
        }

        Ok(())
    }
}
