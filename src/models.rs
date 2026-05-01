use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::gqueues::models::Task;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    CreateTask(Task),
    UpdateTask(Task),
    DeleteTask(String), // key
    CompleteTask(String), // key
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub operation: Operation,
    pub synced: bool,
}
