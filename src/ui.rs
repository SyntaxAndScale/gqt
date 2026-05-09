use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, NavEntry, Pane};
use regex::Regex;

pub fn render(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(frame.area());

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(40),
            Constraint::Percentage(40),
        ])
        .split(chunks[0]);

    // 1. Queues Pane
    let queues_block = Block::default()
        .title(" Queues ")
        .borders(Borders::ALL)
        .border_style(if app.active_pane == Pane::Queues {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });

    let nav_entries = app.get_nav_entries();
    let queue_items: Vec<ListItem> = nav_entries
        .iter()
        .map(|entry| match entry {
            NavEntry::Category { name, expanded } => {
                let icon = if *expanded { "▼" } else { "▶" };
                ListItem::new(format!("{} {}", icon, name))
                    .style(Style::default().add_modifier(Modifier::BOLD))
            }
            NavEntry::Queue(q) => ListItem::new(format!("  {}", q.name)),
        })
        .collect();

    let queues_list = List::new(queue_items)
        .block(queues_block)
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol(">");
    frame.render_stateful_widget(queues_list, main_chunks[0], &mut app.nav_state);

    // 2. Tasks Pane
    let tasks_block = Block::default()
        .title(" Tasks ")
        .borders(Borders::ALL)
        .border_style(if app.active_pane == Pane::Tasks {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });

    let visible_tasks = app.get_visible_tasks();
    let task_items: Vec<ListItem> = visible_tasks
        .iter()
        .map(|(task, depth)| {
            let indent = " ".repeat(*depth);
            let has_subtasks = task.subitems.as_ref().is_some_and(|s| !s.is_empty());
            let expand_icon = if has_subtasks {
                if app.expanded_tasks.contains(&task.key) {
                    "▼"
                } else {
                    "▶"
                }
            } else {
                " "
            };
            let status_icon = if task.completed { "[x]" } else { "[ ]" };
            let unsynced_icon = if task.key.starts_with("local-") {
                " ⏳"
            } else {
                ""
            };

            ListItem::new(format!(
                "{}{} {} {}{}",
                indent,
                expand_icon,
                status_icon,
                clean_html(&task.title),
                unsynced_icon
            ))
        })
        .collect();

    let tasks_list = List::new(task_items)
        .block(tasks_block)
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol(">");
    frame.render_stateful_widget(tasks_list, main_chunks[1], &mut app.task_state);

    // 3. Details Pane
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
        if let Some(ref tags) = task.tags
            && !tags.is_empty()
        {
            let mut tag_spans = Vec::new();
            for tag in tags {
                tag_spans.push(ratatui::text::Span::styled(
                    format!(" #{} ", tag),
                    Style::default().fg(Color::White).bg(Color::Yellow),
                ));
                tag_spans.push(ratatui::text::Span::raw(" "));
            }
            details_text.push(ratatui::text::Line::from(tag_spans));
            details_text.push(ratatui::text::Line::from(""));
        }

        // 2. Title
        details_text.push(ratatui::text::Line::from(ratatui::text::Span::styled(
            clean_html(&task.title),
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        )));
        details_text.push(ratatui::text::Line::from(""));

        // 3. Assignments
        if let Some(ref assignments) = task.assignments
            && !assignments.is_empty()
        {
            let assignee_names: Vec<String> = assignments.iter().map(|a| a.name.clone()).collect();
            details_text.push(ratatui::text::Line::from(vec![
                ratatui::text::Span::raw("👤 "),
                ratatui::text::Span::raw(assignee_names.join(", ")),
            ]));
            details_text.push(ratatui::text::Line::from(""));
        }

        // 4. Dates & Repeat
        let mut date_spans = Vec::new();

        if let Some(ref dd) = task.due_date
            && let Some(ref rd) = dd.raw_date
        {
            date_spans.push(ratatui::text::Span::styled(
                "Due: ",
                Style::default().fg(Color::Magenta),
            ));
            date_spans.push(ratatui::text::Span::raw(rd.clone()));
            date_spans.push(ratatui::text::Span::raw("  "));
        }

        if let Some(ref cd) = task.creation_date {
            date_spans.push(ratatui::text::Span::styled(
                "Created: ",
                Style::default().fg(Color::Blue),
            ));
            date_spans.push(ratatui::text::Span::raw(cd.raw.clone()));
            date_spans.push(ratatui::text::Span::raw("  "));
        }

        // Repeat info
        if task.repeats.is_object() || task.repeats.as_bool().unwrap_or(false) {
            date_spans.push(ratatui::text::Span::styled(
                "🔄",
                Style::default().fg(Color::Green),
            ));
        }

        if !date_spans.is_empty() {
            details_text.push(ratatui::text::Line::from(date_spans));
            details_text.push(ratatui::text::Line::from(""));
        }

        // 5. Notes
        details_text.push(ratatui::text::Line::from(ratatui::text::Span::styled(
            "Notes:",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )));
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

    let status_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(30), // For timestamp
            Constraint::Length(20), // For help hint
        ])
        .split(chunks[1]);

    let status_bar = Paragraph::new(app.status.as_str()).style(status_style);
    frame.render_widget(status_bar, status_chunks[0]);

    if let Some(last_sync) = app.last_synced {
        let sync_text = format!("Last Synced: {}", last_sync.format("%Y-%m-%dT%H:%M:%S"));
        let sync_hint = Paragraph::new(sync_text)
            .style(Style::default().add_modifier(Modifier::DIM))
            .alignment(ratatui::layout::Alignment::Right);
        frame.render_widget(sync_hint, status_chunks[1]);
    }

    let help_hint = Paragraph::new("Press ? for help")
        .style(Style::default().add_modifier(Modifier::DIM))
        .alignment(ratatui::layout::Alignment::Right);
    frame.render_widget(help_hint, status_chunks[2]);

    // Help Modal
    if app.show_help {
        render_help_modal(frame, app);
    }
}

fn render_help_modal(frame: &mut Frame, app: &App) {
    let area = centered_rect(80, 80, frame.area());
    frame.render_widget(Clear, area); // Clear the background

    let block = Block::default()
        .title(" Gqueues TUI Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let mut help_text = Vec::new();
    help_text.push(ratatui::text::Line::from(vec![
        ratatui::text::Span::styled("Gqueues TUI ", Style::default().add_modifier(Modifier::BOLD)),
        ratatui::text::Span::raw(format!("v{}", env!("CARGO_PKG_VERSION"))),
    ]));
    help_text.push(ratatui::text::Line::from("https://github.com/SyntaxAndScale/gqt"));
    help_text.push(ratatui::text::Line::from(""));
    help_text.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        "Disclaimer: This is NOT an official GQueues product.",
        Style::default().add_modifier(Modifier::ITALIC).fg(Color::Yellow),
    )));
    help_text.push(ratatui::text::Line::from(""));
    help_text.push(ratatui::text::Line::from(format!(
        "Config: {}",
        app.config_path.display()
    )));
    help_text.push(ratatui::text::Line::from(format!(
        "Database: {}",
        app.db_path.display()
    )));
    help_text.push(ratatui::text::Line::from(""));
    help_text.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        "Keyboard Shortcuts:",
        Style::default().add_modifier(Modifier::UNDERLINED),
    )));
    help_text.push(ratatui::text::Line::from(""));

    // Categorized actions for display
    let implemented_actions = vec![
        "quit",
        "sync",
        "next_pane",
        "prev_pane",
        "cancel",
        "quick_add",
        "toggle_expand",
        "toggle_subtasks",
        "move_up",
        "move_down",
        "select",
        "go_to_inbox",
        "help",
    ];

    let mut sorted_bindings: Vec<_> = app.keybindings.bindings.iter().collect();
    sorted_bindings.sort_by_key(|(a, _)| *a);

    for (action, key) in sorted_bindings {
        let is_implemented = implemented_actions.contains(&action.as_str())
            || action.starts_with("move_up")
            || action.starts_with("move_down");

        let style = if is_implemented {
            Style::default()
        } else {
            Style::default().add_modifier(Modifier::DIM)
        };

        let action_display = format!(
            "{}{}",
            action,
            if is_implemented {
                ""
            } else {
                " [not yet implemented]"
            }
        );
        help_text.push(ratatui::text::Line::from(vec![
            ratatui::text::Span::styled(format!("{: <25}", action_display), style),
            ratatui::text::Span::styled(key.to_string(), style.fg(Color::Yellow)),
        ]));
    }

    let help_p = Paragraph::new(help_text)
        .block(block)
        .wrap(ratatui::widgets::Wrap { trim: true });

    frame.render_widget(help_p, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn clean_html(html: &str) -> String {
    let re = Regex::new(r"<[^>]*>").unwrap();
    let stripped = re.replace_all(html, "");
    let decoded = html_escape::decode_html_entities(&stripped).into_owned();
    // Normalize non-breaking spaces
    decoded.replace('\u{a0}', " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_html_tags() {
        assert_eq!(clean_html("<b>Bold</b> <i>Italic</i>"), "Bold Italic");
        assert_eq!(clean_html("<div class=\"test\">Content</div>"), "Content");
    }

    #[test]
    fn test_clean_html_entities() {
        assert_eq!(clean_html("Text&nbsp;with&nbsp;spaces"), "Text with spaces");
        assert_eq!(clean_html("Price &lt; $10 &amp; more"), "Price < $10 & more");
    }

    #[test]
    fn test_clean_html_mixed() {
        assert_eq!(
            clean_html("<p>Hello &amp; <b>Welcome</b>!</p>"),
            "Hello & Welcome!"
        );
    }
}
