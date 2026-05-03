use crate::gqueues::{GqueuesClient, Queue, Task};
use crate::db::Database;
use std::sync::{Arc, Mutex};
use std::collections::HashSet;

#[derive(Debug, PartialEq, Eq)]
pub enum Pane {
    Queues,
    Tasks,
    Details,
}

#[derive(Debug, Clone)]
pub enum NavEntry {
    Category { name: String, expanded: bool },
    Queue(Queue),
}

pub struct App {
    pub client: Arc<GqueuesClient>,
    pub db: Arc<Mutex<Database>>,
    pub active_queue_key: Arc<Mutex<Option<String>>>,
    pub expanded_categories: HashSet<String>,
    pub queues: Vec<Queue>,
    pub tasks: Vec<Task>,
    pub selected_nav_index: usize,
    pub selected_task_index: usize,
    pub active_pane: Pane,
    pub running: bool,
    pub status: String,
}

impl App {
    pub fn new(client: GqueuesClient, db: Database) -> Self {
        Self {
            client: Arc::new(client),
            db: Arc::new(Mutex::new(db)),
            active_queue_key: Arc::new(Mutex::new(None)),
            expanded_categories: HashSet::new(),
            queues: Vec::new(),
            tasks: Vec::new(),
            selected_nav_index: 0,
            selected_task_index: 0,
            active_pane: Pane::Queues,
            running: true,
            status: "Ready".into(),
        }
    }

    pub fn get_nav_entries(&self) -> Vec<NavEntry> {
        let mut entries = Vec::new();
        
        // 1. Group queues by their display category
        let mut grouped: std::collections::BTreeMap<String, Vec<Queue>> = std::collections::BTreeMap::new();
        
        for q in &self.queues {
            let cat = if let Some(ref cn) = q.category_name {
                cn.clone()
            } else if let Some(ref tn) = q.team_name {
                tn.clone()
            } else if let Some(ref scope) = q.scope {
                scope.clone()
            } else if q.is_inbox {
                "Personal".to_string()
            } else {
                "Personal".to_string()
            } ;
            
            grouped.entry(cat).or_default().push(q.clone());
        }

        // 2. Sort categories (Inbox/Personal first, alphabetical, then Archive/Shared at the bottom)
        let mut category_names: Vec<String> = grouped.keys().cloned().collect();
        category_names.sort_by(|a, b| {
            // Priority ordering
            let a_is_personal = a == "Personal" || a == "Inbox";
            let b_is_personal = b == "Personal" || b == "Inbox";
            
            if a_is_personal && !b_is_personal { return std::cmp::Ordering::Less; }
            if !a_is_personal && b_is_personal { return std::cmp::Ordering::Greater; }
            
            let a_is_archive = a.to_lowercase().contains("archive");
            let b_is_archive = b.to_lowercase().contains("archive");
            
            if a_is_archive && !b_is_archive { return std::cmp::Ordering::Greater; }
            if !a_is_archive && b_is_archive { return std::cmp::Ordering::Less; }

            a.cmp(b)
        });

        // 3. Build the flat list of nav entries
        for cat in category_names {
            let queues = grouped.get(&cat).unwrap();
            let expanded = self.expanded_categories.contains(&cat);
            
            entries.push(NavEntry::Category { 
                name: cat.clone(), 
                expanded 
            });

            if expanded {
                for q in queues {
                    entries.push(NavEntry::Queue(q.clone()));
                }
            }
        }
        entries
    }

    pub fn selected_queue(&self) -> Option<Queue> {
        let entries = self.get_nav_entries();
        match entries.get(self.selected_nav_index) {
            Some(NavEntry::Queue(q)) => Some(q.clone()),
            _ => None,
        }
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
