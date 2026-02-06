---
id: TK-0207
title: "Implement Keyboard Navigation"
status: TODO
epic: 2
priority: medium
dependencies: [TK-0202, TK-0203, TK-0204]
blockers: []
commits: []
pr: ""
---

# Implement Keyboard Navigation

## Objective
Wire up all keyboard shortcuts for navigating between panels and interacting with items.

## Scope
- All navigation keys: j/k, h/l, Tab, Enter, p, s, d, /, ?, q, Esc
- Track active panel in App state
- Visual focus indicator: bright border on active panel, dim on inactive

## Acceptance Criteria
- [ ] All keybindings work as specified
- [ ] Active panel visually indicated
- [ ] Panel switching is smooth
- [ ] Status cycling persists to DB
- [ ] Help overlay shows all keybindings

## Technical Context
### Relevant Spec Sections
- PRD.md — Keyboard navigation specification

### Related Files/Directories
- `src/tui/mod.rs` — Event handling
- `src/tui/ui.rs` — Focus indicators

### Dependencies on Other Systems
- crossterm for keyboard event handling

## Implementation Guidance
### Approach
Navigation keys: `j/k` move up/down within active panel, `h/l` switch between epics (left) and tasks (right), `Tab` cycles through all panels, `Enter` expands/views details, `p` opens project selector, `s` cycles status, `d` toggles dependency graph view, `/` opens filter/search, `?` shows help overlay, `q` quits, `Esc` closes overlay/popup. Track active panel in App state. Visual focus: bright border on active panel, dim on inactive.

### Considerations
- Key handling should be context-aware (different behavior in overlay vs main view)
- Esc should close overlays before exiting views

### Anti-patterns to Avoid
- Do not have conflicting keybindings in different contexts

## Testing Requirements

### Unit Tests
- [ ] Key dispatch logic for each keybinding
- [ ] Active panel tracking

### Integration Tests
- [ ] Full navigation flow across panels

### Manual Tests
- [ ] Test all keybindings manually

## Notes
TBD
