---
id: TK-0400
title: "Implement `blueprint status` CLI Command"
status: TODO
epic: 4
priority: medium
dependencies: []
blockers: []
commits: []
pr: ""
---

# Implement `blueprint status` CLI Command

## Objective
Implement the non-interactive `blueprint status` command that prints a colorful project overview to the terminal.

## Scope
- Create `src/cli/status.rs` with all-projects and single-project views
- All-projects view: ASCII header, project list with progress bars
- Single-project view: detailed view with epic list, progress bars, blocked items
- Crossterm for terminal colors, respect `NO_COLOR` env var
- Output goes to stdout

## Acceptance Criteria
- [ ] `blueprint status` prints all projects overview
- [ ] `blueprint status --project <id>` prints detailed project view
- [ ] Progress bars render correctly with colors
- [ ] Blocked items highlighted
- [ ] Works in both color and no-color modes
- [ ] Graceful message when no projects exist

## Technical Context
### Relevant Spec Sections
- PRD.md — CLI status command specification

### Related Files/Directories
- `src/cli/status.rs` — Status command implementation
- `src/cli/mod.rs` — CLI module

### Dependencies on Other Systems
- Database layer for project/epic/task queries
- crossterm for terminal colors

## Implementation Guidance
### Approach
`blueprint status` lists all projects with progress. `blueprint status --project <id>` shows detailed view. All-projects view: ASCII header, for each active project: name, epic progress bar, task progress bar. Single-project view: ASCII header, project name, epic/task progress bars, blocked count, list each epic with progress and status. Use crossterm for terminal colors (no ratatui needed). Respect `NO_COLOR` env var.

### Considerations
- Must respect `NO_COLOR` env var for accessibility
- Progress bars should be readable even without color

### Anti-patterns to Avoid
- Do not require ratatui for simple CLI output — use crossterm directly

## Testing Requirements

### Unit Tests
- [ ] Progress bar string generation
- [ ] Color/no-color mode switching

### Integration Tests
- [ ] Full status output with test data

### Manual Tests
- [ ] Visual inspection of colored output
- [ ] Test with `NO_COLOR=1`

## Notes
TBD
