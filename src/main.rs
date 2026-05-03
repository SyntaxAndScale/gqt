mod app;
mod db;
mod gqueues;
mod config;
mod models;
mod sync;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use tokio::sync::mpsc;

use crate::app::{App, Pane, NavEntry};
use crate::gqueues::GqueuesClient;
use crate::sync::{SyncEngine, SyncEvent};

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
    let gq_config = config::load_config()?;
    let client = std::sync::Arc::new(GqueuesClient::new(gq_config.api_endpoint, gq_config.access_token));
    
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
    
    // Setup Sync Engine
    let (sync_tx, mut sync_rx) = mpsc::channel(32);
    let mut sync_engine = SyncEngine::new(client, app.db.clone(), app.active_queue_key.clone(), sync_tx);
    tokio::spawn(async move {
        sync_engine.run().await;
    });

    // Load initial state from DB
    {
        let db = app.db.lock().unwrap();
        if let Ok(queues) = db.get_queues() {
            app.queues = queues;
        }
        if let Some(queue) = app.queues.first() {
            if let Ok(tasks) = db.get_tasks(&queue.key) {
                app.tasks = tasks;
            }
        }
    }

    // Main loop
    while app.running {
        terminal.draw(|f| ui::render(f, &app))?;

        // Check for sync events
        while let Ok(event) = sync_rx.try_recv() {
            match event {
                SyncEvent::Complete => {
                    app.status = "✅ Sync successful".into();
                    // Reload data from DB
                    let db = app.db.lock().unwrap();
                    if let Ok(queues) = db.get_queues() {
                        app.queues = queues;
                    }
                    if let Some(NavEntry::Queue(queue)) = app.get_nav_entries().get(app.selected_nav_index) {
                        if let Ok(tasks) = db.get_tasks(&queue.key) {
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
                    if let Some(NavEntry::Queue(queue)) = app.get_nav_entries().get(app.selected_nav_index) {
                        if let Ok(tasks) = db.get_tasks(&queue.key) {
                            app.tasks = tasks;
                        }
                    }
                }
            }
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => app.running = false,
                    KeyCode::Tab => app.next_pane(),
                    KeyCode::BackTab => app.previous_pane(),
                    KeyCode::Enter => {
                        if app.active_pane == Pane::Queues {
                            let nav_entries = app.get_nav_entries();
                            if let Some(entry) = nav_entries.get(app.selected_nav_index) {
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
                                                    app.selected_task_index = 0;
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
                    KeyCode::Char('a') => {
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
                            };
                            let db = app.db.lock().unwrap();
                            match db.add_task_local(new_task) {
                                Ok(task) => {
                                    app.tasks.push(task);
                                    app.selected_task_index = app.tasks.len() - 1;
                                    app.status = "✅ Local task created".into();
                                }
                                Err(e) => app.status = format!("❌ Failed to create local task: {}", e),
                            }
                        }
                    }
                    KeyCode::Down => {
                        match app.active_pane {
                            Pane::Queues => {
                                let nav_entries = app.get_nav_entries();
                                if app.selected_nav_index < nav_entries.len().saturating_sub(1) {
                                    app.selected_nav_index += 1;
                                }
                            }
                            Pane::Tasks => {
                                if app.selected_task_index < app.tasks.len().saturating_sub(1) {
                                    app.selected_task_index += 1;
                                }
                            }
                            _ => {}
                        }
                    }
                    KeyCode::Up => {
                        match app.active_pane {
                            Pane::Queues => {
                                if app.selected_nav_index > 0 {
                                    app.selected_nav_index -= 1;
                                }
                            }
                            Pane::Tasks => {
                                if app.selected_task_index > 0 {
                                    app.selected_task_index -= 1;
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
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
