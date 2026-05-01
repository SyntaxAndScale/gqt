use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Queue {
    pub key: String,
    pub name: String,
    pub is_inbox: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub key: String,
    pub title: String,
    pub notes: Option<String>,
    pub completed: bool,
    pub queue_key: Option<String>,
}

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
