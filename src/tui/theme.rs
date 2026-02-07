use ratatui::style::{Color, Modifier, Style};

use crate::models::{ItemStatus, ProjectStatus};

// ── Color palette ──────────────────────────────────────────────────

pub const BG: Color = Color::Rgb(0x0a, 0x0a, 0x0f);
pub const NEON_CYAN: Color = Color::Rgb(0x00, 0xff, 0xf5);
pub const NEON_MAGENTA: Color = Color::Rgb(0xff, 0x00, 0xff);
pub const NEON_PINK: Color = Color::Rgb(0xff, 0x2d, 0x6f);
pub const NEON_GREEN: Color = Color::Rgb(0x39, 0xff, 0x14);
pub const NEON_ORANGE: Color = Color::Rgb(0xff, 0x6e, 0x27);
pub const ELECTRIC_BLUE: Color = Color::Rgb(0x00, 0xd4, 0xff);
pub const TEXT_DIM: Color = Color::Rgb(0xb0, 0xb0, 0xb0);
pub const TEXT_BRIGHT: Color = Color::Rgb(0xff, 0xff, 0xff);
pub const BORDER_DIM: Color = Color::Rgb(0x00, 0x5f, 0x5f);
pub const BORDER_BRIGHT: Color = Color::Rgb(0x00, 0xff, 0xf5);
pub const DARK_RED: Color = Color::Rgb(0x66, 0x11, 0x22);
pub const DARK_ORANGE: Color = Color::Rgb(0x66, 0x33, 0x11);

// ── Style presets ──────────────────────────────────────────────────

pub fn panel_border(focused: bool) -> Style {
    if focused {
        Style::default()
            .fg(BORDER_BRIGHT)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(BORDER_DIM)
    }
}

pub fn status_style(status: &ItemStatus) -> Style {
    match status {
        ItemStatus::Todo => Style::default().fg(TEXT_DIM),
        ItemStatus::InProgress => Style::default().fg(NEON_CYAN).add_modifier(Modifier::BOLD),
        ItemStatus::Done => Style::default().fg(NEON_GREEN),
    }
}

pub fn status_symbol(status: &ItemStatus) -> &'static str {
    match status {
        ItemStatus::Todo => "■",
        ItemStatus::InProgress => "▶",
        ItemStatus::Done => "◉",
    }
}

pub fn project_status_style(status: &ProjectStatus) -> Style {
    match status {
        ProjectStatus::Active => Style::default().fg(NEON_GREEN),
        ProjectStatus::Archived => Style::default().fg(TEXT_DIM),
    }
}

// ── Blocked indicator ──────────────────────────────────────────────

pub const BLOCKED_SYMBOL: &str = "⚠";

pub fn blocked_style() -> Style {
    Style::default().fg(NEON_ORANGE)
}

// ── Progress bar ───────────────────────────────────────────────────

pub fn progress_bar(done: usize, total: usize, width: usize) -> String {
    let filled = if total == 0 {
        0
    } else {
        ((done * width) / total).min(width)
    };
    let empty = width - filled;
    "█".repeat(filled) + &"░".repeat(empty)
}

// ── ASCII art header ───────────────────────────────────────────────

pub const HEADER_ART: &str = "\
▐██▌ BLUEPRINT ▐██▌";

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_bar_empty() {
        assert_eq!(progress_bar(0, 10, 10), "░░░░░░░░░░");
    }

    #[test]
    fn progress_bar_half() {
        assert_eq!(progress_bar(5, 10, 10), "█████░░░░░");
    }

    #[test]
    fn progress_bar_full() {
        assert_eq!(progress_bar(10, 10, 10), "██████████");
    }

    #[test]
    fn progress_bar_zero_total() {
        assert_eq!(progress_bar(0, 0, 10), "░░░░░░░░░░");
    }

    #[test]
    fn status_style_returns_correct_fg() {
        let todo_style = status_style(&ItemStatus::Todo);
        assert_eq!(todo_style.fg, Some(TEXT_DIM));

        let in_progress_style = status_style(&ItemStatus::InProgress);
        assert_eq!(in_progress_style.fg, Some(NEON_CYAN));

        let done_style = status_style(&ItemStatus::Done);
        assert_eq!(done_style.fg, Some(NEON_GREEN));
    }

    #[test]
    fn project_status_style_returns_correct_fg() {
        let active_style = project_status_style(&ProjectStatus::Active);
        assert_eq!(active_style.fg, Some(NEON_GREEN));

        let archived_style = project_status_style(&ProjectStatus::Archived);
        assert_eq!(archived_style.fg, Some(TEXT_DIM));
    }

    #[test]
    fn status_symbol_returns_correct_char() {
        assert_eq!(status_symbol(&ItemStatus::Todo), "■");
        assert_eq!(status_symbol(&ItemStatus::InProgress), "▶");
        assert_eq!(status_symbol(&ItemStatus::Done), "◉");
    }

    #[test]
    fn blocked_style_returns_neon_orange() {
        let style = blocked_style();
        assert_eq!(style.fg, Some(NEON_ORANGE));
    }
}
