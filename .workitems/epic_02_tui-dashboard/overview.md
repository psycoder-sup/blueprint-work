# Epic 02: TUI Dashboard

## Description

Build the cyberpunk-styled terminal UI dashboard using ratatui + crossterm. The TUI provides a real-time view of projects, epics, blue-tasks, and their statuses with a neon dystopian aesthetic.

## Status

`in-progress`

## Dependencies

- epic_00_foundation (needs DB layer for reading data)

## Blocked By

- epic_00_foundation

## Blocks

- epic_03_dependency-graph (graph view is an extension of the TUI)

## Tasks

| # | Task | Status |
|---|------|--------|
| 00 | Create ratatui app skeleton with event loop | done |
| 01 | Implement cyberpunk theme & color palette | todo |
| 02 | Build project selector panel | todo |
| 03 | Build epic list panel | todo |
| 04 | Build blue-task list panel | todo |
| 05 | Build project status overview panel | todo |
| 06 | Build mini dependency panel | todo |
| 07 | Implement keyboard navigation | todo |
| 08 | Implement auto-refresh from SQLite | todo |

## Acceptance Criteria

- `blueprint tui` launches a full-screen terminal dashboard
- Cyberpunk neon aesthetic with proper color palette
- Can switch between projects, browse epics, view tasks
- Status indicators and progress bars render correctly
- Keyboard navigation works (j/k/h/l/Tab/Enter/q)
- Dashboard auto-refreshes when data changes
