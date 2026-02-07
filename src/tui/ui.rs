use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::models::{BlueTask, Epic, ItemStatus};
use crate::tui::app::{App, InputMode};
use crate::tui::theme;

pub fn draw(frame: &mut Frame, app: &App) {
    // Fill the entire background
    let bg_block = Block::default().style(Style::default().bg(theme::BG));
    frame.render_widget(bg_block, frame.area());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(frame.area());

    // Header
    let project_name = app
        .selected_project()
        .map(|p| p.name.as_str())
        .unwrap_or("No projects");
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            format!("  {} ", theme::HEADER_ART),
            Style::default()
                .fg(theme::NEON_CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ ", Style::default().fg(theme::BORDER_DIM)),
        Span::styled(project_name, Style::default().fg(theme::NEON_MAGENTA)),
        Span::styled(" [p]", Style::default().fg(theme::TEXT_DIM)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::panel_border(false))
            .style(Style::default().bg(theme::BG)),
    );
    frame.render_widget(header, chunks[0]);

    // Body: Epics (left) and Tasks (right)
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    draw_epic_list(frame, app, body_chunks[0]);

    let tasks_list = build_list(&app.tasks, app.selected_task_idx, " Tasks ", false);
    frame.render_widget(tasks_list, body_chunks[1]);

    // Footer
    let help_text = match app.mode {
        InputMode::Normal => "  q: Quit  p: Projects  j/k: Epics",
        InputMode::ProjectSelector => "  j/k: Navigate  Enter: Select  Esc: Cancel",
    };
    let footer = Paragraph::new(Line::from(vec![Span::styled(
        help_text,
        Style::default().fg(theme::TEXT_DIM),
    )]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::panel_border(false))
            .title(Span::styled(
                " Help ",
                Style::default().fg(theme::TEXT_DIM),
            ))
            .style(Style::default().bg(theme::BG)),
    );
    frame.render_widget(footer, chunks[2]);

    // Popup overlay
    if app.mode == InputMode::ProjectSelector {
        draw_project_selector(frame, app);
    }
}

/// Trait for items that can be rendered in a selectable list.
trait ListEntry {
    fn title(&self) -> &str;
    fn status(&self) -> &ItemStatus;
}

impl ListEntry for Epic {
    fn title(&self) -> &str {
        &self.title
    }
    fn status(&self) -> &ItemStatus {
        &self.status
    }
}

impl ListEntry for BlueTask {
    fn title(&self) -> &str {
        &self.title
    }
    fn status(&self) -> &ItemStatus {
        &self.status
    }
}

/// Returns the marker string and styles for a selected/unselected row.
fn selection_styles(is_selected: bool) -> (&'static str, Style, Style) {
    let marker = if is_selected { "▸ " } else { "  " };

    let marker_style = if is_selected {
        Style::default()
            .fg(theme::NEON_CYAN)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::TEXT_DIM)
    };

    let title_style = if is_selected {
        Style::default().fg(theme::TEXT_BRIGHT)
    } else {
        Style::default().fg(theme::TEXT_DIM)
    };

    (marker, marker_style, title_style)
}

/// Creates a styled panel block with the given title.
fn panel_block(title: &str, focused: bool) -> Block<'_> {
    let title_fg = if focused {
        theme::NEON_CYAN
    } else {
        theme::BORDER_DIM
    };

    Block::default()
        .borders(Borders::ALL)
        .border_style(theme::panel_border(focused))
        .title(Span::styled(
            title,
            Style::default().fg(title_fg).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(theme::BG))
}

fn build_list<'a, T: ListEntry>(
    items: &[T],
    selected_idx: usize,
    title: &'a str,
    focused: bool,
) -> List<'a> {
    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let (marker, marker_style, title_style) = selection_styles(i == selected_idx);
            let symbol = theme::status_symbol(entry.status());
            let status_style = theme::status_style(entry.status());

            let line = Line::from(vec![
                Span::styled(marker, marker_style),
                Span::styled(format!("{symbol} "), status_style),
                Span::styled(entry.title().to_string(), title_style),
            ]);

            ListItem::new(line)
        })
        .collect();

    List::new(list_items).block(panel_block(title, focused))
}

fn draw_epic_list(frame: &mut Frame, app: &App, area: Rect) {
    let list_items: Vec<ListItem> = app
        .epics
        .iter()
        .enumerate()
        .map(|(i, epic)| {
            let (marker, marker_style, title_style) = selection_styles(i == app.selected_epic_idx);
            let symbol = theme::status_symbol(&epic.status);
            let status_style = theme::status_style(&epic.status);

            let mut spans = vec![
                Span::styled(marker, marker_style),
                Span::styled(format!("{symbol} "), status_style),
                Span::styled(epic.title.to_string(), title_style),
            ];

            if app.blocked_epic_ids.contains(&epic.id) {
                spans.push(Span::styled(
                    format!(" {}", theme::BLOCKED_SYMBOL),
                    theme::blocked_style(),
                ));
            }

            spans.push(Span::styled(
                format!(" [{}/{}]", epic.done_count, epic.task_count),
                Style::default().fg(theme::TEXT_DIM),
            ));

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(list_items).block(panel_block(" Epics ", true));
    frame.render_widget(list, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

fn draw_project_selector(frame: &mut Frame, app: &App) {
    let area = centered_rect(50, 60, frame.area());
    frame.render_widget(Clear, area);

    let list_items: Vec<ListItem> = app
        .projects
        .iter()
        .enumerate()
        .map(|(i, project)| {
            let (marker, marker_style, title_style) = selection_styles(i == app.selector_idx);
            let status_style = theme::project_status_style(&project.status);

            let line = Line::from(vec![
                Span::styled(marker, marker_style),
                Span::styled(&project.name, title_style),
                Span::styled(" ", Style::default()),
                Span::styled(format!("[{}]", project.status), status_style),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(list_items).block(panel_block(" Select Project ", true));
    frame.render_widget(list, area);
}
