use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::{Block, Borders, Paragraph};

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

    // Main body: Epics (left) and Tasks (right)
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    let epics_block = Block::default().borders(Borders::ALL).title(" Epics ");
    frame.render_widget(epics_block, body_chunks[0]);

    let tasks_block = Block::default().borders(Borders::ALL).title(" Tasks ");
    frame.render_widget(tasks_block, body_chunks[1]);

    // Footer
    let footer = Paragraph::new("  q: Quit")
        .block(Block::default().borders(Borders::ALL).title(" Help "));
    frame.render_widget(footer, chunks[2]);
}
