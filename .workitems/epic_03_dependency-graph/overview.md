# Epic 03: Dependency Graph View

## Description

Build the full-screen ASCII DAG (directed acyclic graph) visualization for the TUI. Shows dependency relationships between epics or between blue-tasks with cyberpunk neon styling, animated marching dotted borders on in-progress nodes, and pulsing borders on blocked nodes.

## Status

`in-progress`

## Dependencies

- epic_00_foundation (data layer)
- epic_02_tui-dashboard (TUI skeleton, theme, event loop)

## Blocked By

- epic_00_foundation
- epic_02_tui-dashboard

## Blocks

(none)

## Tasks

| # | Task | Status |
|---|------|--------|
| 00 | Implement DAG topological sort & layer assignment | done |
| 01 | Implement edge-crossing minimization | todo |
| 02 | Build ASCII box-node renderer | todo |
| 03 | Implement animated marching dotted border | todo |
| 04 | Implement pulsing border for blocked nodes | todo |
| 05 | Implement edge routing | todo |
| 06 | Build epic-level graph view | todo |
| 07 | Build task-level graph view | todo |
| 08 | Implement graph view toggle & scroll | todo |
| 09 | Build graph summary bar | todo |

## Acceptance Criteria

- `d` key from main TUI opens full-screen dependency graph
- Epic-level and task-level views toggleable with `1`/`2` keys
- Nodes rendered as ASCII boxes with status-appropriate borders and colors
- In-progress nodes have animated marching dotted borders
- Blocked nodes have pulsing red/orange borders
- Edges drawn with directional arrows and proper routing
- Graph scrollable for large dependency trees
- Summary bar shows counts at the bottom
- `Esc` returns to main TUI view
