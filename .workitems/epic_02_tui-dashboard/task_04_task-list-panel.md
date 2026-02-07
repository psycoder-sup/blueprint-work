---
id: TK-0204
title: "Build Blue-Task List Panel"
status: DONE
epic: 2
priority: medium
dependencies: [TK-0201]
blockers: []
commits: []
pr: ""
---

# Build Blue-Task List Panel

## Objective
Build the right panel showing the list of blue-tasks for the currently selected epic, with status indicators and detail expansion.

## Scope
- Panel title "BLUE-TASKS" with neon cyan border
- Each task row with status symbol, title, and status label
- Selection highlighting, scrollable list
- `Enter` expands to show description/details
- `s` cycles task status and persists to DB
- Blocked tasks show dependency hints

## Acceptance Criteria
- [ ] Tasks render for the selected epic
- [ ] Status symbols and colors are correct
- [ ] Selection highlighting works
- [ ] `Enter` shows task details
- [ ] `s` cycles task status and persists to DB
- [ ] Blocked tasks show blocking info
- [ ] Scrolling works for long lists

## Technical Context
### Relevant Spec Sections
- PRD.md — Task list panel specification

### Related Files/Directories
- `src/tui/ui.rs` — Panel rendering
- `src/tui/mod.rs` — App state (selected task)

### Dependencies on Other Systems
- Database layer for task queries and status updates

## Implementation Guidance
### Approach
Each task row: `◆ {title}  [{STATUS}]`. Status symbol and color matching the task status. Blocked tasks shown with warning indicator and "blocked by: {task_name}" hint. `Enter` on a task expands to show description/details inline or in a popup. `s` on a selected task cycles its status: todo → in_progress → done and persists to DB.

### Considerations
- Status cycling must persist to database immediately
- Show dependency hints for blocked tasks

### Anti-patterns to Avoid
- Do not allow status cycling on blocked tasks without warning

## Testing Requirements

### Unit Tests
- [ ] Task row rendering with various statuses
- [ ] Status cycling logic

### Integration Tests
- [ ] Status cycling persists to DB and updates UI

### Manual Tests
- [ ] Visual testing of task list with various states

## Notes
TBD
