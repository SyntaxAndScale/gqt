mod app;
mod db;
mod gqueues;
mod config;
mod models;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use crate::app::{App, Pane};
use crate::gqueues::GqueuesClient;

#[tokio::main]
async fn main() -> Result<()> {
    // Load config
    let gq_config = config::load_config()?;
    let client = GqueuesClient::new(gq_config.api_endpoint, gq_config.access_token);
    
    // Initialize database
    let db = db::Database::new()?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new(client, db);
    
    // Initial fetch of queues
    app.loading = true;
    match app.client.get_queues().await {
        Ok(queues) => {
            let db = app.db.lock().unwrap();
            for q in &queues {
                let _ = db.upsert_queue(q);
            }
        }
        Err(e) => app.error = Some(format!("Failed to fetch queues from API: {}", e)),
    }

    // Load queues from DB
    {
        let db = app.db.lock().unwrap();
        match db.get_queues() {
            Ok(queues) => app.queues = queues,
            Err(e) => app.error = Some(format!("Failed to load queues from DB: {}", e)),
        }
    }
    app.loading = false;

    // Fetch tasks for the first queue if available
    if let Some(queue) = app.queues.first() {
        let queue_key = queue.key.clone();
        app.loading = true;
        match app.client.get_tasks(&queue_key).await {
            Ok(tasks) => {
                let db = app.db.lock().unwrap();
                for t in &tasks {
                    let _ = db.upsert_task(t);
                }
            }
            Err(e) => app.error = Some(format!("Failed to fetch tasks from API: {}", e)),
        }
        
        // Load tasks from DB
        {
            let db = app.db.lock().unwrap();
            match db.get_tasks(&queue_key) {
                Ok(tasks) => app.tasks = tasks,
                Err(e) => app.error = Some(format!("Failed to load tasks from DB: {}", e)),
            }
        }
        app.loading = false;
    }

    // Main loop
    while app.running {
        terminal.draw(|f| ui::render(f, &app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => app.running = false,
                    KeyCode::Tab => app.next_pane(),
                    KeyCode::BackTab => app.previous_pane(),
                    KeyCode::Enter => {
                        if app.active_pane == Pane::Queues {
                            if let Some(queue) = app.selected_queue() {
                                let queue_key = queue.key.clone();
                                app.loading = true;
                                terminal.draw(|f| ui::render(f, &app))?; // Show loading
                                
                                // Try fetching from API
                                match app.client.get_tasks(&queue_key).await {
                                    Ok(tasks) => {
                                        let db = app.db.lock().unwrap();
                                        for t in &tasks {
                                            let _ = db.upsert_task(t);
                                        }
                                    }
                                    Err(e) => app.error = Some(format!("API fetch failed: {}. Showing local data.", e)),
                                }

                                // Always load from DB (which now has latest API data if it succeeded)
                                {
                                    let db = app.db.lock().unwrap();
                                    match db.get_tasks(&queue_key) {
                                        Ok(tasks) => {
                                            app.tasks = tasks;
                                            app.selected_task_index = 0;
                                        }
                                        Err(e) => app.error = Some(format!("Failed to load local tasks: {}", e)),
                                    }
                                }
                                app.loading = false;
                            }
                        }
                    }
                    KeyCode::Char('a') => {
                        if let Some(queue) = app.selected_queue() {
                            // Mock task creation for now (could be an input field later)
                            let new_task = crate::gqueues::models::Task {
                                key: "".into(),
                                title: "New Local Task".into(),
                                notes: Some("Created offline".into()),
                                completed: false,
                                queue_key: Some(queue.key.clone()),
                            };
                            let db = app.db.lock().unwrap();
                            match db.add_task_local(new_task) {
                                Ok(task) => {
                                    app.tasks.push(task);
                                    app.selected_task_index = app.tasks.len() - 1;
                                }
                                Err(e) => app.error = Some(format!("Failed to create local task: {}", e)),
                            }
                        }
                    }
                    KeyCode::Down => {
                        match app.active_pane {
                            Pane::Queues => {
                                if app.selected_queue_index < app.queues.len().saturating_sub(1) {
                                    app.selected_queue_index += 1;
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
                                if app.selected_queue_index > 0 {
                                    app.selected_queue_index -= 1;
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
