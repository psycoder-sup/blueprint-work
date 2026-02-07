---
id: TK-0203
title: "Build Epic List Panel"
status: DONE
epic: 2
priority: medium
dependencies: [TK-0201]
blockers: []
commits: []
pr: ""
---

# Build Epic List Panel

## Objective
Build the left panel showing the list of epics for the selected project, with status indicators and task completion counts.

## Scope
- Panel title "EPICS" with neon cyan border
- Each epic row with status symbol, title, and task completion count
- Selection highlighting with bright border/background
- Scrollable list for long lists
- Selecting an epic updates the task panel
- Visual indicator for blocked epics

## Acceptance Criteria
- [ ] Epics list renders for the selected project
- [ ] Status symbols and colors are correct
- [ ] Task completion count is accurate
- [ ] Selection highlighting works
- [ ] Blocked epics visually indicated
- [ ] Scrolling works for long lists
- [ ] Selecting an epic updates the task panel

## Technical Context
### Relevant Spec Sections
- PRD.md — Epic list panel specification

### Related Files/Directories
- `src/tui/ui.rs` — Panel rendering
- `src/tui/mod.rs` — App state (selected epic)

### Dependencies on Other Systems
- Database layer for epic and task queries

## Implementation Guidance
### Approach
Each epic row: `◉ {title}  [{done}/{total}]`. Status symbol colored by status (green=done, cyan=in_progress, gray=todo, red=blocked). Task count shows completed/total blue-tasks. Currently selected epic highlighted with bright border/background. Scrollable list if epics exceed panel height. When an epic is selected, the task panel (right) updates.

### Considerations
- Blocked epics need visual indicator (warning icon, orange text)
- Must handle scrolling when list exceeds panel height

### Anti-patterns to Avoid
- Do not re-query database on every render — cache data and refresh on change

## Testing Requirements

### Unit Tests
- [ ] Epic row rendering with various statuses
- [ ] Task count calculation

### Integration Tests
- [ ] Epic selection updates task panel

### Manual Tests
- [ ] Visual testing of epic list with various states

## Notes
TBD
