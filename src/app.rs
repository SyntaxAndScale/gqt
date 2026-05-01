use crate::gqueues::{GqueuesClient, Queue, Task};
use crate::db::Database;
use std::sync::{Arc, Mutex};

#[derive(Debug, PartialEq, Eq)]
pub enum Pane {
    Queues,
    Tasks,
    Details,
}

pub struct App {
    pub client: Arc<GqueuesClient>,
    pub db: Arc<Mutex<Database>>,
    pub queues: Vec<Queue>,
    pub tasks: Vec<Task>,
    pub selected_queue_index: usize,
    pub selected_task_index: usize,
    pub active_pane: Pane,
    pub running: bool,
    pub error: Option<String>,
    pub loading: bool,
}

impl App {
    pub fn new(client: GqueuesClient, db: Database) -> Self {
        Self {
            client: Arc::new(client),
            db: Arc::new(Mutex::new(db)),
            queues: Vec::new(),
            tasks: Vec::new(),
            selected_queue_index: 0,
            selected_task_index: 0,
            active_pane: Pane::Queues,
            running: true,
            error: None,
            loading: false,
        }
    }

    pub fn selected_queue(&self) -> Option<&Queue> {
        self.queues.get(self.selected_queue_index)
    }

    pub fn selected_task(&self) -> Option<&Task> {
        self.tasks.get(self.selected_task_index)
    }

    pub fn next_pane(&mut self) {
        self.active_pane = match self.active_pane {
            Pane::Queues => Pane::Tasks,
            Pane::Tasks => Pane::Details,
            Pane::Details => Pane::Queues,
        };
    }

    pub fn previous_pane(&mut self) {
        self.active_pane = match self.active_pane {
            Pane::Queues => Pane::Details,
            Pane::Tasks => Pane::Queues,
            Pane::Details => Pane::Tasks,
        };
    }
}
