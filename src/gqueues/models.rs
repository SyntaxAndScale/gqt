use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Queue {
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub is_inbox: bool,
    pub last_modified: Option<String>,
    pub category: Option<String>,
    pub category_name: Option<String>,
    pub team_name: Option<String>,
    #[serde(skip)]
    pub scope: Option<String>, // "Personal", "Team", or "Shared"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub key: String,
    pub title: String,
    pub notes: Option<String>,
    #[serde(default)]
    pub completed: bool,
    pub queue_key: Option<String>,
    pub parent_key: Option<String>,
    pub subitems: Option<Vec<Task>>,
    pub tags: Option<Vec<String>>,
    pub assignments: Option<Vec<Assignment>>,
    pub creation_date: Option<DateInfo>,
    pub due_date: Option<DueDateInfo>,
    #[serde(default)]
    pub repeats: serde_json::Value, // Can be bool or object
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Assignment {
    pub email: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DateInfo {
    pub raw: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DueDateInfo {
    pub raw_date: Option<String>,
}
