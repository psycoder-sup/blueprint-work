---
id: TK-0206
title: "Build Mini Dependency Panel"
status: TODO
epic: 2
priority: medium
dependencies: [TK-0201]
blockers: []
commits: []
pr: ""
---

# Build Mini Dependency Panel

## Objective
Build the bottom-left panel showing a compact list of dependency relationships and a hint to open the full dependency graph.

## Scope
- Panel title "DEPENDENCIES (mini)"
- Compact dependency display with color coding
- Limit to ~5 visible deps (scrollable if more)
- Footer hint and `d` key handler for full graph view

## Acceptance Criteria
- [ ] Dependencies render in compact format
- [ ] Color coding for active blocks
- [ ] Footer hint visible
- [ ] `d` key triggers graph view transition (handler stub for now)
- [ ] Handles zero dependencies gracefully

## Technical Context
### Relevant Spec Sections
- PRD.md — Mini dependency panel specification

### Related Files/Directories
- `src/tui/ui.rs` — Panel rendering
- `src/tui/mod.rs` — App state (view mode)

### Dependencies on Other Systems
- Database layer for dependency queries

## Implementation Guidance
### Approach
Show the most relevant dependencies (blocking the current epic/project). Format: `{blocker_title} ──blocks──▶ {blocked_title}`. Color code: neon cyan for normal, neon red for actively blocking. Limit to ~5 visible deps (scrollable if more). Footer hint: `[d] Full Dependency Graph`. `d` key switches to the full-screen dependency graph view (epic_03).

### Considerations
- Only show dependencies relevant to the current project/epic context
- Handle zero dependencies with a "No dependencies" message

### Anti-patterns to Avoid
- Do not show all project dependencies at once — filter by relevance

## Testing Requirements

### Unit Tests
- [ ] Dependency row rendering
- [ ] Color coding logic

### Integration Tests
- [ ] Graph view transition on `d` key

### Manual Tests
- [ ] Visual testing of mini dependency panel

## Notes
TBD
