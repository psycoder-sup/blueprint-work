---
id: TK-0103
title: "Implement Epic CRUD Tools"
status: TODO
epic: 1
priority: medium
dependencies: [TK-0101]
blockers: []
commits: []
pr: ""
---

# Implement Epic CRUD Tools

## Objective
Wire up the 5 epic-related MCP tools: create_epic, list_epics, get_epic, update_epic, delete_epic.

## Scope
- Implement all 5 epic tool handlers
- Validate project_id exists on create
- get_epic returns epic with nested blue-tasks list and dependency info
- Return MCP-formatted content with JSON payloads

## Acceptance Criteria
- [ ] All 5 epic tools callable via `tools/call`
- [ ] create_epic validates project_id exists
- [ ] get_epic includes nested tasks and dependency info
- [ ] list_epics supports both project_id and status filtering
- [ ] Proper error handling for invalid inputs

## Technical Context
### Relevant Spec Sections
- PRD.md — Epic MCP tools specification

### Related Files/Directories
- `src/mcp/tools.rs` — Tool dispatch
- `src/db/epic.rs` — Database CRUD layer

### Dependencies on Other Systems
- Database layer from epic_00

## Implementation Guidance
### Approach
For each tool: parse parameters, validate references (e.g., project_id exists), call database function, format result as MCP content. `get_epic` should return the epic with a nested list of its blue-tasks and dependency information. `delete_epic` cascades to tasks and cleans up dependencies.

### Considerations
- Validate project_id exists before creating epics
- get_epic response should be rich enough for LLM context

### Anti-patterns to Avoid
- Do not return raw database errors to the client

## Testing Requirements

### Unit Tests
- [ ] create_epic with valid/invalid project_id
- [ ] list_epics with various filters
- [ ] get_epic includes nested data

### Integration Tests
- [ ] Full CRUD cycle via MCP tools

### Manual Tests
- [ ] Test via MCP client

## Notes
TBD
