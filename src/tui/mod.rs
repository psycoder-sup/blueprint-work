mod app;
pub mod graph;
pub mod graph_render;
mod theme;
mod ui;

pub use app::App;

use std::io::stdout;
use std::panic;

use anyhow::Result;
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::db::Database;

/// Drop guard that restores terminal state when dropped.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
    }
}

pub fn run() -> Result<()> {
    let db = Database::open_default()?;
    db.migrate()?;

    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    let _guard = TerminalGuard;

    // Install panic hook that restores terminal before printing the panic
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
        original_hook(info);
    }));

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(db)?;
    let result = app.run(&mut terminal);

    // Restore the original panic hook before returning
    let _ = panic::take_hook();

    result
}
