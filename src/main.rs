mod actions;
mod app;
mod config;
mod db;
mod keys;
mod models;
mod sync;
mod ui;
mod wizard;
mod commands;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use crate::app::{App, InputMode, NavEntry, Pane};
use crate::config::{load_config, Settings};
use crate::db::Database;
use crate::keys::KeyHandler;
use crate::sync::{SyncCommand, SyncEngine, SyncEvent};
use crate::actions::Action;
use std::sync::Arc;
use tokio::sync::mpsc;
use clap::Parser;
use crate::commands::Cli;
use std::path::PathBuf;

fn save_new_task(
    app: &mut App, 
    title: String, 
    parent_key: Option<String>, 
    local_order: Option<f64>
) -> anyhow::Result<()> {
    if let Some(queue) = app.selected_queue() {
        let now_str = chrono::Utc::now().to_rfc3339();
        let new_task = gqueues_api_rs::models::Task {
            key: "".into(),
            title: Some(title),
            notes: None,
            completed: false,
            queue_key: Some(queue.key),
            parent_key,
            subitems: None,
            tags: None,
            assignments: None,
            creation_date: Some(gqueues_api_rs::models::DateInfo {
                text: None,
                raw: now_str,
            }),
            due_date: None,
            repeats: serde_json::Value::Bool(false),
            section_key: None,
            attachments: None,
            crossed: false,
            num_comments: Some(0),
            has_subitems: false,
            position: None,
            access: Some("user".into()),
            add_comments: true,
            local_order,
        };
        let db = app.db.lock().unwrap();
        db.add_task_local(new_task, false)?;
        app.status = "✅ Task created".into();
    }
    app.reload_tasks();
    Ok(())
}

fn init_app() -> Result<(Settings, PathBuf, Database, gqueues_api_rs::GqueuesClient)> {
    // Initialize logging
    let _ = simplelog::WriteLogger::init(
        simplelog::LevelFilter::Debug,
        simplelog::Config::default(),
        std::fs::File::create("gqt.log")?,
    );

    // Load config
    let settings = load_config()?.unwrap_or_else(|| {
        panic!("Config not found. Please run the setup wizard first.");
    });

    // Initialize database
    let db_path = Database::get_default_db_path()?;
    let db = Database::new(db_path.clone())?;

    // Initialize API client
    let client = gqueues_api_rs::GqueuesClient::new(
        settings.gqueues.api_endpoint.clone().unwrap_or_else(|| "https://api.gqueues.com/beta".to_string()),
        settings.gqueues.access_token.clone().unwrap_or_default(),
    );

    Ok((settings, db_path, db, client))
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let (settings, db_path, db, client) = match init_app() {
        Ok(vals) => vals,
        Err(e) => {
            eprintln!("❌ Initialization error: {}", e);
            std::process::exit(1);
        }
    };

    // Handle CLI commands
    if let Some(title) = cli.input {
        return commands::add::handle_add(&db, title).await;
    }

    if let Some(cmd) = cli.command {
        match cmd {
            commands::Commands::Add { title } => {
                return commands::add::handle_add(&db, title).await;
            }
        }
    }

    // Launch TUI if no CLI command
    run_tui(settings, db_path, db, client).await
}

async fn run_tui(
    settings: Settings, 
    db_path: PathBuf, 
    db: Database, 
    client: gqueues_api_rs::GqueuesClient
) -> Result<()> {
    // Initialize app state
    let mut app = App::new(
        client.clone(),
        db,
        crate::config::get_config_path("config.toml")?,
        db_path,
        settings.keybindings.clone(),
    );

    // Initialize Sync Engine
    let (sync_tx, mut sync_rx) = mpsc::channel(100);
    let (cmd_tx, cmd_rx) = mpsc::channel(100);
    let active_queue_key = app.active_queue_key.clone();
    let db_shared = app.db.clone();

    let mut engine = SyncEngine::new(
        Arc::new(client),
        db_shared,
        active_queue_key,
        sync_tx,
        cmd_rx,
    );

    tokio::spawn(async move {
        engine.run().await;
    });

    // Initialize TUI
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let mut key_handler = KeyHandler::new(&settings.keybindings);

    // Initial data load
    {
        let db = app.db.lock().unwrap();
        if let Ok(queues) = db.get_queues() {
            app.queues = queues;
        }

        let active_key = {
            let active = app.active_queue_key.lock().unwrap();
            active.clone()
        };

        let key_to_load = active_key.or_else(|| {
            app.queues
                .iter()
                .find(|q| q.is_inbox)
                .map(|q| q.key.clone())
        });

        if let Some(key) = key_to_load
            && let Ok(tasks) = db.get_tasks(&key)
        {
            app.tasks = tasks;
            // Update active key if we auto-selected inbox
            let mut active = app.active_queue_key.lock().unwrap();
            if active.is_none() {
                *active = Some(key);
            }
        }
    }

    while app.running {
        terminal.draw(|f| ui::render(f, &mut app))?;

        // Check for sync events
        while let Ok(event) = sync_rx.try_recv() {
            match event {
                SyncEvent::InProgress { message } => {
                    app.status = message;
                }
                SyncEvent::Complete { unfetched, total } => {
                    app.last_synced = Some(chrono::Local::now());
                    if unfetched > 0 {
                        app.status =
                            format!("⏳ Syncing: {}/{} queues remaining...", unfetched, total);
                    } else {
                        app.status = "✅ Sync successful".into();
                    }
                    // Reload data from DB
                    let db = app.db.lock().unwrap();
                    if let Ok(queues) = db.get_queues() {
                        app.queues = queues;
                    }

                    let active_key = {
                        let active = app.active_queue_key.lock().unwrap();
                        active.clone()
                    };

                    if let Some(key) = active_key
                        && let Ok(tasks) = db.get_tasks(&key)
                    {
                        app.tasks = tasks;
                    }
                }
                SyncEvent::Error(e) => {
                    app.status = format!("❌ {}", e);
                    // Still reload data because some push operations might have succeeded
                    let db = app.db.lock().unwrap();
                    if let Ok(queues) = db.get_queues() {
                        app.queues = queues;
                    }

                    let active_key = {
                        let active = app.active_queue_key.lock().unwrap();
                        active.clone()
                    };

                    if let Some(key) = active_key
                        && let Ok(tasks) = db.get_tasks(&key)
                    {
                        app.tasks = tasks;
                    }
                }
            }
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            let event = event::read()?;
            if let Event::Key(key) = event {
                // 1. Handle Input Mode (Text Entry)
                if let InputMode::CreatingTask { title, parent_key, target_index, local_order } = app.input_mode.clone() {
                    match key.code {
                        KeyCode::Enter => {
                            if !title.trim().is_empty() {
                                save_new_task(&mut app, title, parent_key, Some(local_order))?;
                            }
                            app.input_mode = InputMode::Normal;
                        }
                        KeyCode::Tab => {
                            if !title.trim().is_empty() {
                                save_new_task(&mut app, title, parent_key.clone(), Some(local_order))?;
                                // Start another one below
                                let next_local_order = local_order + 1.0;
                                app.input_mode = InputMode::CreatingTask {
                                    title: String::new(),
                                    parent_key,
                                    target_index: target_index + 1,
                                    local_order: next_local_order,
                                };
                                app.task_state.select(Some(target_index + 1));
                            }
                        }
                        KeyCode::Esc => {
                            app.input_mode = InputMode::Normal;
                        }
                        KeyCode::Char(c) => {
                            let mut new_title = title;
                            new_title.push(c);
                            app.input_mode = InputMode::CreatingTask {
                                title: new_title,
                                parent_key,
                                target_index,
                                local_order,
                            };
                        }
                        KeyCode::Backspace => {
                            let mut new_title = title;
                            new_title.pop();
                            app.input_mode = InputMode::CreatingTask {
                                title: new_title,
                                parent_key,
                                target_index,
                                local_order,
                            };
                        }
                        _ => {}
                    }
                    continue; // Skip normal key handler
                }

                // 2. Handle Normal Mode (Actions)
                if let Some(action) = key_handler.handle_key(key) {
                    match action {
                        Action::Quit => app.running = false,
                        Action::Sync => {
                            app.status = "⏳ Manual sync triggered...".into();
                            let _ = cmd_tx.send(SyncCommand::ForceSync).await;
                        }
                        Action::NextPane => app.next_pane(),
                        Action::PrevPane => app.previous_pane(),
                        Action::InsertTaskBelow | Action::InsertTaskAbove | Action::AddTaskBottom | Action::AddTaskTop => {
                            if let Some(_queue) = app.selected_queue() {
                                let visible_tasks = app.get_visible_tasks();
                                let selected = app.task_state.selected().unwrap_or(0);
                                
                                // Calculate local_order
                                let mut local_order = 0.0;
                                if !app.tasks.is_empty() {
                                    match action {
                                        Action::AddTaskTop => {
                                            let min_order = app.tasks.iter()
                                                .filter_map(|t| t.local_order)
                                                .fold(f64::INFINITY, f64::min);
                                            local_order = if min_order == f64::INFINITY { 0.0 } else { min_order - 1.0 };
                                        }
                                        Action::AddTaskBottom => {
                                            let max_order = app.tasks.iter()
                                                .filter_map(|t| t.local_order)
                                                .fold(f64::NEG_INFINITY, f64::max);
                                            local_order = if max_order == f64::NEG_INFINITY { 100.0 } else { max_order + 1.0 };
                                        }
                                        Action::InsertTaskBelow => {
                                            if let Some((task, _)) = visible_tasks.get(selected) {
                                                let current_order = task.local_order.unwrap_or(0.0);
                                                let next_order = visible_tasks.get(selected + 1)
                                                    .and_then(|(t, _)| t.local_order);
                                                local_order = match next_order {
                                                    Some(next) => (current_order + next) / 2.0,
                                                    None => current_order + 1.0,
                                                };
                                            }
                                        }
                                        Action::InsertTaskAbove => {
                                            if let Some((task, _)) = visible_tasks.get(selected) {
                                                let current_order = task.local_order.unwrap_or(0.0);
                                                let prev_order = if selected > 0 {
                                                    visible_tasks.get(selected - 1).and_then(|(t, _)| t.local_order)
                                                } else {
                                                    None
                                                };
                                                local_order = match prev_order {
                                                    Some(prev) => (current_order + prev) / 2.0,
                                                    None => current_order - 1.0,
                                                };
                                            }
                                        }
                                        _ => {}
                                    }
                                }

                                let (parent_key, target_index) = match action {
                                    Action::InsertTaskBelow => {
                                        if let Some((task, _)) = visible_tasks.get(selected) {
                                            (task.parent_key.clone(), selected + 1)
                                        } else {
                                            (None, visible_tasks.len())
                                        }
                                    }
                                    Action::InsertTaskAbove => {
                                        if let Some((task, _)) = visible_tasks.get(selected) {
                                            (task.parent_key.clone(), selected)
                                        } else {
                                            (None, 0)
                                        }
                                    }
                                    Action::AddTaskBottom => (None, visible_tasks.len()),
                                    Action::AddTaskTop => (None, 0),
                                    _ => unreachable!(),
                                };

                                app.active_pane = Pane::Tasks;
                                app.input_mode = InputMode::CreatingTask {
                                    title: String::new(),
                                    parent_key,
                                    target_index,
                                    local_order,
                                };
                                app.task_state.select(Some(target_index));
                            }
                        }
                        Action::Select => {
                            if app.active_pane == Pane::Queues {
                                let nav_entries = app.get_nav_entries();
                                let selected = app.nav_state.selected().unwrap_or(0);
                                if let Some(entry) = nav_entries.get(selected) {
                                    match entry {
                                        NavEntry::Category { name, .. } => {
                                            if app.expanded_categories.contains(name) {
                                                app.expanded_categories.remove(name);
                                            } else {
                                                app.expanded_categories.insert(name.clone());
                                            }
                                        }
                                        NavEntry::Queue(queue) => {
                                            let queue_key = queue.key.clone();
                                            {
                                                let mut active = app.active_queue_key.lock().unwrap();
                                                *active = Some(queue_key.clone());
                                            }
                                            app.status = format!("⏳ Loading {}...", queue.name);
                                            let db = app.db.lock().unwrap();
                                            match db.get_tasks(&queue_key) {
                                                Ok(tasks) => {
                                                    app.tasks = tasks;
                                                    app.task_state.select(Some(0));
                                                    app.detail_scroll = 0;
                                                    app.status = format!("✅ Loaded {}", queue.name);
                                                }
                                                Err(e) => app.status = format!("❌ Local DB error: {}", e),
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Action::QuickAdd => {
                            if let Some(queue) = app.selected_queue() {
                                let queue_key = queue.key.clone();
                                app.status = format!("⏳ Creating task in {}...", queue.name);
                                let now_str = chrono::Utc::now().to_rfc3339();
                                let new_task = gqueues_api_rs::models::Task {
                                    key: "".into(),
                                    title: Some("New Local Task".into()),
                                    notes: Some("Created offline".into()),
                                    completed: false,
                                    queue_key: Some(queue_key),
                                    parent_key: None,
                                    subitems: None,
                                    tags: None,
                                    assignments: None,
                                    creation_date: Some(gqueues_api_rs::models::DateInfo {
                                        text: None,
                                        raw: now_str,
                                    }),
                                    due_date: None,
                                    repeats: serde_json::Value::Bool(false),
                                    section_key: None,
                                    attachments: None,
                                    crossed: false,
                                    num_comments: Some(0),
                                    has_subitems: false,
                                    position: None,
                                    access: Some("user".into()),
                                    add_comments: true,
                                    local_order: Some(1000.0), // Some high value for bottom
                                };
                                let db = app.db.lock().unwrap();
                                match db.add_task_local(new_task, false) {
                                    Ok(task) => {
                                        app.tasks.push(task);
                                        let visible_tasks = app.get_visible_tasks();
                                        app.task_state.select(Some(visible_tasks.len().saturating_sub(1)));
                                        app.detail_scroll = 0;
                                        app.status = "✅ Local task created".into();
                                    }
                                    Err(e) => app.status = format!("❌ Failed to create local task: {}", e),
                                }
                            }
                        }
                        Action::ToggleExpand | Action::ToggleSubtasks => {
                            if app.active_pane == Pane::Tasks {
                                let visible_tasks = app.get_visible_tasks();
                                let selected = app.task_state.selected().unwrap_or(0);
                                if let Some((task, _)) = visible_tasks.get(selected) {
                                    if app.expanded_tasks.contains(&task.key) {
                                        app.expanded_tasks.remove(&task.key);
                                    } else {
                                        app.expanded_tasks.insert(task.key.clone());
                                    }
                                }
                            } else if app.active_pane == Pane::Queues {
                                let nav_entries = app.get_nav_entries();
                                let selected = app.nav_state.selected().unwrap_or(0);
                                if let Some(NavEntry::Category { name, .. }) = nav_entries.get(selected) {
                                    if app.expanded_categories.contains(name) {
                                        app.expanded_categories.remove(name);
                                    } else {
                                        app.expanded_categories.insert(name.clone());
                                    }
                                }
                            }
                        }
                        Action::MoveDown => match app.active_pane {
                            Pane::Queues => {
                                let nav_entries = app.get_nav_entries();
                                let current = app.nav_state.selected().unwrap_or(0);
                                if current < nav_entries.len().saturating_sub(1) {
                                    app.nav_state.select(Some(current + 1));
                                }
                            }
                            Pane::Tasks => {
                                let visible_tasks = app.get_visible_tasks();
                                let current = app.task_state.selected().unwrap_or(0);
                                if current < visible_tasks.len().saturating_sub(1) {
                                    app.task_state.select(Some(current + 1));
                                    app.detail_scroll = 0;
                                }
                            }
                            Pane::Details => {
                                app.detail_scroll = app.detail_scroll.saturating_add(1);
                            }
                        },
                        Action::MoveUp => match app.active_pane {
                            Pane::Queues => {
                                let current = app.nav_state.selected().unwrap_or(0);
                                if current > 0 {
                                    app.nav_state.select(Some(current - 1));
                                }
                            }
                            Pane::Tasks => {
                                let current = app.task_state.selected().unwrap_or(0);
                                if current > 0 {
                                    app.task_state.select(Some(current - 1));
                                    app.detail_scroll = 0;
                                }
                            }
                            Pane::Details => {
                                app.detail_scroll = app.detail_scroll.saturating_sub(1);
                            }
                        },
                        Action::GoToInbox => {
                            if let Some(inbox) = app.queues.iter().find(|q| q.is_inbox) {
                                let inbox_key = inbox.key.clone();
                                let mut active = app.active_queue_key.lock().unwrap();
                                *active = Some(inbox_key.clone());
                                app.status = "⏳ Jumping to Inbox...".into();
                                let db = app.db.lock().unwrap();
                                if let Ok(tasks) = db.get_tasks(&inbox_key) {
                                    app.tasks = tasks;
                                    app.task_state.select(Some(0));
                                    app.active_pane = Pane::Tasks;
                                }
                            }
                        }
                        Action::Help => {
                            app.show_help = true;
                        }
                        Action::Cancel => {
                            if app.show_help {
                                app.show_help = false;
                            } else {
                                app.status = "Ready".into();
                            }
                        }
                        _ => {
                            app.status = format!("Action {:?} not yet implemented", action);
                        }
                    }
                }
            }
        }
    }

    // Clean up TUI
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
    )?;
    terminal.show_cursor()?;

    Ok(())
}
