use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use anyhow::{anyhow, Result};
use directories::ProjectDirs;

#[derive(Deserialize, Serialize, Clone)]
pub struct GqueuesConfig {
    #[serde(rename = "apiEndpoint")]
    pub api_endpoint: String,
    #[serde(rename = "accessToken")]
    pub access_token: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct KeybindingsConfig {
    pub bindings: HashMap<String, String>,
}

impl Default for KeybindingsConfig {
    fn default() -> Self {
        let mut b = HashMap::new();
        // General
        b.insert("quit".into(), "ctrl-c".into());
        b.insert("sync".into(), "r".into());
        b.insert("sync_alt".into(), "s".into());
        b.insert("next_pane".into(), "tab".into());
        b.insert("prev_pane".into(), "shift-tab".into());
        b.insert("search".into(), "/".into());
        b.insert("help".into(), "?".into());
        b.insert("cancel".into(), "esc".into());

        // Task Addition
        b.insert("quick_add".into(), "q".into());
        b.insert("insert_task_below".into(), "i".into());
        b.insert("insert_task_above".into(), "shift-i".into());
        b.insert("add_task_bottom".into(), "o".into());
        b.insert("add_task_top".into(), "shift-o".into());
        b.insert("add_subtask".into(), "s".into());

        // Task Editing
        b.insert("edit_description".into(), "e".into());
        b.insert("edit_notes".into(), "n".into());
        b.insert("toggle_notes".into(), "shift-n".into());
        b.insert("add_tag".into(), "t".into());
        b.insert("toggle_subtasks".into(), "x".into());
        b.insert("edit_date".into(), "d".into());
        b.insert("assign_task".into(), "a".into());
        b.insert("write_comment".into(), "w".into());
        b.insert("toggle_completed".into(), "c".into());
        b.insert("complete_and_archive".into(), "shift-c".into());
        b.insert("delete_task".into(), "ctrl-shift-d".into());
        b.insert("snooze_task".into(), "z".into());
        b.insert("get_task_link".into(), ":".into());
        b.insert("view_comments".into(), "v,c".into());
        b.insert("view_activity".into(), "v,a".into());
        b.insert("go_to_task_overview".into(), "g,o".into());

        // Task Movement
        b.insert("move_task_up".into(), "shift-k".into());
        b.insert("move_task_down".into(), "shift-j".into());
        b.insert("indent_task".into(), "shift-l".into());
        b.insert("unindent_task".into(), "shift-h".into());
        b.insert("move_to_queue".into(), "m,l".into());
        b.insert("copy_to_queue".into(), "shift-m,l".into());

        // Queue Management
        b.insert("make_new_queue".into(), "m,q".into());
        b.insert("make_new_category".into(), "m,c".into());
        b.insert("toggle_my_queues".into(), ".,q".into());
        b.insert("toggle_shared_queues".into(), ".,s".into());
        b.insert("share_queue".into(), "m,s".into());
        b.insert("view_queue_details".into(), "v,d".into());
        b.insert("view_queue_activity".into(), "v,h".into());
        b.insert("print_queue".into(), "p,q".into());
        b.insert("toggle_fullscreen".into(), "shift-f".into());

        // Queue Navigation
        b.insert("go_to_inbox".into(), "g,i".into());
        b.insert("go_to_trash".into(), "g,h".into());
        b.insert("go_to_default_queue".into(), "g,d".into());
        b.insert("go_to_queue".into(), "g,q".into());
        b.insert("go_to_active_tasks".into(), "g,a".into());
        b.insert("go_to_archived_tasks".into(), "g,r".into());
        b.insert("go_back".into(), "g,b".into());
        b.insert("go_next".into(), "g,n".into());

        // Navigation (Generic)
        b.insert("move_up".into(), "k".into());
        b.insert("move_up_alt".into(), "up".into());
        b.insert("move_down".into(), "j".into());
        b.insert("move_down_alt".into(), "down".into());
        b.insert("select".into(), "enter".into());
        b.insert("toggle_expand".into(), "space".into());

        // Global Toggles
        b.insert("toggle_all_notes".into(), ".,n".into());
        b.insert("toggle_all_tags".into(), ".,t".into());
        b.insert("toggle_all_subtasks".into(), ".,x".into());
        b.insert("toggle_all_assignments".into(), ".,a".into());
        b.insert("toggle_all_attachments".into(), ".,u".into());
        b.insert("toggle_all_created_dates".into(), ".,d".into());
        b.insert("toggle_everything".into(), ".,e".into());

        Self { bindings: b }
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Settings {
    pub gqueues: GqueuesConfig,
    pub keybindings: KeybindingsConfig,
}

pub fn load_config() -> Result<Settings> {
    let toml_path = get_config_path("config.toml")?;
    
    let mut settings = if toml_path.exists() {
        let content = fs::read_to_string(&toml_path)?;
        toml::from_str::<Settings>(&content)?
    } else {
        // Migration / Fallback logic
        let json_path = get_config_path("config.json")?;
        let legacy_local_path = std::path::Path::new(".gemini/settings.local.json");

        let (_legacy_path, content) = if json_path.exists() {
            (Some(json_path), fs::read_to_string(&get_config_path("config.json")?)?)
        } else if legacy_local_path.exists() {
            (Some(legacy_local_path.to_path_buf()), fs::read_to_string(legacy_local_path)?)
        } else {
            return Err(anyhow!("Configuration file not found. Please create one at {:?}", toml_path));
        };

        // Parse legacy JSON
        let gq_config: GqueuesConfig = serde_json::from_str(&content)
            .or_else(|_| {
                // If it's the old full Settings structure
                let s: serde_json::Value = serde_json::from_str(&content)?;
                serde_json::from_value(s["gqueues"].clone())
            })?;
        
        Settings {
            gqueues: gq_config,
            keybindings: KeybindingsConfig::default(),
        }
    };

    // Ensure all default keybindings are present and save back
    let defaults = KeybindingsConfig::default();
    let mut modified = false;
    for (action, key) in defaults.bindings {
        if !settings.keybindings.bindings.contains_key(&action) {
            settings.keybindings.bindings.insert(action, key);
            modified = true;
        }
    }

    if modified || !toml_path.exists() {
        let toml_content = toml::to_string_pretty(&settings)?;
        fs::write(&toml_path, toml_content)?;
        log::info!("Updated configuration at {:?}", toml_path);
        
        // Cleanup old JSON if it was in the XDG path and we just migrated
        let json_path = get_config_path("config.json")?;
        if json_path.exists() {
            let _ = fs::remove_file(json_path);
        }
    }

    Ok(settings)
}

fn get_config_path(filename: &str) -> Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("com", "gqt", "gqt")
        .ok_or_else(|| anyhow!("Could not determine project directories"))?;
    let config_dir = proj_dirs.config_dir();
    if !config_dir.exists() {
        fs::create_dir_all(config_dir)?;
    }
    Ok(config_dir.join(filename))
}
