---
id: TK-0309
title: "Build Graph Summary Bar"
status: DONE
epic: 3
priority: medium
dependencies: [TK-0306]
blockers: []
commits: []
pr: ""
---

# Build Graph Summary Bar

## Objective
Build the summary footer bar at the bottom of the dependency graph view showing aggregate graph statistics.

## Scope
- Format: `◉ {n} epics/tasks │ ─▶ {n} edges │ ⚠ {n} blocked │ ■ {n} done`
- Adaptive labels based on current view (epics vs tasks)
- Color coding per theme
- Fixed at bottom of graph view (doesn't scroll)

## Acceptance Criteria
- [ ] Summary bar renders at bottom of graph view
- [ ] Counts are accurate for current view
- [ ] Colors match the theme
- [ ] Label changes between epic/task views
- [ ] Stays fixed (doesn't scroll)

## Technical Context
### Relevant Spec Sections
- PRD.md — Graph summary bar specification

### Related Files/Directories
- `src/tui/graph.rs` — Summary bar rendering
- `src/tui/ui.rs` — Layout (fixed footer)

### Dependencies on Other Systems
- Epic graph view (TK-0306) for data and layout

## Implementation Guidance
### Approach
Format: `◉ {n} epics/tasks │ ─▶ {n} edges │ ⚠ {n} blocked │ ■ {n} done`. Adapt label based on current view: "epics" for epic-level, "tasks" for task-level. Color coding: node count = neon cyan, edge count = neon cyan, blocked count = neon orange (red if > 0), done count = neon green. Separated by `│` with dim gray color. Fixed at the bottom of the graph view, doesn't scroll with canvas.

### Considerations
- Counts should be derived from the current graph data, not re-queried
- Must stay fixed even when graph canvas scrolls

### Anti-patterns to Avoid
- Do not re-query database for summary — use already-loaded graph data

## Testing Requirements

### Unit Tests
- [ ] Count calculation from graph data
- [ ] Label adaptation between views
- [ ] Color selection logic

### Integration Tests
- [ ] Summary bar updates when switching views

### Manual Tests
- [ ] Visual inspection of summary bar

## Code Quality

After implementation is complete, run the following steps:

1. **Run code-simplifier agent** — Simplify and refine the code for clarity, consistency, and maintainability
2. **Run code-reviewer agent** — Review the code for bugs, security issues, and quality problems

## Notes
TBD
