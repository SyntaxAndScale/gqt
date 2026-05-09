use gqueues_api_rs::models::Task;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    Create(Task),
    Update(Task),
    Delete(String),   // key
    Complete(String), // key
}
