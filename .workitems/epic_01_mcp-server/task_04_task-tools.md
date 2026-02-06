---
id: TK-0104
title: "Implement BlueTask CRUD Tools"
status: TODO
epic: 1
priority: medium
dependencies: [TK-0101]
blockers: []
commits: []
pr: ""
---

# Implement BlueTask CRUD Tools

## Objective
Wire up the 5 blue-task-related MCP tools: create_task, list_tasks, get_task, update_task, delete_task.

## Scope
- Implement all 5 task tool handlers
- Validate epic_id exists on create
- get_task returns task with dependency info (what it blocks, what blocks it)
- Return MCP-formatted content with JSON payloads

## Acceptance Criteria
- [ ] All 5 task tools callable via `tools/call`
- [ ] create_task validates epic_id exists
- [ ] get_task includes dependency info
- [ ] list_tasks supports both epic_id and status filtering
- [ ] Proper error handling for invalid inputs

## Technical Context
### Relevant Spec Sections
- PRD.md — BlueTask MCP tools specification

### Related Files/Directories
- `src/mcp/tools.rs` — Tool dispatch
- `src/db/task.rs` — Database CRUD layer

### Dependencies on Other Systems
- Database layer from epic_00

## Implementation Guidance
### Approach
For each tool: parse parameters, validate references (e.g., epic_id exists), call database function, format result as MCP content. `get_task` should return the task with dependency info (what it blocks, what blocks it). `delete_task` should clean up related dependencies.

### Considerations
- Validate epic_id exists before creating tasks
- get_task dependency info helps LLMs understand task relationships

### Anti-patterns to Avoid
- Do not return raw database errors to the client

## Testing Requirements

### Unit Tests
- [ ] create_task with valid/invalid epic_id
- [ ] list_tasks with various filters
- [ ] get_task includes dependency info

### Integration Tests
- [ ] Full CRUD cycle via MCP tools

### Manual Tests
- [ ] Test via MCP client

## Notes
TBD
