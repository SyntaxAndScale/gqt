use anyhow::{anyhow, Result};
use directories::ProjectDirs;
use rusqlite::{Connection, OptionalExtension};
use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use gqueues_api_rs::models::{Queue, Task};
use uuid::Uuid;

/// Manages local SQLite persistence and synchronization state.
pub struct Database {
    pub conn: Connection,
}

impl Database {
    /// Connects to a SQLite database at the specified path and initializes the schema.
    pub fn new(path: PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Resolves the default XDG path for the gqt database.
    pub fn get_default_db_path() -> Result<PathBuf> {
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
                parent_key TEXT,
                title TEXT NOT NULL,
                notes TEXT,
                completed INTEGER NOT NULL,
                last_modified TEXT NOT NULL,
                tags TEXT,
                assignments TEXT,
                creation_date TEXT,
                due_date TEXT,
                repeats TEXT,
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

    /// Upserts a queue into the local database.
    pub fn upsert_queue(&self, queue: &Queue) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO queues (local_id, remote_key, name, is_inbox, last_modified, last_synced_at, category, category_name, team_name, scope, tasks_fetched)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(remote_key) DO UPDATE SET
                name = excluded.name,
                is_inbox = excluded.is_inbox,
                category = excluded.category,
                category_name = excluded.category_name,
                team_name = excluded.team_name,
                scope = excluded.scope",
            (
                Uuid::new_v4().to_string(),
                &queue.key,
                &queue.name,
                if queue.is_inbox { 1 } else { 0 },
                "", 
                &now,
                queue.category.as_deref(),
                queue.category_name.as_deref(),
                queue.team_name.as_deref(),
                queue.scope.as_deref(),
                0,
            ),
        )?;
        Ok(())
    }

    /// Updates the local sync point for a queue after a successful task pull.
    pub fn update_queue_sync_point(&self, queue_key: &str, server_timestamp: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE queues SET last_modified = ?1, last_synced_at = ?2, tasks_fetched = 1 WHERE remote_key = ?3",
            [server_timestamp, &now, queue_key],
        )?;
        Ok(())
    }

    /// Retrieves all queues from the local database.
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

    /// Marks a queue as needing a full task fetch.
    pub fn mark_queue_unfetched(&self, queue_key: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE queues SET last_modified = '', tasks_fetched = 0 WHERE remote_key = ?1",
            [queue_key],
        )?;
        Ok(())
    }

    /// Marks a queue's tasks as having been successfully fetched at least once.
    pub fn mark_tasks_fetched(&self, queue_key: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE queues SET tasks_fetched = 1 WHERE remote_key = ?1",
            [queue_key],
        )?;
        Ok(())
    }

    /// Returns the number of queues that have never been synchronized.
    pub fn get_unfetched_queues_count(&self) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM queues WHERE tasks_fetched = 0",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Returns the total number of queues in the database.
    pub fn get_total_queues_count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM queues", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Upserts a task and its sub-tasks recursively.
    pub fn upsert_task(&self, task: &Task) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let tags_json = serde_json::to_string(&task.tags)?;
        let assignments_json = serde_json::to_string(&task.assignments)?;
        let creation_date_json = serde_json::to_string(&task.creation_date)?;
        let due_date_json = serde_json::to_string(&task.due_date)?;
        let repeats_json = serde_json::to_string(&task.repeats)?;

        self.conn.execute(
            "INSERT INTO tasks (local_id, remote_key, queue_id, parent_key, title, notes, completed, last_modified, tags, assignments, creation_date, due_date, repeats)
             VALUES (?1, ?2, 
                (SELECT local_id FROM queues WHERE remote_key IS ?3 OR local_id IS ?3), 
                (SELECT local_id FROM tasks WHERE remote_key IS ?4 OR local_id IS ?4), 
                ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(remote_key) DO UPDATE SET
                title = excluded.title,
                notes = excluded.notes,
                completed = excluded.completed,
                parent_key = excluded.parent_key,
                queue_id = excluded.queue_id,
                last_modified = excluded.last_modified,
                tags = excluded.tags,
                assignments = excluded.assignments,
                creation_date = excluded.creation_date,
                due_date = excluded.due_date,
                repeats = excluded.repeats",
            (
                Uuid::new_v4().to_string(),
                &task.key,
                &task.queue_key,
                &task.parent_key,
                &task.title,
                &task.notes,
                if task.completed { 1 } else { 0 },
                now,
                tags_json,
                assignments_json,
                creation_date_json,
                due_date_json,
                repeats_json,
            ),
        )?;

        if let Some(ref items) = task.subitems {
            for sub in items {
                let mut sub_clone = sub.clone();
                sub_clone.queue_key = task.queue_key.clone();
                sub_clone.parent_key = Some(task.key.clone());
                self.upsert_task(&sub_clone)?;
            }
        }
        Ok(())
    }

    /// Retrieves all tasks for a specific queue, including their hierarchical context.
    pub fn get_tasks(&self, queue_key: &str) -> Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(
            "SELECT 
                t.remote_key, 
                t.title, 
                t.notes, 
                t.completed, 
                q.remote_key as queue_key, 
                (SELECT p.remote_key FROM tasks p WHERE p.local_id = t.parent_key) as parent_key,
                t.tags, 
                t.assignments, 
                t.creation_date, 
                t.due_date, 
                t.repeats
             FROM tasks t
             JOIN queues q ON t.queue_id = q.local_id
             WHERE q.remote_key = ?1 OR q.local_id = ?1
             ORDER BY t.creation_date ASC, t.local_id ASC",
        )?;
        let rows = stmt.query_map([queue_key], |row| {
            Ok(Task {
                key: row.get(0)?,
                title: row.get(1)?,
                notes: row.get(2)?,
                completed: row.get::<_, i32>(3)? != 0,
                queue_key: row.get(4)?,
                parent_key: row.get(5)?,
                subitems: None,
                tags: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or(None),
                assignments: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or(None),
                creation_date: serde_json::from_str(&row.get::<_, String>(8)?).unwrap_or(None),
                due_date: serde_json::from_str(&row.get::<_, String>(9)?).unwrap_or(None),
                repeats: serde_json::from_str(&row.get::<_, String>(10)?)
                    .unwrap_or(serde_json::Value::Bool(false)),
            })
        })?;

        let mut tasks = Vec::new();
        for task in rows {
            tasks.push(task?);
        }
        Ok(tasks)
    }

    /// Saves a task locally and logs a pending transaction for the Sync Engine.
    pub fn add_task_local(&self, mut task: Task) -> Result<Task> {
        let local_id = Uuid::new_v4().to_string();
        task.key = format!("local-{}", local_id);
        let now = Utc::now().to_rfc3339();
        let tags_json = serde_json::to_string(&task.tags)?;
        let assignments_json = serde_json::to_string(&task.assignments)?;
        let creation_date_json = serde_json::to_string(&task.creation_date)?;
        let due_date_json = serde_json::to_string(&task.due_date)?;
        let repeats_json = serde_json::to_string(&task.repeats)?;

        self.conn.execute(
            "INSERT INTO tasks (local_id, remote_key, queue_id, parent_key, title, notes, completed, last_modified, tags, assignments, creation_date, due_date, repeats)
             VALUES (?1, ?2, 
                (SELECT local_id FROM queues WHERE remote_key IS ?3 OR local_id IS ?3), 
                (SELECT local_id FROM tasks WHERE remote_key IS ?4 OR local_id IS ?4), 
                ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            (
                &local_id,
                &task.key,
                &task.queue_key,
                &task.parent_key,
                &task.title,
                &task.notes,
                if task.completed { 1 } else { 0 },
                &now,
                tags_json,
                assignments_json,
                creation_date_json,
                due_date_json,
                repeats_json,
            ),
        )?;

        let transaction_id = Uuid::new_v4().to_string();
        let idempotency_key = Uuid::new_v4().to_string();
        let operation = serde_json::to_string(&crate::models::Operation::Create(task.clone()))?;
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

    /// Retrieves all pending transactions to be pushed to the remote API.
    pub fn get_pending_transactions(&self) -> Result<Vec<(String, String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, operation, idempotency_key FROM transactions WHERE sync_status = 'pending' ORDER BY timestamp ASC"
        )?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;

        let mut txs = Vec::new();
        for tx in rows {
            txs.push(tx?);
        }
        Ok(txs)
    }

    /// Returns the last modified timestamp for a queue stored in the local database.
    pub fn get_queue_last_modified(&self, queue_key: &str) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT last_modified FROM queues WHERE remote_key = ?1")?;
        let res = stmt.query_row([queue_key], |row| row.get(0)).optional()?;
        Ok(res)
    }

    /// Marks a transaction as successfully reconciled with the remote API.
    pub fn mark_transaction_synced(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE transactions SET sync_status = 'synced' WHERE id = ?1",
            [id],
        )?;
        Ok(())
    }

    /// Updates a task's remote key after it has been promoted from a local item.
    pub fn update_task_remote_key(&self, local_key: &str, remote_key: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE tasks SET remote_key = ?1 WHERE remote_key = ?2",
            [remote_key, local_key],
        )?;
        Ok(())
    }
}
