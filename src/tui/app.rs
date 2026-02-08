use std::collections::{HashMap, HashSet};
use std::io::Stdout;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::db::Database;
use crate::settings::Settings;
use crate::db::dependency::{get_blocked_by, get_blockers, is_blocked};
use crate::db::epic::list_epics;
use crate::db::project::list_projects;
use crate::db::status::{
    DependencyDisplayRow, count_epics_by_status, count_tasks_by_status, get_blocked_items,
    get_dependency_display_rows, get_max_updated_at,
};
use crate::db::task::{get_task, list_tasks, update_task};
use crate::models::{BlueTask, DependencyType, Epic, ItemStatus, Project, UpdateTaskInput};
use crate::tui::graph::{DagLayout, Edge, Node};
use crate::tui::graph_render::{self, NODE_HEIGHT_EPIC, NODE_HEIGHT_TASK, NODE_WIDTH};
use crate::tui::ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    ProjectSelector,
    TaskDetail,
    HelpOverlay,
    GraphView,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphLevel {
    Epic,
    Task,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphPane {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Cached graph layout data (recomputed only when data changes).
pub struct GraphCache {
    pub layout: DagLayout,
    pub node_positions: HashMap<String, (usize, usize)>,
    pub level: GraphLevel,
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
    /// Global animation frame counter (0–47) for animation effects.
    /// Advances every tick (~42ms) for ~24 fps refresh.
    pub animation_frame: u8,
    pub graph_mode: GraphLevel,
    pub graph_cache: Option<GraphCache>,
    pub scroll_x: usize,
    pub scroll_y: usize,
    pub dual_pane: bool,
    pub active_pane: GraphPane,
    pub epic_graph_cache: Option<GraphCache>,
    pub task_graph_cache: Option<GraphCache>,
    pub epic_scroll_x: usize,
    pub epic_scroll_y: usize,
    pub task_scroll_x: usize,
    pub task_scroll_y: usize,
    /// Focused node ID in single-pane graph view.
    pub focused_node: Option<String>,
    /// Focused node ID in dual-pane left (epic) graph.
    pub epic_focused_node: Option<String>,
    /// Focused node ID in dual-pane right (task) graph.
    pub task_focused_node: Option<String>,
    /// Viewport size (width, height) for auto-scroll, updated each frame.
    pub graph_viewport_size: (u16, u16),
}

/// Wraps an index by `delta` within `len`, returning `None` when the list is empty.
fn wrap_index(current: usize, len: usize, delta: isize) -> Option<usize> {
    if len == 0 {
        return None;
    }
    Some(((current as isize + delta).rem_euclid(len as isize)) as usize)
}

/// Build a [`GraphCache`] from a set of nodes, edges, and the node height used
/// for vertical spacing. This is the shared logic behind both epic and task
/// graph construction.
fn build_graph_cache(
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    node_height: usize,
    level: GraphLevel,
) -> GraphCache {
    let layout = DagLayout::new(nodes, edges);

    let h_spacing = NODE_WIDTH + 4;
    // Use the max possible height (2-line title) for spacing so all nodes fit.
    let max_height = node_height + 1; // +1 for potential 2-line title
    let v_spacing = max_height + 2;

    let mut node_positions = HashMap::new();

    for (layer_idx, layer) in layout.layers.iter().enumerate() {
        for (x_idx, node_id) in layer.iter().enumerate() {
            node_positions.insert(node_id.clone(), (1 + x_idx * h_spacing, 1 + layer_idx * v_spacing));
        }
    }

    if !layout.orphans.is_empty() {
        let orphan_y = layout.layers.len() * v_spacing;
        for (x_idx, node_id) in layout.orphans.iter().enumerate() {
            node_positions.insert(node_id.clone(), (1 + x_idx * h_spacing, 1 + orphan_y));
        }
    }

    GraphCache {
        layout,
        node_positions,
        level,
    }
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
            animation_frame: 0,
            graph_mode: GraphLevel::Epic,
            graph_cache: None,
            scroll_x: 0,
            scroll_y: 0,
            dual_pane: false,
            active_pane: GraphPane::Left,
            epic_graph_cache: None,
            task_graph_cache: None,
            epic_scroll_x: 0,
            epic_scroll_y: 0,
            task_scroll_x: 0,
            task_scroll_y: 0,
            focused_node: None,
            epic_focused_node: None,
            task_focused_node: None,
            graph_viewport_size: (0, 0),
        };
        app.refresh_data();
        Ok(app)
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        while self.running {
            // Store viewport size for auto-scroll calculations.
            if let Ok(size) = terminal.size() {
                self.graph_viewport_size = (size.width, size.height);
            }

            terminal.draw(|frame| ui::draw(frame, self))?;

            if event::poll(Duration::from_millis(42))?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                self.handle_key(key);
            }

            // Advance animation frame every tick (~42ms ≈ 24 fps).
            self.animation_frame = (self.animation_frame + 1) % 48;

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

        // Rebuild graph caches in-place if currently viewing the graph,
        // preserving scroll positions and focused node state.
        // Otherwise just invalidate so they get rebuilt on next entry.
        if self.mode == InputMode::GraphView {
            if self.dual_pane {
                self.build_epic_graph();
                self.epic_graph_cache = self.graph_cache.take();
                self.build_task_graph();
                self.task_graph_cache = self.graph_cache.take();
            } else {
                match self.graph_mode {
                    GraphLevel::Epic => self.build_epic_graph(),
                    GraphLevel::Task => self.build_task_graph(),
                }
            }
        } else {
            self.invalidate_graph_caches();
        }
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
            InputMode::GraphView => self.handle_graph_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.running = false,
            KeyCode::Char('p') => self.open_project_selector(),
            KeyCode::Char('?') => self.mode = InputMode::HelpOverlay,
            KeyCode::Char('d') => {
                self.reset_scroll();
                self.graph_mode = GraphLevel::Epic;
                self.build_epic_graph();
                self.focused_node = None;
                self.mode = InputMode::GraphView;
            }
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

    fn handle_graph_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                if self.dual_pane {
                    self.exit_dual_to_single_epic();
                } else {
                    self.mode = InputMode::Normal;
                }
            }
            KeyCode::Char('1') => {
                if self.dual_pane || self.graph_mode != GraphLevel::Epic {
                    self.exit_dual_to_single_epic();
                }
                self.focused_node = None;
            }
            KeyCode::Char('2') => {
                self.dual_pane = false;
                self.reset_scroll();
                self.graph_mode = GraphLevel::Task;
                self.build_task_graph();
                self.focused_node = None;
            }
            KeyCode::Char('3') => {
                if self.dual_pane {
                    self.exit_dual_to_single_epic();
                } else {
                    self.dual_pane = true;
                    self.active_pane = GraphPane::Left;
                    self.build_dual_graphs();
                    self.epic_focused_node = None;
                    self.task_focused_node = None;
                }
            }
            KeyCode::Tab if self.dual_pane => {
                self.active_pane = match self.active_pane {
                    GraphPane::Left => GraphPane::Right,
                    GraphPane::Right => GraphPane::Left,
                };
            }
            // Arrow keys: node navigation
            KeyCode::Down | KeyCode::Up | KeyCode::Right | KeyCode::Left => {
                let direction = match key.code {
                    KeyCode::Down => GraphDirection::Down,
                    KeyCode::Up => GraphDirection::Up,
                    KeyCode::Right => GraphDirection::Right,
                    _ => GraphDirection::Left,
                };
                self.navigate_graph_node(direction);
                self.ensure_focused_node_visible();
                self.sync_task_graph_to_focused_epic();
            }
            // hjkl: viewport panning
            KeyCode::Char('j') => {
                let (_, sy) = self.active_scroll_mut();
                *sy = sy.saturating_add(1);
            }
            KeyCode::Char('k') => {
                let (_, sy) = self.active_scroll_mut();
                *sy = sy.saturating_sub(1);
            }
            KeyCode::Char('l') => {
                let (sx, _) = self.active_scroll_mut();
                *sx = sx.saturating_add(1);
            }
            KeyCode::Char('h') => {
                let (sx, _) = self.active_scroll_mut();
                *sx = sx.saturating_sub(1);
            }
            _ => {}
        }
    }

    fn active_scroll_mut(&mut self) -> (&mut usize, &mut usize) {
        if self.dual_pane {
            match self.active_pane {
                GraphPane::Left => (&mut self.epic_scroll_x, &mut self.epic_scroll_y),
                GraphPane::Right => (&mut self.task_scroll_x, &mut self.task_scroll_y),
            }
        } else {
            (&mut self.scroll_x, &mut self.scroll_y)
        }
    }

    /// Returns a mutable reference to the focused node for the active pane/mode.
    fn active_focused_node_mut(&mut self) -> &mut Option<String> {
        if self.dual_pane {
            match self.active_pane {
                GraphPane::Left => &mut self.epic_focused_node,
                GraphPane::Right => &mut self.task_focused_node,
            }
        } else {
            &mut self.focused_node
        }
    }

    /// Returns the active graph cache for the current pane/mode.
    fn active_graph_cache(&self) -> Option<&GraphCache> {
        if self.dual_pane {
            match self.active_pane {
                GraphPane::Left => self.epic_graph_cache.as_ref(),
                GraphPane::Right => self.task_graph_cache.as_ref(),
            }
        } else {
            self.graph_cache.as_ref()
        }
    }

    /// Returns the focused node ID for the current pane/mode.
    pub fn active_focused_node(&self) -> Option<&str> {
        if self.dual_pane {
            match self.active_pane {
                GraphPane::Left => self.epic_focused_node.as_deref(),
                GraphPane::Right => self.task_focused_node.as_deref(),
            }
        } else {
            self.focused_node.as_deref()
        }
    }

    /// Build a navigation grid from the graph cache layers + orphans.
    fn navigation_grid(cache: &GraphCache) -> Vec<Vec<String>> {
        let mut grid: Vec<Vec<String>> = cache.layout.layers.clone();
        if !cache.layout.orphans.is_empty() {
            grid.push(cache.layout.orphans.clone());
        }
        grid
    }

    /// Find the (row, col) of a node ID in the grid.
    fn find_in_grid(grid: &[Vec<String>], node_id: &str) -> Option<(usize, usize)> {
        for (row, layer) in grid.iter().enumerate() {
            for (col, id) in layer.iter().enumerate() {
                if id == node_id {
                    return Some((row, col));
                }
            }
        }
        None
    }

    /// Navigate to a neighboring node in the graph grid.
    fn navigate_graph_node(&mut self, direction: GraphDirection) {
        let Some(cache) = self.active_graph_cache() else {
            return;
        };

        let grid = Self::navigation_grid(cache);
        if grid.is_empty() || grid.iter().all(|row| row.is_empty()) {
            return;
        }

        let current_focus = self.active_focused_node().map(str::to_owned);

        let new_id = match current_focus.as_deref() {
            Some(current_id) => {
                let (row, col) = Self::find_in_grid(&grid, current_id).unwrap_or_default();
                match direction {
                    GraphDirection::Up => {
                        let new_row = row.saturating_sub(1);
                        let new_col = col.min(grid[new_row].len().saturating_sub(1));
                        grid[new_row][new_col].clone()
                    }
                    GraphDirection::Down => {
                        let new_row = (row + 1).min(grid.len() - 1);
                        let new_col = col.min(grid[new_row].len().saturating_sub(1));
                        grid[new_row][new_col].clone()
                    }
                    GraphDirection::Left => {
                        let row_len = grid[row].len();
                        let new_col = (col + row_len - 1) % row_len;
                        grid[row][new_col].clone()
                    }
                    GraphDirection::Right => {
                        let row_len = grid[row].len();
                        let new_col = (col + 1) % row_len;
                        grid[row][new_col].clone()
                    }
                }
            }
            None => {
                // No current focus -- select the first node in the first non-empty row.
                grid.iter()
                    .find(|row| !row.is_empty())
                    .map(|row| row[0].clone())
                    .unwrap_or_default()
            }
        };

        if !new_id.is_empty() {
            *self.active_focused_node_mut() = Some(new_id);
        }
    }

    /// When the focused epic changes in dual-pane left pane, update the selected
    /// epic and rebuild the task graph for the right pane.
    fn sync_task_graph_to_focused_epic(&mut self) {
        if !self.dual_pane || self.active_pane != GraphPane::Left {
            return;
        }
        let Some(ref focused_id) = self.epic_focused_node else {
            return;
        };
        // Find the epic index matching the focused node ID.
        let Some(idx) = self.epics.iter().position(|e| e.id == *focused_id) else {
            return;
        };
        if idx == self.selected_epic_idx {
            return; // no change
        }
        self.selected_epic_idx = idx;
        self.selected_task_idx = 0;
        self.refresh_tasks();
        // Rebuild the task graph cache for the right pane.
        self.build_task_graph();
        self.task_graph_cache = self.graph_cache.take();
        self.task_scroll_x = 0;
        self.task_scroll_y = 0;
        self.task_focused_node = None;
    }

    /// Auto-scroll to keep the focused node visible, with 2-cell padding.
    fn ensure_focused_node_visible(&mut self) {
        let focused_id = self.active_focused_node().map(str::to_owned);

        let Some(focused_id) = focused_id else {
            return;
        };

        let Some(cache) = self.active_graph_cache() else {
            return;
        };

        let Some(&(node_x, node_y)) = cache.node_positions.get(&focused_id) else {
            return;
        };

        let has_progress = cache.level == GraphLevel::Epic;
        let default_height = if has_progress { NODE_HEIGHT_EPIC } else { NODE_HEIGHT_TASK };
        let node_height = cache
            .layout
            .nodes
            .get(&focused_id)
            .map(|n| graph_render::node_height(&n.label, has_progress))
            .unwrap_or(default_height);

        // Approximate viewport size: use stored terminal size minus chrome.
        // In dual-pane mode, the viewport is roughly half the terminal width.
        let vw = if self.dual_pane {
            (self.graph_viewport_size.0 as usize) / 2
        } else {
            self.graph_viewport_size.0 as usize
        };
        // Subtract header/footer/summary chrome (~7 rows).
        let vh = (self.graph_viewport_size.1 as usize).saturating_sub(7);

        if vw == 0 || vh == 0 {
            return;
        }

        let padding: usize = 2;

        let (sx, sy) = self.active_scroll_mut();

        // Horizontal
        if node_x < sx.saturating_add(padding) {
            *sx = node_x.saturating_sub(padding);
        } else if node_x + NODE_WIDTH + padding > *sx + vw {
            *sx = (node_x + NODE_WIDTH + padding).saturating_sub(vw);
        }

        // Vertical
        if node_y < sy.saturating_add(padding) {
            *sy = node_y.saturating_sub(padding);
        } else if node_y + node_height + padding > *sy + vh {
            *sy = (node_y + node_height + padding).saturating_sub(vh);
        }
    }

    fn exit_dual_to_single_epic(&mut self) {
        self.dual_pane = false;
        self.reset_scroll();
        self.graph_mode = GraphLevel::Epic;
        self.build_epic_graph();
    }

    fn node_label(short_id: &Option<String>, title: &str) -> String {
        match short_id {
            Some(sid) => format!("[{sid}] {title}"),
            None => title.to_string(),
        }
    }

    pub fn build_epic_graph(&mut self) {
        let nodes: Vec<Node> = self
            .epics
            .iter()
            .map(|e| Node {
                id: e.id.clone(),
                label: Self::node_label(&e.short_id, &e.title),
                status: e.status.clone(),
                layer: None,
                x_position: 0,
            })
            .collect();

        let edges = self.collect_dependency_edges(
            self.epics.iter().map(|e| &e.id),
            &DependencyType::Epic,
        );

        self.graph_cache = Some(build_graph_cache(nodes, edges, NODE_HEIGHT_EPIC, GraphLevel::Epic));
    }

    pub fn build_dual_graphs(&mut self) {
        self.build_epic_graph();
        self.epic_graph_cache = self.graph_cache.take();

        self.build_task_graph();
        self.task_graph_cache = self.graph_cache.take();

        self.epic_scroll_x = 0;
        self.epic_scroll_y = 0;
        self.task_scroll_x = 0;
        self.task_scroll_y = 0;
    }

    pub fn build_task_graph(&mut self) {
        if self.selected_epic().is_none() {
            self.graph_cache = None;
            return;
        }

        let nodes: Vec<Node> = self
            .tasks
            .iter()
            .map(|t| Node {
                id: t.id.clone(),
                label: Self::node_label(&t.short_id, &t.title),
                status: t.status.clone(),
                layer: None,
                x_position: 0,
            })
            .collect();

        let edges = self.collect_dependency_edges(
            self.tasks.iter().map(|t| &t.id),
            &DependencyType::Task,
        );

        self.graph_cache = Some(build_graph_cache(nodes, edges, NODE_HEIGHT_TASK, GraphLevel::Task));
    }

    /// Clear all graph caches so they are rebuilt on next entry.
    fn invalidate_graph_caches(&mut self) {
        self.graph_cache = None;
        self.epic_graph_cache = None;
        self.task_graph_cache = None;
    }

    /// Collect outgoing dependency edges for the given item IDs and type.
    fn collect_dependency_edges<'a>(
        &self,
        item_ids: impl Iterator<Item = &'a String>,
        dep_type: &DependencyType,
    ) -> Vec<Edge> {
        let mut edges = Vec::new();
        for id in item_ids {
            if let Ok(deps) = get_blocked_by(&self.db, dep_type, id) {
                for dep in deps {
                    if &dep.blocked_type == dep_type {
                        edges.push(Edge {
                            from: id.clone(),
                            to: dep.blocked_id,
                        });
                    }
                }
            }
        }
        edges
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

        // Auto-initialize .blueprint/setting.json if the directory exists but the file doesn't
        if let (Some(project), Ok(cwd)) = (self.selected_project(), std::env::current_dir())
            && Settings::blueprint_dir_exists_in(&cwd)
            && !Settings::exists_in(&cwd)
            && let Err(e) = Settings::save_to(&cwd, &project.id)
        {
            eprintln!("Warning: failed to write .blueprint/setting.json: {e}");
        }
    }

    fn reset_scroll(&mut self) {
        self.scroll_x = 0;
        self.scroll_y = 0;
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

    #[test]
    fn d_switches_to_graph_view() {
        let (mut app, _dir) = app_with_epics(2);
        assert_eq!(app.mode, InputMode::Normal);

        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        assert_eq!(app.mode, InputMode::GraphView);
        assert_eq!(app.graph_mode, GraphLevel::Epic);
        assert!(app.graph_cache.is_some());
    }

    #[test]
    fn esc_in_graph_view_returns_to_normal() {
        let (mut app, _dir) = app_with_epics(2);

        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        assert_eq!(app.mode, InputMode::GraphView);

        app.handle_key(KeyEvent::from(KeyCode::Esc));
        assert_eq!(app.mode, InputMode::Normal);
    }

    #[test]
    fn build_epic_graph_produces_correct_layout() {
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
                project_id: project.id.clone(),
                title: "Epic B".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let epic_c = create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id,
                title: "Epic C".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        // A blocks B, A blocks C
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
        add_dependency(
            &db,
            AddDependencyInput {
                blocker_type: DependencyType::Epic,
                blocker_id: epic_a.id.clone(),
                blocked_type: DependencyType::Epic,
                blocked_id: epic_c.id.clone(),
            },
        )
        .unwrap();

        let mut app = App::new(db).unwrap();
        app.build_epic_graph();

        let cache = app.graph_cache.as_ref().unwrap();

        // Should have 3 nodes, 2 edges, 2 layers
        assert_eq!(cache.layout.nodes.len(), 3);
        assert_eq!(cache.layout.edges.len(), 2);
        assert_eq!(cache.layout.layers.len(), 2);

        // All 3 nodes should have positions
        assert_eq!(cache.node_positions.len(), 3);

        // Epic A should be in layer 0, B and C in layer 1
        assert_eq!(cache.layout.nodes[&epic_a.id].layer, Some(0));
        assert_eq!(cache.layout.nodes[&epic_b.id].layer, Some(1));
        assert_eq!(cache.layout.nodes[&epic_c.id].layer, Some(1));
    }

    #[test]
    fn graph_key_1_stays_epic_2_switches_to_task() {
        let (mut app, _dir) = app_with_epics(2);
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        assert_eq!(app.graph_mode, GraphLevel::Epic);
        assert!(app.graph_cache.is_some());

        // Pressing 1 while already in Epic mode does nothing
        app.handle_key(KeyEvent::from(KeyCode::Char('1')));
        assert_eq!(app.graph_mode, GraphLevel::Epic);

        // Pressing 2 switches to Task mode and builds task graph
        app.handle_key(KeyEvent::from(KeyCode::Char('2')));
        assert_eq!(app.graph_mode, GraphLevel::Task);
        assert!(app.graph_cache.is_some());
        assert_eq!(app.graph_cache.as_ref().unwrap().level, GraphLevel::Task);
    }

    #[test]
    fn graph_cache_rebuilt_on_data_refresh_in_graph_view() {
        let (mut app, _dir) = app_with_epics(2);

        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        assert!(app.graph_cache.is_some());

        // Refresh while in GraphView should rebuild, not clear
        app.refresh_data();
        assert!(app.graph_cache.is_some());
    }

    #[test]
    fn graph_cache_invalidated_on_data_refresh_outside_graph_view() {
        let (mut app, _dir) = app_with_epics(2);

        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        assert!(app.graph_cache.is_some());

        // Leave graph view, then refresh — cache should be cleared
        app.handle_key(KeyEvent::from(KeyCode::Esc));
        app.refresh_data();
        assert!(app.graph_cache.is_none());
    }

    #[test]
    fn build_task_graph_produces_correct_layout() {
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
                title: "Task A".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let t2 = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id.clone(),
                title: "Task B".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let t3 = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id.clone(),
                title: "Task C".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        // t1 -> t2, t1 -> t3
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
        add_dependency(
            &db,
            AddDependencyInput {
                blocker_type: DependencyType::Task,
                blocker_id: t1.id.clone(),
                blocked_type: DependencyType::Task,
                blocked_id: t3.id.clone(),
            },
        )
        .unwrap();

        let mut app = App::new(db).unwrap();
        app.build_task_graph();

        let cache = app.graph_cache.as_ref().unwrap();
        assert_eq!(cache.level, GraphLevel::Task);
        assert_eq!(cache.layout.nodes.len(), 3);
        assert_eq!(cache.layout.edges.len(), 2);
        assert_eq!(cache.layout.layers.len(), 2);
        // All 3 nodes should have positions
        assert_eq!(cache.node_positions.len(), 3);
    }

    #[test]
    fn build_task_graph_no_epic_selected_clears_cache() {
        let (mut app, _dir) = app_with_projects(1);
        // No epics loaded
        assert!(app.epics.is_empty());
        app.build_task_graph();
        assert!(app.graph_cache.is_none());
    }

    #[test]
    fn pressing_2_in_graph_view_sets_task_mode() {
        let (mut app, _dir) = app_with_tasks(2);
        // Enter graph view (starts in epic mode)
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        assert_eq!(app.mode, InputMode::GraphView);
        assert_eq!(app.graph_mode, GraphLevel::Epic);

        // Press 2 to switch to task mode
        app.handle_key(KeyEvent::from(KeyCode::Char('2')));
        assert_eq!(app.graph_mode, GraphLevel::Task);
        assert!(app.graph_cache.is_some());
        assert_eq!(app.graph_cache.as_ref().unwrap().level, GraphLevel::Task);
    }

    #[test]
    fn build_task_graph_orphan_tasks_positioned() {
        // Tasks with no dependencies should be placed as orphans
        let (mut app, _dir) = app_with_tasks(3);
        app.build_task_graph();

        let cache = app.graph_cache.as_ref().unwrap();
        assert_eq!(cache.level, GraphLevel::Task);
        // 3 tasks, no deps → all orphans
        assert_eq!(cache.layout.orphans.len(), 3);
        assert_eq!(cache.node_positions.len(), 3);
    }

    // ==================== Scroll tests ====================

    #[test]
    fn scroll_initial_state_is_zero() {
        let (app, _dir) = app_with_epics(2);
        assert_eq!(app.scroll_x, 0);
        assert_eq!(app.scroll_y, 0);
    }

    #[test]
    fn scroll_j_k_adjusts_scroll_y() {
        let (mut app, _dir) = app_with_epics(2);
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        assert_eq!(app.mode, InputMode::GraphView);

        app.handle_key(KeyEvent::from(KeyCode::Char('j')));
        assert_eq!(app.scroll_y, 1);
        app.handle_key(KeyEvent::from(KeyCode::Char('j')));
        assert_eq!(app.scroll_y, 2);
        app.handle_key(KeyEvent::from(KeyCode::Char('k')));
        assert_eq!(app.scroll_y, 1);
    }

    #[test]
    fn scroll_h_l_adjusts_scroll_x() {
        let (mut app, _dir) = app_with_epics(2);
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));

        app.handle_key(KeyEvent::from(KeyCode::Char('l')));
        assert_eq!(app.scroll_x, 1);
        app.handle_key(KeyEvent::from(KeyCode::Char('l')));
        assert_eq!(app.scroll_x, 2);
        app.handle_key(KeyEvent::from(KeyCode::Char('h')));
        assert_eq!(app.scroll_x, 1);
    }

    #[test]
    fn arrow_keys_navigate_nodes_not_scroll() {
        let (mut app, _dir) = app_with_epics(2);
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));

        // Arrow keys should set focused_node, not scroll
        app.handle_key(KeyEvent::from(KeyCode::Down));
        assert!(app.focused_node.is_some(), "Down arrow should focus a node");
        assert_eq!(app.scroll_y, 0, "scroll_y should not change from arrow keys (unless auto-scroll)");

        // hjkl should still scroll
        app.handle_key(KeyEvent::from(KeyCode::Char('j')));
        assert_eq!(app.scroll_y, 1, "j should scroll");
    }

    #[test]
    fn scroll_k_does_not_go_below_zero() {
        let (mut app, _dir) = app_with_epics(2);
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        assert_eq!(app.scroll_y, 0);

        app.handle_key(KeyEvent::from(KeyCode::Char('k')));
        assert_eq!(app.scroll_y, 0);
    }

    #[test]
    fn scroll_h_does_not_go_below_zero() {
        let (mut app, _dir) = app_with_epics(2);
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        assert_eq!(app.scroll_x, 0);

        app.handle_key(KeyEvent::from(KeyCode::Char('h')));
        assert_eq!(app.scroll_x, 0);
    }

    #[test]
    fn scroll_resets_on_view_switch_1() {
        let (mut app, _dir) = app_with_tasks(2);
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));

        // Scroll down
        app.handle_key(KeyEvent::from(KeyCode::Char('j')));
        app.handle_key(KeyEvent::from(KeyCode::Char('l')));
        assert_eq!(app.scroll_y, 1);
        assert_eq!(app.scroll_x, 1);

        // Switch to tasks
        app.handle_key(KeyEvent::from(KeyCode::Char('2')));
        assert_eq!(app.scroll_x, 0);
        assert_eq!(app.scroll_y, 0);
    }

    #[test]
    fn scroll_resets_on_view_switch_2() {
        let (mut app, _dir) = app_with_tasks(2);
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));

        // Switch to task then scroll
        app.handle_key(KeyEvent::from(KeyCode::Char('2')));
        app.handle_key(KeyEvent::from(KeyCode::Char('j')));
        assert_eq!(app.scroll_y, 1);

        // Switch back to epic
        app.handle_key(KeyEvent::from(KeyCode::Char('1')));
        assert_eq!(app.scroll_x, 0);
        assert_eq!(app.scroll_y, 0);
    }

    #[test]
    fn scroll_resets_when_entering_graph_view() {
        let (mut app, _dir) = app_with_epics(2);

        // Manually set scroll values as if they were leftover
        app.scroll_x = 5;
        app.scroll_y = 10;

        // Enter graph view
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        assert_eq!(app.scroll_x, 0);
        assert_eq!(app.scroll_y, 0);
    }

    // ==================== Dual-pane tests ====================

    #[test]
    fn key_3_toggles_dual_pane_on_off() {
        let (mut app, _dir) = app_with_epics(2);
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        assert!(!app.dual_pane);

        // Toggle on
        app.handle_key(KeyEvent::from(KeyCode::Char('3')));
        assert!(app.dual_pane);
        assert_eq!(app.active_pane, GraphPane::Left);
        assert!(app.epic_graph_cache.is_some());

        // Toggle off
        app.handle_key(KeyEvent::from(KeyCode::Char('3')));
        assert!(!app.dual_pane);
        assert_eq!(app.graph_mode, GraphLevel::Epic);
    }

    #[test]
    fn tab_switches_active_pane_in_dual_mode() {
        let (mut app, _dir) = app_with_epics(2);
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        app.handle_key(KeyEvent::from(KeyCode::Char('3')));
        assert_eq!(app.active_pane, GraphPane::Left);

        app.handle_key(KeyEvent::from(KeyCode::Tab));
        assert_eq!(app.active_pane, GraphPane::Right);

        app.handle_key(KeyEvent::from(KeyCode::Tab));
        assert_eq!(app.active_pane, GraphPane::Left);
    }

    #[test]
    fn tab_noop_in_single_graph_mode() {
        let (mut app, _dir) = app_with_epics(2);
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        assert!(!app.dual_pane);
        assert_eq!(app.graph_mode, GraphLevel::Epic);

        // Tab in single graph mode should do nothing (no Tab handler for single)
        app.handle_key(KeyEvent::from(KeyCode::Tab));
        assert_eq!(app.mode, InputMode::GraphView);
        assert!(!app.dual_pane);
    }

    #[test]
    fn dual_scroll_routes_to_active_pane() {
        let (mut app, _dir) = app_with_tasks(2);
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        app.handle_key(KeyEvent::from(KeyCode::Char('3')));

        // Active pane is Left (epics)
        app.handle_key(KeyEvent::from(KeyCode::Char('j')));
        assert_eq!(app.epic_scroll_y, 1);
        assert_eq!(app.task_scroll_y, 0);

        app.handle_key(KeyEvent::from(KeyCode::Char('l')));
        assert_eq!(app.epic_scroll_x, 1);
        assert_eq!(app.task_scroll_x, 0);

        // Switch to right pane
        app.handle_key(KeyEvent::from(KeyCode::Tab));
        app.handle_key(KeyEvent::from(KeyCode::Char('j')));
        assert_eq!(app.task_scroll_y, 1);
        assert_eq!(app.epic_scroll_y, 1); // unchanged

        app.handle_key(KeyEvent::from(KeyCode::Char('l')));
        assert_eq!(app.task_scroll_x, 1);
        assert_eq!(app.epic_scroll_x, 1); // unchanged
    }

    #[test]
    fn key_1_in_dual_exits_to_single_epic() {
        let (mut app, _dir) = app_with_epics(2);
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        app.handle_key(KeyEvent::from(KeyCode::Char('3')));
        assert!(app.dual_pane);

        app.handle_key(KeyEvent::from(KeyCode::Char('1')));
        assert!(!app.dual_pane);
        assert_eq!(app.graph_mode, GraphLevel::Epic);
    }

    #[test]
    fn key_2_in_dual_exits_to_single_task() {
        let (mut app, _dir) = app_with_tasks(2);
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        app.handle_key(KeyEvent::from(KeyCode::Char('3')));
        assert!(app.dual_pane);

        app.handle_key(KeyEvent::from(KeyCode::Char('2')));
        assert!(!app.dual_pane);
        assert_eq!(app.graph_mode, GraphLevel::Task);
    }

    #[test]
    fn esc_in_dual_returns_to_single_not_normal() {
        let (mut app, _dir) = app_with_epics(2);
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        app.handle_key(KeyEvent::from(KeyCode::Char('3')));
        assert!(app.dual_pane);

        app.handle_key(KeyEvent::from(KeyCode::Esc));
        assert!(!app.dual_pane);
        assert_eq!(app.mode, InputMode::GraphView);
    }

    #[test]
    fn dual_caches_rebuilt_on_refresh_in_graph_view() {
        let (mut app, _dir) = app_with_tasks(2);
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        app.handle_key(KeyEvent::from(KeyCode::Char('3')));
        assert!(app.epic_graph_cache.is_some());

        // Refresh while in dual-pane GraphView should rebuild, not clear
        app.refresh_data();
        assert!(app.epic_graph_cache.is_some());
        assert!(app.task_graph_cache.is_some());
    }

    #[test]
    fn dual_caches_invalidated_on_refresh_outside_graph_view() {
        let (mut app, _dir) = app_with_tasks(2);
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        app.handle_key(KeyEvent::from(KeyCode::Char('3')));
        assert!(app.epic_graph_cache.is_some());

        // Leave graph view, then refresh — caches should be cleared
        app.handle_key(KeyEvent::from(KeyCode::Esc));
        app.handle_key(KeyEvent::from(KeyCode::Esc));
        app.refresh_data();
        assert!(app.epic_graph_cache.is_none());
        assert!(app.task_graph_cache.is_none());
    }

    // ==================== Node navigation tests ====================

    #[test]
    fn first_arrow_key_selects_first_node() {
        let (mut app, _dir) = app_with_epics(3);
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        assert!(app.focused_node.is_none());

        app.handle_key(KeyEvent::from(KeyCode::Down));
        assert!(app.focused_node.is_some(), "first arrow key should focus a node");
    }

    #[test]
    fn arrow_down_up_navigates_between_layers() {
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
                title: "A".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let epic_b = create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id,
                title: "B".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        // A blocks B → two layers
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

        let mut app = App::new(db).unwrap();
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));

        // First arrow selects first node (layer 0)
        app.handle_key(KeyEvent::from(KeyCode::Down));
        let first = app.focused_node.clone().unwrap();
        assert_eq!(first, epic_a.id, "layer 0 should contain the blocker");

        // Down again should move to layer 1
        app.handle_key(KeyEvent::from(KeyCode::Down));
        let second = app.focused_node.clone().unwrap();
        assert_eq!(second, epic_b.id, "layer 1 should contain the blocked epic");

        // Up should go back to layer 0
        app.handle_key(KeyEvent::from(KeyCode::Up));
        let third = app.focused_node.clone().unwrap();
        assert_eq!(third, epic_a.id, "Up should return to layer 0");
    }

    #[test]
    fn left_right_wraps_within_layer() {
        // 3 orphans = all in one layer row
        let (mut app, _dir) = app_with_epics(3);
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));

        // Focus first node
        app.handle_key(KeyEvent::from(KeyCode::Right));
        let first = app.focused_node.clone().unwrap();

        // Right should move to second
        app.handle_key(KeyEvent::from(KeyCode::Right));
        let second = app.focused_node.clone().unwrap();
        assert_ne!(first, second, "Right should move to a different node");

        // Keep pressing Right to wrap back to the first
        app.handle_key(KeyEvent::from(KeyCode::Right));
        app.handle_key(KeyEvent::from(KeyCode::Right));
        let wrapped = app.focused_node.clone().unwrap();
        assert_eq!(wrapped, first, "Right should wrap around to the first node");
    }

    #[test]
    fn focus_cleared_on_entering_graph_view() {
        let (mut app, _dir) = app_with_epics(2);
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        app.handle_key(KeyEvent::from(KeyCode::Down));
        assert!(app.focused_node.is_some());

        // Exit and re-enter
        app.handle_key(KeyEvent::from(KeyCode::Esc));
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        assert!(app.focused_node.is_none(), "Focus should be cleared on re-entry");
    }

    #[test]
    fn focus_cleared_on_mode_switch_1_and_2() {
        let (mut app, _dir) = app_with_tasks(2);
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        app.handle_key(KeyEvent::from(KeyCode::Down));
        assert!(app.focused_node.is_some());

        // Switch to task mode
        app.handle_key(KeyEvent::from(KeyCode::Char('2')));
        assert!(app.focused_node.is_none(), "Focus should be cleared on mode 2");

        app.handle_key(KeyEvent::from(KeyCode::Down));
        assert!(app.focused_node.is_some());

        // Switch to epic mode
        app.handle_key(KeyEvent::from(KeyCode::Char('1')));
        assert!(app.focused_node.is_none(), "Focus should be cleared on mode 1");
    }

    #[test]
    fn dual_pane_tracks_focus_independently() {
        let (mut app, _dir) = app_with_tasks(2);
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        app.handle_key(KeyEvent::from(KeyCode::Char('3')));

        // Focus a node in the left (epic) pane
        app.handle_key(KeyEvent::from(KeyCode::Down));
        assert!(app.epic_focused_node.is_some());
        assert!(app.task_focused_node.is_none());

        // Switch to right pane
        app.handle_key(KeyEvent::from(KeyCode::Tab));
        app.handle_key(KeyEvent::from(KeyCode::Down));
        assert!(app.task_focused_node.is_some());
        // Epic focus should remain unchanged
        assert!(app.epic_focused_node.is_some());
    }

    #[test]
    fn hjkl_still_scrolls_in_graph_view() {
        let (mut app, _dir) = app_with_epics(2);
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));

        app.handle_key(KeyEvent::from(KeyCode::Char('j')));
        assert_eq!(app.scroll_y, 1);
        app.handle_key(KeyEvent::from(KeyCode::Char('k')));
        assert_eq!(app.scroll_y, 0);
        app.handle_key(KeyEvent::from(KeyCode::Char('l')));
        assert_eq!(app.scroll_x, 1);
        app.handle_key(KeyEvent::from(KeyCode::Char('h')));
        assert_eq!(app.scroll_x, 0);
    }

    #[test]
    fn dual_pane_epic_focus_updates_task_graph() {
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
                project_id: project.id.clone(),
                title: "Epic B".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        // A blocks B so they're in separate layers
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
        // Create a task in each epic
        create_task(
            &db,
            CreateTaskInput {
                epic_id: epic_a.id.clone(),
                title: "Task in A".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        create_task(
            &db,
            CreateTaskInput {
                epic_id: epic_b.id.clone(),
                title: "Task in B".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        let mut app = App::new(db).unwrap();

        // Enter graph view then dual-pane mode
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        app.handle_key(KeyEvent::from(KeyCode::Char('3')));
        assert!(app.dual_pane);

        // Focus the first epic node (layer 0 = epic_a, the blocker)
        app.handle_key(KeyEvent::from(KeyCode::Down));
        let first_focused = app.epic_focused_node.clone().unwrap();
        assert_eq!(first_focused, epic_a.id, "First focus should be on epic A (layer 0)");
        let epic_a_idx = app.selected_epic_idx;
        let tasks_for_first = app.tasks.iter().map(|t| t.title.clone()).collect::<Vec<_>>();

        // Navigate down to the second epic (layer 1 = epic_b)
        app.handle_key(KeyEvent::from(KeyCode::Down));
        let second_focused = app.epic_focused_node.clone().unwrap();
        assert_eq!(second_focused, epic_b.id, "Second focus should be on epic B (layer 1)");

        // The selected epic should have changed
        assert_ne!(
            app.selected_epic_idx, epic_a_idx,
            "selected_epic_idx should change when focusing a different epic"
        );

        // The task graph should have been rebuilt for the new epic
        assert!(app.task_graph_cache.is_some(), "task graph cache should be rebuilt");
        let tasks_for_second = app.tasks.iter().map(|t| t.title.clone()).collect::<Vec<_>>();
        assert_ne!(
            tasks_for_first, tasks_for_second,
            "tasks should reflect the newly focused epic"
        );

        // Task focus and scroll should be reset
        assert!(app.task_focused_node.is_none(), "task focus should be cleared");
        assert_eq!(app.task_scroll_x, 0);
        assert_eq!(app.task_scroll_y, 0);
    }
}
