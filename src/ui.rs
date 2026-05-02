use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Modifier},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, Pane};

pub fn render(frame: &mut Frame, app: &App) {
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
    let queue_items: Vec<ListItem> = app.queues.iter().enumerate()
        .map(|(i, q)| {
            let style = if i == app.selected_queue_index {
                Style::default().bg(Color::Blue).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(q.name.as_str()).style(style)
        })
        .collect();
    let queues_list = List::new(queue_items).block(queues_block);
    frame.render_widget(queues_list, main_chunks[0]);

    // Tasks Pane
    let tasks_block = Block::default()
        .title(" Tasks ")
        .borders(Borders::ALL)
        .border_style(if app.active_pane == Pane::Tasks {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });
    let task_items: Vec<ListItem> = app.tasks.iter().enumerate()
        .map(|(i, t)| {
            let prefix = if t.completed { "[x] " } else { "[ ] " };
            let sync_indicator = if t.key.is_empty() || t.key.starts_with("local-") {
                " ⏳"
            } else {
                ""
            };
            let style = if i == app.selected_task_index {
                Style::default().bg(Color::Blue).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(format!("{}{}{}", prefix, t.title, sync_indicator)).style(style)
        })
        .collect();
    let tasks_list = List::new(task_items).block(tasks_block);
    frame.render_widget(tasks_list, main_chunks[1]);

    // Details Pane
    let details_block = Block::default()
        .title(" Details ")
        .borders(Borders::ALL)
        .border_style(if app.active_pane == Pane::Details {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });
    let detail_text = if let Some(task) = app.selected_task() {
        format!(
            "Title: {}\nStatus: {}\nNotes:\n{}",
            task.title,
            if task.completed { "Completed" } else { "Open" },
            task.notes.as_deref().unwrap_or("None")
        )
    } else {
        "No task selected".to_string()
    };
    let details_paragraph = Paragraph::new(detail_text).block(details_block);
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
