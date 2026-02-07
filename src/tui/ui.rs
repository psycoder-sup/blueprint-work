use std::fmt::Display;

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::tui::app::App;

pub fn draw(frame: &mut Frame, app: &App) {
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
    let header = Paragraph::new(format!("  BLUEPRINT  |  {project_name}"))
        .block(Block::default().borders(Borders::ALL).title(" Blueprint "));
    frame.render_widget(header, chunks[0]);

    // Body: Epics (left) and Tasks (right)
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    let epics_list = build_list(&app.epics, app.selected_epic_idx, " Epics ");
    frame.render_widget(epics_list, body_chunks[0]);

    let tasks_list = build_list(&app.tasks, app.selected_task_idx, " Tasks ");
    frame.render_widget(tasks_list, body_chunks[1]);

    // Footer
    let footer = Paragraph::new("  q: Quit")
        .block(Block::default().borders(Borders::ALL).title(" Help "));
    frame.render_widget(footer, chunks[2]);
}

/// Trait for items that can be rendered in a selectable list.
trait ListEntry {
    fn title(&self) -> &str;
    fn status(&self) -> &dyn Display;
}

impl ListEntry for crate::models::Epic {
    fn title(&self) -> &str {
        &self.title
    }
    fn status(&self) -> &dyn Display {
        &self.status
    }
}

impl ListEntry for crate::models::BlueTask {
    fn title(&self) -> &str {
        &self.title
    }
    fn status(&self) -> &dyn Display {
        &self.status
    }
}

fn build_list<'a, T: ListEntry>(items: &[T], selected_idx: usize, title: &'a str) -> List<'a> {
    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let marker = if i == selected_idx { "> " } else { "  " };
            ListItem::new(Line::raw(format!(
                "{marker}{} [{}]",
                entry.title(),
                entry.status()
            )))
        })
        .collect();

    List::new(list_items).block(Block::default().borders(Borders::ALL).title(title))
}
