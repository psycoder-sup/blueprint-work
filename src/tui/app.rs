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

pub struct App {
    pub db: Database,
    pub running: bool,
    pub projects: Vec<Project>,
    pub selected_project_idx: usize,
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
            projects: Vec::new(),
            selected_project_idx: 0,
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
        match key.code {
            KeyCode::Char('q') => self.running = false,
            _ => {}
        }
    }
}
