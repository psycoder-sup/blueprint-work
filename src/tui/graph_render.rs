//! ASCII box-node renderer for the dependency graph.
//!
//! Provides a 2D character buffer ([`Canvas`]) and box-node rendering
//! ([`NodeBox`] / [`render_node`]) so that graph nodes can be composited
//! at arbitrary positions before being painted to a ratatui frame.

use std::collections::{HashMap, HashSet};

use ratatui::style::Style;

use crate::models::ItemStatus;
use super::graph::DagLayout;
use super::theme;

// ── Constants ────────────────────────────────────────────────────────

/// Total width of a rendered node box (including border characters).
pub const NODE_WIDTH: usize = 22;

/// Height of a task node (top border + title + bottom border).
pub const NODE_HEIGHT_TASK: usize = 3;

/// Height of an epic node (top border + title + progress + bottom border).
pub const NODE_HEIGHT_EPIC: usize = 4;

/// Interior width available for content (NODE_WIDTH minus the two border columns).
const INNER_WIDTH: usize = NODE_WIDTH - 2;

// ── Cell ─────────────────────────────────────────────────────────────

/// A single character cell on the canvas.
#[derive(Debug, Clone)]
pub struct Cell {
    pub ch: char,
    pub style: Style,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            style: Style::default(),
        }
    }
}

// ── Canvas ───────────────────────────────────────────────────────────

/// A 2D grid of [`Cell`]s that nodes (and later edges) are drawn onto.
#[derive(Debug)]
pub struct Canvas {
    pub width: usize,
    pub height: usize,
    cells: Vec<Cell>,
}

impl Canvas {
    /// Create a blank canvas filled with space characters.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![Cell::default(); width * height],
        }
    }

    /// Place a single character at `(x, y)`. Out-of-bounds writes are silently
    /// ignored.
    pub fn put_char(&mut self, x: usize, y: usize, ch: char, style: Style) {
        if x < self.width && y < self.height {
            let idx = y * self.width + x;
            self.cells[idx] = Cell { ch, style };
        }
    }

    /// Write a string horizontally starting at `(x, y)`.  Characters that
    /// fall outside the canvas are silently clipped.
    pub fn put_str(&mut self, x: usize, y: usize, s: &str, style: Style) {
        for (i, ch) in s.chars().enumerate() {
            self.put_char(x + i, y, ch, style);
        }
    }

    /// Read the cell at `(x, y)`.
    ///
    /// # Panics
    /// Panics if `(x, y)` is out of bounds.
    pub fn get(&self, x: usize, y: usize) -> &Cell {
        assert!(
            x < self.width && y < self.height,
            "Canvas::get out of bounds"
        );
        &self.cells[y * self.width + x]
    }
}

// ── NodeBox ──────────────────────────────────────────────────────────

/// Rendering descriptor for a single graph node.
#[derive(Debug, Clone)]
pub struct NodeBox {
    /// Display title (will be truncated if too long).
    pub title: String,
    /// Current status -- determines border style/color and symbol.
    pub status: ItemStatus,
    /// For epic nodes: `(done_count, total_count)` to render a progress bar.
    /// `None` for plain task nodes.
    pub progress: Option<(usize, usize)>,
    /// Top-left X position on the canvas.
    pub x: usize,
    /// Top-left Y position on the canvas.
    pub y: usize,
    /// Whether this node is blocked by unfinished dependencies.
    pub blocked: bool,
}

// ── Border helpers ───────────────────────────────────────────────────

/// Return the ratatui [`Style`] for a node's border based on its status.
///
/// When `blocked` is true, the border color pulses between bright orange
/// and dim orange using the global `animation_frame` counter.
pub fn border_style(status: &ItemStatus, animation_frame: u8, blocked: bool) -> Style {
    if blocked {
        let bright = (animation_frame / 12) % 2 == 0;
        let color = if bright {
            theme::NEON_ORANGE
        } else {
            theme::DARK_ORANGE
        };
        return Style::default().fg(color);
    }
    match status {
        ItemStatus::Done => Style::default().fg(theme::NEON_GREEN),
        ItemStatus::Todo => Style::default().fg(theme::TEXT_DIM),
        ItemStatus::InProgress => Style::default().fg(theme::NEON_CYAN),
    }
}

/// Border character set for a given status.
#[derive(Clone, Copy)]
struct BorderChars {
    tl: char,
    tr: char,
    bl: char,
    br: char,
    h: char,
    v: char,
}

/// Double-line border set used for TODO, DONE, and blocked nodes.
const DOUBLE_LINE_BORDERS: BorderChars = BorderChars {
    tl: '\u{2554}', // ╔
    tr: '\u{2557}', // ╗
    bl: '\u{255A}', // ╚
    br: '\u{255D}', // ╝
    h: '\u{2550}',  // ═
    v: '\u{2551}',  // ║
};

fn border_chars(status: &ItemStatus, animation_frame: u8, blocked: bool) -> BorderChars {
    if blocked {
        return DOUBLE_LINE_BORDERS;
    }
    match status {
        // IN_PROGRESS uses rounded corners with animated dashed borders.
        ItemStatus::InProgress => {
            let (h, v) = match animation_frame % 4 {
                0 | 2 => ('\u{254C}', '\u{254E}'), // ╌ ╎
                _ => ('\u{2504}', '\u{2506}'),      // ┄ ┆
            };
            BorderChars {
                tl: '\u{256D}', // ╭
                tr: '\u{256E}', // ╮
                bl: '\u{2570}', // ╰
                br: '\u{256F}', // ╯
                h,
                v,
            }
        }
        _ => DOUBLE_LINE_BORDERS,
    }
}

// ── Marching border ─────────────────────────────────────────────────

/// Compute the character and style for a single marching-border cell at
/// the given `perimeter_index`.  The pattern has period 6: 3 bright cells
/// (solid line, NEON_CYAN) followed by 3 dim cells (dashed line, BORDER_DIM).
///
/// `is_horizontal` selects the line character orientation.
fn marching_cell(perimeter_index: usize, animation_frame: u8, is_horizontal: bool) -> (char, Style) {
    let phase = (perimeter_index + animation_frame as usize) % 6;
    if phase < 3 {
        // Bright segment: solid line
        let ch = if is_horizontal { '\u{2500}' } else { '\u{2502}' }; // ─ │
        (ch, Style::default().fg(theme::NEON_CYAN))
    } else {
        // Dim segment: dashed line
        let ch = if is_horizontal { '\u{254C}' } else { '\u{254E}' }; // ╌ ╎
        (ch, Style::default().fg(theme::BORDER_DIM))
    }
}

/// Render the marching-ants border for an InProgress, non-blocked node.
///
/// Walks the perimeter clockwise assigning each cell a sequential index:
///   top-left corner (0) → top edge → top-right corner → right edge →
///   bottom-right corner → bottom edge (reversed) → bottom-left corner →
///   left edge (reversed).
fn render_marching_border(canvas: &mut Canvas, x: usize, y: usize, node_height: usize, animation_frame: u8) {
    let mut p: usize = 0;

    // --- Top-left corner (index 0) ---
    let (_, corner_style) = marching_cell(p, animation_frame, true);
    canvas.put_char(x, y, '\u{256D}', corner_style); // ╭
    p += 1;

    // --- Top edge (indices 1 .. NODE_WIDTH-2) ---
    for i in 1..NODE_WIDTH - 1 {
        let (ch, st) = marching_cell(p, animation_frame, true);
        canvas.put_char(x + i, y, ch, st);
        p += 1;
    }

    // --- Top-right corner ---
    let (_, corner_style) = marching_cell(p, animation_frame, true);
    canvas.put_char(x + NODE_WIDTH - 1, y, '\u{256E}', corner_style); // ╮
    p += 1;

    // --- Right edge (top+1 .. bottom-1) ---
    for row in 1..node_height - 1 {
        let (ch, st) = marching_cell(p, animation_frame, false);
        canvas.put_char(x + NODE_WIDTH - 1, y + row, ch, st);
        p += 1;
    }

    // --- Bottom-right corner ---
    let (_, corner_style) = marching_cell(p, animation_frame, true);
    canvas.put_char(x + NODE_WIDTH - 1, y + node_height - 1, '\u{256F}', corner_style); // ╯
    p += 1;

    // --- Bottom edge (reversed: right-to-left, indices along the bottom) ---
    for i in (1..NODE_WIDTH - 1).rev() {
        let (ch, st) = marching_cell(p, animation_frame, true);
        canvas.put_char(x + i, y + node_height - 1, ch, st);
        p += 1;
    }

    // --- Bottom-left corner ---
    let (_, corner_style) = marching_cell(p, animation_frame, true);
    canvas.put_char(x, y + node_height - 1, '\u{2570}', corner_style); // ╰
    p += 1;

    // --- Left edge (reversed: bottom-1 .. top+1) ---
    for row in (1..node_height - 1).rev() {
        let (ch, st) = marching_cell(p, animation_frame, false);
        canvas.put_char(x, y + row, ch, st);
        p += 1;
    }
}

// ── Rendering ────────────────────────────────────────────────────────

/// Render a node box onto the canvas at the position specified in `node_box`.
///
/// `animation_frame` is the global animation counter (0–5) used for the
/// marching border on in-progress nodes and the pulsing color effect
/// on blocked nodes.
pub fn render_node(canvas: &mut Canvas, node_box: &NodeBox, animation_frame: u8) {
    let is_marching = node_box.status == ItemStatus::InProgress && !node_box.blocked;

    let node_height = if node_box.progress.is_some() {
        NODE_HEIGHT_EPIC
    } else {
        NODE_HEIGHT_TASK
    };

    if is_marching {
        // Positionally-aware marching border
        render_marching_border(canvas, node_box.x, node_box.y, node_height, animation_frame);
    } else {
        // Uniform border for Todo / Done / blocked
        let bstyle = border_style(&node_box.status, animation_frame, node_box.blocked);
        let bc = border_chars(&node_box.status, animation_frame, node_box.blocked);
        let x = node_box.x;
        let y = node_box.y;

        // Top border
        canvas.put_char(x, y, bc.tl, bstyle);
        for i in 1..NODE_WIDTH - 1 {
            canvas.put_char(x + i, y, bc.h, bstyle);
        }
        canvas.put_char(x + NODE_WIDTH - 1, y, bc.tr, bstyle);

        // Side borders for inner rows
        let bottom_y = y + node_height - 1;
        for row in 1..node_height - 1 {
            canvas.put_char(x, y + row, bc.v, bstyle);
            canvas.put_char(x + NODE_WIDTH - 1, y + row, bc.v, bstyle);
        }

        // Bottom border
        canvas.put_char(x, bottom_y, bc.bl, bstyle);
        for i in 1..NODE_WIDTH - 1 {
            canvas.put_char(x + i, bottom_y, bc.h, bstyle);
        }
        canvas.put_char(x + NODE_WIDTH - 1, bottom_y, bc.br, bstyle);
    }

    // ── Content (shared by both paths) ──

    let x = node_box.x;
    let y = node_box.y;
    let content_style = if is_marching {
        Style::default().fg(theme::NEON_CYAN)
    } else {
        border_style(&node_box.status, animation_frame, node_box.blocked)
    };

    // Title line
    let title_y = y + 1;

    let symbol = theme::status_symbol(&node_box.status);
    let sym_style = theme::status_style(&node_box.status);

    let symbol_display_width: usize = 1;
    let title_budget = INNER_WIDTH
        .saturating_sub(1)
        .saturating_sub(symbol_display_width)
        .saturating_sub(1);

    let truncated_title = truncate_with_ellipsis(&node_box.title, title_budget);

    // Leading space
    canvas.put_char(x + 1, title_y, ' ', content_style);

    // Status symbol
    let symbol_x = x + 2;
    canvas.put_str(symbol_x, title_y, symbol, sym_style);

    // Space after symbol
    canvas.put_char(symbol_x + symbol_display_width, title_y, ' ', content_style);

    // Title text
    let title_x = symbol_x + symbol_display_width + 1;
    let title_style = Style::default().fg(theme::TEXT_BRIGHT);
    canvas.put_str(title_x, title_y, &truncated_title, title_style);

    // Fill remaining inner space
    let used = 1 + symbol_display_width + 1 + truncated_title.chars().count();
    for i in used..INNER_WIDTH {
        canvas.put_char(x + 1 + i, title_y, ' ', content_style);
    }

    // Progress line (epic nodes only)
    if let Some((done, total)) = node_box.progress {
        let progress_y = y + 2;

        let bar_width = INNER_WIDTH.saturating_sub(4);
        let bar = theme::progress_bar(done, total, bar_width);

        canvas.put_char(x + 1, progress_y, ' ', content_style);
        canvas.put_char(x + 2, progress_y, '[', content_style);
        let bar_style = Style::default().fg(theme::NEON_GREEN);
        canvas.put_str(x + 3, progress_y, &bar, bar_style);
        canvas.put_char(x + 3 + bar_width, progress_y, ']', content_style);

        let used_progress = 1 + 1 + bar_width + 1;
        for i in (1 + used_progress)..INNER_WIDTH {
            canvas.put_char(x + 1 + i, progress_y, ' ', content_style);
        }
    }
}

// ── Edge rendering ──────────────────────────────────────────────────

/// Draw directed edges between connected nodes on the canvas.
///
/// Edges route from the bottom-center of the source node to the top-center
/// of the target node, using straight vertical lines for same-column edges
/// and L/Z-shaped routing for cross-column edges.
///
/// `node_height` determines the vertical offset from each source node's
/// top-left corner to the edge departure point (bottom-center).
///
/// Existing non-space characters (i.e. node content) are never overwritten.
pub fn render_edges(
    canvas: &mut Canvas,
    layout: &DagLayout,
    node_positions: &HashMap<String, (usize, usize)>,
    blocked_ids: &HashSet<String>,
    node_height: usize,
) {
    for edge in &layout.edges {
        let Some(&(from_x, from_y)) = node_positions.get(&edge.from) else {
            continue;
        };
        let Some(&(to_x, to_y)) = node_positions.get(&edge.to) else {
            continue;
        };

        let style = if blocked_ids.contains(&edge.to) {
            Style::default().fg(theme::NEON_PINK)
        } else {
            Style::default().fg(theme::NEON_CYAN)
        };

        // Source: bottom-center of `from` node.
        let src_x = from_x + NODE_WIDTH / 2;
        let src_y = from_y + node_height; // one row below bottom border

        // Target: top-center of `to` node, one row above.
        let dst_x = to_x + NODE_WIDTH / 2;
        let dst_y = to_y.saturating_sub(1);

        if src_y > dst_y {
            continue; // degenerate: target above source
        }

        if src_x == dst_x {
            // Straight vertical edge.
            for y in src_y..dst_y {
                put_edge_char(canvas, src_x, y, '\u{2502}', style); // │
            }
            put_edge_char(canvas, dst_x, dst_y, '\u{25BC}', style); // ▼
        } else {
            // L/Z-shaped routing.
            // Step 1: one cell down from source.
            put_edge_char(canvas, src_x, src_y, '\u{2502}', style); // │

            // Step 2: horizontal row at src_y + 1.
            let mid_y = src_y + 1;

            // Corner at the turn from vertical to horizontal.
            if dst_x > src_x {
                put_edge_char(canvas, src_x, mid_y, '\u{2570}', style); // ╰
            } else {
                put_edge_char(canvas, src_x, mid_y, '\u{256F}', style); // ╯
            }

            // Horizontal segment.
            let (hx_start, hx_end) = if dst_x > src_x {
                (src_x + 1, dst_x)
            } else {
                (dst_x + 1, src_x)
            };
            for x in hx_start..hx_end {
                put_edge_char(canvas, x, mid_y, '\u{2500}', style); // ─
            }

            // Corner at the turn from horizontal to vertical towards target.
            if dst_x > src_x {
                put_edge_char(canvas, dst_x, mid_y, '\u{256E}', style); // ╮
            } else {
                put_edge_char(canvas, dst_x, mid_y, '\u{256D}', style); // ╭
            }

            // Vertical segment down to target.
            for y in (mid_y + 1)..dst_y {
                put_edge_char(canvas, dst_x, y, '\u{2502}', style); // │
            }

            // Arrow head at target.
            put_edge_char(canvas, dst_x, dst_y, '\u{25BC}', style); // ▼
        }
    }
}

/// Render a focus highlight as a 1-cell outer glow around a node.
///
/// Called after `render_node()` to draw a rounded single-line border one cell
/// outside the node, leaving the inner status-based border fully visible.
pub fn render_focus_highlight(canvas: &mut Canvas, x: usize, y: usize, node_height: usize) {
    let style = Style::default()
        .fg(theme::NEON_MAGENTA)
        .add_modifier(ratatui::style::Modifier::BOLD);

    let outer_w = NODE_WIDTH + 2;
    // Use wrapping_sub to get usize coordinates; put_char clips out-of-bounds.
    let ox = x.wrapping_sub(1);
    let oy = y.wrapping_sub(1);

    // Top border: ╭───╮ at oy
    canvas.put_char(ox, oy, '\u{256D}', style); // ╭
    for i in 1..outer_w - 1 {
        canvas.put_char(ox + i, oy, '\u{2500}', style); // ─
    }
    canvas.put_char(ox + outer_w - 1, oy, '\u{256E}', style); // ╮

    // Side borders: │ at ox and ox+outer_w-1 for each row of the node
    for row in 0..node_height {
        canvas.put_char(ox, y + row, '\u{2502}', style); // │
        canvas.put_char(ox + outer_w - 1, y + row, '\u{2502}', style); // │
    }

    // Bottom border: ╰───╯ at y+node_height
    let bottom_oy = y + node_height;
    canvas.put_char(ox, bottom_oy, '\u{2570}', style); // ╰
    for i in 1..outer_w - 1 {
        canvas.put_char(ox + i, bottom_oy, '\u{2500}', style); // ─
    }
    canvas.put_char(ox + outer_w - 1, bottom_oy, '\u{256F}', style); // ╯
}

/// Place an edge character on the canvas, but only if the cell is currently a space.
fn put_edge_char(canvas: &mut Canvas, x: usize, y: usize, ch: char, style: Style) {
    if x >= canvas.width || y >= canvas.height {
        return;
    }
    if canvas.get(x, y).ch != ' ' {
        return; // don't overwrite node content
    }
    canvas.put_char(x, y, ch, style);
}

/// Truncate `s` to at most `max_chars` characters, appending an ellipsis if needed.
fn truncate_with_ellipsis(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else if max_chars == 0 {
        String::new()
    } else {
        let mut result: String = s.chars().take(max_chars - 1).collect();
        result.push('\u{2026}');
        result
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn canvas_row(canvas: &Canvas, y: usize) -> String {
        (0..canvas.width).map(|x| canvas.get(x, y).ch).collect()
    }

    // ── Truncation ──────────────────────────────────────────────

    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(truncate_with_ellipsis("Hello", 10), "Hello");
    }

    #[test]
    fn truncate_exact_fit() {
        assert_eq!(truncate_with_ellipsis("12345", 5), "12345");
    }

    #[test]
    fn truncate_long_string_adds_ellipsis() {
        let result = truncate_with_ellipsis("Hello, World!", 5);
        assert_eq!(result, "Hell\u{2026}");
        assert_eq!(result.chars().count(), 5);
    }

    #[test]
    fn truncate_zero_budget() {
        assert_eq!(truncate_with_ellipsis("Hello", 0), "");
    }

    // ── Canvas basics ───────────────────────────────────────────

    #[test]
    fn canvas_defaults_to_spaces() {
        let c = Canvas::new(5, 3);
        assert_eq!(c.get(0, 0).ch, ' ');
        assert_eq!(c.get(4, 2).ch, ' ');
    }

    #[test]
    fn canvas_put_char_and_get() {
        let mut c = Canvas::new(10, 5);
        c.put_char(3, 2, 'X', Style::default());
        assert_eq!(c.get(3, 2).ch, 'X');
    }

    #[test]
    fn canvas_put_str() {
        let mut c = Canvas::new(10, 5);
        c.put_str(1, 0, "Hi", Style::default());
        assert_eq!(c.get(1, 0).ch, 'H');
        assert_eq!(c.get(2, 0).ch, 'i');
        assert_eq!(c.get(3, 0).ch, ' ');
    }

    #[test]
    fn canvas_out_of_bounds_write_does_not_panic() {
        let mut c = Canvas::new(5, 5);
        c.put_char(10, 10, 'X', Style::default());
        c.put_str(3, 0, "ABCDEFGH", Style::default());
        assert_eq!(c.get(3, 0).ch, 'A');
        assert_eq!(c.get(4, 0).ch, 'B');
    }

    // ── Node rendering: TODO status ─────────────────────────────

    #[test]
    fn render_todo_node_borders() {
        let mut canvas = Canvas::new(30, 5);
        let node = NodeBox {
            title: "Setup".to_string(),
            status: ItemStatus::Todo,
            progress: None,
            x: 0,
            y: 0,
            blocked: false,
        };
        render_node(&mut canvas, &node, 0);

        let row0 = canvas_row(&canvas, 0);
        assert!(row0.starts_with('\u{2554}'), "top-left should be double-line corner");
        assert!(row0.contains('\u{2557}'), "top-right should be double-line corner");

        let row2 = canvas_row(&canvas, 2);
        assert!(row2.starts_with('\u{255A}'), "bottom-left should be double-line corner");
        assert!(row2.contains('\u{255D}'), "bottom-right should be double-line corner");
    }

    #[test]
    fn render_todo_node_contains_symbol() {
        let mut canvas = Canvas::new(30, 5);
        let node = NodeBox {
            title: "Setup".to_string(),
            status: ItemStatus::Todo,
            progress: None,
            x: 0,
            y: 0,
            blocked: false,
        };
        render_node(&mut canvas, &node, 0);

        let row1 = canvas_row(&canvas, 1);
        assert!(row1.contains('\u{25A0}'), "TODO node should contain filled square symbol");
    }

    #[test]
    fn render_todo_node_has_dim_border_color() {
        let mut canvas = Canvas::new(30, 5);
        let node = NodeBox {
            title: "Setup".to_string(),
            status: ItemStatus::Todo,
            progress: None,
            x: 0,
            y: 0,
            blocked: false,
        };
        render_node(&mut canvas, &node, 0);

        let tl = canvas.get(0, 0);
        assert_eq!(tl.style.fg, Some(theme::TEXT_DIM));
    }

    // ── Node rendering: DONE status ─────────────────────────────

    #[test]
    fn render_done_node_borders_and_color() {
        let mut canvas = Canvas::new(30, 5);
        let node = NodeBox {
            title: "Init".to_string(),
            status: ItemStatus::Done,
            progress: None,
            x: 0,
            y: 0,
            blocked: false,
        };
        render_node(&mut canvas, &node, 0);

        let row0 = canvas_row(&canvas, 0);
        assert!(row0.starts_with('\u{2554}'));

        let row1 = canvas_row(&canvas, 1);
        assert!(row1.contains('\u{25C9}'), "DONE node should contain fisheye symbol");

        assert_eq!(canvas.get(0, 0).style.fg, Some(theme::NEON_GREEN));
    }

    // ── Node rendering: IN_PROGRESS status ──────────────────────

    #[test]
    fn render_in_progress_node_uses_rounded_borders() {
        let mut canvas = Canvas::new(30, 5);
        let node = NodeBox {
            title: "Work".to_string(),
            status: ItemStatus::InProgress,
            progress: None,
            x: 0,
            y: 0,
            blocked: false,
        };
        render_node(&mut canvas, &node, 0);

        let row0 = canvas_row(&canvas, 0);
        assert!(row0.starts_with('\u{256D}'), "IN_PROGRESS top-left should be rounded corner");
        assert!(row0.contains('\u{256E}'), "IN_PROGRESS top-right should be rounded corner");

        let row1 = canvas_row(&canvas, 1);
        assert!(row1.contains('\u{25B6}'), "IN_PROGRESS should contain play symbol");

        let row2 = canvas_row(&canvas, 2);
        assert!(row2.starts_with('\u{2570}'), "IN_PROGRESS bottom-left should be rounded corner");
        assert!(row2.contains('\u{256F}'), "IN_PROGRESS bottom-right should be rounded corner");

        assert_eq!(canvas.get(0, 0).style.fg, Some(theme::NEON_CYAN));
    }

    // ── Title truncation in node ────────────────────────────────

    #[test]
    fn render_node_truncates_long_title() {
        let mut canvas = Canvas::new(30, 5);
        let node = NodeBox {
            title: "This Is A Very Long Title That Should Be Truncated".to_string(),
            status: ItemStatus::Todo,
            progress: None,
            x: 0,
            y: 0,
            blocked: false,
        };
        render_node(&mut canvas, &node, 0);

        let row1 = canvas_row(&canvas, 1);
        assert!(
            row1.contains('\u{2026}'),
            "Long title should contain ellipsis: {row1}"
        );

        assert_eq!(row1.chars().count(), 30, "Row should be full canvas width");
    }

    // ── Progress bar in epic node ───────────────────────────────

    #[test]
    fn render_epic_node_has_progress_bar() {
        let mut canvas = Canvas::new(30, 6);
        let node = NodeBox {
            title: "Epic 1".to_string(),
            status: ItemStatus::InProgress,
            progress: Some((3, 10)),
            x: 0,
            y: 0,
            blocked: false,
        };
        render_node(&mut canvas, &node, 0);

        let row0 = canvas_row(&canvas, 0);
        assert!(row0.starts_with('\u{256D}'), "top border");

        let row2 = canvas_row(&canvas, 2);
        assert!(row2.contains('['), "progress line should have [");
        assert!(row2.contains(']'), "progress line should have ]");
        assert!(
            row2.contains('\u{2588}') || row2.contains('\u{2591}'),
            "progress bar chars"
        );

        let row3 = canvas_row(&canvas, 3);
        assert!(row3.starts_with('\u{2570}'), "bottom border at row 3 for epic node");
    }

    #[test]
    fn render_epic_node_full_progress() {
        let mut canvas = Canvas::new(30, 6);
        let node = NodeBox {
            title: "Done Epic".to_string(),
            status: ItemStatus::Done,
            progress: Some((5, 5)),
            x: 0,
            y: 0,
            blocked: false,
        };
        render_node(&mut canvas, &node, 0);

        let row2 = canvas_row(&canvas, 2);
        assert!(!row2.contains('\u{2591}'), "full progress should have no empty blocks");
        assert!(row2.contains('\u{2588}'), "full progress should have filled blocks");
    }

    // ── Multiple nodes on canvas ────────────────────────────────

    #[test]
    fn two_nodes_at_different_positions() {
        let mut canvas = Canvas::new(60, 10);

        let node_a = NodeBox {
            title: "Alpha".to_string(),
            status: ItemStatus::Done,
            progress: None,
            x: 0,
            y: 0,
            blocked: false,
        };
        let node_b = NodeBox {
            title: "Beta".to_string(),
            status: ItemStatus::Todo,
            progress: None,
            x: 30,
            y: 5,
            blocked: false,
        };

        render_node(&mut canvas, &node_a, 0);
        render_node(&mut canvas, &node_b, 0);

        assert_eq!(canvas.get(0, 0).ch, '\u{2554}');
        let row1_a = canvas_row(&canvas, 1);
        assert!(row1_a.contains('\u{25C9}'));

        assert_eq!(canvas.get(30, 5).ch, '\u{2554}');
        let row6 = canvas_row(&canvas, 6);
        assert!(row6.contains('\u{25A0}'));

        assert_eq!(canvas.get(30, 0).ch, ' ');
        assert_eq!(canvas.get(0, 5).ch, ' ');
    }

    // ── Node positioning ────────────────────────────────────────

    #[test]
    fn node_rendered_at_offset_position() {
        let mut canvas = Canvas::new(40, 10);
        let node = NodeBox {
            title: "Offset".to_string(),
            status: ItemStatus::Todo,
            progress: None,
            x: 5,
            y: 3,
            blocked: false,
        };
        render_node(&mut canvas, &node, 0);

        assert_eq!(canvas.get(5, 3).ch, '\u{2554}');
        assert_eq!(canvas.get(5 + NODE_WIDTH - 1, 3).ch, '\u{2557}');
        assert_eq!(canvas.get(5, 5).ch, '\u{255A}');
        assert_eq!(canvas.get(5 + NODE_WIDTH - 1, 5).ch, '\u{255D}');
    }

    // ── Border style function ───────────────────────────────────

    #[test]
    fn border_style_returns_correct_colors() {
        let done = border_style(&ItemStatus::Done, 0, false);
        assert_eq!(done.fg, Some(theme::NEON_GREEN));

        let todo = border_style(&ItemStatus::Todo, 0, false);
        assert_eq!(todo.fg, Some(theme::TEXT_DIM));

        let ip = border_style(&ItemStatus::InProgress, 0, false);
        assert_eq!(ip.fg, Some(theme::NEON_CYAN));
    }

    // ── Blocked node pulsing border ─────────────────────────────

    #[test]
    fn blocked_border_style_bright_on_first_half() {
        // (frame / 12) % 2 == 0 → bright (NEON_ORANGE)
        let style = border_style(&ItemStatus::Todo, 0, true);
        assert_eq!(style.fg, Some(theme::NEON_ORANGE));

        let style = border_style(&ItemStatus::Todo, 11, true);
        assert_eq!(style.fg, Some(theme::NEON_ORANGE));
    }

    #[test]
    fn blocked_border_style_dim_on_second_half() {
        // (frame / 12) % 2 == 1 → dim (DARK_ORANGE)
        let style = border_style(&ItemStatus::Todo, 12, true);
        assert_eq!(style.fg, Some(theme::DARK_ORANGE));

        let style = border_style(&ItemStatus::Todo, 23, true);
        assert_eq!(style.fg, Some(theme::DARK_ORANGE));
    }

    #[test]
    fn blocked_border_style_cycles_correctly() {
        // Full cycle: frames 0-11 bright, 12-23 dim, 24-35 bright, 36-47 dim
        let bright = theme::NEON_ORANGE;
        let dim = theme::DARK_ORANGE;

        assert_eq!(border_style(&ItemStatus::Todo, 0, true).fg, Some(bright));
        assert_eq!(border_style(&ItemStatus::Todo, 11, true).fg, Some(bright));
        assert_eq!(border_style(&ItemStatus::Todo, 12, true).fg, Some(dim));
        assert_eq!(border_style(&ItemStatus::Todo, 23, true).fg, Some(dim));
        assert_eq!(border_style(&ItemStatus::Todo, 24, true).fg, Some(bright));
        assert_eq!(border_style(&ItemStatus::Todo, 35, true).fg, Some(bright));
        assert_eq!(border_style(&ItemStatus::Todo, 36, true).fg, Some(dim));
        assert_eq!(border_style(&ItemStatus::Todo, 47, true).fg, Some(dim));
    }

    #[test]
    fn blocked_border_style_ignores_status() {
        // Blocked nodes should pulse regardless of their status
        let bright = theme::NEON_ORANGE;
        assert_eq!(border_style(&ItemStatus::Todo, 0, true).fg, Some(bright));
        assert_eq!(border_style(&ItemStatus::InProgress, 0, true).fg, Some(bright));
        assert_eq!(border_style(&ItemStatus::Done, 0, true).fg, Some(bright));
    }

    #[test]
    fn blocked_node_uses_double_line_borders() {
        let mut canvas = Canvas::new(30, 5);
        let node = NodeBox {
            title: "Blocked".to_string(),
            status: ItemStatus::InProgress,
            progress: None,
            x: 0,
            y: 0,
            blocked: true,
        };
        render_node(&mut canvas, &node, 0);

        let row0 = canvas_row(&canvas, 0);
        assert!(row0.starts_with('\u{2554}'), "blocked node should use double-line top-left");
        assert!(row0.contains('\u{2557}'), "blocked node should use double-line top-right");

        let row2 = canvas_row(&canvas, 2);
        assert!(row2.starts_with('\u{255A}'), "blocked node should use double-line bottom-left");
        assert!(row2.contains('\u{255D}'), "blocked node should use double-line bottom-right");
    }

    #[test]
    fn blocked_node_border_color_pulses() {
        // Bright phase (frame 0)
        let mut canvas = Canvas::new(30, 5);
        let node = NodeBox {
            title: "Blocked".to_string(),
            status: ItemStatus::Todo,
            progress: None,
            x: 0,
            y: 0,
            blocked: true,
        };
        render_node(&mut canvas, &node, 0);
        assert_eq!(canvas.get(0, 0).style.fg, Some(theme::NEON_ORANGE));

        // Dim phase (frame 12)
        let mut canvas = Canvas::new(30, 5);
        render_node(&mut canvas, &node, 12);
        assert_eq!(canvas.get(0, 0).style.fg, Some(theme::DARK_ORANGE));
    }

    // ── Marching border animation ─────────────────────────────────

    #[test]
    fn border_chars_todo_unchanged_by_animation_frame() {
        for frame in 0..6 {
            let bc = border_chars(&ItemStatus::Todo, frame, false);
            assert_eq!(bc.h, '\u{2550}', "TODO horizontal unchanged at frame {frame}");
            assert_eq!(bc.v, '\u{2551}', "TODO vertical unchanged at frame {frame}");
        }
    }

    #[test]
    fn border_chars_done_unchanged_by_animation_frame() {
        for frame in 0..6 {
            let bc = border_chars(&ItemStatus::Done, frame, false);
            assert_eq!(bc.h, '\u{2550}', "DONE horizontal unchanged at frame {frame}");
            assert_eq!(bc.v, '\u{2551}', "DONE vertical unchanged at frame {frame}");
        }
    }

    #[test]
    fn marching_cell_bright_when_phase_below_3() {
        // perimeter_index=0, frame=0 → phase = (0 + 0) % 6 = 0 → bright
        let (ch, st) = marching_cell(0, 0, true);
        assert_eq!(ch, '\u{2500}', "bright horizontal = solid ─");
        assert_eq!(st.fg, Some(theme::NEON_CYAN));
    }

    #[test]
    fn marching_cell_dim_when_phase_ge_3() {
        // perimeter_index=3, frame=0 → phase = (3 + 0) % 6 = 3 → dim
        let (ch, st) = marching_cell(3, 0, true);
        assert_eq!(ch, '\u{254C}', "dim horizontal = dashed ╌");
        assert_eq!(st.fg, Some(theme::BORDER_DIM));
    }

    #[test]
    fn marching_cell_vertical_chars() {
        let (ch, _) = marching_cell(0, 0, false);
        assert_eq!(ch, '\u{2502}', "bright vertical = solid │");

        let (ch, _) = marching_cell(3, 0, false);
        assert_eq!(ch, '\u{254E}', "dim vertical = dashed ╎");
    }

    #[test]
    fn marching_cell_phase_shifts_with_frame() {
        // perimeter_index=0, frame=0 → phase (0 + 0) % 6 = 0 (bright)
        let (_, st) = marching_cell(0, 0, true);
        assert_eq!(st.fg, Some(theme::NEON_CYAN));

        // perimeter_index=0, frame=6 → phase (0 + 6) % 6 = 0 (bright, wraps)
        let (_, st) = marching_cell(0, 6, true);
        assert_eq!(st.fg, Some(theme::NEON_CYAN));

        // perimeter_index=0, frame=3 → phase (0 + 3) % 6 = 3 (dim)
        let (_, st) = marching_cell(0, 3, true);
        assert_eq!(st.fg, Some(theme::BORDER_DIM));
    }

    #[test]
    fn render_in_progress_task_has_marching_border() {
        let mut canvas = Canvas::new(30, 5);
        let node = NodeBox {
            title: "Work".to_string(),
            status: ItemStatus::InProgress,
            progress: None,
            x: 0,
            y: 0,
            blocked: false,
        };
        render_node(&mut canvas, &node, 0);

        // Corners should be rounded
        assert_eq!(canvas.get(0, 0).ch, '\u{256D}', "top-left rounded");
        assert_eq!(canvas.get(NODE_WIDTH - 1, 0).ch, '\u{256E}', "top-right rounded");
        assert_eq!(canvas.get(0, 2).ch, '\u{2570}', "bottom-left rounded");
        assert_eq!(canvas.get(NODE_WIDTH - 1, 2).ch, '\u{256F}', "bottom-right rounded");

        // First few top-edge cells at frame 0: perimeter indices 1,2,3
        // p=1 → phase (1+0)%6=1 → bright → ─
        assert_eq!(canvas.get(1, 0).ch, '\u{2500}', "top edge idx 1 bright");
        // p=3 → phase (3+0)%6=3 → dim → ╌
        assert_eq!(canvas.get(3, 0).ch, '\u{254C}', "top edge idx 3 dim");
    }

    #[test]
    fn marching_border_shifts_between_frames() {
        // At frame 0, position 3 should be dim; at frame 3 it should shift to bright
        let mut canvas0 = Canvas::new(30, 5);
        let mut canvas3 = Canvas::new(30, 5);
        let node = NodeBox {
            title: "Work".to_string(),
            status: ItemStatus::InProgress,
            progress: None,
            x: 0,
            y: 0,
            blocked: false,
        };
        render_node(&mut canvas0, &node, 0);
        render_node(&mut canvas3, &node, 3);

        // Frame 0, position (3,0): p=3, phase=(3+0)%6=3 → dim (╌)
        assert_eq!(canvas0.get(3, 0).ch, '\u{254C}');
        // Frame 3, position (3,0): p=3, phase=(3+3)%6=0 → bright (─)
        assert_eq!(canvas3.get(3, 0).ch, '\u{2500}');
    }

    #[test]
    fn marching_border_epic_has_correct_height() {
        let mut canvas = Canvas::new(30, 6);
        let node = NodeBox {
            title: "Epic".to_string(),
            status: ItemStatus::InProgress,
            progress: Some((2, 5)),
            x: 0,
            y: 0,
            blocked: false,
        };
        render_node(&mut canvas, &node, 0);

        // Epic height = 4, so bottom border at row 3
        assert_eq!(canvas.get(0, 0).ch, '\u{256D}', "top-left");
        assert_eq!(canvas.get(0, 3).ch, '\u{2570}', "bottom-left at row 3");
        assert_eq!(canvas.get(NODE_WIDTH - 1, 3).ch, '\u{256F}', "bottom-right at row 3");
    }

    // ── Edge rendering ─────────────────────────────────────────

    use super::super::graph::{DagLayout, Edge, Node};

    fn make_node(id: &str) -> Node {
        Node {
            id: id.to_string(),
            label: id.to_string(),
            status: ItemStatus::Todo,
            layer: None,
            x_position: 0,
        }
    }

    fn make_edge(from: &str, to: &str) -> Edge {
        Edge {
            from: from.to_string(),
            to: to.to_string(),
        }
    }

    #[test]
    fn straight_vertical_edge_between_adjacent_layers() {
        // Two nodes in the same column, adjacent layers.
        // A at (0,0), B at (0,5) — gap of 2 rows between bottom of A (y=2) and top of B (y=5).
        let layout = DagLayout::new(
            vec![make_node("A"), make_node("B")],
            vec![make_edge("A", "B")],
        );

        let mut canvas = Canvas::new(30, 10);
        let mut positions = HashMap::new();
        positions.insert("A".to_string(), (0_usize, 0_usize));
        positions.insert("B".to_string(), (0_usize, 5_usize));
        let blocked = HashSet::new();

        render_edges(&mut canvas, &layout, &positions, &blocked, NODE_HEIGHT_TASK);

        // src_x = 0 + 22/2 = 11, src_y = 0 + 3 = 3, dst_y = 5 - 1 = 4
        // Vertical │ at (11, 3), ▼ at (11, 4)
        assert_eq!(canvas.get(11, 3).ch, '\u{2502}'); // │
        assert_eq!(canvas.get(11, 4).ch, '\u{25BC}'); // ▼
        // Color should be cyan (not blocked)
        assert_eq!(canvas.get(11, 3).style.fg, Some(theme::NEON_CYAN));
    }

    #[test]
    fn l_shaped_edge_routing_right() {
        // A at (0, 0), B at (24, 6) — different x-columns.
        let layout = DagLayout::new(
            vec![make_node("A"), make_node("B")],
            vec![make_edge("A", "B")],
        );

        let mut canvas = Canvas::new(60, 12);
        let mut positions = HashMap::new();
        positions.insert("A".to_string(), (0_usize, 0_usize));
        positions.insert("B".to_string(), (24_usize, 6_usize));
        let blocked = HashSet::new();

        render_edges(&mut canvas, &layout, &positions, &blocked, NODE_HEIGHT_TASK);

        // src_x = 11, src_y = 3, dst_x = 35, dst_y = 5
        // (11, 3) = │, (11, 4) = ╰, horizontal ─ from 12..35, (35, 4) = ╮, (35, 5) = ▼
        assert_eq!(canvas.get(11, 3).ch, '\u{2502}'); // │ down from source
        assert_eq!(canvas.get(11, 4).ch, '\u{2570}'); // ╰ corner going right
        assert_eq!(canvas.get(12, 4).ch, '\u{2500}'); // ─ horizontal
        assert_eq!(canvas.get(35, 4).ch, '\u{256E}'); // ╮ corner going down
        assert_eq!(canvas.get(35, 5).ch, '\u{25BC}'); // ▼ arrow at target
    }

    #[test]
    fn l_shaped_edge_routing_left() {
        // A at (24, 0), B at (0, 6) — target to the left.
        let layout = DagLayout::new(
            vec![make_node("A"), make_node("B")],
            vec![make_edge("A", "B")],
        );

        let mut canvas = Canvas::new(60, 12);
        let mut positions = HashMap::new();
        positions.insert("A".to_string(), (24_usize, 0_usize));
        positions.insert("B".to_string(), (0_usize, 6_usize));
        let blocked = HashSet::new();

        render_edges(&mut canvas, &layout, &positions, &blocked, NODE_HEIGHT_TASK);

        // src_x = 35, src_y = 3, dst_x = 11, dst_y = 5
        // (35, 3) = │, (35, 4) = ╯, horizontal ─ from 12..35, (11, 4) = ╭, (11, 5) = ▼
        assert_eq!(canvas.get(35, 3).ch, '\u{2502}'); // │ down from source
        assert_eq!(canvas.get(35, 4).ch, '\u{256F}'); // ╯ corner going left
        assert_eq!(canvas.get(34, 4).ch, '\u{2500}'); // ─ horizontal
        assert_eq!(canvas.get(11, 4).ch, '\u{256D}'); // ╭ corner going down
        assert_eq!(canvas.get(11, 5).ch, '\u{25BC}'); // ▼ arrow at target
    }

    #[test]
    fn edge_color_cyan_for_normal_pink_for_blocked() {
        let layout = DagLayout::new(
            vec![make_node("A"), make_node("B"), make_node("C")],
            vec![make_edge("A", "B"), make_edge("A", "C")],
        );

        let mut canvas = Canvas::new(60, 12);
        let mut positions = HashMap::new();
        positions.insert("A".to_string(), (0_usize, 0_usize));
        positions.insert("B".to_string(), (0_usize, 5_usize));
        positions.insert("C".to_string(), (24_usize, 5_usize));

        let mut blocked = HashSet::new();
        blocked.insert("C".to_string());

        render_edges(&mut canvas, &layout, &positions, &blocked, NODE_HEIGHT_TASK);

        // Edge A->B (not blocked) should be cyan.
        assert_eq!(canvas.get(11, 3).style.fg, Some(theme::NEON_CYAN));

        // Edge A->C (blocked target) should be pink.
        // The arrow at C's position: dst_x = 35, dst_y = 4
        assert_eq!(canvas.get(35, 4).style.fg, Some(theme::NEON_PINK));
    }

    #[test]
    fn edges_do_not_overwrite_node_content() {
        let layout = DagLayout::new(
            vec![make_node("A"), make_node("B")],
            vec![make_edge("A", "B")],
        );

        let mut canvas = Canvas::new(30, 10);
        let mut positions = HashMap::new();
        positions.insert("A".to_string(), (0_usize, 0_usize));
        positions.insert("B".to_string(), (0_usize, 5_usize));
        let blocked = HashSet::new();

        // Place a node character on the canvas first.
        canvas.put_char(11, 3, 'X', Style::default());

        render_edges(&mut canvas, &layout, &positions, &blocked, NODE_HEIGHT_TASK);

        // The 'X' should NOT be overwritten by the edge character.
        assert_eq!(canvas.get(11, 3).ch, 'X');
    }

    #[test]
    fn empty_edge_list_produces_no_changes() {
        let layout = DagLayout::new(
            vec![make_node("A"), make_node("B")],
            vec![], // no edges
        );

        let mut canvas = Canvas::new(30, 10);
        let positions = HashMap::new();
        let blocked = HashSet::new();

        render_edges(&mut canvas, &layout, &positions, &blocked, NODE_HEIGHT_TASK);

        // Canvas should remain all spaces.
        for y in 0..canvas.height {
            for x in 0..canvas.width {
                assert_eq!(canvas.get(x, y).ch, ' ');
            }
        }
    }

    // ── Focus highlight (outer glow) ─────────────────────────────

    #[test]
    fn focus_highlight_outer_glow_preserves_inner_border() {
        // Canvas needs extra room: node at (2,2) with outer glow at (1,1)
        let mut canvas = Canvas::new(30, 8);
        let node = NodeBox {
            title: "Node".to_string(),
            status: ItemStatus::Todo,
            progress: None,
            x: 2,
            y: 2,
            blocked: false,
        };
        render_node(&mut canvas, &node, 0);

        // Before highlight: inner border color should be TEXT_DIM (todo status)
        assert_eq!(canvas.get(2, 2).style.fg, Some(theme::TEXT_DIM));
        assert_eq!(canvas.get(2, 2).ch, '\u{2554}'); // inner top-left

        render_focus_highlight(&mut canvas, 2, 2, NODE_HEIGHT_TASK);

        // Inner border should be UNCHANGED (still TEXT_DIM, still double-line)
        assert_eq!(canvas.get(2, 2).style.fg, Some(theme::TEXT_DIM));
        assert_eq!(canvas.get(2, 2).ch, '\u{2554}');
        assert_eq!(canvas.get(2 + NODE_WIDTH - 1, 2).ch, '\u{2557}');
        assert_eq!(canvas.get(2, 4).ch, '\u{255A}');
        assert_eq!(canvas.get(2 + NODE_WIDTH - 1, 4).ch, '\u{255D}');

        // Outer glow should appear at (x-1, y-1) = (1, 1)
        assert_eq!(canvas.get(1, 1).ch, '\u{256D}'); // ╭
        assert_eq!(canvas.get(1, 1).style.fg, Some(theme::NEON_MAGENTA));
        assert!(
            canvas.get(1, 1).style.add_modifier.contains(ratatui::style::Modifier::BOLD),
            "outer glow should be bold"
        );
        // Top-right outer corner at (x + NODE_WIDTH, y - 1) = (24, 1)
        assert_eq!(canvas.get(2 + NODE_WIDTH, 1).ch, '\u{256E}'); // ╮
        // Bottom-left outer corner at (x-1, y + node_height) = (1, 5)
        assert_eq!(canvas.get(1, 5).ch, '\u{2570}'); // ╰
        // Bottom-right outer corner at (x + NODE_WIDTH, y + node_height) = (24, 5)
        assert_eq!(canvas.get(2 + NODE_WIDTH, 5).ch, '\u{256F}'); // ╯
        // Top edge glow
        assert_eq!(canvas.get(2, 1).ch, '\u{2500}'); // ─
    }

    #[test]
    fn focus_highlight_epic_height() {
        // Node at (2,2), outer glow needs room: canvas 30 wide, 8 tall
        let mut canvas = Canvas::new(30, 8);
        render_focus_highlight(&mut canvas, 2, 2, NODE_HEIGHT_EPIC);

        // Bottom outer glow at y + NODE_HEIGHT_EPIC = 2 + 4 = 6
        assert_eq!(canvas.get(1, 6).ch, '\u{2570}'); // ╰
        assert_eq!(canvas.get(1, 6).style.fg, Some(theme::NEON_MAGENTA));
    }
}
