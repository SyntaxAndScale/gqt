use serde::{Deserialize, Serialize};
use crate::gqueues::models::Task;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    CreateTask(Task),
    UpdateTask(Task),
    DeleteTask(String), // key
    CompleteTask(String), // key
}
