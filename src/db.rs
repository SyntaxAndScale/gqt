use anyhow::{anyhow, Result};
use directories::ProjectDirs;
use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;

use crate::gqueues::models::{Queue, Task};
use chrono::Utc;
use uuid::Uuid;

pub struct Database {
    pub conn: Connection,
}

impl Database {
    pub fn new() -> Result<Self> {
        let path = Self::get_db_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    fn get_db_path() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("com", "gqt", "gqt")
            .ok_or_else(|| anyhow!("Could not determine project directories"))?;
        let data_dir = proj_dirs.data_dir();
        Ok(data_dir.join("gqt.db"))
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS queues (
                local_id TEXT PRIMARY KEY,
                remote_key TEXT UNIQUE,
                name TEXT NOT NULL,
                is_inbox INTEGER NOT NULL,
                last_modified TEXT NOT NULL
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS tasks (
                local_id TEXT PRIMARY KEY,
                remote_key TEXT UNIQUE,
                queue_id TEXT NOT NULL,
                title TEXT NOT NULL,
                notes TEXT,
                completed INTEGER NOT NULL,
                last_modified TEXT NOT NULL,
                FOREIGN KEY(queue_id) REFERENCES queues(local_id)
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS transactions (
                id TEXT PRIMARY KEY,
                operation TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                sync_status TEXT NOT NULL
            )",
            [],
        )?;

        Ok(())
    }

    pub fn upsert_queue(&self, queue: &Queue) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO queues (local_id, remote_key, name, is_inbox, last_modified)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(remote_key) DO UPDATE SET
                name = excluded.name,
                is_inbox = excluded.is_inbox,
                last_modified = excluded.last_modified",
            (
                Uuid::new_v4().to_string(),
                &queue.key,
                &queue.name,
                if queue.is_inbox { 1 } else { 0 },
                now,
            ),
        )?;
        Ok(())
    }

    pub fn get_queues(&self) -> Result<Vec<Queue>> {
        let mut stmt = self.conn.prepare("SELECT remote_key, name, is_inbox FROM queues")?;
        let rows = stmt.query_map([], |row| {
            Ok(Queue {
                key: row.get(0)?,
                name: row.get(1)?,
                is_inbox: row.get::<_, i32>(2)? != 0,
            })
        })?;

        let mut queues = Vec::new();
        for queue in rows {
            queues.push(queue?);
        }
        Ok(queues)
    }

    pub fn upsert_task(&self, task: &Task) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO tasks (local_id, remote_key, queue_id, title, notes, completed, last_modified)
             VALUES (?1, ?2, (SELECT local_id FROM queues WHERE remote_key = ?3), ?4, ?5, ?6, ?7)
             ON CONFLICT(remote_key) DO UPDATE SET
                title = excluded.title,
                notes = excluded.notes,
                completed = excluded.completed,
                last_modified = excluded.last_modified",
            (
                Uuid::new_v4().to_string(),
                &task.key,
                &task.queue_key,
                &task.title,
                &task.notes,
                if task.completed { 1 } else { 0 },
                now,
            ),
        )?;
        Ok(())
    }

    pub fn get_tasks(&self, queue_key: &str) -> Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.remote_key, t.title, t.notes, t.completed, q.remote_key
             FROM tasks t
             JOIN queues q ON t.queue_id = q.local_id
             WHERE q.remote_key = ?1",
        )?;
        let rows = stmt.query_map([queue_key], |row| {
            Ok(Task {
                key: row.get(0)?,
                title: row.get(1)?,
                notes: row.get(2)?,
                completed: row.get::<_, i32>(3)? != 0,
                queue_key: row.get(4)?,
            })
        })?;

        let mut tasks = Vec::new();
        for task in rows {
            tasks.push(task?);
        }
        Ok(tasks)
    }

    pub fn add_task_local(&self, mut task: Task) -> Result<Task> {
        let local_id = Uuid::new_v4().to_string();
        // For local tasks, we set a placeholder key that UI recognizes as pending
        task.key = format!("local-{}", local_id);
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            "INSERT INTO tasks (local_id, remote_key, queue_id, title, notes, completed, last_modified)
             VALUES (?1, ?2, (SELECT local_id FROM queues WHERE remote_key = ?3), ?4, ?5, ?6, ?7)",
            (
                &local_id,
                &task.key,
                &task.queue_key,
                &task.title,
                &task.notes,
                if task.completed { 1 } else { 0 },
                &now,
            ),
        )?;

        // Log transaction
        let transaction_id = Uuid::new_v4().to_string();
        let operation = serde_json::to_string(&crate::models::Operation::CreateTask(task.clone()))?;
        self.conn.execute(
            "INSERT INTO transactions (id, operation, timestamp, sync_status)
             VALUES (?1, ?2, ?3, ?4)",
            (
                &transaction_id,
                &operation,
                &now,
                "pending",
            ),
        )?;

        Ok(task)
    }
}
