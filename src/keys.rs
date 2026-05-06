use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;
use crate::actions::Action;
use crate::config::KeybindingsConfig;

pub struct KeyHandler {
    bindings: HashMap<String, Action>,
    current_sequence: Vec<String>,
}

impl KeyHandler {
    pub fn new(config: &KeybindingsConfig) -> Self {
        let mut bindings = HashMap::new();
        for (action_str, key_str) in &config.bindings {
            let action = match action_str.as_str() {
                "quit" => Action::Quit,
                "sync" | "sync_alt" => Action::Sync,
                "next_pane" => Action::NextPane,
                "prev_pane" => Action::PrevPane,
                "search" => Action::Search,
                "help" => Action::Help,
                "cancel" => Action::Cancel,
                "quick_add" => Action::QuickAdd,
                "insert_task_below" => Action::InsertTaskBelow,
                "insert_task_above" => Action::InsertTaskAbove,
                "add_task_bottom" => Action::AddTaskBottom,
                "add_task_top" => Action::AddTaskTop,
                "add_subtask" => Action::AddSubtask,
                "edit_description" => Action::EditDescription,
                "edit_notes" => Action::EditNotes,
                "toggle_notes" => Action::ToggleNotes,
                "add_tag" => Action::AddTag,
                "toggle_subtasks" => Action::ToggleSubtasks,
                "edit_date" => Action::EditDate,
                "assign_task" => Action::AssignTask,
                "write_comment" => Action::WriteComment,
                "toggle_completed" => Action::ToggleCompleted,
                "complete_and_archive" => Action::CompleteAndArchive,
                "delete_task" => Action::DeleteTask,
                "snooze_task" => Action::SnoozeTask,
                "get_task_link" => Action::GetTaskLink,
                "view_comments" => Action::ViewComments,
                "view_activity" => Action::ViewActivity,
                "go_to_task_overview" => Action::GoToTaskOverview,
                "move_task_up" => Action::MoveTaskUp,
                "move_task_down" => Action::MoveTaskDown,
                "indent_task" => Action::IndentTask,
                "unindent_task" => Action::UnindentTask,
                "move_to_queue" => Action::MoveToQueue,
                "copy_to_queue" => Action::CopyToQueue,
                "make_new_queue" => Action::MakeNewQueue,
                "make_new_category" => Action::MakeNewCategory,
                "toggle_my_queues" => Action::ToggleMyQueues,
                "toggle_shared_queues" => Action::ToggleSharedQueues,
                "share_queue" => Action::ShareQueue,
                "view_queue_details" => Action::ViewQueueDetails,
                "view_queue_activity" => Action::ViewQueueActivity,
                "print_queue" => Action::PrintQueue,
                "toggle_fullscreen" => Action::ToggleFullscreen,
                "go_to_inbox" => Action::GoToInbox,
                "go_to_trash" => Action::GoToTrash,
                "go_to_default_queue" => Action::GoToDefaultQueue,
                "go_to_queue" => Action::GoToQueue,
                "go_to_active_tasks" => Action::GoToActiveTasks,
                "go_to_archived_tasks" => Action::GoToArchivedTasks,
                "go_back" => Action::GoBack,
                "go_next" => Action::GoNext,
                "move_up" | "move_up_alt" => Action::MoveUp,
                "move_down" | "move_down_alt" => Action::MoveDown,
                "select" => Action::Select,
                "toggle_expand" => Action::ToggleExpand,
                "toggle_all_notes" => Action::ToggleAllNotes,
                "toggle_all_tags" => Action::ToggleAllTags,
                "toggle_all_subtasks" => Action::ToggleAllSubtasks,
                "toggle_all_assignments" => Action::ToggleAllAssignments,
                "toggle_all_attachments" => Action::ToggleAllAttachments,
                "toggle_all_created_dates" => Action::ToggleAllCreatedDates,
                "toggle_everything" => Action::ToggleEverything,
                _ => continue,
            };
            bindings.insert(key_str.to_lowercase(), action);
        }

        Self {
            bindings,
            current_sequence: Vec::new(),
        }
    }

    pub fn handle_key(&mut self, event: KeyEvent) -> Option<Action> {
        let key_str = self.key_event_to_string(event);
        self.current_sequence.push(key_str);

        let sequence_str = self.current_sequence.join(",");
        
        // Check for exact match
        if let Some(action) = self.bindings.get(&sequence_str) {
            self.current_sequence.clear();
            return Some(*action);
        }

        // Check if this is a prefix of any binding
        let is_prefix = self.bindings.keys().any(|k| k.starts_with(&format!("{},", sequence_str)));
        
        if !is_prefix {
            // Not a match and not a prefix, clear sequence
            // But if the LAST key alone matches something, return that
            let last_key = self.current_sequence.last().unwrap().clone();
            self.current_sequence.clear();
            if let Some(action) = self.bindings.get(&last_key) {
                return Some(*action);
            }
        }

        None
    }

    fn key_event_to_string(&self, event: KeyEvent) -> String {
        let mut s = String::new();
        if event.modifiers.contains(KeyModifiers::CONTROL) {
            s.push_str("ctrl-");
        }
        if event.modifiers.contains(KeyModifiers::ALT) {
            s.push_str("alt-");
        }
        if event.modifiers.contains(KeyModifiers::SHIFT) && !matches!(event.code, KeyCode::Char(_)) {
            s.push_str("shift-");
        }

        match event.code {
            KeyCode::Char(c) => {
                if event.modifiers.contains(KeyModifiers::SHIFT) {
                    s.push(c.to_ascii_uppercase());
                } else {
                    s.push(c);
                }
            }
            KeyCode::Backspace => s.push_str("backspace"),
            KeyCode::Enter => s.push_str("enter"),
            KeyCode::Left => s.push_str("left"),
            KeyCode::Right => s.push_str("right"),
            KeyCode::Up => s.push_str("up"),
            KeyCode::Down => s.push_str("down"),
            KeyCode::Home => s.push_str("home"),
            KeyCode::End => s.push_str("end"),
            KeyCode::PageUp => s.push_str("pageup"),
            KeyCode::PageDown => s.push_str("pagedown"),
            KeyCode::Tab => s.push_str("tab"),
            KeyCode::BackTab => s.push_str("shift-tab"),
            KeyCode::Delete => s.push_str("delete"),
            KeyCode::Insert => s.push_str("insert"),
            KeyCode::F(n) => s.push_str(&format!("f{}", n)),
            KeyCode::Esc => s.push_str("esc"),
            _ => s.push_str("unknown"),
        }
        s.to_lowercase()
    }
}
