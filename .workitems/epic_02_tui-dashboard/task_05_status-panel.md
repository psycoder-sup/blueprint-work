---
id: TK-0205
title: "Build Project Status Overview Panel"
status: TODO
epic: 2
priority: medium
dependencies: [TK-0201]
blockers: []
commits: []
pr: ""
---

# Build Project Status Overview Panel

## Objective
Build the bottom-right panel showing aggregate project status with progress bars and blocked item count.

## Scope
- Panel title "PROJECT STATUS"
- Epic and task progress bars with counts
- Blocked item count with color highlighting
- Updates when project or data changes

## Acceptance Criteria
- [ ] Progress bars render correctly
- [ ] Counts match actual data
- [ ] Blocked count highlighted when > 0
- [ ] Updates when project or data changes

## Technical Context
### Relevant Spec Sections
- PRD.md — Status panel specification

### Related Files/Directories
- `src/tui/ui.rs` — Panel rendering
- `src/tui/theme.rs` — Progress bar helper

### Dependencies on Other Systems
- Database layer for aggregate queries

## Implementation Guidance
### Approach
Content: `Epics: ██░░░ {done}/{total}` — progress bar, `Tasks: ████░ {done}/{total}` — progress bar, `Blocked: {count} items`. Progress bars use `█` for completed and `░` for remaining. Colors: neon green for progress fill, dim gray for remaining. Blocked count in neon orange/red if > 0.

### Considerations
- Progress bar width should adapt to panel width
- Blocked count color should clearly indicate warning state

### Anti-patterns to Avoid
- Do not hardcode progress bar width — make it responsive

## Testing Requirements

### Unit Tests
- [ ] Progress bar rendering for various ratios
- [ ] Blocked count color logic

### Integration Tests
- [ ] Panel updates on data change

### Manual Tests
- [ ] Visual testing of status panel

## Notes
TBD
