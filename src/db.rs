use anyhow::{anyhow, Result};
use directories::ProjectDirs;
use rusqlite::{Connection, OptionalExtension};
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
                last_modified TEXT NOT NULL,
                last_synced_at TEXT,
                category TEXT,
                category_name TEXT,
                team_name TEXT,
                scope TEXT,
                tasks_fetched INTEGER NOT NULL DEFAULT 0
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
                sync_status TEXT NOT NULL,
                idempotency_key TEXT NOT NULL
            )",
            [],
        )?;

        Ok(())
    }

    pub fn upsert_queue(&self, queue: &Queue) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO queues (local_id, remote_key, name, is_inbox, last_modified, last_synced_at, category, category_name, team_name, scope, tasks_fetched)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(remote_key) DO UPDATE SET
                name = excluded.name,
                is_inbox = excluded.is_inbox,
                last_modified = excluded.last_modified,
                category = excluded.category,
                category_name = excluded.category_name,
                team_name = excluded.team_name,
                scope = excluded.scope",
            (
                Uuid::new_v4().to_string(),
                &queue.key,
                &queue.name,
                if queue.is_inbox { 1 } else { 0 },
                queue.last_modified.as_deref().unwrap_or(&now),
                &now,
                queue.category.as_deref(),
                queue.category_name.as_deref(),
                queue.team_name.as_deref(),
                queue.scope.as_deref(),
                0, // tasks_fetched defaults to 0 on new insert
            ),
        )?;
        Ok(())
    }

    pub fn get_queues(&self) -> Result<Vec<Queue>> {
        let mut stmt = self.conn.prepare("SELECT remote_key, name, is_inbox, last_modified, category, category_name, team_name, scope FROM queues")?;
        let rows = stmt.query_map([], |row| {
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
        })?;

        let mut queues = Vec::new();
        for queue in rows {
            queues.push(queue?);
        }
        Ok(queues)
    }

    pub fn mark_tasks_fetched(&self, queue_key: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE queues SET tasks_fetched = 1 WHERE remote_key = ?1",
            [queue_key],
        )?;
        Ok(())
    }

    pub fn get_unfetched_queues_count(&self) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM queues WHERE tasks_fetched = 0",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    pub fn get_total_queues_count(&self) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM queues",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    pub fn get_next_unfetched_queue(&self) -> Result<Option<Queue>> {
        let mut stmt = self.conn.prepare(
            "SELECT remote_key, name, is_inbox, last_modified, category, category_name, team_name, scope 
             FROM queues 
             WHERE tasks_fetched = 0 
             ORDER BY is_inbox DESC, last_synced_at ASC 
             LIMIT 1"
        )?;
        let mut rows = stmt.query_map([], |row| {
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
        })?;

        if let Some(queue) = rows.next() {
            Ok(Some(queue?))
        } else {
            Ok(None)
        }
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
        let idempotency_key = Uuid::new_v4().to_string();
        let operation = serde_json::to_string(&crate::models::Operation::CreateTask(task.clone()))?;
        self.conn.execute(
            "INSERT INTO transactions (id, operation, timestamp, sync_status, idempotency_key)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            (
                &transaction_id,
                &operation,
                &now,
                "pending",
                &idempotency_key,
            ),
        )?;

        Ok(task)
    }

    pub fn get_pending_transactions(&self) -> Result<Vec<(String, String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, operation, idempotency_key FROM transactions WHERE sync_status = 'pending' ORDER BY timestamp ASC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;

        let mut txs = Vec::new();
        for tx in rows {
            txs.push(tx?);
        }
        Ok(txs)
    }

    pub fn update_queue_sync_time(&self, queue_key: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE queues SET last_synced_at = ?1 WHERE remote_key = ?2",
            [now, queue_key.to_string()],
        )?;
        Ok(())
    }

    pub fn get_queue_last_modified(&self, queue_key: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare("SELECT last_modified FROM queues WHERE remote_key = ?1")?;
        let res = stmt.query_row([queue_key], |row| row.get(0)).optional()?;
        Ok(res)
    }

    pub fn mark_transaction_synced(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE transactions SET sync_status = 'synced' WHERE id = ?1",
            [id],
        )?;
        Ok(())
    }

    pub fn update_task_remote_key(&self, local_key: &str, remote_key: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE tasks SET remote_key = ?1 WHERE remote_key = ?2",
            [remote_key, local_key],
        )?;
        Ok(())
    }
}
