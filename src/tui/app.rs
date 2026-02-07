use std::io::Stdout;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::db::Database;
use crate::db::epic::list_epics;
use crate::db::project::list_projects;
use crate::db::task::list_tasks;
use crate::models::{BlueTask, Epic, Project};
use crate::tui::ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    ProjectSelector,
}

pub struct App {
    pub db: Database,
    pub running: bool,
    pub mode: InputMode,
    pub projects: Vec<Project>,
    pub selected_project_idx: usize,
    pub selector_idx: usize,
    pub epics: Vec<Epic>,
    pub selected_epic_idx: usize,
    pub tasks: Vec<BlueTask>,
    pub selected_task_idx: usize,
}

impl App {
    pub fn new(db: Database) -> Result<Self> {
        let mut app = Self {
            db,
            running: true,
            mode: InputMode::Normal,
            projects: Vec::new(),
            selected_project_idx: 0,
            selector_idx: 0,
            epics: Vec::new(),
            selected_epic_idx: 0,
            tasks: Vec::new(),
            selected_task_idx: 0,
        };
        app.refresh_data();
        Ok(app)
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        while self.running {
            terminal.draw(|frame| ui::draw(frame, self))?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key);
                    }
                }
            }
        }
        Ok(())
    }

    /// Returns the currently selected project, if any.
    pub fn selected_project(&self) -> Option<&Project> {
        self.projects.get(self.selected_project_idx)
    }

    /// Returns the currently selected epic, if any.
    pub fn selected_epic(&self) -> Option<&Epic> {
        self.epics.get(self.selected_epic_idx)
    }

    pub fn refresh_data(&mut self) {
        self.projects = list_projects(&self.db, None).unwrap_or_default();
        self.selected_project_idx = self.selected_project_idx.min(self.projects.len().saturating_sub(1));

        self.epics = self
            .selected_project()
            .and_then(|p| list_epics(&self.db, Some(&p.id), None).ok())
            .unwrap_or_default();
        self.selected_epic_idx = self.selected_epic_idx.min(self.epics.len().saturating_sub(1));

        self.tasks = self
            .selected_epic()
            .and_then(|e| list_tasks(&self.db, Some(&e.id), None).ok())
            .unwrap_or_default();
        self.selected_task_idx = self.selected_task_idx.min(self.tasks.len().saturating_sub(1));
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match self.mode {
            InputMode::Normal => self.handle_normal_key(key),
            InputMode::ProjectSelector => self.handle_selector_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.running = false,
            KeyCode::Char('p') => self.open_project_selector(),
            _ => {}
        }
    }

    fn handle_selector_key(&mut self, key: KeyEvent) {
        let len = self.projects.len();
        match key.code {
            KeyCode::Char('j') | KeyCode::Down if len > 0 => {
                self.selector_idx = (self.selector_idx + 1) % len;
            }
            KeyCode::Char('k') | KeyCode::Up if len > 0 => {
                self.selector_idx = (self.selector_idx + len - 1) % len;
            }
            KeyCode::Enter => self.confirm_project_selection(),
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = InputMode::Normal;
            }
            _ => {}
        }
    }

    fn open_project_selector(&mut self) {
        if self.projects.is_empty() {
            return;
        }
        self.selector_idx = self.selected_project_idx;
        self.mode = InputMode::ProjectSelector;
    }

    fn confirm_project_selection(&mut self) {
        self.selected_project_idx = self.selector_idx;
        self.selected_epic_idx = 0;
        self.selected_task_idx = 0;
        self.refresh_data();
        self.mode = InputMode::Normal;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::project::create_project;
    use crate::models::CreateProjectInput;
    use tempfile::TempDir;

    fn open_temp_db() -> (Database, TempDir) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let db = Database::open(&path).unwrap();
        db.migrate().unwrap();
        (db, dir)
    }

    fn app_with_projects(n: usize) -> (App, TempDir) {
        let (db, dir) = open_temp_db();
        for i in 0..n {
            create_project(
                &db,
                CreateProjectInput {
                    name: format!("Project {i}"),
                    description: String::new(),
                },
            )
            .unwrap();
        }
        let app = App::new(db).unwrap();
        (app, dir)
    }

    #[test]
    fn initial_mode_is_normal() {
        let (app, _dir) = app_with_projects(0);
        assert_eq!(app.mode, InputMode::Normal);
    }

    #[test]
    fn p_opens_selector_when_projects_exist() {
        let (mut app, _dir) = app_with_projects(2);
        app.handle_key(KeyEvent::from(KeyCode::Char('p')));
        assert_eq!(app.mode, InputMode::ProjectSelector);
        assert_eq!(app.selector_idx, app.selected_project_idx);
    }

    #[test]
    fn p_is_noop_when_no_projects() {
        let (mut app, _dir) = app_with_projects(0);
        app.handle_key(KeyEvent::from(KeyCode::Char('p')));
        assert_eq!(app.mode, InputMode::Normal);
    }

    #[test]
    fn esc_closes_selector_without_changing_selection() {
        let (mut app, _dir) = app_with_projects(3);
        let original_idx = app.selected_project_idx;
        app.handle_key(KeyEvent::from(KeyCode::Char('p')));
        app.handle_key(KeyEvent::from(KeyCode::Char('j')));
        app.handle_key(KeyEvent::from(KeyCode::Esc));
        assert_eq!(app.mode, InputMode::Normal);
        assert_eq!(app.selected_project_idx, original_idx);
    }

    #[test]
    fn enter_confirms_selection() {
        let (mut app, _dir) = app_with_projects(3);
        app.handle_key(KeyEvent::from(KeyCode::Char('p')));
        app.handle_key(KeyEvent::from(KeyCode::Char('j')));
        let expected_idx = app.selector_idx;
        app.handle_key(KeyEvent::from(KeyCode::Enter));
        assert_eq!(app.mode, InputMode::Normal);
        assert_eq!(app.selected_project_idx, expected_idx);
    }

    #[test]
    fn j_k_wrap_around() {
        let (mut app, _dir) = app_with_projects(3);
        app.handle_key(KeyEvent::from(KeyCode::Char('p')));

        // Wrap forward: start at 0, go j -> 1, j -> 2, j -> 0
        app.selector_idx = 0;
        app.handle_key(KeyEvent::from(KeyCode::Char('j')));
        assert_eq!(app.selector_idx, 1);
        app.handle_key(KeyEvent::from(KeyCode::Char('j')));
        assert_eq!(app.selector_idx, 2);
        app.handle_key(KeyEvent::from(KeyCode::Char('j')));
        assert_eq!(app.selector_idx, 0);

        // Wrap backward: start at 0, go k -> 2
        app.handle_key(KeyEvent::from(KeyCode::Char('k')));
        assert_eq!(app.selector_idx, 2);
    }

    #[test]
    fn q_in_normal_quits() {
        let (mut app, _dir) = app_with_projects(1);
        app.handle_key(KeyEvent::from(KeyCode::Char('q')));
        assert!(!app.running);
    }

    #[test]
    fn q_in_selector_closes_popup_without_quitting() {
        let (mut app, _dir) = app_with_projects(1);
        app.handle_key(KeyEvent::from(KeyCode::Char('p')));
        assert_eq!(app.mode, InputMode::ProjectSelector);
        app.handle_key(KeyEvent::from(KeyCode::Char('q')));
        assert_eq!(app.mode, InputMode::Normal);
        assert!(app.running);
    }
}
