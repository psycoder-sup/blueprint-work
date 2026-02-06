---
id: TK-0308
title: "Implement Graph View Toggle & Scroll"
status: TODO
epic: 3
priority: medium
dependencies: [TK-0306, TK-0307]
blockers: []
commits: []
pr: ""
---

# Implement Graph View Toggle & Scroll

## Objective
Implement keyboard controls for the graph view: toggling between epic/task views, scrolling large graphs, and returning to the main TUI.

## Scope
- Key bindings: `1` (epic view), `2` (task view), `Esc` (exit), j/k/h/l (scroll)
- Scroll state tracking (scroll_x, scroll_y) in App
- Viewport clipping to visible area
- Scroll indicators at edges
- Scroll reset on view switch

## Acceptance Criteria
- [ ] `1`/`2` keys toggle between views
- [ ] `Esc` returns to main TUI
- [ ] Scrolling works in both directions
- [ ] Scroll indicators shown when content overflows
- [ ] Scroll resets on view switch

## Technical Context
### Relevant Spec Sections
- PRD.md — Graph view navigation

### Related Files/Directories
- `src/tui/mod.rs` — Event handling and scroll state
- `src/tui/graph.rs` — Viewport clipping

### Dependencies on Other Systems
- Epic graph (TK-0306) and task graph (TK-0307)

## Implementation Guidance
### Approach
Key bindings in graph mode: `1` switch to epic-level, `2` switch to task-level, `Esc` exit to main TUI, `j/k` or arrow keys scroll vertically, `h/l` scroll horizontally. Track scroll state (scroll_x, scroll_y) in App. Viewport clips the graph canvas to the visible area. Show scroll indicators at edges when content extends beyond viewport (e.g., "▼ more"). Reset scroll position when switching between epic/task views.

### Considerations
- Scroll step size should feel natural (1 row/column per key press)
- Scroll indicators should be visible but not intrusive

### Anti-patterns to Avoid
- Do not allow scrolling past the edge of the canvas

## Testing Requirements

### Unit Tests
- [ ] Scroll state management
- [ ] Viewport clipping logic

### Integration Tests
- [ ] Scrolling large graphs

### Manual Tests
- [ ] Test scrolling and view switching

## Notes
TBD
