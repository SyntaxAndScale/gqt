use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Modifier},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, Pane, NavEntry};
use regex::Regex;
use std::sync::OnceLock;

static HTML_TAG_RE: OnceLock<Regex> = OnceLock::new();

/// Utility to strip HTML tags and decode entities.
/// This is a temporary measure until full HTML-to-Ratatui mapping is implemented.
fn clean_html(input: &str) -> String {
    let re = HTML_TAG_RE.get_or_init(|| Regex::new(r"<[^>]*>").unwrap());
    let stripped = re.replace_all(input, "");
    html_escape::decode_html_entities(&stripped).to_string()
}

pub fn render(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(1), // Status bar
        ])
        .split(frame.area());

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20), // Queues
            Constraint::Percentage(40), // Tasks
            Constraint::Percentage(40), // Details
        ])
        .split(chunks[0]);

    // Queues Pane
    let queues_block = Block::default()
        .title(" Queues ")
        .borders(Borders::ALL)
        .border_style(if app.active_pane == Pane::Queues {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });
    
    let nav_entries = app.get_nav_entries();
    let queue_items: Vec<ListItem> = nav_entries.iter()
        .map(|entry| {
            match entry {
                NavEntry::Category { name, expanded } => {
                    let icon = if *expanded { "▼" } else { "▶" };
                    ListItem::new(format!("{} {}", icon, name.to_uppercase()))
                        .style(Style::default().add_modifier(Modifier::BOLD))
                }
                NavEntry::Queue(q) => {
                    ListItem::new(format!("  {}", q.name))
                }
            }
        })
        .collect();
    
    let queues_list = List::new(queue_items)
        .block(queues_block)
        .highlight_style(Style::default().bg(Color::Blue).add_modifier(Modifier::BOLD));
    
    frame.render_stateful_widget(queues_list, main_chunks[0], &mut app.nav_state);

    // Tasks Pane
    let tasks_block = Block::default()
        .title(" Tasks ")
        .borders(Borders::ALL)
        .border_style(if app.active_pane == Pane::Tasks {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });
    
    let visible_tasks = app.get_visible_tasks();
    let task_items: Vec<ListItem> = visible_tasks.iter()
        .map(|(t, depth)| {
            let prefix = if t.completed { "[x] " } else { "[ ] " };
            let has_subtasks = app.tasks.iter().any(|st| st.parent_key.as_ref() == Some(&t.key));
            
            let expand_icon = if has_subtasks {
                if app.expanded_tasks.contains(&t.key) { "▼" } else { "▶" }
            } else {
                " "
            };

            let sync_indicator = if t.key.is_empty() || t.key.starts_with("local-") {
                " ⏳"
            } else {
                ""
            };
            
            let indentation = " ".repeat(*depth);
            let title = clean_html(&t.title);
            ListItem::new(format!("{}{}{} {}{}", indentation, expand_icon, prefix, title, sync_indicator))
        })
        .collect();

    let tasks_list = List::new(task_items)
        .block(tasks_block)
        .highlight_style(Style::default().bg(Color::Blue).add_modifier(Modifier::BOLD));
    
    frame.render_stateful_widget(tasks_list, main_chunks[1], &mut app.task_state);

    // Details Pane
    let details_block = Block::default()
        .title(" Details ")
        .borders(Borders::ALL)
        .border_style(if app.active_pane == Pane::Details {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });
    
    let mut details_text = Vec::new();
    if let Some(task) = app.selected_task() {
        // 1. Tags
        if let Some(ref tags) = task.tags {
            if !tags.is_empty() {
                let mut tag_spans = Vec::new();
                for tag in tags {
                    tag_spans.push(ratatui::text::Span::styled(
                        format!(" #{} ", tag),
                        Style::default().bg(Color::Yellow).fg(Color::White).add_modifier(Modifier::BOLD),
                    ));
                    tag_spans.push(ratatui::text::Span::raw(" "));
                }
                details_text.push(ratatui::text::Line::from(tag_spans));
                details_text.push(ratatui::text::Line::from(""));
            }
        }

        // 2. Title
        details_text.push(ratatui::text::Line::from(ratatui::text::Span::styled(
            clean_html(&task.title),
            Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan),
        )));
        details_text.push(ratatui::text::Line::from(""));

        // 3. Assignees
        if let Some(ref assignments) = task.assignments {
            if !assignments.is_empty() {
                let assignee_names: Vec<String> = assignments.iter().map(|a| a.name.clone()).collect();
                details_text.push(ratatui::text::Line::from(format!("👤 {}", assignee_names.join(", "))));
                details_text.push(ratatui::text::Line::from(""));
            }
        }

        // 4. Dates & Repeat
        let mut date_spans = Vec::new();
        if let Some(ref cd) = task.creation_date {
            date_spans.push(ratatui::text::Span::styled("Created: ", Style::default().add_modifier(Modifier::DIM)));
            date_spans.push(ratatui::text::Span::raw(format!("{}  ", cd.raw)));
        }
        if let Some(ref dd) = task.due_date {
            if let Some(ref rd) = dd.raw_date {
                date_spans.push(ratatui::text::Span::styled("Due: ", Style::default().add_modifier(Modifier::DIM).fg(Color::Magenta)));
                date_spans.push(ratatui::text::Span::styled(format!("{}  ", rd), Style::default().fg(Color::Magenta)));
            }
        }
        
        let repeats_str = match &task.repeats {
            serde_json::Value::Bool(b) => if *b { Some("Repeats".to_string()) } else { None },
            serde_json::Value::Object(obj) => obj.get("title").and_then(|v| v.as_str()).map(|s| s.to_string()).or(Some("Repeats".to_string())),
            _ => None,
        };
        if let Some(r) = repeats_str {
            date_spans.push(ratatui::text::Span::styled("🔁 ", Style::default().fg(Color::Blue)));
            date_spans.push(ratatui::text::Span::raw(r));
        }

        if !date_spans.is_empty() {
            details_text.push(ratatui::text::Line::from(date_spans));
            details_text.push(ratatui::text::Line::from(""));
        }

        // 5. Notes
        details_text.push(ratatui::text::Line::from(ratatui::text::Span::styled("Notes:", Style::default().add_modifier(Modifier::UNDERLINED))));
        let notes_text = task.notes.clone().unwrap_or_else(|| "None".to_string());
        details_text.push(ratatui::text::Line::from(clean_html(&notes_text)));
    } else {
        details_text.push(ratatui::text::Line::from("No task selected"));
    }

    let details_paragraph = Paragraph::new(details_text)
        .block(details_block)
        .wrap(ratatui::widgets::Wrap { trim: true })
        .scroll((app.detail_scroll, 0));
    frame.render_widget(details_paragraph, main_chunks[2]);

    // Status Bar
    let status_style = if app.status.contains("❌") {
        Style::default().fg(Color::Red)
    } else if app.status.contains("✅") {
        Style::default().fg(Color::Green)
    } else if app.status.contains("⏳") {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().add_modifier(Modifier::DIM)
    };
    let status_bar = Paragraph::new(app.status.as_str()).style(status_style);
    frame.render_widget(status_bar, chunks[1]);
}
