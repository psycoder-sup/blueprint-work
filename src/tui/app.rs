use std::collections::{HashMap, HashSet};
use std::io::Stdout;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::db::Database;
use crate::db::dependency::{get_blockers, is_blocked};
use crate::db::epic::list_epics;
use crate::db::project::list_projects;
use crate::db::status::{
    DependencyDisplayRow, count_epics_by_status, count_tasks_by_status, get_blocked_items,
    get_dependency_display_rows, get_max_updated_at,
};
use crate::db::task::{get_task, list_tasks, update_task};
use crate::models::{BlueTask, DependencyType, Epic, ItemStatus, Project, UpdateTaskInput};
use crate::tui::ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    ProjectSelector,
    TaskDetail,
    HelpOverlay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPanel {
    Epics,
    Tasks,
    Dependencies,
    Status,
}

pub struct App {
    pub db: Database,
    pub running: bool,
    pub mode: InputMode,
    pub focused_panel: FocusedPanel,
    pub projects: Vec<Project>,
    pub selected_project_idx: usize,
    pub selector_idx: usize,
    pub epics: Vec<Epic>,
    pub selected_epic_idx: usize,
    pub blocked_epic_ids: HashSet<String>,
    pub tasks: Vec<BlueTask>,
    pub selected_task_idx: usize,
    pub blocked_task_ids: HashSet<String>,
    /// Cached blocker names per task ID, computed in `refresh_tasks()`.
    pub task_blocker_names: HashMap<String, Vec<String>>,
    pub epic_status_counts: HashMap<String, i64>,
    pub task_status_counts: HashMap<String, i64>,
    pub blocked_count: usize,
    pub dep_display_rows: Vec<DependencyDisplayRow>,
    pub last_refresh: Instant,
    pub last_db_watermark: String,
}

/// Wraps an index by `delta` within `len`, returning `None` when the list is empty.
fn wrap_index(current: usize, len: usize, delta: isize) -> Option<usize> {
    if len == 0 {
        return None;
    }
    Some(((current as isize + delta).rem_euclid(len as isize)) as usize)
}

impl App {
    pub fn new(db: Database) -> Result<Self> {
        let mut app = Self {
            db,
            running: true,
            mode: InputMode::Normal,
            focused_panel: FocusedPanel::Epics,
            projects: Vec::new(),
            selected_project_idx: 0,
            selector_idx: 0,
            epics: Vec::new(),
            selected_epic_idx: 0,
            blocked_epic_ids: HashSet::new(),
            tasks: Vec::new(),
            selected_task_idx: 0,
            blocked_task_ids: HashSet::new(),
            task_blocker_names: HashMap::new(),
            epic_status_counts: HashMap::new(),
            task_status_counts: HashMap::new(),
            blocked_count: 0,
            dep_display_rows: Vec::new(),
            last_refresh: Instant::now(),
            last_db_watermark: String::new(),
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

            // Auto-refresh: poll DB for changes every ~1 second
            if self.last_refresh.elapsed() >= Duration::from_secs(1) {
                self.check_for_db_changes();
            }
        }
        Ok(())
    }

    /// Check if the database has changed since our last refresh, and reload if so.
    fn check_for_db_changes(&mut self) {
        let watermark = get_max_updated_at(&self.db).unwrap_or_default();
        if watermark != self.last_db_watermark {
            self.refresh_data();
        } else {
            self.last_refresh = Instant::now();
        }
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

        self.blocked_epic_ids = self
            .epics
            .iter()
            .filter(|e| is_blocked(&self.db, &DependencyType::Epic, &e.id).unwrap_or(false))
            .map(|e| e.id.clone())
            .collect();

        self.refresh_tasks();
        self.refresh_status_and_deps();
    }

    fn refresh_status_and_deps(&mut self) {
        let pid = self.selected_project().map(|p| p.id.clone());
        let pid = pid.as_deref();

        self.epic_status_counts = count_epics_by_status(&self.db, pid).unwrap_or_default();
        self.task_status_counts = count_tasks_by_status(&self.db, pid).unwrap_or_default();
        self.blocked_count = get_blocked_items(&self.db, pid)
            .map(|v| v.len())
            .unwrap_or(0);
        self.dep_display_rows = get_dependency_display_rows(&self.db, pid).unwrap_or_default();
        self.last_db_watermark = get_max_updated_at(&self.db).unwrap_or_default();
        self.last_refresh = Instant::now();
    }

    pub fn refresh_tasks(&mut self) {
        self.tasks = self
            .selected_epic()
            .and_then(|e| list_tasks(&self.db, Some(&e.id), None).ok())
            .unwrap_or_default();
        self.selected_task_idx = self.selected_task_idx.min(self.tasks.len().saturating_sub(1));

        self.blocked_task_ids = self
            .tasks
            .iter()
            .filter(|t| is_blocked(&self.db, &DependencyType::Task, &t.id).unwrap_or(false))
            .map(|t| t.id.clone())
            .collect();

        self.task_blocker_names = self
            .blocked_task_ids
            .iter()
            .map(|task_id| {
                let names = get_blockers(&self.db, &DependencyType::Task, task_id)
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|dep| {
                        get_task(&self.db, &dep.blocker_id)
                            .ok()
                            .flatten()
                            .map(|t| t.title)
                    })
                    .collect();
                (task_id.clone(), names)
            })
            .collect();
    }

    /// Returns the currently selected task, if any.
    pub fn selected_task(&self) -> Option<&BlueTask> {
        self.tasks.get(self.selected_task_idx)
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match self.mode {
            InputMode::Normal => self.handle_normal_key(key),
            InputMode::ProjectSelector => self.handle_selector_key(key),
            InputMode::TaskDetail => self.handle_task_detail_key(key),
            InputMode::HelpOverlay => self.handle_help_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.running = false,
            KeyCode::Char('p') => self.open_project_selector(),
            KeyCode::Char('?') => self.mode = InputMode::HelpOverlay,
            KeyCode::Char('d') => { /* TODO: toggle dependency graph view (epic_03) */ }
            KeyCode::Tab => self.toggle_focus(),
            KeyCode::Char('h') | KeyCode::Left => self.focus_left(),
            KeyCode::Char('l') | KeyCode::Right => self.focus_right(),
            KeyCode::Char('j') | KeyCode::Down => self.navigate(1),
            KeyCode::Char('k') | KeyCode::Up => self.navigate(-1),
            KeyCode::Char('s') if self.focused_panel == FocusedPanel::Tasks => {
                self.cycle_task_status();
            }
            KeyCode::Enter
                if self.focused_panel == FocusedPanel::Tasks
                    && self.selected_task().is_some() =>
            {
                self.mode = InputMode::TaskDetail;
            }
            _ => {}
        }
    }

    fn handle_help_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
                self.mode = InputMode::Normal;
            }
            _ => {}
        }
    }

    fn handle_task_detail_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => {
                self.mode = InputMode::Normal;
            }
            _ => {}
        }
    }

    fn toggle_focus(&mut self) {
        self.focused_panel = match self.focused_panel {
            FocusedPanel::Epics => FocusedPanel::Tasks,
            FocusedPanel::Tasks => FocusedPanel::Dependencies,
            FocusedPanel::Dependencies => FocusedPanel::Status,
            FocusedPanel::Status => FocusedPanel::Epics,
        };
    }

    /// Switch focus between left/right panels on the same row.
    fn focus_left(&mut self) {
        self.focused_panel = match self.focused_panel {
            FocusedPanel::Tasks => FocusedPanel::Epics,
            FocusedPanel::Status => FocusedPanel::Dependencies,
            other => other,
        };
    }

    fn focus_right(&mut self) {
        self.focused_panel = match self.focused_panel {
            FocusedPanel::Epics => FocusedPanel::Tasks,
            FocusedPanel::Dependencies => FocusedPanel::Status,
            other => other,
        };
    }

    /// Moves the selection cursor by `delta` (+1 for down, -1 for up) in the
    /// currently focused panel, wrapping around at both ends.
    fn navigate(&mut self, delta: isize) {
        match self.focused_panel {
            FocusedPanel::Epics => {
                if let Some(next) = wrap_index(self.selected_epic_idx, self.epics.len(), delta) {
                    self.selected_epic_idx = next;
                    self.selected_task_idx = 0;
                    self.refresh_tasks();
                }
            }
            FocusedPanel::Tasks => {
                if let Some(next) = wrap_index(self.selected_task_idx, self.tasks.len(), delta) {
                    self.selected_task_idx = next;
                }
            }
            FocusedPanel::Dependencies | FocusedPanel::Status => {
                // Bottom panels don't have navigable items
            }
        }
    }

    fn cycle_task_status(&mut self) {
        let Some(task) = self.tasks.get(self.selected_task_idx) else {
            return;
        };
        let next = match task.status {
            ItemStatus::Todo => ItemStatus::InProgress,
            ItemStatus::InProgress => ItemStatus::Done,
            ItemStatus::Done => ItemStatus::Todo,
        };
        let task_id = task.id.clone();
        let _ = update_task(
            &self.db,
            &task_id,
            UpdateTaskInput {
                status: Some(next),
                ..Default::default()
            },
        );
        self.refresh_data();
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
    use crate::db::dependency::add_dependency;
    use crate::db::epic::create_epic;
    use crate::db::project::create_project;
    use crate::db::task::create_task;
    use crate::models::{AddDependencyInput, CreateEpicInput, CreateProjectInput, CreateTaskInput};
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

    fn app_with_epics(epic_count: usize) -> (App, TempDir) {
        let (db, dir) = open_temp_db();
        let project = create_project(
            &db,
            CreateProjectInput {
                name: "Test Project".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        for i in 0..epic_count {
            create_epic(
                &db,
                CreateEpicInput {
                    project_id: project.id.clone(),
                    title: format!("Epic {i}"),
                    description: String::new(),
                },
            )
            .unwrap();
        }
        let app = App::new(db).unwrap();
        (app, dir)
    }

    #[test]
    fn j_k_navigates_epics_with_wrapping() {
        let (mut app, _dir) = app_with_epics(3);
        assert_eq!(app.selected_epic_idx, 0);

        app.handle_key(KeyEvent::from(KeyCode::Char('j')));
        assert_eq!(app.selected_epic_idx, 1);

        app.handle_key(KeyEvent::from(KeyCode::Char('j')));
        assert_eq!(app.selected_epic_idx, 2);

        // Wrap forward
        app.handle_key(KeyEvent::from(KeyCode::Char('j')));
        assert_eq!(app.selected_epic_idx, 0);

        // Wrap backward
        app.handle_key(KeyEvent::from(KeyCode::Char('k')));
        assert_eq!(app.selected_epic_idx, 2);
    }

    #[test]
    fn j_k_noop_when_no_epics() {
        let (mut app, _dir) = app_with_projects(1);
        assert!(app.epics.is_empty());
        assert_eq!(app.selected_epic_idx, 0);

        app.handle_key(KeyEvent::from(KeyCode::Char('j')));
        assert_eq!(app.selected_epic_idx, 0);

        app.handle_key(KeyEvent::from(KeyCode::Char('k')));
        assert_eq!(app.selected_epic_idx, 0);
    }

    #[test]
    fn navigating_epics_refreshes_tasks() {
        let (db, _dir) = open_temp_db();
        let project = create_project(
            &db,
            CreateProjectInput {
                name: "P".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let epic_a = create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id.clone(),
                title: "Epic A".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let epic_b = create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id,
                title: "Epic B".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        create_task(
            &db,
            CreateTaskInput {
                epic_id: epic_a.id.clone(),
                title: "Task A".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        create_task(
            &db,
            CreateTaskInput {
                epic_id: epic_b.id.clone(),
                title: "Task B".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        let mut app = App::new(db).unwrap();

        // Epics are ordered by created_at DESC, so epic_b is first
        let initial_task_title = app.tasks[0].title.clone();

        app.handle_key(KeyEvent::from(KeyCode::Char('j')));
        let new_task_title = app.tasks[0].title.clone();

        assert_ne!(initial_task_title, new_task_title);
    }

    #[test]
    fn blocked_epic_ids_populated_after_refresh() {
        let (db, _dir) = open_temp_db();
        let project = create_project(
            &db,
            CreateProjectInput {
                name: "P".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let epic_a = create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id.clone(),
                title: "Blocker".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let epic_b = create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id,
                title: "Blocked".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        // epic_a blocks epic_b
        add_dependency(
            &db,
            AddDependencyInput {
                blocker_type: DependencyType::Epic,
                blocker_id: epic_a.id.clone(),
                blocked_type: DependencyType::Epic,
                blocked_id: epic_b.id.clone(),
            },
        )
        .unwrap();

        let app = App::new(db).unwrap();

        assert!(app.blocked_epic_ids.contains(&epic_b.id));
        assert!(!app.blocked_epic_ids.contains(&epic_a.id));
    }

    /// Creates an app with one project, one epic, and `n` tasks.
    fn app_with_tasks(n: usize) -> (App, TempDir) {
        let (db, dir) = open_temp_db();
        let project = create_project(
            &db,
            CreateProjectInput {
                name: "P".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let epic = create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id,
                title: "Epic".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        for i in 0..n {
            create_task(
                &db,
                CreateTaskInput {
                    epic_id: epic.id.clone(),
                    title: format!("Task {i}"),
                    description: String::new(),
                },
            )
            .unwrap();
        }
        let app = App::new(db).unwrap();
        (app, dir)
    }

    #[test]
    fn tab_cycles_through_all_panels() {
        let (mut app, _dir) = app_with_tasks(1);
        assert_eq!(app.focused_panel, FocusedPanel::Epics);

        app.handle_key(KeyEvent::from(KeyCode::Tab));
        assert_eq!(app.focused_panel, FocusedPanel::Tasks);

        app.handle_key(KeyEvent::from(KeyCode::Tab));
        assert_eq!(app.focused_panel, FocusedPanel::Dependencies);

        app.handle_key(KeyEvent::from(KeyCode::Tab));
        assert_eq!(app.focused_panel, FocusedPanel::Status);

        app.handle_key(KeyEvent::from(KeyCode::Tab));
        assert_eq!(app.focused_panel, FocusedPanel::Epics);
    }

    #[test]
    fn j_k_navigates_tasks_when_task_panel_focused() {
        let (mut app, _dir) = app_with_tasks(3);
        app.focused_panel = FocusedPanel::Tasks;
        assert_eq!(app.selected_task_idx, 0);

        app.handle_key(KeyEvent::from(KeyCode::Char('j')));
        assert_eq!(app.selected_task_idx, 1);

        app.handle_key(KeyEvent::from(KeyCode::Char('j')));
        assert_eq!(app.selected_task_idx, 2);

        // Wrap forward
        app.handle_key(KeyEvent::from(KeyCode::Char('j')));
        assert_eq!(app.selected_task_idx, 0);

        // Wrap backward
        app.handle_key(KeyEvent::from(KeyCode::Char('k')));
        assert_eq!(app.selected_task_idx, 2);
    }

    #[test]
    fn j_k_still_navigates_epics_when_epic_panel_focused() {
        let (mut app, _dir) = app_with_epics(3);
        assert_eq!(app.focused_panel, FocusedPanel::Epics);
        assert_eq!(app.selected_epic_idx, 0);

        app.handle_key(KeyEvent::from(KeyCode::Char('j')));
        assert_eq!(app.selected_epic_idx, 1);
    }

    #[test]
    fn j_k_task_noop_when_no_tasks() {
        let (mut app, _dir) = app_with_tasks(0);
        app.focused_panel = FocusedPanel::Tasks;
        assert_eq!(app.selected_task_idx, 0);

        app.handle_key(KeyEvent::from(KeyCode::Char('j')));
        assert_eq!(app.selected_task_idx, 0);

        app.handle_key(KeyEvent::from(KeyCode::Char('k')));
        assert_eq!(app.selected_task_idx, 0);
    }

    #[test]
    fn s_cycles_task_status() {
        let (mut app, _dir) = app_with_tasks(1);
        app.focused_panel = FocusedPanel::Tasks;

        assert_eq!(app.tasks[0].status, ItemStatus::Todo);

        app.handle_key(KeyEvent::from(KeyCode::Char('s')));
        assert_eq!(app.tasks[0].status, ItemStatus::InProgress);

        app.handle_key(KeyEvent::from(KeyCode::Char('s')));
        assert_eq!(app.tasks[0].status, ItemStatus::Done);

        app.handle_key(KeyEvent::from(KeyCode::Char('s')));
        assert_eq!(app.tasks[0].status, ItemStatus::Todo);
    }

    #[test]
    fn s_persists_to_db() {
        let (mut app, _dir) = app_with_tasks(1);
        app.focused_panel = FocusedPanel::Tasks;
        let task_id = app.tasks[0].id.clone();

        app.handle_key(KeyEvent::from(KeyCode::Char('s')));

        // Read directly from DB
        let task = crate::db::task::get_task(&app.db, &task_id)
            .unwrap()
            .unwrap();
        assert_eq!(task.status, ItemStatus::InProgress);
    }

    #[test]
    fn s_is_noop_when_no_tasks() {
        let (mut app, _dir) = app_with_tasks(0);
        app.focused_panel = FocusedPanel::Tasks;

        // Should not panic
        app.handle_key(KeyEvent::from(KeyCode::Char('s')));
    }

    #[test]
    fn enter_opens_task_detail_popup() {
        let (mut app, _dir) = app_with_tasks(1);
        app.focused_panel = FocusedPanel::Tasks;

        app.handle_key(KeyEvent::from(KeyCode::Enter));
        assert_eq!(app.mode, InputMode::TaskDetail);
    }

    #[test]
    fn esc_closes_task_detail_popup() {
        let (mut app, _dir) = app_with_tasks(1);
        app.focused_panel = FocusedPanel::Tasks;

        app.handle_key(KeyEvent::from(KeyCode::Enter));
        assert_eq!(app.mode, InputMode::TaskDetail);

        app.handle_key(KeyEvent::from(KeyCode::Esc));
        assert_eq!(app.mode, InputMode::Normal);
    }

    #[test]
    fn enter_is_noop_when_no_tasks() {
        let (mut app, _dir) = app_with_tasks(0);
        app.focused_panel = FocusedPanel::Tasks;

        app.handle_key(KeyEvent::from(KeyCode::Enter));
        assert_eq!(app.mode, InputMode::Normal);
    }

    #[test]
    fn blocked_task_ids_populated_correctly() {
        let (db, _dir) = open_temp_db();
        let project = create_project(
            &db,
            CreateProjectInput {
                name: "P".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let epic = create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id,
                title: "Epic".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let t1 = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id.clone(),
                title: "Blocker".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let t2 = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id,
                title: "Blocked".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        add_dependency(
            &db,
            AddDependencyInput {
                blocker_type: DependencyType::Task,
                blocker_id: t1.id.clone(),
                blocked_type: DependencyType::Task,
                blocked_id: t2.id.clone(),
            },
        )
        .unwrap();

        let app = App::new(db).unwrap();
        assert!(app.blocked_task_ids.contains(&t2.id));
        assert!(!app.blocked_task_ids.contains(&t1.id));
    }

    #[test]
    fn test_status_counts_populated() {
        let (db, dir) = open_temp_db();
        let project = create_project(
            &db,
            CreateProjectInput {
                name: "P".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let epic1 = create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id.clone(),
                title: "Epic 1".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let epic2 = create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id.clone(),
                title: "Epic 2".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        // Mark epic2 as in_progress
        crate::db::epic::update_epic(
            &db,
            &epic2.id,
            crate::models::UpdateEpicInput {
                status: Some(ItemStatus::InProgress),
                ..Default::default()
            },
        )
        .unwrap();

        // Create tasks in various statuses
        let _t1 = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic1.id.clone(),
                title: "Task todo".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let t2 = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic1.id.clone(),
                title: "Task in_progress".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let t3 = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic1.id.clone(),
                title: "Task done".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        update_task(
            &db,
            &t2.id,
            UpdateTaskInput {
                status: Some(ItemStatus::InProgress),
                ..Default::default()
            },
        )
        .unwrap();
        update_task(
            &db,
            &t3.id,
            UpdateTaskInput {
                status: Some(ItemStatus::Done),
                ..Default::default()
            },
        )
        .unwrap();

        let app = App::new(db).unwrap();

        // Epic counts: 1 todo, 1 in_progress, 0 done
        assert_eq!(app.epic_status_counts["todo"], 1);
        assert_eq!(app.epic_status_counts["in_progress"], 1);
        assert_eq!(app.epic_status_counts["done"], 0);

        // Task counts: 1 todo, 1 in_progress, 1 done
        assert_eq!(app.task_status_counts["todo"], 1);
        assert_eq!(app.task_status_counts["in_progress"], 1);
        assert_eq!(app.task_status_counts["done"], 1);

        drop(dir);
    }

    #[test]
    fn test_blocked_count_populated() {
        let (db, dir) = open_temp_db();
        let project = create_project(
            &db,
            CreateProjectInput {
                name: "P".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let epic = create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id.clone(),
                title: "E".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let t1 = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id.clone(),
                title: "Blocker".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let t2 = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id.clone(),
                title: "Blocked".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        add_dependency(
            &db,
            AddDependencyInput {
                blocker_type: DependencyType::Task,
                blocker_id: t1.id.clone(),
                blocked_type: DependencyType::Task,
                blocked_id: t2.id.clone(),
            },
        )
        .unwrap();

        let app = App::new(db).unwrap();
        assert_eq!(app.blocked_count, 1);

        drop(dir);
    }

    #[test]
    fn test_check_for_db_changes_refreshes_on_change() {
        let (db, dir) = open_temp_db();
        let project = create_project(
            &db,
            CreateProjectInput {
                name: "P".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let epic = create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id.clone(),
                title: "E".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let t1 = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id.clone(),
                title: "Task 1".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        let mut app = App::new(db).unwrap();

        // Verify initial status is todo
        assert_eq!(app.tasks[0].status, ItemStatus::Todo);
        assert_eq!(app.task_status_counts["todo"], 1);
        assert_eq!(app.task_status_counts["in_progress"], 0);

        // Bypass the app and update the task directly in the DB, using a future
        // timestamp so the watermark is guaranteed to change even within the
        // same second.
        app.db
            .conn()
            .execute(
                "UPDATE tasks SET status = 'in_progress', updated_at = datetime('now', '+1 minute') WHERE id = ?1",
                [&t1.id],
            )
            .unwrap();

        // Set last_refresh to the past so the watermark check triggers
        app.last_refresh = Instant::now() - Duration::from_secs(2);

        // This should detect the watermark change and refresh
        app.check_for_db_changes();

        // Verify the app state was refreshed
        assert_eq!(app.task_status_counts["in_progress"], 1);

        drop(dir);
    }

    #[test]
    fn h_l_switches_left_right_panels() {
        let (mut app, _dir) = app_with_tasks(1);

        // Start at Epics (top-left), 'l' moves to Tasks (top-right)
        assert_eq!(app.focused_panel, FocusedPanel::Epics);
        app.handle_key(KeyEvent::from(KeyCode::Char('l')));
        assert_eq!(app.focused_panel, FocusedPanel::Tasks);

        // 'h' moves Tasks back to Epics
        app.handle_key(KeyEvent::from(KeyCode::Char('h')));
        assert_eq!(app.focused_panel, FocusedPanel::Epics);

        // 'h' on Epics stays on Epics (no panel to the left)
        app.handle_key(KeyEvent::from(KeyCode::Char('h')));
        assert_eq!(app.focused_panel, FocusedPanel::Epics);

        // 'l' on Tasks stays on Tasks (no panel to the right)
        app.focused_panel = FocusedPanel::Tasks;
        app.handle_key(KeyEvent::from(KeyCode::Char('l')));
        assert_eq!(app.focused_panel, FocusedPanel::Tasks);

        // Bottom row: Status -> Dependencies via 'h'
        app.focused_panel = FocusedPanel::Status;
        app.handle_key(KeyEvent::from(KeyCode::Char('h')));
        assert_eq!(app.focused_panel, FocusedPanel::Dependencies);

        // Bottom row: Dependencies -> Status via 'l'
        app.handle_key(KeyEvent::from(KeyCode::Char('l')));
        assert_eq!(app.focused_panel, FocusedPanel::Status);
    }

    #[test]
    fn question_mark_opens_help_overlay() {
        let (mut app, _dir) = app_with_tasks(1);
        assert_eq!(app.mode, InputMode::Normal);

        app.handle_key(KeyEvent::from(KeyCode::Char('?')));
        assert_eq!(app.mode, InputMode::HelpOverlay);
    }

    #[test]
    fn esc_closes_help_overlay() {
        let (mut app, _dir) = app_with_tasks(1);

        // Open the help overlay
        app.handle_key(KeyEvent::from(KeyCode::Char('?')));
        assert_eq!(app.mode, InputMode::HelpOverlay);

        // Esc closes it back to Normal
        app.handle_key(KeyEvent::from(KeyCode::Esc));
        assert_eq!(app.mode, InputMode::Normal);
    }
}
