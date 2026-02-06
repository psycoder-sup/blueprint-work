---
id: TK-0005
title: "Implement Dependency Management"
status: TODO
epic: 0
priority: medium
dependencies: [TK-0003, TK-0004]
blockers: []
commits: []
pr: ""
---

# Implement Dependency Management

## Objective
Implement the dependency (blocks/blocked-by) system for both epics and tasks. A single polymorphic dependency table handles all relationships.

## Scope
- Create `src/models/dependency.rs` with `Dependency` struct, `DependencyType` enum, and input struct
- Create `src/db/dependency.rs` with full dependency management operations
- Validate: prevent self-references, prevent duplicate deps, verify referenced items exist
- Excluded: MCP tool wiring (handled by epic_01)

## Acceptance Criteria
- [ ] Can add/remove dependencies between epics
- [ ] Can add/remove dependencies between tasks
- [ ] Can add cross-type dependencies (epic blocks task, etc.) if needed
- [ ] Duplicate dependencies are rejected gracefully (UNIQUE constraint)
- [ ] Self-referencing dependencies are rejected
- [ ] `is_blocked` correctly checks blocker status
- [ ] Unit tests for all operations

## Technical Context
### Relevant Spec Sections
- PRD.md — Dependency model definition

### Related Files/Directories
- `src/models/dependency.rs` — Model struct and enums
- `src/db/dependency.rs` — Database operations

### Dependencies on Other Systems
- Relies on epics and tasks tables for foreign key validation

## Implementation Guidance
### Approach
Define `Dependency` struct: id, blocker_type, blocker_id, blocked_type, blocked_id. Define `DependencyType` enum: `Epic`, `Task`. Implement: add_dependency, remove_dependency, get_blockers, get_blocked_by, get_all_dependencies, is_blocked. Validate referenced items exist by checking epics/tasks table. Prevent self-references and duplicate dependencies.

### Considerations
- `is_blocked` must check if any blocker is not in `done` status
- `get_all_dependencies` is used for graph rendering and should be efficient

### Anti-patterns to Avoid
- Do not skip validation — always verify referenced items exist
- Do not allow self-referencing dependencies

## Testing Requirements

### Unit Tests
- [ ] Add dependency between two epics
- [ ] Add dependency between two tasks
- [ ] Add cross-type dependency
- [ ] Reject duplicate dependency
- [ ] Reject self-reference
- [ ] is_blocked returns correct result based on blocker status
- [ ] Remove dependency works

### Integration Tests
- [ ] Full dependency lifecycle with status changes

### Manual Tests
- TBD

## Notes
No direct blocks, but epic_01 and epic_03 depend on this model.
