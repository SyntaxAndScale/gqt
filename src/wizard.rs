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

#[derive(PartialEq)]
enum WizardState {
    Welcome,
    InputApiKey,
    Confirm,
}

pub fn run() -> Result<Settings> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = WizardState::Welcome;
    let mut api_key = String::new();
    let db_path = crate::db::Database::get_default_db_path()?.to_string_lossy().to_string();

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

            let title = Paragraph::new("Welcome to Gqueues TUI")
                .alignment(Alignment::Center)
                .style(Style::default().add_modifier(Modifier::BOLD));
            f.render_widget(title, chunks[0]);

            let content = match state {
                WizardState::Welcome => {
                    "This wizard will help you set up Gqueues TUI.\n\nPress any key to begin setup."
                }
                WizardState::InputApiKey => {
                    "Please enter your GQueues API Key:\n(You can get this from your GQueues account settings)\n\n> "
                }
                WizardState::Confirm => {
                    "Review your settings:\n\nMode: Sync with GQueues"
                }
            };

            let mut display_text = content.to_string();
            if state == WizardState::InputApiKey {
                display_text.push_str(&api_key);
                display_text.push_str("█"); // Cursor
            } else if state == WizardState::Confirm {
                display_text.push_str(&format!("\nAPI Key: ****{}", if api_key.len() > 4 { &api_key[api_key.len()-4..] } else { "" }));
                display_text.push_str(&format!("\nDatabase Path: {}\n\nPress Enter to save and start Gqueues TUI.", db_path));
            }

            let p = Paragraph::new(display_text).alignment(Alignment::Center);
            f.render_widget(p, chunks[1]);

            if let Some(err) = &error {
                let err_p = Paragraph::new(format!("❌ {}", err))
                    .style(Style::default().fg(Color::Red))
                    .alignment(Alignment::Center);
                f.render_widget(err_p, chunks[2]);
            } else {
                let footer = Paragraph::new("Esc: Quit")
                    .alignment(Alignment::Center)
                    .style(Style::default().add_modifier(Modifier::DIM));
                f.render_widget(footer, chunks[2]);
            }
        })?;

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
                            let settings = Settings {
                                gqueues: GqueuesConfig {
                                    api_endpoint: Some("https://api.gqueues.com".to_string()),
                                    access_token: Some(api_key),
                                },
                                keybindings: KeybindingsConfig::default(),
                                database_path: Some(PathBuf::from(db_path)),
                            };
                            save_config(&settings)?;
                            
                            disable_raw_mode()?;
                            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                            return Ok(settings);
                        }
                    }
                }
            }
        }
    }
}
