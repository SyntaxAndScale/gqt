use serde::{Deserialize, Serialize};

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
