mod actions;
mod app;
mod db;
mod config;
mod keys;
mod models;
mod sync;
mod ui;
mod wizard;

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
use crate::app::{App, NavEntry, Pane};
use gqueues_api_rs::GqueuesClient;
use crate::keys::KeyHandler;
use crate::sync::{SyncCommand, SyncEngine, SyncEvent};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let _ = simplelog::WriteLogger::init(
        simplelog::LevelFilter::Debug,
        simplelog::Config::default(),
        std::fs::File::create("gqt.log")?,
    );
    log::info!("Starting Gqueues TUI");

    // Load config or run wizard
    let settings = match config::load_config()? {
        Some(s) => s,
        None => {
            let default_db_path = db::Database::get_default_db_path()?;
            wizard::run(default_db_path).await?
        }
    };

    let db_path = settings
        .database_path
        .clone()
        .unwrap_or_else(|| db::Database::get_default_db_path().expect("Could not determine default DB path"));

    // Initialize database
    let db = db::Database::new(db_path.clone())?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initialize client for App
    let endpoint = settings
        .gqueues
        .api_endpoint
        .clone()
        .unwrap_or_else(|| "https://api.gqueues.com".into());
    let token = settings.gqueues.access_token.clone().unwrap_or_default();
    let client = std::sync::Arc::new(GqueuesClient::new(endpoint, token));

    // Create app state
    let config_path = config::get_config_path("config.toml")?;
    let mut app = App::new(
        (*client).clone(),
        db,
        config_path,
        db_path,
        settings.keybindings.clone(),
    );

    // Setup Key Handler
    let mut key_handler = KeyHandler::new(&settings.keybindings);

    // Setup Sync Engine (only if token is available)
    let (cmd_tx, _cmd_rx_sink) = mpsc::channel(32);
    let (sync_tx, sync_rx) = mpsc::channel(32);

    if settings.gqueues.access_token.is_some() {
        let (tx, rx) = mpsc::channel(32);
        let mut sync_engine = SyncEngine::new(
            client.clone(),
            app.db.clone(),
            app.active_queue_key.clone(),
            sync_tx,
            rx,
        );
        tokio::spawn(async move {
            sync_engine.run().await;
        });

        run_main_loop(&mut terminal, &mut app, &mut key_handler, sync_rx, tx).await?;
    } else {
        app.status = "Offline Mode".into();
        run_main_loop(&mut terminal, &mut app, &mut key_handler, sync_rx, cmd_tx).await?;
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
    terminal.show_cursor()?;

    Ok(())
}

async fn run_main_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    key_handler: &mut KeyHandler,
    mut sync_rx: mpsc::Receiver<SyncEvent>,
    cmd_tx: mpsc::Sender<SyncCommand>,
) -> Result<()> {
    // Load initial state from DB
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
        terminal.draw(|f| ui::render(f, app))?;

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

        if event::poll(std::time::Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && let Some(action) = key_handler.handle_key(key)
        {
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
                        let new_task = gqueues_api_rs::models::Task {
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
    Ok(())
}
