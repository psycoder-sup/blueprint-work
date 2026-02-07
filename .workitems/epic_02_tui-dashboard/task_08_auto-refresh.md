---
id: TK-0208
title: "Implement Auto-Refresh from SQLite"
status: DONE
epic: 2
priority: medium
dependencies: [TK-0200]
blockers: []
commits: [532b067]
pr: ""
---

# Implement Auto-Refresh from SQLite

## Objective
Implement automatic data refresh so the TUI stays in sync when the MCP server modifies data concurrently.

## Scope
- Poll SQLite database at 1-second intervals for changes
- Track "last_updated" watermark and re-query on newer data
- Efficient: only re-fetch when changes detected
- Handle concurrent access via SQLite WAL mode

## Acceptance Criteria
- [ ] TUI reflects changes made by MCP server within ~1 second
- [ ] No excessive DB queries when data hasn't changed
- [ ] No crashes from concurrent access
- [ ] Smooth visual updates (no flicker)

## Technical Context
### Relevant Spec Sections
- PRD.md — Auto-refresh mechanism

### Related Files/Directories
- `src/tui/mod.rs` — App event loop (polling logic)
- `src/db/mod.rs` — Database queries

### Dependencies on Other Systems
- SQLite WAL mode for concurrent access

## Implementation Guidance
### Approach
Poll the SQLite database at 1-second intervals for changes. Track a "last_updated" watermark and re-query if any row's updated_at is newer. On change detected: re-fetch projects, epics, tasks for the current view, update App state, trigger re-render. Only re-fetch when changes are detected, not on every tick. SQLite WAL mode allows one writer + multiple readers.

### Considerations
- Be efficient: only re-fetch when changes are detected
- Handle concurrent access gracefully — WAL mode handles this
- Avoid visual flicker on updates

### Anti-patterns to Avoid
- Do not re-fetch all data on every tick — use watermark-based change detection

## Testing Requirements

### Unit Tests
- [ ] Watermark comparison logic
- [ ] Change detection triggers refresh

### Integration Tests
- [ ] TUI reflects external database changes

### Manual Tests
- [ ] Run TUI and MCP server concurrently, verify sync

## Notes
TBD
