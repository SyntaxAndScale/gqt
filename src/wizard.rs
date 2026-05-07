use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Alignment},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::io;
use std::path::PathBuf;
use crate::config::{Settings, GqueuesConfig, save_config, KeybindingsConfig};
use crate::db::Database;
use crate::gqueues::GqueuesClient;

#[derive(PartialEq)]
enum WizardState {
    Welcome,
    InputApiKey,
    Confirm,
    Syncing,
}

pub async fn run(db_path: PathBuf) -> Result<Settings> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = WizardState::Welcome;
    let mut api_key = String::new();
    let mut error: Option<String> = None;

    loop {
        terminal.draw(|f| {
            let size = f.area();
            let block = Block::default()
                .title(" Gqueues TUI Setup Wizard ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));
            f.render_widget(block, size);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(4)
                .constraints([
                    Constraint::Length(2), // Title
                    Constraint::Min(0),    // Content
                    Constraint::Length(2), // Footer
                ])
                .split(size);

            let title_text = if state == WizardState::Syncing {
                "Initial Sync in Progress"
            } else {
                "Welcome to Gqueues TUI"
            };

            let title = Paragraph::new(title_text)
                .alignment(Alignment::Center)
                .style(Style::default().add_modifier(Modifier::BOLD));
            f.render_widget(title, chunks[0]);

            let content = match state {
                WizardState::Welcome => {
                    "This wizard will help you set up Gqueues TUI.\n\nPress any key to begin setup.".to_string()
                }
                WizardState::InputApiKey => {
                    "Please enter your GQueues API Key:\n(You can get this from your GQueues account settings)\n\n> ".to_string()
                }
                WizardState::Confirm => {
                    format!("Review your settings:\n\nMode: Sync with GQueues\nAPI Key: ****{}\nDatabase Path: {}\n\nPress Enter to save and start initial sync.", 
                        if api_key.len() > 4 { &api_key[api_key.len()-4..] } else { "" },
                        db_path.display())
                }
                WizardState::Syncing => {
                    "Connecting to GQueues and performing initial sync...\n\n\
                     Fetching queues and Inbox tasks...".to_string()
                }
            };

            let mut display_text = content;
            if state == WizardState::InputApiKey {
                display_text.push_str(&api_key);
                display_text.push_str("█"); // Cursor
            }

            let p = Paragraph::new(display_text).alignment(Alignment::Center);
            f.render_widget(p, chunks[1]);

            if let Some(err) = &error {
                let err_p = Paragraph::new(format!("❌ {}", err))
                    .style(Style::default().fg(Color::Red))
                    .alignment(Alignment::Center);
                f.render_widget(err_p, chunks[2]);
            } else if state != WizardState::Syncing {
                let footer = Paragraph::new("Esc: Quit")
                    .alignment(Alignment::Center)
                    .style(Style::default().add_modifier(Modifier::DIM));
                f.render_widget(footer, chunks[2]);
            }
        })?;

        if state == WizardState::Syncing {
            let db = Database::new(db_path.clone())?;
            let endpoint = "https://api.gqueues.com".to_string();
            let client = GqueuesClient::new(endpoint.clone(), api_key.clone());
            
            match client.get_queues().await {
                Ok(queues) => {
                    for q in &queues {
                        let _ = db.upsert_queue(q);
                    }
                    if let Some(inbox) = queues.iter().find(|q| q.is_inbox) {
                        if let Ok(tasks) = client.get_tasks(&inbox.key).await {
                            for mut t in tasks {
                                t.queue_key = Some(inbox.key.clone());
                                let _ = db.upsert_task(&t);
                            }
                            let _ = db.mark_tasks_fetched(&inbox.key);
                        }
                    }
                }
                Err(e) => {
                    error = Some(format!("Sync failed: {}. Press any key to try again or Esc to quit.", e));
                    state = WizardState::InputApiKey;
                    continue;
                }
            }

            let settings = Settings {
                gqueues: GqueuesConfig {
                    api_endpoint: Some("https://api.gqueues.com".to_string()),
                    access_token: Some(api_key),
                },
                keybindings: KeybindingsConfig::default(),
                database_path: Some(db_path),
            };
            save_config(&settings)?;

            disable_raw_mode()?;
            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
            return Ok(settings);
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Esc {
                    disable_raw_mode()?;
                    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                    std::process::exit(0);
                }

                match state {
                    WizardState::Welcome => {
                        state = WizardState::InputApiKey;
                    }
                    WizardState::InputApiKey => {
                        match key.code {
                            KeyCode::Enter => {
                                if api_key.trim().is_empty() {
                                    error = Some("API Key is required".to_string());
                                } else {
                                    error = None;
                                    state = WizardState::Confirm;
                                }
                            }
                            KeyCode::Char(c) => { error = None; api_key.push(c); }
                            KeyCode::Backspace => { error = None; api_key.pop(); }
                            _ => {}
                        }
                    }
                    WizardState::Confirm => {
                        if key.code == KeyCode::Enter {
                            state = WizardState::Syncing;
                        }
                    }
                    WizardState::Syncing => {}
                }
            }
        }
    }
}
