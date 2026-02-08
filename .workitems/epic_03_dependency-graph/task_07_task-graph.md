---
id: TK-0307
title: "Build Task-Level Graph View"
status: DONE
epic: 3
priority: medium
dependencies: [TK-0301, TK-0302, TK-0305]
blockers: []
commits: []
pr: ""
---

# Build Task-Level Graph View

## Objective
Build the task-level dependency graph showing blue-task dependencies within the currently selected epic.

## Scope
- Full-screen layout with header, sub-header (epic title), graph canvas, and footer
- Query tasks and task-to-task dependencies scoped to selected epic
- Build DAG, layout, render nodes, route edges
- Task nodes show title and status label
- Tab indicators: epics dimmed, [TASKS] highlighted

## Acceptance Criteria
- [ ] Task dependency graph renders for selected epic
- [ ] All task nodes visible with correct borders
- [ ] Edges correctly show task dependencies
- [ ] Orphan tasks in separate row
- [ ] `1` key switches back to epic-level view
- [ ] Handles epics with no tasks gracefully

## Technical Context
### Relevant Spec Sections
- PRD.md — Task graph view specification

### Related Files/Directories
- `src/tui/graph.rs` — Graph rendering pipeline
- `src/tui/ui.rs` — View switching logic

### Dependencies on Other Systems
- DAG layout (TK-0300), crossing minimization (TK-0301), node renderer (TK-0302), edge routing (TK-0305)
- Database layer for task and dependency queries

## Implementation Guidance
### Approach
Full-screen layout: Header "▓▓ DEPENDENCY GRAPH ▓▓ epics [TASKS] [ESC to go back]", sub-header "EPIC: {epic_title}", graph canvas, footer summary bar. Data flow: (1) Query all tasks for selected epic, (2) Query task-to-task dependencies scoped to this epic, (3) Build DAG, layout, render. Task nodes show title and status label. If no epic is selected, show a message prompting selection.

### Considerations
- Must handle the case where no epic is selected
- Graph may be larger than viewport

### Anti-patterns to Avoid
- Do not show tasks from other epics — scope to selected epic only

## Testing Requirements

### Unit Tests
- [ ] Layout computation for sample task graph
- [ ] No-epic-selected message rendering

### Integration Tests
- [ ] Full graph rendering pipeline with real data

### Manual Tests
- [ ] Visual inspection of task graph view

## Notes
TBD
