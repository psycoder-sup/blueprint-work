# Blueprint JSON Schema Documentation

This document describes the JSON file format used by Blueprint for file-based storage in the `.blueprint/` directory.

## Directory Structure

```
.blueprint/
├── project.json          # Project metadata
├── setting.json          # Local settings (gitignored)
├── schema.md             # This documentation
├── dependencies.json     # All dependencies between epics/tasks
├── epics/
│   ├── E1.json
│   └── E2.json
└── tasks/
    ├── E1-T1.json
    ├── E1-T2.json
    └── E2-T1.json
```

## Design Principles

### Derived Fields

Several fields are **not stored** in JSON files because they can be derived:

| Field | Derived From |
|-------|-------------|
| `project_id` (Epic) | Directory location (one project per `.blueprint/`) |
| `short_id` (Epic) | Filename: `E1.json` → `"E1"` |
| `short_id` (Task) | Filename: `E1-T2.json` → `"E1-T2"` |
| `epic_id` (Task) | Filename prefix: `E1-T2.json` → epic `E1` |
| `task_count` (Epic) | Count of tasks matching `E{n}-T*.json` |
| `done_count` (Epic) | Count of tasks with `"status": "done"` |

### Human Readability

- Short IDs (`E1`, `E1-T2`) are used in filenames and dependencies for readability
- ULIDs are stored in `id` fields for unique identification
- All timestamps use ISO 8601 format

---

## Entity Schemas

### project.json

Root project metadata.

```json
{
  "id": "01KH0DPG02JCPAK1Y87Q4NKRMW",
  "name": "My Project",
  "description": "Project description",
  "status": "active",
  "created_at": "2025-02-09T14:00:00Z",
  "updated_at": "2025-02-09T14:00:00Z"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | Yes | ULID unique identifier |
| `name` | string | Yes | Project name |
| `description` | string | Yes | Project description (may be empty) |
| `status` | string | Yes | `"active"` or `"archived"` |
| `created_at` | string | Yes | ISO 8601 timestamp |
| `updated_at` | string | Yes | ISO 8601 timestamp |

---

### epics/E{n}.json

Epic files are named with their short ID (e.g., `E1.json`, `E2.json`).

```json
{
  "id": "01KH0DPG02JCPAK1Y87Q4NKRMX",
  "title": "Epic title",
  "description": "Epic description",
  "status": "in_progress",
  "created_at": "2025-02-09T14:00:00Z",
  "updated_at": "2025-02-09T14:00:00Z"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | Yes | ULID unique identifier |
| `title` | string | Yes | Epic title |
| `description` | string | Yes | Epic description (may be empty) |
| `status` | string | Yes | `"todo"`, `"in_progress"`, or `"done"` |
| `created_at` | string | Yes | ISO 8601 timestamp |
| `updated_at` | string | Yes | ISO 8601 timestamp |

**Excluded (derived) fields:**
- `project_id` — inferred from directory
- `short_id` — derived from filename
- `task_count` — computed at runtime
- `done_count` — computed at runtime

---

### tasks/E{n}-T{m}.json

Task files are named with their short ID (e.g., `E1-T1.json`, `E2-T3.json`).

```json
{
  "id": "01KH0DPG02JCPAK1Y87Q4NKRMY",
  "title": "Task title",
  "description": "Task description",
  "status": "todo",
  "session_id": null,
  "created_at": "2025-02-09T14:00:00Z",
  "updated_at": "2025-02-09T14:00:00Z"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | Yes | ULID unique identifier |
| `title` | string | Yes | Task title |
| `description` | string | Yes | Task description (may be empty) |
| `status` | string | Yes | `"todo"`, `"in_progress"`, or `"done"` |
| `session_id` | string\|null | Yes | Session ID if being worked on, else `null` |
| `created_at` | string | Yes | ISO 8601 timestamp |
| `updated_at` | string | Yes | ISO 8601 timestamp |

**Excluded (derived) fields:**
- `epic_id` — derived from filename prefix (`E1-T2` → epic `E1`)
- `short_id` — derived from filename

---

### dependencies.json

All dependencies between epics and tasks.

```json
{
  "version": 1,
  "dependencies": [
    { "blocker": "E1-T1", "blocked": "E1-T2" },
    { "blocker": "E1", "blocked": "E2" }
  ]
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `version` | integer | Yes | Schema version (currently `1`) |
| `dependencies` | array | Yes | List of dependency entries |

**Dependency Entry:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `blocker` | string | Yes | Short ID of the blocking item |
| `blocked` | string | Yes | Short ID of the blocked item |

**Type inference:**
- Epic short IDs: `E{n}` (e.g., `E1`, `E2`)
- Task short IDs: `E{n}-T{m}` (e.g., `E1-T1`, `E2-T3`)

---

## Status Values

### Project Status

| Value | Description |
|-------|-------------|
| `active` | Project is active |
| `archived` | Project is archived |

### Item Status (Epics & Tasks)

| Value | Description |
|-------|-------------|
| `todo` | Not started |
| `in_progress` | Currently being worked on |
| `done` | Completed |

---

## Relationship Derivation

When loading data, relationships are reconstructed:

1. **Epic → Project**: All epics in `.blueprint/epics/` belong to the project in `.blueprint/project.json`

2. **Task → Epic**: Task filename prefix determines the parent epic:
   - `E1-T1.json` → belongs to epic `E1`
   - `E2-T3.json` → belongs to epic `E2`

3. **Dependencies**: The `blocker` and `blocked` short IDs in `dependencies.json` are resolved to ULIDs by looking up epics and tasks by their short IDs.

---

## Example: Complete Project

```
.blueprint/
├── project.json
├── dependencies.json
├── epics/
│   ├── E1.json
│   └── E2.json
└── tasks/
    ├── E1-T1.json
    ├── E1-T2.json
    └── E2-T1.json
```

**project.json:**
```json
{
  "id": "01KH0DPG02JCPAK1Y87Q4NKRMW",
  "name": "Blueprint",
  "description": "Project management for AI agents",
  "status": "active",
  "created_at": "2025-02-09T14:00:00Z",
  "updated_at": "2025-02-09T14:00:00Z"
}
```

**epics/E1.json:**
```json
{
  "id": "01KH0DPG02JCPAK1Y87Q4NKRMX",
  "title": "Core Features",
  "description": "Essential functionality",
  "status": "in_progress",
  "created_at": "2025-02-09T14:00:00Z",
  "updated_at": "2025-02-09T14:00:00Z"
}
```

**tasks/E1-T1.json:**
```json
{
  "id": "01KH0DPG02JCPAK1Y87Q4NKRMY",
  "title": "Implement login",
  "description": "Add user authentication",
  "status": "done",
  "session_id": null,
  "created_at": "2025-02-09T14:00:00Z",
  "updated_at": "2025-02-09T14:00:00Z"
}
```

**dependencies.json:**
```json
{
  "version": 1,
  "dependencies": [
    { "blocker": "E1-T1", "blocked": "E1-T2" },
    { "blocker": "E1", "blocked": "E2" }
  ]
}
```
