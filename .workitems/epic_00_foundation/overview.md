# Epic 00: Foundation

## Description

Set up the Rust project scaffolding, SQLite database schema, and the core data layer (models + CRUD) for Projects, Epics, Blue-Tasks, and Dependencies. This is the foundation everything else builds on.

## Status

`in-progress`

## Dependencies

None — this is the root epic.

## Blocked By

(none)

## Blocks

- epic_01_mcp-server
- epic_02_tui-dashboard
- epic_03_dependency-graph
- epic_04_cli-polish

## Tasks

| # | Task | Status |
|---|------|--------|
| 00 | Initialize Cargo project with clap CLI | done |
| 01 | Create SQLite schema & migration system | done |
| 02 | Implement Project model & CRUD | done |
| 03 | Implement Epic model & CRUD | done |
| 04 | Implement BlueTask model & CRUD | todo |
| 05 | Implement Dependency management | todo |

## Acceptance Criteria

- `blueprint serve`, `blueprint tui`, `blueprint status` subcommands parse (even if they just print "not implemented")
- SQLite database is auto-created on first run with all tables and indexes
- Full CRUD operations work for projects, epics, tasks, and dependencies at the DB layer
- All models serialize/deserialize with serde
- Unit tests pass for all CRUD operations
