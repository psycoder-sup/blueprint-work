use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::models::{BlueTask, Epic, ItemStatus};
use crate::tui::app::App;
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

    let epics_list = build_list(&app.epics, app.selected_epic_idx, " Epics ", true);
    frame.render_widget(epics_list, body_chunks[0]);

    let tasks_list = build_list(&app.tasks, app.selected_task_idx, " Tasks ", false);
    frame.render_widget(tasks_list, body_chunks[1]);

    // Footer
    let footer = Paragraph::new(Line::from(vec![Span::styled(
        "  q: Quit",
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
            let is_selected = i == selected_idx;
            let marker = if is_selected { "▸ " } else { "  " };
            let symbol = theme::status_symbol(entry.status());
            let status_style = theme::status_style(entry.status());

            let marker_style = if is_selected {
                Style::default()
                    .fg(theme::NEON_CYAN)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT_DIM)
            };

            let title_fg = if is_selected {
                theme::TEXT_BRIGHT
            } else {
                theme::TEXT_DIM
            };

            let line = Line::from(vec![
                Span::styled(marker, marker_style),
                Span::styled(format!("{symbol} "), status_style),
                Span::styled(entry.title().to_string(), Style::default().fg(title_fg)),
            ]);

            ListItem::new(line)
        })
        .collect();

    let title_fg = if focused {
        theme::NEON_CYAN
    } else {
        theme::BORDER_DIM
    };

    List::new(list_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::panel_border(focused))
            .title(Span::styled(
                title,
                Style::default().fg(title_fg).add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(theme::BG)),
    )
}
