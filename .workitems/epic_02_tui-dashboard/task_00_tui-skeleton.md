---
id: TK-0200
title: "Create Ratatui App Skeleton with Event Loop"
status: DONE
epic: 2
priority: medium
dependencies: []
blockers: []
commits: [8cbe24c, 6181656]
pr: ""
---

# Create Ratatui App Skeleton with Event Loop

## Objective
Set up the base ratatui application with crossterm backend, terminal setup/teardown, and the main event loop handling keyboard input and render ticks.

## Scope
- Create `src/tui/mod.rs` with `App` struct and main loop
- Terminal setup: enable raw mode, enter alternate screen, enable mouse capture
- Terminal teardown: restore on exit/panic (drop guard)
- Event loop with crossterm event polling
- Create `src/tui/ui.rs` with placeholder layout
- Wire up `blueprint tui` subcommand

## Acceptance Criteria
- [ ] `blueprint tui` launches a full-screen terminal UI
- [ ] Terminal properly enters and exits alternate screen
- [ ] `q` key exits cleanly
- [ ] Panic hook restores terminal state
- [ ] Placeholder panels render with borders

## Technical Context
### Relevant Spec Sections
- PRD.md — TUI dashboard architecture

### Related Files/Directories
- `src/tui/mod.rs` — App struct and main loop
- `src/tui/ui.rs` — UI rendering

### Dependencies on Other Systems
- ratatui and crossterm crates

## Implementation Guidance
### Approach
Create `App` struct holding app state (selected panel, selected indices, db connection, running flag). Implement `App::new(db_path)` and `App::run()`. Set up terminal with raw mode, alternate screen, mouse capture. Event loop polls crossterm events with ~100ms timeout. On tick: re-render. On `q`: exit. Use a drop guard for teardown.

### Considerations
- Panic hook must restore terminal state to avoid leaving terminal in raw mode
- Poll timeout of ~100ms balances responsiveness with CPU usage

### Anti-patterns to Avoid
- Do not forget terminal teardown on panic — use a drop guard pattern

## Testing Requirements

### Unit Tests
- [ ] App struct initializes correctly

### Integration Tests
- [ ] Terminal setup/teardown works without corruption

### Manual Tests
- [ ] Launch `blueprint tui` and verify visual output
- [ ] Press `q` and verify clean exit
- [ ] Force a panic and verify terminal state is restored

## Notes
Blocks: TK-0201, TK-0202, TK-0203, TK-0204, TK-0205, TK-0206, TK-0207, TK-0208
