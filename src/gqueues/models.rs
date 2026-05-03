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
}
