use crate::config::KeybindingsConfig;
use crate::db::Database;
use gqueues_api_rs::{GqueuesClient, Queue, Task};
use ratatui::widgets::ListState;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Represents the available panes in the TUI.
#[derive(Debug, PartialEq, Eq)]
pub enum Pane {
    Queues,
    Tasks,
    Details,
}

/// Represents an entry in the navigation sidebar (left pane).
#[derive(Debug, Clone)]
pub enum NavEntry {
    /// A category header that can be expanded/collapsed.
    Category { name: String, expanded: bool },
    /// An actual task queue.
    Queue(Queue),
}

/// The central application state.
pub struct App {
    /// Underscored as it's currently held for ownership but not directly called from App.
    pub _client: Arc<GqueuesClient>,
    /// Thread-safe handle to the local SQLite database.
    pub db: Arc<Mutex<Database>>,
    /// The currently selected queue key, shared with the Sync Engine.
    pub active_queue_key: Arc<Mutex<Option<String>>>,
    /// Tracks which categories are expanded in the sidebar.
    pub expanded_categories: HashSet<String>,
    /// Tracks which tasks are expanded in the task list.
    pub expanded_tasks: HashSet<String>,
    /// Current keyboard configuration.
    pub keybindings: KeybindingsConfig,
    /// All queues available in the database.
    pub queues: Vec<Queue>,
    /// All tasks for the currently selected queue.
    pub tasks: Vec<Task>,
    /// State for the queues list widget.
    pub nav_state: ListState,
    /// State for the tasks list widget.
    pub task_state: ListState,
    /// Scroll offset for the details pane.
    pub detail_scroll: u16,
    /// Whether the help modal is currently visible.
    pub show_help: bool,
    /// The timestamp of the last successful sync cycle.
    pub last_synced: Option<chrono::DateTime<chrono::Local>>,
    /// Path to the configuration file for display.
    pub config_path: PathBuf,
    /// Path to the database file for display.
    pub db_path: PathBuf,
    /// The currently focused pane.
    pub active_pane: Pane,
    /// Controls the main execution loop.
    pub running: bool,
    /// The message displayed in the status bar.
    pub status: String,
}

impl App {
    /// Initializes a new application state.
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
            _client: Arc::new(client),
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
            last_synced: None,
            config_path,
            db_path,
            active_pane: Pane::Queues,
            running: true,
            status: "Ready".into(),
        }
    }

    /// Generates the flat list of navigation entries (categories and queues) based on expansion state.
    pub fn get_nav_entries(&self) -> Vec<NavEntry> {
        let mut entries = Vec::new();
        let mut grouped: std::collections::BTreeMap<String, Vec<Queue>> =
            std::collections::BTreeMap::new();

        for q in &self.queues {
            let cat = if let Some(ref cn) = q.category_name {
                cn.clone()
            } else if let Some(ref tn) = q.team_name {
                tn.clone()
            } else if let Some(ref scope) = q.scope {
                scope.clone()
            } else {
                "Personal".to_string()
            };

            grouped.entry(cat).or_default().push(q.clone());
        }

        let mut category_names: Vec<String> = grouped.keys().cloned().collect();
        category_names.sort_by(|a, b| {
            let a_is_personal = a == "Personal" || a == "Inbox";
            let b_is_personal = b == "Personal" || b == "Inbox";

            if a_is_personal && !b_is_personal {
                return std::cmp::Ordering::Less;
            }
            if !a_is_personal && b_is_personal {
                return std::cmp::Ordering::Greater;
            }

            let a_is_archive = a.to_lowercase().contains("archive");
            let b_is_archive = b.to_lowercase().contains("archive");

            if a_is_archive && !b_is_archive {
                return std::cmp::Ordering::Greater;
            }
            if !a_is_archive && b_is_archive {
                return std::cmp::Ordering::Less;
            }

            a.cmp(b)
        });

        for cat in category_names {
            let queues = grouped.get(&cat).unwrap();
            let expanded = self.expanded_categories.contains(&cat);

            entries.push(NavEntry::Category {
                name: cat.clone(),
                expanded,
            });

            if expanded {
                for q in queues {
                    entries.push(NavEntry::Queue(q.clone()));
                }
            }
        }
        entries
    }

    /// Returns the currently selected queue, if any.
    pub fn selected_queue(&self) -> Option<Queue> {
        let entries = self.get_nav_entries();
        match entries.get(self.nav_state.selected().unwrap_or(0)) {
            Some(NavEntry::Queue(q)) => Some(q.clone()),
            _ => None,
        }
    }

    /// Generates the flat list of visible tasks, respecting hierarchical expansion.
    pub fn get_visible_tasks(&self) -> Vec<(Task, usize)> {
        let mut visible = Vec::new();
        let top_level: Vec<&Task> = self
            .tasks
            .iter()
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
            let subtasks: Vec<&Task> = self
                .tasks
                .iter()
                .filter(|t| t.parent_key.as_ref() == Some(&task.key))
                .collect();

            for sub in subtasks {
                self.flatten_task(sub, depth + 1, visible);
            }
        }
    }

    /// Returns the currently selected task, if any.
    pub fn selected_task(&self) -> Option<Task> {
        let visible = self.get_visible_tasks();
        visible
            .get(self.task_state.selected().unwrap_or(0))
            .map(|(t, _)| t.clone())
    }

    /// Cycles focus to the next pane.
    pub fn next_pane(&mut self) {
        self.active_pane = match self.active_pane {
            Pane::Queues => Pane::Tasks,
            Pane::Tasks => Pane::Details,
            Pane::Details => Pane::Queues,
        };
    }

    /// Cycles focus to the previous pane.
    pub fn previous_pane(&mut self) {
        self.active_pane = match self.active_pane {
            Pane::Queues => Pane::Details,
            Pane::Tasks => Pane::Queues,
            Pane::Details => Pane::Tasks,
        };
    }
}
