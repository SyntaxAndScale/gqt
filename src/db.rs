use anyhow::{anyhow, Result};
use directories::ProjectDirs;
use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;

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
                remote_key TEXT,
                name TEXT NOT NULL,
                is_inbox INTEGER NOT NULL,
                last_modified TEXT NOT NULL
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS tasks (
                local_id TEXT PRIMARY KEY,
                remote_key TEXT,
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
}
