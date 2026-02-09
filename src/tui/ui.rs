use std::cell::Cell;
use std::collections::HashMap;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::models::ItemStatus;
use crate::tui::app::{App, FocusedPanel, GraphCache, GraphLevel, GraphPane, InputMode};
use crate::tui::graph_render::{
    Canvas, NodeBox, node_height, render_edges, render_focus_highlight, render_node,
    NODE_HEIGHT_EPIC, NODE_HEIGHT_TASK,
};
use crate::tui::theme;

/// Bundles the per-pane graph rendering parameters so callers don't need to
/// pass many individual fields.
struct GraphPaneParams<'a> {
    cache: Option<&'a GraphCache>,
    level: GraphLevel,
    scroll_x: usize,
    scroll_y: usize,
    focused_node_id: Option<&'a str>,
    max_scroll_out: &'a Cell<(usize, usize)>,
}

pub fn draw(frame: &mut Frame, app: &App) {
    if app.mode == InputMode::GraphView {
        draw_graph_view(frame, app);
        return;
    }

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

    // Body: 2x2 grid
    let body_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(chunks[1]);

    let top_panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(body_rows[0]);

    let bottom_panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(body_rows[1]);

    draw_epic_list(frame, app, top_panels[0]);
    draw_task_list(frame, app, top_panels[1]);
    draw_deps_panel(frame, app, bottom_panels[0]);
    draw_status_panel(frame, app, bottom_panels[1]);

    // Footer
    let help_text = match app.mode {
        InputMode::Normal => {
            "  q: Quit  p: Projects  Tab: Focus  h/l: Left/Right  j/k: Navigate  s: Status  ?: Help"
        }
        InputMode::ProjectSelector => "  j/k: Navigate  Enter: Select  Esc: Cancel",
        InputMode::TaskDetail | InputMode::HelpOverlay => "  Esc: Close",
        InputMode::GraphView => "  Esc: Back  1: Epics  2: Tasks  3: Dual  Tab: Pane  \u{2190}\u{2191}\u{2192}\u{2193}: Focus  hjkl: Scroll",
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
    match app.mode {
        InputMode::ProjectSelector => draw_project_selector(frame, app),
        InputMode::TaskDetail => draw_task_detail(frame, app),
        InputMode::HelpOverlay => draw_help_overlay(frame),
        InputMode::Normal | InputMode::GraphView => {}
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

/// Returns a dim `[short_id] ` span if present, or `None`.
fn short_id_span(short_id: Option<&str>) -> Option<Span<'static>> {
    short_id.map(|sid| Span::styled(format!("[{sid}] "), Style::default().fg(theme::TEXT_DIM)))
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
            ];
            spans.extend(short_id_span(epic.short_id.as_deref()));
            spans.push(Span::styled(&epic.title, title_style));

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

    let focused = app.focused_panel == FocusedPanel::Epics;
    let list = List::new(list_items).block(panel_block(" Epics ", focused));
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

fn draw_task_list(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focused_panel == FocusedPanel::Tasks;

    let list_items: Vec<ListItem> = app
        .tasks
        .iter()
        .enumerate()
        .map(|(i, task)| {
            let (marker, marker_style, title_style) =
                selection_styles(i == app.selected_task_idx);
            let symbol = theme::status_symbol(&task.status);
            let status_style = theme::status_style(&task.status);

            let mut spans = vec![
                Span::styled(marker, marker_style),
                Span::styled(format!("{symbol} "), status_style),
            ];
            spans.extend(short_id_span(task.short_id.as_deref()));
            spans.push(Span::styled(&task.title, title_style));

            if task.session_id.is_some() {
                spans.push(Span::styled(
                    format!(" {}", theme::SESSION_SYMBOL),
                    theme::session_style(),
                ));
            }

            if app.blocked_task_ids.contains(&task.id) {
                spans.push(Span::styled(
                    format!(" {}", theme::BLOCKED_SYMBOL),
                    theme::blocked_style(),
                ));
            }

            spans.push(Span::styled(
                format!(" [{}]", task.status.as_str()),
                status_style,
            ));

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(list_items).block(panel_block(" Tasks ", focused));
    frame.render_widget(list, area);
}

fn draw_task_detail(frame: &mut Frame, app: &App) {
    let Some(task) = app.selected_task() else {
        return;
    };
    let area = centered_rect(60, 50, frame.area());
    frame.render_widget(Clear, area);

    let symbol = theme::status_symbol(&task.status);

    let mut header_spans: Vec<Span> = Vec::new();
    header_spans.extend(short_id_span(task.short_id.as_deref()));
    header_spans.push(Span::styled(
        &task.title,
        Style::default()
            .fg(theme::NEON_CYAN)
            .add_modifier(Modifier::BOLD),
    ));

    let mut lines = vec![
        Line::from(header_spans),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("{symbol} "),
                theme::status_style(&task.status),
            ),
            Span::styled(
                task.status.as_str(),
                theme::status_style(&task.status),
            ),
        ]),
        Line::from(""),
    ];

    if task.session_id.is_some() {
        lines.push(Line::from(vec![
            Span::styled(
                format!("{} Session: ", theme::SESSION_SYMBOL),
                theme::session_style(),
            ),
            Span::styled("active", Style::default().fg(theme::TEXT_DIM)),
        ]));
        lines.push(Line::from(""));
    }

    if !task.description.is_empty() {
        lines.push(Line::from(Span::styled(
            &task.description,
            Style::default().fg(theme::TEXT_DIM),
        )));
        lines.push(Line::from(""));
    }

    if let Some(blocker_names) = app.task_blocker_names.get(&task.id) {
        lines.push(Line::from(vec![
            Span::styled(
                format!("{} Blocked by: ", theme::BLOCKED_SYMBOL),
                theme::blocked_style(),
            ),
            Span::styled(
                blocker_names.join(", "),
                Style::default().fg(theme::TEXT_DIM),
            ),
        ]));
    }

    let paragraph = Paragraph::new(lines)
        .block(panel_block(" Task Detail ", true))
        .wrap(ratatui::widgets::Wrap { trim: true });
    frame.render_widget(paragraph, area);
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
                Span::styled(format!(" [{}]", project.status), status_style),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(list_items).block(panel_block(" Select Project ", true));
    frame.render_widget(list, area);
}

/// Builds a styled progress line like "  Label: ████░░ 3/10".
fn progress_line(label: &str, counts: &HashMap<String, i64>, area_width: u16) -> Line<'static> {
    let done = *counts.get("done").unwrap_or(&0) as usize;
    let total = counts.values().sum::<i64>() as usize;

    let count_text = format!(" {done}/{total}");
    // "  Label: " (2 indent + label + ": ") + count_text + borders (2)
    let label_str = format!("  {label}: ");
    let bar_width = area_width
        .saturating_sub(label_str.len() as u16 + count_text.len() as u16 + 2)
        as usize;

    let bar = theme::progress_bar(done, total, bar_width.max(1));
    let filled: String = bar.chars().filter(|&c| c == '\u{2588}').collect();
    let remaining: String = bar.chars().filter(|&c| c == '\u{2591}').collect();

    Line::from(vec![
        Span::styled(label_str, Style::default().fg(theme::TEXT_BRIGHT)),
        Span::styled(filled, Style::default().fg(theme::NEON_GREEN)),
        Span::styled(remaining, Style::default().fg(theme::TEXT_DIM)),
        Span::styled(count_text, Style::default().fg(theme::TEXT_DIM)),
    ])
}

fn draw_status_panel(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focused_panel == FocusedPanel::Status;

    let blocked_style = if app.blocked_count > 0 {
        Style::default().fg(theme::NEON_ORANGE)
    } else {
        Style::default().fg(theme::TEXT_DIM)
    };

    let lines = vec![
        progress_line("Epics", &app.epic_status_counts, area.width),
        progress_line("Tasks", &app.task_status_counts, area.width),
        Line::from(""),
        Line::from(Span::styled(
            format!("  Blocked: {} items", app.blocked_count),
            blocked_style,
        )),
    ];

    let paragraph = Paragraph::new(lines).block(panel_block(" Project Status ", focused));
    frame.render_widget(paragraph, area);
}

/// Truncates `text` to at most `max_chars`, appending an ellipsis if truncated.
fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let truncated: String = text.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{truncated}\u{2026}")
}

fn draw_deps_panel(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focused_panel == FocusedPanel::Dependencies;
    let block = panel_block(" Dependencies (mini) ", focused);

    if app.dep_display_rows.is_empty() {
        let paragraph = Paragraph::new("No dependencies")
            .style(Style::default().fg(theme::TEXT_DIM))
            .block(block);
        frame.render_widget(paragraph, area);
        return;
    }

    let inner_width = area.width.saturating_sub(2) as usize;
    let arrow = " \u{2500}\u{2500}blocks\u{2500}\u{2500}\u{25b6} ";
    let arrow_len = arrow.chars().count();

    let mut lines: Vec<Line> = app
        .dep_display_rows
        .iter()
        .take(5)
        .map(|row| {
            let color = if row.is_active {
                theme::NEON_PINK
            } else {
                theme::NEON_CYAN
            };
            let style = Style::default().fg(color);

            let available = inner_width.saturating_sub(arrow_len);
            let half = available / 2;
            let blocker = truncate(&row.blocker_title, half);
            let remaining = inner_width.saturating_sub(blocker.chars().count() + arrow_len);
            let blocked = truncate(&row.blocked_title, remaining);

            Line::from(vec![
                Span::styled(blocker, style),
                Span::styled(arrow, style),
                Span::styled(blocked, style),
            ])
        })
        .collect();

    lines.push(Line::from(vec![
        Span::styled("[d]", Style::default().fg(theme::NEON_CYAN)),
        Span::styled(
            " Full Dependency Graph",
            Style::default().fg(theme::TEXT_DIM),
        ),
    ]));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn draw_help_overlay(frame: &mut Frame) {
    let area = centered_rect(60, 60, frame.area());
    frame.render_widget(Clear, area);

    let title_style = Style::default()
        .fg(theme::NEON_CYAN)
        .add_modifier(Modifier::BOLD);
    let section_style = Style::default()
        .fg(theme::NEON_MAGENTA)
        .add_modifier(Modifier::BOLD);
    let key_style = Style::default().fg(theme::NEON_GREEN);
    let desc_style = Style::default().fg(theme::TEXT_DIM);

    let key_line = |key: &'static str, desc: &'static str| -> Line<'static> {
        Line::from(vec![
            Span::styled(format!("   {key:<14}"), key_style),
            Span::styled(desc, desc_style),
        ])
    };

    let lines = vec![
        Line::from(Span::styled(" KEYBOARD SHORTCUTS", title_style)),
        Line::from(""),
        Line::from(Span::styled(" Navigation", section_style)),
        key_line("j/k, \u{2191}/\u{2193}", "Move up/down in active panel"),
        key_line("h/l, \u{2190}/\u{2192}", "Switch left/right between panels"),
        key_line("Tab", "Cycle through all panels"),
        Line::from(""),
        Line::from(Span::styled(" Actions", section_style)),
        key_line("Enter", "Open task detail"),
        key_line("s", "Cycle task status (todo \u{2192} in_progress \u{2192} done)"),
        key_line("p", "Open project selector"),
        key_line("d", "Toggle dependency graph view"),
        Line::from(""),
        Line::from(Span::styled(" General", section_style)),
        key_line("?", "Toggle this help overlay"),
        key_line("q", "Quit / Close overlay"),
        key_line("Esc", "Close overlay/popup"),
    ];

    let help = Paragraph::new(lines).block(panel_block(" Help ", true));
    frame.render_widget(help, area);
}

fn draw_graph_view(frame: &mut Frame, app: &App) {
    if app.dual_pane {
        draw_dual_pane_graph(frame, app);
        return;
    }

    // Fill the entire background
    let bg_block = Block::default().style(Style::default().bg(theme::BG));
    frame.render_widget(bg_block, frame.area());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(frame.area());

    // Header with tab indicators
    let epic_tab = if app.graph_mode == GraphLevel::Epic {
        Span::styled(
            "[EPICS]",
            Style::default()
                .fg(theme::NEON_CYAN)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled("epics", Style::default().fg(theme::TEXT_DIM))
    };

    let task_tab = if app.graph_mode == GraphLevel::Task {
        Span::styled(
            "[TASKS]",
            Style::default()
                .fg(theme::NEON_CYAN)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled("tasks", Style::default().fg(theme::TEXT_DIM))
    };

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            "  \u{2593}\u{2593} DEPENDENCY GRAPH \u{2593}\u{2593} ",
            Style::default()
                .fg(theme::NEON_CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("\u{2502} ", Style::default().fg(theme::BORDER_DIM)),
        epic_tab,
        Span::styled("  ", Style::default()),
        task_tab,
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::panel_border(false))
            .style(Style::default().bg(theme::BG)),
    );
    frame.render_widget(header, chunks[0]);

    // Sub-header for task-level view
    if app.graph_mode == GraphLevel::Task {
        if let Some(epic) = app.selected_epic() {
            let sub_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(0)])
                .split(chunks[1]);

            let sub_header = Paragraph::new(Line::from(vec![
                Span::styled("  EPIC: ", Style::default().fg(theme::TEXT_DIM)),
                Span::styled(
                    &epic.title,
                    Style::default()
                        .fg(theme::NEON_MAGENTA)
                        .add_modifier(Modifier::BOLD),
                ),
            ]))
            .style(Style::default().bg(theme::BG));
            frame.render_widget(sub_header, sub_chunks[0]);

            // Render task graph in the remaining area
            draw_graph_canvas(frame, app, sub_chunks[1]);
        } else {
            // No epic selected — show centered message
            let msg = Paragraph::new("Select an epic first")
                .style(Style::default().fg(theme::TEXT_DIM).bg(theme::BG))
                .alignment(ratatui::layout::Alignment::Center);
            frame.render_widget(msg, chunks[1]);
        }
    } else {
        // Epic-level graph
        draw_graph_canvas(frame, app, chunks[1]);
    }

    // Summary bar
    if let Some(cache) = &app.graph_cache {
        let summary = Paragraph::new(graph_summary_line(cache))
            .style(Style::default().bg(theme::BG));
        frame.render_widget(summary, chunks[2]);
    }

    // Footer
    draw_graph_footer(frame, chunks[3]);
}

fn draw_graph_footer(frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![Span::styled(
        "  Esc: Back  1: Epics  2: Tasks  3: Dual  Tab: Pane  \u{2190}\u{2191}\u{2192}\u{2193}: Focus  hjkl: Scroll",
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
    frame.render_widget(footer, area);
}

/// Summary statistics derived from the current graph cache.
struct GraphSummary {
    total_nodes: usize,
    total_edges: usize,
    blocked_count: usize,
    done_count: usize,
}

/// Returns `true` if the node has any incoming edge from a non-done node.
fn has_incomplete_blocker(node_id: &str, layout: &crate::tui::graph::DagLayout) -> bool {
    layout.edges.iter().any(|e| {
        e.to == node_id
            && layout
                .nodes
                .get(&e.from)
                .is_some_and(|src| src.status != ItemStatus::Done)
    })
}

/// Compute summary statistics from a graph cache.
fn compute_graph_summary(cache: &GraphCache) -> GraphSummary {
    let nodes = &cache.layout.nodes;

    let done_count = nodes
        .values()
        .filter(|n| n.status == ItemStatus::Done)
        .count();

    let blocked_count = nodes
        .values()
        .filter(|n| n.status != ItemStatus::Done)
        .filter(|n| has_incomplete_blocker(&n.id, &cache.layout))
        .count();

    GraphSummary {
        total_nodes: nodes.len(),
        total_edges: cache.layout.edges.len(),
        blocked_count,
        done_count,
    }
}

/// Build the summary bar spans for the graph view footer.
fn graph_summary_line(cache: &GraphCache) -> Line<'static> {
    let summary = compute_graph_summary(cache);
    let label = match cache.level {
        GraphLevel::Epic => "epics",
        GraphLevel::Task => "tasks",
    };

    let sep = Style::default().fg(theme::TEXT_DIM);
    let cyan = Style::default().fg(theme::NEON_CYAN);
    let green = Style::default().fg(theme::NEON_GREEN);
    let blocked_fg = if summary.blocked_count > 0 {
        theme::NEON_ORANGE
    } else {
        theme::TEXT_DIM
    };
    let blocked = Style::default().fg(blocked_fg);

    Line::from(vec![
        Span::styled("  ◉ ", cyan),
        Span::styled(format!("{} {}", summary.total_nodes, label), cyan),
        Span::styled(" │ ", sep),
        Span::styled(format!("─▶ {} edges", summary.total_edges), cyan),
        Span::styled(" │ ", sep),
        Span::styled(format!("⚠ {} blocked", summary.blocked_count), blocked),
        Span::styled(" │ ", sep),
        Span::styled(format!("■ {} done", summary.done_count), green),
    ])
}

fn draw_graph_canvas(frame: &mut Frame, app: &App, area: Rect) {
    let params = GraphPaneParams {
        cache: app.graph_cache.as_ref(),
        level: app.graph_mode,
        scroll_x: app.scroll_x,
        scroll_y: app.scroll_y,
        focused_node_id: app.focused_node.as_deref(),
        max_scroll_out: &app.max_scroll,
    };
    draw_graph_pane(frame, app, area, &params);
}

/// Core graph canvas rendering, parameterized for reuse in both single and dual-pane modes.
fn draw_graph_pane(frame: &mut Frame, app: &App, area: Rect, params: &GraphPaneParams) {
    let viewport_width = area.width as usize;
    let viewport_height = area.height as usize;

    if viewport_width == 0 || viewport_height == 0 {
        return;
    }

    if let Some(cache) = params.cache {
        // For task-level, check if there are no tasks
        if cache.level == GraphLevel::Task && app.tasks.is_empty() {
            let msg = Paragraph::new("No tasks in this epic")
                .style(Style::default().fg(theme::TEXT_DIM).bg(theme::BG))
                .alignment(ratatui::layout::Alignment::Center);
            frame.render_widget(msg, area);
            return;
        }

        let (blocked_ids, default_height) = match cache.level {
            GraphLevel::Epic => (&app.blocked_epic_ids, NODE_HEIGHT_EPIC),
            GraphLevel::Task => (&app.blocked_task_ids, NODE_HEIGHT_TASK),
        };

        // Compute per-node heights based on title length and whether it has a progress bar.
        let mut per_node_heights: HashMap<String, usize> = HashMap::new();
        for (node_id, _) in &cache.node_positions {
            if let Some(node) = cache.layout.nodes.get(node_id) {
                let has_progress = cache.level == GraphLevel::Epic;
                let h = node_height(&node.label, has_progress);
                per_node_heights.insert(node_id.clone(), h);
            }
        }

        // Compute the full canvas extent from node positions.
        let (full_width, full_height) = graph_canvas_extent(cache, &per_node_heights, default_height);

        // Use the larger of the full extent or the viewport so nodes always render.
        let canvas_w = full_width.max(viewport_width);
        let canvas_h = full_height.max(viewport_height);
        let mut canvas = Canvas::new(canvas_w, canvas_h);

        // Render nodes
        for (node_id, &(x, y)) in &cache.node_positions {
            if let Some(node) = cache.layout.nodes.get(node_id) {
                let progress = match cache.level {
                    GraphLevel::Epic => {
                        app.epics
                            .iter()
                            .find(|e| e.id == *node_id)
                            .map(|e| (e.done_count as usize, e.task_count as usize))
                    }
                    GraphLevel::Task => None,
                };

                let node_box = NodeBox {
                    title: node.label.clone(),
                    status: node.status.clone(),
                    progress,
                    x,
                    y,
                    blocked: blocked_ids.contains(node_id),
                };
                render_node(&mut canvas, &node_box, app.animation_frame);
            }
        }

        // Render edges
        render_edges(
            &mut canvas,
            &cache.layout,
            &cache.node_positions,
            blocked_ids,
            &per_node_heights,
            default_height,
        );

        // Render focus highlight on the selected node
        if let Some(fid) = params.focused_node_id
            && let Some(&(fx, fy)) = cache.node_positions.get(fid)
        {
            let fh = per_node_heights.get(fid).copied().unwrap_or(default_height);
            render_focus_highlight(&mut canvas, fx, fy, fh);
        }

        // Clamp scroll offsets to valid bounds.
        let max_scroll_x = canvas_w.saturating_sub(viewport_width);
        let max_scroll_y = canvas_h.saturating_sub(viewport_height);
        params.max_scroll_out.set((max_scroll_x, max_scroll_y));
        let sx = params.scroll_x.min(max_scroll_x);
        let sy = params.scroll_y.min(max_scroll_y);

        // Blit the visible portion of the canvas to the frame.
        let lines: Vec<Line> = (0..viewport_height)
            .map(|vy| {
                let cy = sy + vy;
                let spans: Vec<Span> = (0..viewport_width)
                    .map(|vx| {
                        let cx = sx + vx;
                        let cell = canvas.get(cx, cy);
                        Span::styled(cell.ch.to_string(), cell.style)
                    })
                    .collect();
                Line::from(spans)
            })
            .collect();

        let paragraph = Paragraph::new(lines).style(Style::default().bg(theme::BG));
        frame.render_widget(paragraph, area);

        render_scroll_indicators(frame, area, sx, sy, max_scroll_x, max_scroll_y);
    } else {
        let msg = match params.level {
            GraphLevel::Task if app.selected_epic().is_none() => "Select an epic first",
            _ => "No graph data",
        };
        let empty = Paragraph::new(msg)
            .style(Style::default().fg(theme::TEXT_DIM).bg(theme::BG))
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(empty, area);
    }
}

/// Compute the minimum canvas size needed to contain all nodes (with padding).
fn graph_canvas_extent(
    cache: &GraphCache,
    per_node_heights: &HashMap<String, usize>,
    default_height: usize,
) -> (usize, usize) {
    use crate::tui::graph_render::NODE_WIDTH;

    let mut max_x: usize = 0;
    let mut max_y: usize = 0;

    for (node_id, &(x, y)) in &cache.node_positions {
        let h = per_node_heights.get(node_id).copied().unwrap_or(default_height);
        max_x = max_x.max(x + NODE_WIDTH);
        max_y = max_y.max(y + h);
    }

    // Add small padding for edge routing below the lowest nodes.
    (max_x + 2, max_y + 2)
}

/// Render scroll indicators showing which directions are scrollable.
fn render_scroll_indicators(
    frame: &mut Frame,
    area: Rect,
    scroll_x: usize,
    scroll_y: usize,
    max_scroll_x: usize,
    max_scroll_y: usize,
) {
    let indicator_style = Style::default().fg(theme::TEXT_DIM);

    if scroll_y > 0 {
        let label = " \u{25B2} ";
        let x_pos = area.x + (area.width.saturating_sub(label.len() as u16)) / 2;
        frame.render_widget(
            Paragraph::new(Span::styled(label, indicator_style)),
            Rect::new(x_pos, area.y, label.len() as u16, 1),
        );
    }

    if scroll_y < max_scroll_y {
        let label = " \u{25BC} ";
        let x_pos = area.x + (area.width.saturating_sub(label.len() as u16)) / 2;
        let y_pos = area.y + area.height.saturating_sub(1);
        frame.render_widget(
            Paragraph::new(Span::styled(label, indicator_style)),
            Rect::new(x_pos, y_pos, label.len() as u16, 1),
        );
    }

    if scroll_x > 0 {
        let y_pos = area.y + area.height / 2;
        frame.render_widget(
            Paragraph::new(Span::styled("\u{25C0}", indicator_style)),
            Rect::new(area.x, y_pos, 1, 1),
        );
    }

    if scroll_x < max_scroll_x {
        let y_pos = area.y + area.height / 2;
        let x_pos = area.x + area.width.saturating_sub(1);
        frame.render_widget(
            Paragraph::new(Span::styled("\u{25B6}", indicator_style)),
            Rect::new(x_pos, y_pos, 1, 1),
        );
    }
}

fn draw_dual_pane_graph(frame: &mut Frame, app: &App) {
    // Fill the entire background
    let bg_block = Block::default().style(Style::default().bg(theme::BG));
    frame.render_widget(bg_block, frame.area());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(0),   // graph panes
            Constraint::Length(1), // summary bar
            Constraint::Length(3), // footer
        ])
        .split(frame.area());

    // Header with [DUAL] indicator
    draw_dual_header(frame, chunks[0]);

    // Horizontal 50/50 split for graph panes
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    // Left pane: Epics
    let left_focused = app.active_pane == GraphPane::Left;
    let left_block = panel_block(" Epics ", left_focused);
    let left_inner = left_block.inner(panes[0]);
    frame.render_widget(left_block, panes[0]);
    draw_graph_pane(frame, app, left_inner, &GraphPaneParams {
        cache: app.epic_graph_cache.as_ref(),
        level: GraphLevel::Epic,
        scroll_x: app.epic_scroll_x,
        scroll_y: app.epic_scroll_y,
        focused_node_id: app.epic_focused_node.as_deref(),
        max_scroll_out: &app.epic_max_scroll,
    });

    // Right pane: Tasks
    let right_focused = app.active_pane == GraphPane::Right;
    let right_title = match app.selected_epic() {
        Some(epic) => format!(" Tasks: {} ", epic.title),
        None => " Tasks ".to_string(),
    };
    let right_block = panel_block(&right_title, right_focused);
    let right_inner = right_block.inner(panes[1]);
    frame.render_widget(right_block, panes[1]);
    draw_graph_pane(frame, app, right_inner, &GraphPaneParams {
        cache: app.task_graph_cache.as_ref(),
        level: GraphLevel::Task,
        scroll_x: app.task_scroll_x,
        scroll_y: app.task_scroll_y,
        focused_node_id: app.task_focused_node.as_deref(),
        max_scroll_out: &app.task_max_scroll,
    });

    // Summary bar
    draw_dual_summary(frame, app, chunks[2]);

    // Footer
    draw_graph_footer(frame, chunks[3]);
}

fn draw_dual_header(frame: &mut Frame, area: Rect) {
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            "  \u{2593}\u{2593} DEPENDENCY GRAPH \u{2593}\u{2593} ",
            Style::default()
                .fg(theme::NEON_CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("\u{2502} ", Style::default().fg(theme::BORDER_DIM)),
        Span::styled(
            "[DUAL]",
            Style::default()
                .fg(theme::NEON_MAGENTA)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::panel_border(false))
            .style(Style::default().bg(theme::BG)),
    );
    frame.render_widget(header, area);
}

fn cache_stats_span(label: &str, cache: Option<&GraphCache>) -> Span<'static> {
    match cache {
        Some(c) => {
            let s = compute_graph_summary(c);
            Span::styled(
                format!("{label}: {} nodes, {} edges", s.total_nodes, s.total_edges),
                Style::default().fg(theme::NEON_CYAN),
            )
        }
        None => Span::styled(
            format!("{label}: --"),
            Style::default().fg(theme::TEXT_DIM),
        ),
    }
}

fn draw_dual_summary(frame: &mut Frame, app: &App, area: Rect) {
    let sep = Style::default().fg(theme::TEXT_DIM);

    let pane_label = match app.active_pane {
        GraphPane::Left => "Active: Epics",
        GraphPane::Right => "Active: Tasks",
    };

    let line = Line::from(vec![
        Span::styled("  ", sep),
        cache_stats_span("Epics", app.epic_graph_cache.as_ref()),
        Span::styled(" \u{2502} ", sep),
        cache_stats_span("Tasks", app.task_graph_cache.as_ref()),
        Span::styled(" \u{2502} ", sep),
        Span::styled(pane_label, Style::default().fg(theme::NEON_MAGENTA)),
    ]);

    let summary = Paragraph::new(line).style(Style::default().bg(theme::BG));
    frame.render_widget(summary, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::graph::{DagLayout, Edge, Node};

    fn test_node(id: &str, status: ItemStatus) -> Node {
        Node {
            id: id.to_string(),
            label: id.to_string(),
            status,
            layer: None,
            x_position: 0,
        }
    }

    fn test_edge(from: &str, to: &str) -> Edge {
        Edge {
            from: from.to_string(),
            to: to.to_string(),
        }
    }

    fn test_cache(nodes: Vec<Node>, edges: Vec<Edge>, level: GraphLevel) -> GraphCache {
        GraphCache {
            layout: DagLayout::new(nodes, edges),
            node_positions: HashMap::new(),
            level,
        }
    }

    #[test]
    fn summary_counts_empty_graph() {
        let cache = test_cache(vec![], vec![], GraphLevel::Epic);
        let summary = compute_graph_summary(&cache);
        assert_eq!(summary.total_nodes, 0);
        assert_eq!(summary.total_edges, 0);
        assert_eq!(summary.blocked_count, 0);
        assert_eq!(summary.done_count, 0);
    }

    #[test]
    fn summary_counts_all_done() {
        let cache = test_cache(
            vec![
                test_node("A", ItemStatus::Done),
                test_node("B", ItemStatus::Done),
            ],
            vec![test_edge("A", "B")],
            GraphLevel::Epic,
        );
        let summary = compute_graph_summary(&cache);
        assert_eq!(summary.total_nodes, 2);
        assert_eq!(summary.total_edges, 1);
        assert_eq!(summary.done_count, 2);
        assert_eq!(summary.blocked_count, 0);
    }

    #[test]
    fn summary_counts_blocked_nodes() {
        // A (todo) -> B (todo): B is blocked by non-done A
        let cache = test_cache(
            vec![
                test_node("A", ItemStatus::Todo),
                test_node("B", ItemStatus::Todo),
            ],
            vec![test_edge("A", "B")],
            GraphLevel::Task,
        );
        let summary = compute_graph_summary(&cache);
        assert_eq!(summary.total_nodes, 2);
        assert_eq!(summary.total_edges, 1);
        assert_eq!(summary.blocked_count, 1); // B is blocked
        assert_eq!(summary.done_count, 0);
    }

    #[test]
    fn summary_not_blocked_when_blocker_is_done() {
        // A (done) -> B (todo): B is NOT blocked because A is done
        let cache = test_cache(
            vec![
                test_node("A", ItemStatus::Done),
                test_node("B", ItemStatus::Todo),
            ],
            vec![test_edge("A", "B")],
            GraphLevel::Task,
        );
        let summary = compute_graph_summary(&cache);
        assert_eq!(summary.blocked_count, 0);
        assert_eq!(summary.done_count, 1);
    }

    #[test]
    fn summary_done_node_not_counted_as_blocked() {
        // A (todo) -> B (done): B is done, so not blocked
        let cache = test_cache(
            vec![
                test_node("A", ItemStatus::Todo),
                test_node("B", ItemStatus::Done),
            ],
            vec![test_edge("A", "B")],
            GraphLevel::Epic,
        );
        let summary = compute_graph_summary(&cache);
        assert_eq!(summary.blocked_count, 0);
        assert_eq!(summary.done_count, 1);
    }

    #[test]
    fn summary_mixed_statuses() {
        // A (done) -> C (todo), B (in_progress) -> C (todo)
        // C is blocked because B (non-done) points to C
        let cache = test_cache(
            vec![
                test_node("A", ItemStatus::Done),
                test_node("B", ItemStatus::InProgress),
                test_node("C", ItemStatus::Todo),
            ],
            vec![test_edge("A", "C"), test_edge("B", "C")],
            GraphLevel::Task,
        );
        let summary = compute_graph_summary(&cache);
        assert_eq!(summary.total_nodes, 3);
        assert_eq!(summary.total_edges, 2);
        assert_eq!(summary.done_count, 1);
        assert_eq!(summary.blocked_count, 1); // C blocked by B
    }

    #[test]
    fn summary_label_epic() {
        let cache = test_cache(
            vec![test_node("A", ItemStatus::Todo)],
            vec![],
            GraphLevel::Epic,
        );
        let line = graph_summary_line(&cache);
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains("epics"), "expected 'epics' in: {text}");
    }

    #[test]
    fn summary_label_task() {
        let cache = test_cache(
            vec![test_node("A", ItemStatus::Todo)],
            vec![],
            GraphLevel::Task,
        );
        let line = graph_summary_line(&cache);
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains("tasks"), "expected 'tasks' in: {text}");
    }

    #[test]
    fn summary_blocked_color_orange_when_nonzero() {
        let cache = test_cache(
            vec![
                test_node("A", ItemStatus::Todo),
                test_node("B", ItemStatus::Todo),
            ],
            vec![test_edge("A", "B")],
            GraphLevel::Task,
        );
        let line = graph_summary_line(&cache);
        let blocked_span = line.spans.iter().find(|s| s.content.contains("blocked")).unwrap();
        assert_eq!(blocked_span.style.fg, Some(theme::NEON_ORANGE));
    }

    #[test]
    fn summary_blocked_color_dim_when_zero() {
        let cache = test_cache(
            vec![test_node("A", ItemStatus::Done)],
            vec![],
            GraphLevel::Epic,
        );
        let line = graph_summary_line(&cache);
        let blocked_span = line.spans.iter().find(|s| s.content.contains("blocked")).unwrap();
        assert_eq!(blocked_span.style.fg, Some(theme::TEXT_DIM));
    }

    #[test]
    fn summary_done_color_green() {
        let cache = test_cache(
            vec![test_node("A", ItemStatus::Done)],
            vec![],
            GraphLevel::Epic,
        );
        let line = graph_summary_line(&cache);
        let done_span = line.spans.iter().find(|s| s.content.contains("done")).unwrap();
        assert_eq!(done_span.style.fg, Some(theme::NEON_GREEN));
    }

    // ==================== Dual summary tests ====================

    fn dual_summary_text(
        epic_cache: Option<&GraphCache>,
        task_cache: Option<&GraphCache>,
        active_pane: GraphPane,
    ) -> String {
        let sep = Style::default().fg(theme::TEXT_DIM);
        let pane_label = match active_pane {
            GraphPane::Left => "Active: Epics",
            GraphPane::Right => "Active: Tasks",
        };

        let spans = vec![
            Span::styled("  ", sep),
            cache_stats_span("Epics", epic_cache),
            Span::styled(" \u{2502} ", sep),
            cache_stats_span("Tasks", task_cache),
            Span::styled(" \u{2502} ", sep),
            Span::styled(pane_label, Style::default().fg(theme::NEON_MAGENTA)),
        ];

        spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn dual_summary_renders_both_pane_stats() {
        let epic_cache = test_cache(
            vec![test_node("A", ItemStatus::Todo), test_node("B", ItemStatus::Done)],
            vec![test_edge("A", "B")],
            GraphLevel::Epic,
        );
        let task_cache = test_cache(
            vec![test_node("T1", ItemStatus::Todo)],
            vec![],
            GraphLevel::Task,
        );
        let text = dual_summary_text(Some(&epic_cache), Some(&task_cache), GraphPane::Left);
        assert!(text.contains("Epics: 2 nodes, 1 edges"), "got: {text}");
        assert!(text.contains("Tasks: 1 nodes, 0 edges"), "got: {text}");
        assert!(text.contains("Active: Epics"), "got: {text}");
    }

    #[test]
    fn dual_summary_shows_dashes_when_task_cache_none() {
        let epic_cache = test_cache(
            vec![test_node("A", ItemStatus::Todo)],
            vec![],
            GraphLevel::Epic,
        );
        let text = dual_summary_text(Some(&epic_cache), None, GraphPane::Right);
        assert!(text.contains("Tasks: --"), "got: {text}");
        assert!(text.contains("Active: Tasks"), "got: {text}");
    }

    // ── Short-ID display tests ────────────────────────────────────────

    use crate::models::{BlueTask, Epic};

    fn epic_row_spans(epic: &Epic, is_selected: bool) -> Vec<Span<'_>> {
        let (marker, marker_style, title_style) = selection_styles(is_selected);
        let symbol = theme::status_symbol(&epic.status);
        let status_style = theme::status_style(&epic.status);

        let mut spans = vec![
            Span::styled(marker, marker_style),
            Span::styled(format!("{symbol} "), status_style),
        ];
        spans.extend(short_id_span(epic.short_id.as_deref()));
        spans.push(Span::styled(&epic.title, title_style));
        spans
    }

    fn task_row_spans(task: &BlueTask, is_selected: bool) -> Vec<Span<'_>> {
        let (marker, marker_style, title_style) = selection_styles(is_selected);
        let symbol = theme::status_symbol(&task.status);
        let status_style = theme::status_style(&task.status);

        let mut spans = vec![
            Span::styled(marker, marker_style),
            Span::styled(format!("{symbol} "), status_style),
        ];
        spans.extend(short_id_span(task.short_id.as_deref()));
        spans.push(Span::styled(&task.title, title_style));
        spans
    }

    fn task_detail_header_spans(task: &BlueTask) -> Vec<Span<'_>> {
        let mut spans: Vec<Span> = Vec::new();
        spans.extend(short_id_span(task.short_id.as_deref()));
        spans.push(Span::styled(
            &task.title,
            Style::default()
                .fg(theme::NEON_CYAN)
                .add_modifier(Modifier::BOLD),
        ));
        spans
    }

    fn stub_epic(short_id: Option<&str>) -> Epic {
        Epic {
            id: "e1".into(),
            project_id: "p1".into(),
            title: "Test Epic".into(),
            description: String::new(),
            status: ItemStatus::Todo,
            short_id: short_id.map(String::from),
            created_at: String::new(),
            updated_at: String::new(),
            task_count: 0,
            done_count: 0,
        }
    }

    fn stub_task(short_id: Option<&str>) -> BlueTask {
        BlueTask {
            id: "t1".into(),
            epic_id: "e1".into(),
            title: "Test Task".into(),
            description: String::new(),
            status: ItemStatus::Todo,
            short_id: short_id.map(String::from),
            created_at: String::new(),
            updated_at: String::new(),
            session_id: None,
        }
    }

    #[test]
    fn epic_list_shows_short_id_before_title() {
        let epic = stub_epic(Some("E1"));
        let spans = epic_row_spans(&epic, true);
        let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(
            text.contains("[E1] Test Epic"),
            "short ID should precede title, got: {text}"
        );
    }

    #[test]
    fn task_list_shows_short_id_before_title() {
        let task = stub_task(Some("E1-T3"));
        let spans = task_row_spans(&task, true);
        let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(
            text.contains("[E1-T3] Test Task"),
            "short ID should precede title, got: {text}"
        );
    }

    #[test]
    fn no_short_id_span_when_none() {
        let epic = stub_epic(None);
        let spans = epic_row_spans(&epic, false);
        let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(
            !text.contains('['),
            "no bracket should appear when short_id is None, got: {text}"
        );

        let task = stub_task(None);
        let spans = task_row_spans(&task, false);
        let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(
            !text.contains('['),
            "no bracket should appear when short_id is None, got: {text}"
        );
    }

    #[test]
    fn short_id_span_uses_text_dim_color() {
        let epic = stub_epic(Some("E1"));
        let spans = epic_row_spans(&epic, true);
        let sid_span = spans.iter().find(|s| s.content.contains("[E1]")).unwrap();
        assert_eq!(
            sid_span.style.fg,
            Some(theme::TEXT_DIM),
            "short ID span should use TEXT_DIM"
        );

        let task = stub_task(Some("E1-T3"));
        let spans = task_row_spans(&task, true);
        let sid_span = spans
            .iter()
            .find(|s| s.content.contains("[E1-T3]"))
            .unwrap();
        assert_eq!(
            sid_span.style.fg,
            Some(theme::TEXT_DIM),
            "short ID span should use TEXT_DIM"
        );
    }

    #[test]
    fn task_detail_header_shows_short_id() {
        let task = stub_task(Some("E2-T5"));
        let spans = task_detail_header_spans(&task);
        let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(
            text.contains("[E2-T5] Test Task"),
            "detail header should show short ID, got: {text}"
        );
        let sid_span = spans
            .iter()
            .find(|s| s.content.contains("[E2-T5]"))
            .unwrap();
        assert_eq!(sid_span.style.fg, Some(theme::TEXT_DIM));
    }

    #[test]
    fn task_detail_header_no_short_id_when_none() {
        let task = stub_task(None);
        let spans = task_detail_header_spans(&task);
        assert_eq!(spans.len(), 1, "only the title span when short_id is None");
        assert_eq!(spans[0].content.as_ref(), "Test Task");
    }
}
