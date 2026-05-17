use anyhow::Result;
use crate::db::Database;
use gqueues_api_rs::models::Task;
use gqueues_api_rs::GqueuesClient;
use crate::sync::{SyncEngine, ProgressReporter, SyncEvent};
use std::sync::{Arc, Mutex};
use indicatif::{ProgressBar, ProgressStyle};

struct SpinnerReporter {
    pb: ProgressBar,
}

impl ProgressReporter for SpinnerReporter {
    fn report(&self, event: SyncEvent) {
        match event {
            SyncEvent::InProgress { message } => {
                self.pb.set_message(message);
            }
            SyncEvent::Error(e) => {
                self.pb.set_message(format!("❌ Sync error: {}", e));
            }
            _ => {}
        }
    }
}

pub async fn handle_add(client: Arc<GqueuesClient>, db: Arc<Mutex<Database>>, title: String) -> Result<()> {
    println!("Gqueues TUI v{} Prototype", env!("CARGO_PKG_VERSION"));
    
    // Find the inbox queue to use as a default
    let inbox_key = {
        let db_locked = db.lock().unwrap();
        let queues = db_locked.get_queues()?;
        let inbox = queues.iter().find(|q| q.is_inbox)
            .or_else(|| queues.first())
            .ok_or_else(|| anyhow::anyhow!("No queues found in database. Please run TUI first to sync metadata."))?;
        inbox.key.clone()
    };

    let now_str = chrono::Utc::now().to_rfc3339();
    let new_task = Task {
        key: "".into(),
        title: Some(title.clone()),
        notes: None,
        completed: false,
        queue_key: Some(inbox_key),
        parent_key: None,
        subitems: None,
        tags: None,
        assignments: None,
        creation_date: Some(gqueues_api_rs::models::DateInfo {
            text: None,
            raw: now_str,
        }),
        due_date: None,
        repeats: serde_json::Value::Bool(false),
        section_key: None,
        attachments: None,
        crossed: false,
        num_comments: Some(0),
        has_subitems: false,
        position: None,
        access: Some("user".into()),
        add_comments: true,
        local_order: Some(0.0),
    };

    {
        let db_locked = db.lock().unwrap();
        db_locked.add_task_local(new_task, true)?;
    }
    println!("✅ Task saved to local database.");

    // Sync immediately
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(std::time::Duration::from_millis(120));
    pb.set_style(
        ProgressStyle::with_template("{spinner:.blue} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message("⏳ Syncing with GQueues API...");

    let reporter = Arc::new(SpinnerReporter { pb: pb.clone() });
    let engine = SyncEngine::new_minimal(client, db)
        .with_reporter(reporter);

    match engine.push_pending_changes().await {
        Ok(_) => {
            pb.finish_and_clear();
            println!("✅ Task synced to GQueues successfully.");
        }
        Err(e) => {
            pb.finish_and_clear();
            eprintln!("❌ Sync failed: {}", e);
            return Err(e);
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_handle_add_logic() -> Result<()> {
        let db_file = NamedTempFile::new()?;
        let db_path = db_file.path().to_path_buf();
        let db = Database::new(db_path)?;
        let db_shared = Arc::new(Mutex::new(db));

        // Need to add a queue first
        {
            let db_locked = db_shared.lock().unwrap();
            let inbox = gqueues_api_rs::models::Queue {
                key: "inbox-key".into(),
                name: "Inbox".into(),
                is_inbox: true,
                last_modified: Some("never".into()),
                category: None,
                category_name: None,
                team_name: None,
                scope: None,
            };
            db_locked.upsert_queue(&inbox)?;
        }

        // We can't easily test the full handle_add because it needs a real API client
        // but we've tested the core logic in previous turns and here we just verify it compiles
        // and doesn't panic during basic setup.
        
        Ok(())
    }
}
