---
id: TK-0002
title: "Implement Project Model & CRUD"
status: TODO
epic: 0
priority: medium
dependencies: [TK-0001]
blockers: []
commits: []
pr: ""
---

# Implement Project Model & CRUD

## Objective
Define the `Project` model struct and implement full CRUD operations at the database layer.

## Scope
- Create `src/models/project.rs` with `Project` struct, `ProjectStatus` enum, and input structs
- Create `src/db/project.rs` with all 5 CRUD operations
- ULID generation for IDs
- `updated_at` auto-set on updates
- Excluded: MCP tool wiring (handled by epic_01)

## Acceptance Criteria
- [ ] All 5 CRUD operations work correctly
- [ ] ULID IDs are generated for new projects
- [ ] Status filter works on list
- [ ] Deleting a project cascades to epics, tasks, and PRDs
- [ ] Unit tests for all operations

## Technical Context
### Relevant Spec Sections
- PRD.md — Project model definition

### Related Files/Directories
- `src/models/project.rs` — Model struct and enums
- `src/db/project.rs` — Database CRUD operations

### Dependencies on Other Systems
- ulid crate for ID generation
- serde for serialization

## Implementation Guidance
### Approach
Define `Project` struct with id (ULID), name, description, status (Active/Archived), created_at, updated_at. Define `ProjectStatus` enum: `Active`, `Archived` with serde serialization. Define `CreateProjectInput` and `UpdateProjectInput` structs. Implement create_project, get_project, list_projects, update_project, delete_project.

### Considerations
- `updated_at` should be automatically set on every update
- Status filter on list_projects should be optional
- CASCADE delete via foreign keys handles cleanup

### Anti-patterns to Avoid
- Do not manually delete child records — rely on CASCADE

## Testing Requirements

### Unit Tests
- [ ] Create project returns valid ULID
- [ ] Get project by ID returns correct data
- [ ] List projects with and without status filter
- [ ] Update project modifies only provided fields
- [ ] Delete project cascades to children

### Integration Tests
- [ ] Full lifecycle: create → read → update → delete

### Manual Tests
- TBD

## Notes
Epic_01 depends on this model for MCP tool implementation.
