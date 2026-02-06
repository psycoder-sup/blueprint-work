# Blueprint - Product Requirements Document

## Overview

**Blueprint** is a Rust-based MCP (Model Context Protocol) server and TUI dashboard for managing project work breakdown. It enables LLMs (like Claude) to create, manage, and track **Projects**, **Epics**, and **Blue-Tasks** through MCP tools, while providing a cyberpunk-styled terminal dashboard for human visibility.

### Hierarchy

```
Project → Epics → Blue-Tasks
```

A **Project** is the top-level container (e.g. "Blueprint v1.0"). Each project has **Epics** (major workstreams), and each epic has **Blue-Tasks** (concrete deliverables).

### Core Flow

```
Create Project --> Feed PRD text --> LLM analyzes & breaks down --> Creates Epics via MCP --> Creates Blue-Tasks per Epic via MCP
```

The MCP server provides the tools. The LLM does the thinking — reading a PRD, deciding how to decompose it into epics and tasks, and calling the MCP tools to persist them.

---

## Architecture

### Single Binary, Multiple Modes

```
blueprint serve    # Start MCP server (stdio JSON-RPC)
blueprint tui      # Launch cyberpunk TUI dashboard
blueprint status   # Quick CLI status overview
```

All modes share the same SQLite database.

### Tech Stack

| Component        | Choice                          |
|------------------|---------------------------------|
| Language         | Rust                            |
| MCP Transport    | stdio (JSON-RPC 2.0)           |
| Storage          | SQLite via rusqlite             |
| TUI Framework    | ratatui + crossterm             |
| Serialization    | serde + serde_json              |
| Async Runtime    | tokio                           |

### Project Structure

```
blueprint/
├── Cargo.toml
├── src/
│   ├── main.rs              # CLI entry point (clap)
│   ├── mcp/
│   │   ├── mod.rs            # MCP server setup, JSON-RPC handler
│   │   ├── tools.rs          # Tool definitions & dispatch
│   │   └── types.rs          # MCP protocol types
│   ├── db/
│   │   ├── mod.rs            # Database connection & migrations
│   │   ├── project.rs        # Project CRUD operations
│   │   ├── epic.rs           # Epic CRUD operations
│   │   └── task.rs           # Blue-Task CRUD operations
│   ├── models/
│   │   ├── mod.rs
│   │   ├── project.rs        # Project struct
│   │   ├── epic.rs           # Epic struct & status
│   │   └── task.rs           # BlueTask struct & status
│   ├── tui/
│   │   ├── mod.rs            # TUI app state & event loop
│   │   ├── ui.rs             # Main layout & rendering
│   │   ├── theme.rs          # Cyberpunk neon color palette
│   │   ├── graph.rs          # DAG layout engine & renderer
│   │   └── widgets/          # Custom styled widgets
│   └── cli/
│       └── status.rs         # Quick status command
├── migrations/
│   └── 001_init.sql
└── PRD.md
```

---

## Data Model

### Project

| Field          | Type        | Description                                  |
|----------------|-------------|----------------------------------------------|
| id             | TEXT (ULID) | Unique identifier                            |
| name           | TEXT        | Project name                                 |
| description    | TEXT        | Project description                          |
| status         | TEXT        | `active` \| `archived`                       |
| created_at     | TIMESTAMP   | Creation time                                |
| updated_at     | TIMESTAMP   | Last modification time                       |

### Epic

| Field          | Type        | Description                                  |
|----------------|-------------|----------------------------------------------|
| id             | TEXT (ULID) | Unique identifier                            |
| project_id     | TEXT        | Parent project (foreign key)                 |
| title          | TEXT        | Epic title                                   |
| description    | TEXT        | Detailed description                         |
| status         | TEXT        | `todo` \| `in_progress` \| `done`            |
| created_at     | TIMESTAMP   | Creation time                                |
| updated_at     | TIMESTAMP   | Last modification time                       |

### Blue-Task

| Field          | Type        | Description                                  |
|----------------|-------------|----------------------------------------------|
| id             | TEXT (ULID) | Unique identifier                            |
| epic_id        | TEXT        | Parent epic (foreign key)                    |
| title          | TEXT        | Task title                                   |
| description    | TEXT        | Detailed description / acceptance criteria   |
| status         | TEXT        | `todo` \| `in_progress` \| `done`            |
| created_at     | TIMESTAMP   | Creation time                                |
| updated_at     | TIMESTAMP   | Last modification time                       |

### Dependencies

| Field          | Type        | Description                                  |
|----------------|-------------|----------------------------------------------|
| id             | INTEGER     | Auto-increment primary key                   |
| blocker_type   | TEXT        | `epic` \| `task`                             |
| blocker_id     | TEXT        | ID of the blocking item                      |
| blocked_type   | TEXT        | `epic` \| `task`                             |
| blocked_id     | TEXT        | ID of the blocked item                       |

A single dependency table handles both epic-level and task-level blocks/blocked-by relationships.

### Status Rules

- An item with unresolved blockers (blocker not `done`) is effectively **blocked** — the TUI and status tool should surface this.
- Status transitions: `todo` → `in_progress` → `done`. No backwards transitions enforced (soft model).

---

## MCP Tools

The server exposes the following tools over stdio JSON-RPC:

### 1. `create_project`

Create a new project.

**Input:**
```json
{
  "name": "Blueprint v1.0",
  "description": "MCP task management server and TUI"
}
```

**Output:** Created project object with generated ID.

### 2. `list_projects`

List all projects with optional status filter.

**Input:**
```json
{
  "status": "active"  // optional: "active" | "archived"
}
```

**Output:** Array of project objects with epic counts.

### 3. `get_project`

Get a single project by ID, including its epics summary.

**Input:**
```json
{
  "id": "01HXK..."
}
```

**Output:** Project object with nested epics overview.

### 4. `update_project`

Update a project's name, description, or status.

**Input:**
```json
{
  "id": "01HXK...",
  "name": "Updated Name",            // optional
  "description": "Updated desc",     // optional
  "status": "archived"               // optional
}
```

### 5. `delete_project`

Delete a project and all its epics and blue-tasks.

**Input:**
```json
{
  "id": "01HXK..."
}
```

### 6. `create_epic`

Create a new epic under a project.

**Input:**
```json
{
  "project_id": "01HXK...",
  "title": "User Authentication",
  "description": "Implement login, signup, and session management"
}
```

**Output:** Created epic object with generated ID.

### 7. `list_epics`

List epics with optional filters.

**Input:**
```json
{
  "project_id": "01HXK...",   // optional: filter by project
  "status": "in_progress"     // optional: filter by status
}
```

**Output:** Array of epic objects.

### 8. `get_epic`

Get a single epic by ID, including its blue-tasks and dependencies.

**Input:**
```json
{
  "id": "01HXK..."
}
```

**Output:** Epic object with nested tasks and dependency info.

### 9. `update_epic`

Update an epic's title, description, or status.

**Input:**
```json
{
  "id": "01HXK...",
  "title": "Updated Title",        // optional
  "description": "Updated desc",   // optional
  "status": "in_progress"          // optional
}
```

### 10. `delete_epic`

Delete an epic and all its blue-tasks.

**Input:**
```json
{
  "id": "01HXK..."
}
```

### 11. `create_task`

Create a new blue-task under an epic.

**Input:**
```json
{
  "epic_id": "01HXK...",
  "title": "Implement JWT token generation",
  "description": "Create JWT tokens with RS256 signing..."
}
```

### 12. `list_tasks`

List blue-tasks with optional filters.

**Input:**
```json
{
  "epic_id": "01HXK...",   // optional: filter by epic
  "status": "todo"          // optional: filter by status
}
```

### 13. `get_task`

Get a single blue-task by ID with dependency info.

**Input:**
```json
{
  "id": "01HXK..."
}
```

### 14. `update_task`

Update a blue-task's title, description, or status.

**Input:**
```json
{
  "id": "01HXK...",
  "title": "Updated Title",        // optional
  "description": "Updated desc",   // optional
  "status": "done"                  // optional
}
```

### 15. `delete_task`

Delete a blue-task.

**Input:**
```json
{
  "id": "01HXK..."
}
```

### 16. `add_dependency`

Create a blocks/blocked-by relationship.

**Input:**
```json
{
  "blocker_type": "task",
  "blocker_id": "01HXK...",
  "blocked_type": "task",
  "blocked_id": "01HXK..."
}
```

### 17. `remove_dependency`

Remove a dependency relationship.

**Input:**
```json
{
  "blocker_type": "task",
  "blocker_id": "01HXK...",
  "blocked_type": "task",
  "blocked_id": "01HXK..."
}
```

### 18. `get_status`

Get a status overview for a specific project or all projects.

**Input:**
```json
{
  "project_id": "01HXK..."  // optional: if omitted, returns overview across all projects
}
```

**Output:**
```json
{
  "project": "Blueprint v1.0",
  "total_epics": 5,
  "epics_by_status": { "todo": 2, "in_progress": 2, "done": 1 },
  "total_tasks": 23,
  "tasks_by_status": { "todo": 10, "in_progress": 8, "done": 5 },
  "blocked_items": [
    { "type": "task", "id": "01HXK...", "title": "...", "blocked_by": ["01HXK..."] }
  ]
}
```

### 19. `feed_prd`

Accept PRD text, associate it with a project, and store it. Returns guidance for the LLM to break it down.

**Input:**
```json
{
  "project_id": "01HXK...",
  "content": "# My Project PRD\n\n## Goals\n...",
  "title": "My Project PRD"
}
```

**Output:**
```json
{
  "message": "PRD stored. Please analyze the content and create epics and blue-tasks.",
  "prd_id": "01HXK...",
  "guide": "Break down this PRD into epics (major workstreams) and blue-tasks (concrete deliverables). For each epic, create it with create_epic using project_id, then create its tasks with create_task. Set up dependencies with add_dependency where tasks or epics have ordering constraints."
}
```

The LLM then reads this guidance and uses `create_epic`, `create_task`, and `add_dependency` to build out the full work breakdown.

---

## TUI Dashboard

### Design: Cyberpunk Terminal

**Color Palette:**
- Background: Deep black (`#0a0a0f`)
- Primary neon: Cyan (`#00fff5`)
- Secondary neon: Magenta/Hot pink (`#ff00ff` / `#ff2d6f`)
- Accent: Neon green (`#39ff14`)
- Warning: Neon orange (`#ff6e27`)
- Done/Success: Electric blue (`#00d4ff`)
- Text: Light gray (`#b0b0b0`), bright white for emphasis
- Borders: Dim cyan (`#005f5f`) with bright cyan for focused panels

**Visual Elements:**
- Heavy box-drawing characters for panel borders (`╔═╗║╚═╝`)
- ASCII art header with "BLUEPRINT" in stylized glitch/cyber font
- Status indicators with neon-colored unicode symbols (`◉ ◆ ▶ ■`)
- Progress bars with block characters (`█░`)
- Blinking cursor effects on active elements
- Dim scan-line separators between items

### Layout

```
╔══════════════════════════════════════════════════════════════════════╗
║  ▓▓ BLUEPRINT ▓▓                                 [STATUS: ONLINE]  ║
║  PROJECT: Blueprint v1.0                      [◀ prev | next ▶]   ║
╠════════════════════════╦═════════════════════════════════════════════╣
║  EPICS                 ║  BLUE-TASKS                                ║
║  ───────────────────── ║  ───────────────────────────────────────── ║
║  ◉ Auth System   [3/7] ║  ◆ Implement JWT tokens         [TODO]    ║
║  ◉ API Layer     [1/5] ║  ◆ Create login endpoint     [IN_PROG]    ║
║  ◉ Frontend      [0/4] ║  ▶ Add password hashing         [TODO]    ║
║  ◉ Deploy        [0/3] ║  ■ Setup user model             [DONE]    ║
║                        ║  ◆ Session management            [TODO]    ║
║                        ║  ◆ OAuth integration             [TODO]    ║
║                        ║  ◆ Rate limiting                 [TODO]    ║
╠════════════════════════╩═════════════════════════════════════════════╣
║  DEPENDENCIES (mini)               ║  PROJECT STATUS                ║
║  JWT tokens ──blocks──▶ Login      ║  Epics:  ██░░░  1/5           ║
║  User model ──blocks──▶ JWT       ║  Tasks:  ████░  5/19          ║
║  [d] Full Dependency Graph         ║  Blocked: 2 items              ║
╚══════════════════════════════════════════════════════════════════════╝
```

### Dependency Graph View (Full-Screen, press `d`)

A dedicated full-screen DAG (directed acyclic graph) view rendered with ASCII art and neon edges. Toggles between **epic-level** and **task-level** graphs.

**Epic-Level Graph** — shows dependencies between epics across the project:

```
╔══════════════════════════════════════════════════════════════════════════╗
║  ▓▓ DEPENDENCY GRAPH ▓▓           [EPICS]  tasks    [ESC to go back]   ║
╠══════════════════════════════════════════════════════════════════════════╣
║                                                                         ║
║   ┌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┐         ┌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┐                         ║
║   ╎ ◉ Auth System  ╎         ╎ ◉ API Layer    ╎                         ║
║   ╎   [3/7] ░░██░  ╎         ╎   [1/5] ░████  ╎                         ║
║   └╌╌╌╌╌╌╌┰╌╌╌╌╌╌╌┘         └╌╌╌╌╌╌╌┰╌╌╌╌╌╌╌┘                         ║
║           │                         │                                    ║
║           ╰─────────┬───────────────╯                                    ║
║                     │                                                    ║
║                     ▼                                                    ║
║           ╔═══════════════╗                                              ║
║           ║ ◉ Frontend    ║                                              ║
║           ║   [0/4] ░░░░░ ║                                              ║
║           ╚═══════╤═══════╝                                              ║
║                   │                                                      ║
║                   ▼                                                      ║
║           ╔═══════════════╗         ╔═══════════════╗                    ║
║           ║ ◉ Deploy      ║ ◄────── ║ ◉ Monitoring  ║                    ║
║           ║   [0/3] ░░░░░ ║         ║   [1/3] ░░░██ ║                    ║
║           ╚═══════════════╝         ╚═══════════════╝                    ║
║                                                                         ║
╠══════════════════════════════════════════════════════════════════════════╣
║  ◉ 5 epics  │  ─▶ 4 edges  │  ⚠ 2 blocked  │  ■ 0 done               ║
╚══════════════════════════════════════════════════════════════════════════╝
```

**Task-Level Graph** — shows dependencies between blue-tasks within the selected epic:

```
╔══════════════════════════════════════════════════════════════════════════╗
║  ▓▓ DEPENDENCY GRAPH ▓▓            epics  [TASKS]   [ESC to go back]   ║
║  EPIC: Auth System                                                      ║
╠══════════════════════════════════════════════════════════════════════════╣
║                                                                         ║
║   ╔══════════════════╗                                                   ║
║   ║ ■ Setup user     ║                                                   ║
║   ║   model    [DONE]║                                                   ║
║   ╚════════╤═════════╝                                                   ║
║            │                                                             ║
║            ▼                                                             ║
║   ╔══════════════════╗        ╔══════════════════╗                       ║
║   ║ ◆ Implement JWT  ║        ║ ◆ Add password   ║                       ║
║   ║   tokens   [TODO]║        ║   hashing  [TODO]║                       ║
║   ╚════════╤═════════╝        ╚════════╤═════════╝                       ║
║            │                           │                                 ║
║            ▼                           │                                 ║
║   ┌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┐                │                                 ║
║   ╎ ◆ Create login    ╎                │                                 ║
║   ╎   endpoint [PROG] ╎◄───────────────╯                                 ║
║   └╌╌╌╌╌╌╌╌┰╌╌╌╌╌╌╌╌╌┘                                                   ║
║            │                                                             ║
║            ╰──────────┬─────────────────────────┐                        ║
║                       ▼                         ▼                        ║
║   ╔══════════════════╗        ╔══════════════════╗                       ║
║   ║ ◆ Session mgmt   ║        ║ ◆ OAuth          ║                       ║
║   ║          [TODO]  ║        ║   integr.  [TODO]║                       ║
║   ╚══════════════════╝        ╚══════════════════╝                       ║
║                                                                         ║
║            ╔══════════════════╗                                           ║
║            ║ ◆ Rate limiting  ║  (no dependencies)                       ║
║            ║          [TODO]  ║                                           ║
║            ╚══════════════════╝                                           ║
║                                                                         ║
╠══════════════════════════════════════════════════════════════════════════╣
║  ◆ 7 tasks  │  ─▶ 5 edges  │  ⚠ 0 blocked  │  ■ 1 done               ║
╚══════════════════════════════════════════════════════════════════════════╝
```

**Graph Rendering Rules:**
- **Topological sort** — nodes laid out top-to-bottom by dependency depth (roots at top, leaves at bottom)
- **Node colors by status:** neon green (`DONE`), cyan (`IN_PROG`), dim gray (`TODO`), pulsing red/orange (`BLOCKED`)
- **Node border by status:**
  - `DONE` — solid double-line border (`╔═╗║╚═╝`), neon green
  - `TODO` — solid double-line border (`╔═╗║╚═╝`), dim gray
  - `IN_PROGRESS` — **animated dotted/dashed border** (`┌╌╌┐╎└╌╌┘`), neon cyan. The dots "march" around the border — each frame shifts the dash pattern by one position, creating a rotating/flowing effect around the node (cycle period: ~500ms per step, 4 frames total)
  - `BLOCKED` — solid double-line border, pulsing red/orange (alternates brightness)
- **Edge colors:** bright cyan for normal edges, neon red for edges into blocked nodes
- **Edge drawing:** `│` for vertical, `─` for horizontal, `╰` `┬` `╯` for corners, `▼` `▶` `◄` for direction arrows
- **Orphan nodes** (no dependencies) rendered in a separate row labeled "(no dependencies)"
- **Summary bar** at bottom: node count, edge count, blocked count, done count

**Marching Border Animation Detail:**

The in-progress dotted border cycles through 4 frames to create the illusion of movement:

```
Frame 1:  ┌╌╌╌╌╌╌┐    Frame 2:  ┌┄╌╌╌╌┄┐    Frame 3:  ┌╌┄╌╌┄╌┐    Frame 4:  ┌┄╌╌╌╌┄┐
          ╎      ╎              ┆      ┆              ╎      ╎              ┆      ┆
          └╌╌╌╌╌╌┘              └┄╌╌╌╌┄┘              └╌┄╌╌┄╌┘              └┄╌╌╌╌┄┘
```

Implementation: alternate between `╌` (box drawings light double dash horizontal) and `┄` (box drawings light triple dash horizontal) characters, shifting the pattern each frame. Vertical sides alternate between `╎` and `┆`. This creates a subtle "electricity flowing through wires" cyberpunk effect.

**Graph Layout Algorithm:**
1. Compute topological ordering of the DAG
2. Assign layers (depth) based on longest path from roots
3. Within each layer, minimize edge crossings (barycenter heuristic)
4. Render nodes as fixed-width boxes, route edges through available space
5. Scroll support for graphs larger than the viewport

### Interactions

| Key       | Action                              |
|-----------|-------------------------------------|
| `j/k`     | Navigate up/down                    |
| `h/l`     | Switch panels (epics ↔ tasks)      |
| `Enter`   | Expand/view details                 |
| `Tab`     | Cycle through panels                |
| `p`       | Switch project (project selector)   |
| `s`       | Cycle status (todo → in_progress → done) |
| `d`       | Toggle dependency graph (full-screen) |
| `1`       | Graph: show epic-level deps         |
| `2`       | Graph: show task-level deps         |
| `Esc`     | Exit graph / back to main view      |
| `q`       | Quit                                |
| `/`       | Filter/search                       |
| `?`       | Help overlay                        |

### Auto-refresh

The TUI watches the SQLite database for changes (polling at 1s interval) so it stays in sync when the MCP server modifies data.

---

## CLI: `blueprint status`

Quick, non-interactive status dump for terminal use. Shows a specific project or lists all projects.

```
$ blueprint status                      # list all projects
$ blueprint status --project 01HXK...   # specific project

 BLUEPRINT  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 PROJECT: Blueprint v1.0

 Epics:  ██████░░░░  3/5
 Tasks:  ████████░░  16/20
 Blocked: 2 items

 ◉ Auth System     ████████░░  7/8   in_progress
 ◉ API Layer       ██████░░░░  5/5   done
 ◉ Frontend        ████░░░░░░  3/4   in_progress
 ◉ Deploy          ░░░░░░░░░░  0/3   todo (blocked)
 ◉ Monitoring      ░░░░░░░░░░  1/3   todo
```

---

## Database Schema

```sql
CREATE TABLE projects (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status      TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'archived')),
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE epics (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title       TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status      TEXT NOT NULL DEFAULT 'todo' CHECK(status IN ('todo', 'in_progress', 'done')),
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE tasks (
    id          TEXT PRIMARY KEY,
    epic_id     TEXT NOT NULL REFERENCES epics(id) ON DELETE CASCADE,
    title       TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status      TEXT NOT NULL DEFAULT 'todo' CHECK(status IN ('todo', 'in_progress', 'done')),
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE dependencies (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    blocker_type TEXT NOT NULL CHECK(blocker_type IN ('epic', 'task')),
    blocker_id   TEXT NOT NULL,
    blocked_type TEXT NOT NULL CHECK(blocked_type IN ('epic', 'task')),
    blocked_id   TEXT NOT NULL,
    UNIQUE(blocker_type, blocker_id, blocked_type, blocked_id)
);

CREATE TABLE prds (
    id         TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title      TEXT NOT NULL,
    content    TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_epics_project_id ON epics(project_id);
CREATE INDEX idx_epics_status ON epics(status);
CREATE INDEX idx_tasks_epic_id ON tasks(epic_id);
CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_deps_blocker ON dependencies(blocker_type, blocker_id);
CREATE INDEX idx_deps_blocked ON dependencies(blocked_type, blocked_id);
CREATE INDEX idx_prds_project_id ON prds(project_id);
```

---

## MCP Server Configuration

Users add Blueprint to their MCP client config:

```json
{
  "mcpServers": {
    "blueprint": {
      "command": "blueprint",
      "args": ["serve"],
      "env": {
        "BLUEPRINT_DB": "~/.blueprint/blueprint.db"
      }
    }
  }
}
```

### Environment Variables

| Variable       | Default                      | Description               |
|----------------|------------------------------|---------------------------|
| `BLUEPRINT_DB` | `~/.blueprint/blueprint.db`  | SQLite database path      |

---

## Rust Dependencies (Cargo.toml)

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.31", features = ["bundled"] }
tokio = { version = "1", features = ["full"] }
ratatui = "0.28"
crossterm = "0.28"
ulid = "1"
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1"
```

---

## Milestones

### M1: Foundation
- [ ] Project scaffolding (Cargo workspace, clap CLI)
- [ ] SQLite schema & migrations
- [ ] Project CRUD (db layer)
- [ ] Epic and Blue-Task CRUD (db layer)
- [ ] Dependency management (db layer)

### M2: MCP Server
- [ ] stdio JSON-RPC transport
- [ ] MCP protocol handshake (initialize, tools/list)
- [ ] All 19 tool handlers wired up
- [ ] Integration test: Claude creates project/epics/tasks via MCP

### M3: TUI Dashboard
- [ ] ratatui app skeleton with event loop
- [ ] Cyberpunk theme & color palette
- [ ] Project selector panel
- [ ] Epic list panel + Task list panel
- [ ] Mini dependency panel (bottom bar)
- [ ] Status overview panel
- [ ] Keyboard navigation
- [ ] Auto-refresh from SQLite

### M3.5: Dependency Graph View
- [ ] DAG topological sort & layer assignment
- [ ] Barycenter heuristic for edge-crossing minimization
- [ ] ASCII box-node renderer with neon status colors
- [ ] Animated marching dotted border for in-progress nodes (4-frame cycle, ~500ms)
- [ ] Pulsing border for blocked nodes
- [ ] Edge routing (vertical/horizontal with corners & arrows)
- [ ] Epic-level graph view
- [ ] Task-level graph view (per selected epic)
- [ ] Toggle between epic/task views (`1`/`2` keys)
- [ ] Scroll support for large graphs
- [ ] Summary bar (node count, edge count, blocked, done)

### M4: CLI Status & Polish
- [ ] `blueprint status` quick overview command
- [ ] ASCII art header / branding
- [ ] Error handling & user-friendly messages
- [ ] README & usage docs
