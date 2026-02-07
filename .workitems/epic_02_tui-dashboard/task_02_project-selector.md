---
id: TK-0202
title: "Build Project Selector Panel"
status: DONE
epic: 2
priority: medium
dependencies: [TK-0201]
blockers: []
commits: []
pr: ""
---

# Build Project Selector Panel

## Objective
Build the top header bar showing the current project name with prev/next navigation, plus the BLUEPRINT ASCII art title and status indicator.

## Scope
- Header bar layout with ASCII title, project name, and status indicator
- `p` key opens project selector overlay/popup
- Arrow keys or j/k to select, Enter to confirm
- Track `selected_project_id` in App state
- Refresh epic and task lists on project change

## Acceptance Criteria
- [ ] Header bar renders with project name
- [ ] `p` key opens project selector
- [ ] Can navigate and select a different project
- [ ] Epic/task panels refresh on project change
- [ ] Handles 0 projects gracefully (show "No projects" message)

## Technical Context
### Relevant Spec Sections
- PRD.md — TUI header bar layout

### Related Files/Directories
- `src/tui/ui.rs` — Header bar rendering
- `src/tui/mod.rs` — App state (selected_project_id)

### Dependencies on Other Systems
- Database layer for project listing

## Implementation Guidance
### Approach
Header bar layout: Left: "▓▓ BLUEPRINT ▓▓" ASCII title. Center: "PROJECT: {name}" with [◀ prev | next ▶] indicators. Right: "[STATUS: ONLINE]". `p` key opens a project selector overlay listing all active projects. Track selected_project_id in App state. When project changes, refresh epic and task lists.

### Considerations
- Handle case where no projects exist gracefully
- Project selector should be an overlay that doesn't destroy the underlying view

### Anti-patterns to Avoid
- Do not block the event loop while the project selector is open

## Testing Requirements

### Unit Tests
- [ ] Header bar rendering with project name
- [ ] Project selector overlay rendering

### Integration Tests
- [ ] Project selection updates epic/task panels

### Manual Tests
- [ ] Visual testing of header bar and project selector overlay

## Notes
TBD
