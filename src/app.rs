use crate::gqueues::{GqueuesClient, Queue, Task};
use crate::db::Database;
use std::sync::{Arc, Mutex};
use std::collections::HashSet;
use ratatui::widgets::ListState;
use std::path::PathBuf;
use crate::config::KeybindingsConfig;

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
    pub expanded_tasks: HashSet<String>,
    pub keybindings: KeybindingsConfig,
    pub queues: Vec<Queue>,
    pub tasks: Vec<Task>,
    pub nav_state: ListState,
    pub task_state: ListState,
    pub detail_scroll: u16,
    pub show_help: bool,
    pub config_path: PathBuf,
    pub db_path: PathBuf,
    pub active_pane: Pane,
    pub running: bool,
    pub status: String,
}

impl App {
    pub fn new(
        client: GqueuesClient, 
        db: Database, 
        config_path: PathBuf, 
        db_path: PathBuf,
        keybindings: KeybindingsConfig,
    ) -> Self {
        let mut nav_state = ListState::default();
        nav_state.select(Some(0));
        let mut task_state = ListState::default();
        task_state.select(Some(0));

        Self {
            client: Arc::new(client),
            db: Arc::new(Mutex::new(db)),
            active_queue_key: Arc::new(Mutex::new(None)),
            expanded_categories: HashSet::new(),
            expanded_tasks: HashSet::new(),
            keybindings,
            queues: Vec::new(),
            tasks: Vec::new(),
            nav_state,
            task_state,
            detail_scroll: 0,
            show_help: false,
            config_path,
            db_path,
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
        match entries.get(self.nav_state.selected().unwrap_or(0)) {
            Some(NavEntry::Queue(q)) => Some(q.clone()),
            _ => None,
        }
    }

    pub fn get_visible_tasks(&self) -> Vec<(Task, usize)> {
        let mut visible = Vec::new();
        let top_level: Vec<&Task> = self.tasks.iter()
            .filter(|t| t.parent_key.is_none())
            .collect();
        
        for task in top_level {
            self.flatten_task(task, 0, &mut visible);
        }
        visible
    }

    fn flatten_task(&self, task: &Task, depth: usize, visible: &mut Vec<(Task, usize)>) {
        visible.push((task.clone(), depth));
        
        if self.expanded_tasks.contains(&task.key) {
            let subtasks: Vec<&Task> = self.tasks.iter()
                .filter(|t| t.parent_key.as_ref() == Some(&task.key))
                .collect();
            
            for sub in subtasks {
                self.flatten_task(sub, depth + 1, visible);
            }
        }
    }

    pub fn selected_task(&self) -> Option<Task> {
        let visible = self.get_visible_tasks();
        visible.get(self.task_state.selected().unwrap_or(0)).map(|(t, _)| t.clone())
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
