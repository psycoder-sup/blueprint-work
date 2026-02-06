---
id: TK-0004
title: "Implement BlueTask Model & CRUD"
status: DONE
epic: 0
priority: medium
dependencies: [TK-0003]
blockers: []
commits: []
pr: ""
---

# Implement BlueTask Model & CRUD

## Objective
Define the `BlueTask` model struct and implement full CRUD operations at the database layer. Blue-Tasks belong to an Epic.

## Scope
- Create `src/models/task.rs` with `BlueTask` struct and input structs
- Create `src/db/task.rs` with all 5 CRUD operations
- Reuse `ItemStatus` enum from epic model
- Validate that epic_id exists before creating
- Excluded: MCP tool wiring (handled by epic_01)

## Acceptance Criteria
- [ ] All 5 CRUD operations work correctly
- [ ] Tasks are scoped to an epic (epic_id FK enforced)
- [ ] list_tasks supports filtering by epic_id and status
- [ ] Deleting a task cleans up its dependencies
- [ ] Unit tests for all operations

## Technical Context
### Relevant Spec Sections
- PRD.md — BlueTask model definition

### Related Files/Directories
- `src/models/task.rs` — Model struct
- `src/db/task.rs` — Database CRUD operations

### Dependencies on Other Systems
- ulid crate for ID generation
- serde for serialization

## Implementation Guidance
### Approach
Define `BlueTask` struct with id (ULID), epic_id, title, description, status (ItemStatus), created_at, updated_at. Define `CreateTaskInput` and `UpdateTaskInput` structs. Implement create_task, get_task, list_tasks, update_task, delete_task. Validate epic_id references an existing epic.

### Considerations
- Deleting a task should clean up associated dependency records
- Reuse the `ItemStatus` enum from the epic model

### Anti-patterns to Avoid
- Do not define a separate status enum — reuse `ItemStatus`

## Testing Requirements

### Unit Tests
- [ ] Create task with valid epic_id
- [ ] Create task with invalid epic_id fails
- [ ] List tasks with epic_id and status filters
- [ ] Delete task cleans up dependencies
- [ ] All CRUD operations return correct data

### Integration Tests
- [ ] Full lifecycle: create → read → update → delete

### Manual Tests
- TBD

## Notes
No direct blocks, but epic_01 depends on this model.
