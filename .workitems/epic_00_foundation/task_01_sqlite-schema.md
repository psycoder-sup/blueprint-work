---
id: TK-0001
title: "Create SQLite Schema & Migration System"
status: DONE
epic: 0
priority: medium
dependencies: [TK-0000]
blockers: []
commits: [94f666c]
pr: ""
---

# Create SQLite Schema & Migration System

## Objective
Set up the SQLite database connection layer and create the initial schema with all tables. Implement a simple migration system that auto-creates the database and runs migrations on startup.

## Scope
- Create `src/db/mod.rs` with `Database` struct wrapping `rusqlite::Connection`
- Implement `Database::open(path)` and `Database::migrate()`
- Create `migrations/001_init.sql` with full schema (projects, epics, tasks, dependencies, prds tables)
- Enable WAL mode and foreign keys on connection open
- Database path from `BLUEPRINT_DB` env var, defaulting to `~/.blueprint/blueprint.db`
- Auto-create parent directories if they don't exist

## Acceptance Criteria
- [ ] Database file is auto-created on first open
- [ ] All 5 tables created with correct columns, types, and constraints
- [ ] Foreign keys are enforced (CASCADE deletes work)
- [ ] All indexes exist
- [ ] WAL mode enabled
- [ ] Migration is idempotent (safe to run multiple times)

## Technical Context
### Relevant Spec Sections
- PRD.md — Database schema definition

### Related Files/Directories
- `src/db/mod.rs` — Database connection and migration logic
- `migrations/001_init.sql` — Initial schema migration

### Dependencies on Other Systems
- rusqlite with `bundled` feature

## Implementation Guidance
### Approach
Create a `Database` struct wrapping a `rusqlite::Connection`. On open: enable WAL mode and foreign keys. Migration system reads SQL files and applies them idempotently. Schema includes: projects, epics, tasks, dependencies, prds tables with all indexes.

### Considerations
- Ensure WAL mode for concurrent read access (TUI + MCP server)
- Auto-create `~/.blueprint/` directory if it doesn't exist
- Make migrations idempotent with `CREATE TABLE IF NOT EXISTS`

### Anti-patterns to Avoid
- Do not hardcode database paths — use env var with sensible default

## Testing Requirements

### Unit Tests
- [ ] Database opens and creates file successfully
- [ ] All tables and indexes created after migration
- [ ] Foreign key constraints enforced
- [ ] Migration is idempotent

### Integration Tests
- [ ] Full schema creation from scratch
- [ ] CASCADE delete behavior

### Manual Tests
- [ ] Inspect created SQLite file with `sqlite3` CLI

## Notes
Blocks: TK-0002, TK-0003, TK-0004, TK-0005
