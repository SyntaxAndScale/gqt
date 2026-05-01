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
        Ok(queues) => app.queues = queues,
        Err(e) => app.error = Some(format!("Failed to fetch queues: {}", e)),
    }
    app.loading = false;

    // Fetch tasks for the first queue if available
    if let Some(queue) = app.queues.first() {
        app.loading = true;
        match app.client.get_tasks(&queue.key).await {
            Ok(tasks) => app.tasks = tasks,
            Err(e) => app.error = Some(format!("Failed to fetch tasks: {}", e)),
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
                                match app.client.get_tasks(&queue_key).await {
                                    Ok(tasks) => {
                                        app.tasks = tasks;
                                        app.selected_task_index = 0;
                                    }
                                    Err(e) => app.error = Some(format!("Failed to fetch tasks: {}", e)),
                                }
                                app.loading = false;
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
