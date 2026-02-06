---
id: TK-0003
title: "Implement Epic Model & CRUD"
status: DONE
epic: 0
priority: medium
dependencies: [TK-0001, TK-0002]
blockers: []
commits: []
pr: ""
---

# Implement Epic Model & CRUD

## Objective
Define the `Epic` model struct and implement full CRUD operations at the database layer. Epics belong to a Project.

## Scope
- Create `src/models/epic.rs` with `Epic` struct, `ItemStatus` enum (shared with tasks), and input structs
- Create `src/db/epic.rs` with all 5 CRUD operations
- Validate that project_id exists before creating
- Excluded: MCP tool wiring (handled by epic_01)

## Acceptance Criteria
- [x] All 5 CRUD operations work correctly
- [x] Epics are scoped to a project (project_id FK enforced)
- [x] list_epics supports filtering by project_id and status
- [x] Deleting an epic cascades to its tasks
- [x] Unit tests for all operations

## Technical Context
### Relevant Spec Sections
- PRD.md — Epic model definition

### Related Files/Directories
- `src/models/epic.rs` — Model struct and enums
- `src/db/epic.rs` — Database CRUD operations

### Dependencies on Other Systems
- ulid crate for ID generation
- serde for serialization

## Implementation Guidance
### Approach
Define `Epic` struct with id (ULID), project_id, title, description, status, created_at, updated_at. Define shared `ItemStatus` enum: `Todo`, `InProgress`, `Done` (used by tasks too). Implement create_epic, get_epic (with task count summary), list_epics, update_epic, delete_epic. Validate project_id references an existing project.

### Considerations
- The `ItemStatus` enum is shared between epics and tasks — define it in a common location
- get_epic should include task count / summary information

### Anti-patterns to Avoid
- Do not skip foreign key validation — verify project_id exists before insert

## Testing Requirements

### Unit Tests
- [x] Create epic with valid project_id
- [x] Create epic with invalid project_id fails
- [x] List epics with project_id and status filters
- [x] Delete epic cascades to tasks
- [x] get_epic includes task count

### Integration Tests
- [x] Full lifecycle: create → read → update → delete

### Manual Tests
- TBD

## Notes
Blocks: TK-0004
