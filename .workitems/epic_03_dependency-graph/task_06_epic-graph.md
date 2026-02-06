---
id: TK-0306
title: "Build Epic-Level Graph View"
status: TODO
epic: 3
priority: medium
dependencies: [TK-0301, TK-0302, TK-0305]
blockers: []
commits: []
pr: ""
---

# Build Epic-Level Graph View

## Objective
Compose all graph components into the full-screen epic-level dependency graph view, showing epic-to-epic dependency relationships for the current project.

## Scope
- Full-screen layout with header, graph canvas, and footer
- Query epics and epic-to-epic dependencies for current project
- Build DAG, compute layout, minimize crossings, render nodes, route edges
- Epic nodes show title and task completion progress bar
- Tab indicators: [EPICS] highlighted, tasks dimmed

## Acceptance Criteria
- [ ] Full-screen epic dependency graph renders correctly
- [ ] All epic nodes visible with correct status borders
- [ ] Edges correctly represent dependency relationships
- [ ] Orphan epics shown in separate row
- [ ] `Esc` returns to main TUI view
- [ ] `2` key switches to task-level view

## Technical Context
### Relevant Spec Sections
- PRD.md — Epic graph view specification

### Related Files/Directories
- `src/tui/graph.rs` — Graph rendering pipeline
- `src/tui/ui.rs` — View switching logic

### Dependencies on Other Systems
- DAG layout (TK-0300), crossing minimization (TK-0301), node renderer (TK-0302), edge routing (TK-0305)
- Database layer for epic and dependency queries

## Implementation Guidance
### Approach
Full-screen layout: Header "▓▓ DEPENDENCY GRAPH ▓▓ [EPICS] tasks [ESC to go back]", graph canvas fills remaining space, footer summary bar (TK-0309). Data flow: (1) Query all epics for current project, (2) Query epic-to-epic dependencies, (3) Build DAG and compute layout, (4) Minimize crossings, (5) Render nodes with animated borders, (6) Route edges, (7) Blit to ratatui frame. Epic nodes show title and task completion progress bar.

### Considerations
- Graph may be larger than viewport — coordinate with TK-0308 for scrolling
- Orphan epics need to be visible in a separate row

### Anti-patterns to Avoid
- Do not re-compute layout on every render — cache and invalidate on data change

## Testing Requirements

### Unit Tests
- [ ] Layout computation for sample epic graph
- [ ] Tab indicator rendering

### Integration Tests
- [ ] Full graph rendering pipeline with real data

### Manual Tests
- [ ] Visual inspection of epic graph view

## Notes
TBD
