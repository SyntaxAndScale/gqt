mod actions;
mod app;
mod db;
mod gqueues;
mod config;
mod keys;
mod models;
mod sync;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use tokio::sync::mpsc;

use crate::actions::Action;
use crate::app::{App, Pane, NavEntry};
use crate::gqueues::GqueuesClient;
use crate::keys::KeyHandler;
use crate::sync::{SyncEngine, SyncEvent, SyncCommand};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let _ = simplelog::WriteLogger::init(
        simplelog::LevelFilter::Debug,
        simplelog::Config::default(),
        std::fs::File::create("gqt.log")?,
    );
    log::info!("Starting Gqueues TUI");

    // Load config
    let settings = config::load_config()?;
    let client = std::sync::Arc::new(GqueuesClient::new(settings.gqueues.api_endpoint, settings.gqueues.access_token));
    
    // Initialize database
    let db = db::Database::new()?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new((*client).clone(), db);
    
    // Setup Key Handler
    let mut key_handler = KeyHandler::new(&settings.keybindings);

    // Setup Sync Engine
    let (sync_tx, mut sync_rx) = mpsc::channel(32);
    let (cmd_tx, cmd_rx) = mpsc::channel(32);
    let mut sync_engine = SyncEngine::new(client, app.db.clone(), app.active_queue_key.clone(), sync_tx, cmd_rx);
    tokio::spawn(async move {
        sync_engine.run().await;
    });

    // Load initial state from DB
    {
        let db = app.db.lock().unwrap();
        if let Ok(queues) = db.get_queues() {
            app.queues = queues;
        }
        let selected = app.nav_state.selected().unwrap_or(0);
        if let Some(NavEntry::Queue(queue)) = app.get_nav_entries().get(selected) {
            if let Ok(tasks) = db.get_tasks(&queue.key) {
                app.tasks = tasks;
            }
        }
    }

    // Main loop
    while app.running {
        terminal.draw(|f| ui::render(f, &mut app))?;

        // Check for sync events
        while let Ok(event) = sync_rx.try_recv() {
            match event {
                SyncEvent::Complete { unfetched, total } => {
                    if unfetched > 0 {
                        app.status = format!("⏳ Syncing: {}/{} queues remaining...", unfetched, total);
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

                    if let Some(key) = active_key {
                        if let Ok(tasks) = db.get_tasks(&key) {
                            app.tasks = tasks;
                        }
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

                    if let Some(key) = active_key {
                        if let Ok(tasks) = db.get_tasks(&key) {
                            app.tasks = tasks;
                        }
                    }
                }
            }
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if let Some(action) = key_handler.handle_key(key) {
                    match action {
                        Action::Quit => app.running = false,
                        Action::Sync => {
                            app.status = "⏳ Manual sync triggered...".into();
                            let _ = cmd_tx.send(SyncCommand::ForceSync).await;
                        }


                        Action::NextPane => app.next_pane(),
                        Action::PrevPane => app.previous_pane(),
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
                                            // Set active queue for sync engine
                                            {
                                                let mut active = app.active_queue_key.lock().unwrap();
                                                *active = Some(queue_key.clone());
                                            }
                                            app.status = format!("⏳ Loading {}...", queue.name);
                                            
                                            {
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
                        }
                        Action::QuickAdd => {
                            if let Some(queue) = app.selected_queue() {
                                let queue_key = queue.key.clone();
                                app.status = format!("⏳ Creating task in {}...", queue.name);
                                // Mock task creation for now (could be an input field later)
                                let new_task = crate::gqueues::models::Task {
                                    key: "".into(),
                                    title: "New Local Task".into(),
                                    notes: Some("Created offline".into()),
                                    completed: false,
                                    queue_key: Some(queue_key),
                                    parent_key: None,
                                    subitems: None,
                                    tags: None,
                                    assignments: None,
                                    creation_date: None,
                                    due_date: None,
                                    repeats: serde_json::Value::Bool(false),
                                };
                                let db = app.db.lock().unwrap();
                                match db.add_task_local(new_task) {
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
                                // Also support category toggling per todo list
                                let nav_entries = app.get_nav_entries();
                                let selected = app.nav_state.selected().unwrap_or(0);
                                if let Some(entry) = nav_entries.get(selected) {
                                    if let NavEntry::Category { name, .. } = entry {
                                        if app.expanded_categories.contains(name) {
                                            app.expanded_categories.remove(name);
                                        } else {
                                            app.expanded_categories.insert(name.clone());
                                        }
                                    }
                                }
                            }
                        }
                        Action::MoveDown => {
                            match app.active_pane {
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
                                        app.detail_scroll = 0; // Reset scroll on task change
                                    }
                                }
                                Pane::Details => {
                                    app.detail_scroll = app.detail_scroll.saturating_add(1);
                                }
                            }
                        }
                        Action::MoveUp => {
                            match app.active_pane {
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
                                        app.detail_scroll = 0; // Reset scroll on task change
                                    }
                                }
                                Pane::Details => {
                                    app.detail_scroll = app.detail_scroll.saturating_sub(1);
                                }
                            }
                        }
                        Action::GoToInbox => {
                            // Find inbox in queues
                            if let Some(inbox) = app.queues.iter().find(|q| q.is_inbox) {
                                let inbox_key = inbox.key.clone();
                                let mut active = app.active_queue_key.lock().unwrap();
                                *active = Some(inbox_key.clone());
                                // We don't change selection index for now, but we could find it in nav_entries
                                app.status = "⏳ Jumping to Inbox...".into();
                                let db = app.db.lock().unwrap();
                                if let Ok(tasks) = db.get_tasks(&inbox_key) {
                                    app.tasks = tasks;
                                    app.task_state.select(Some(0));
                                    app.active_pane = Pane::Tasks;
                                }
                            }
                        }
                        Action::Cancel => {
                            // Clear sequence or search
                            app.status = "Ready".into();
                        }
                        _ => {
                            app.status = format!("Action {:?} not yet implemented", action);
                        }
                    }
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
    )?;
    terminal.show_cursor()?;

    Ok(())
}
