---
id: TK-0102
title: "Implement Project CRUD Tools"
status: TODO
epic: 1
priority: medium
dependencies: [TK-0101]
blockers: []
commits: []
pr: ""
---

# Implement Project CRUD Tools

## Objective
Wire up the 5 project-related MCP tools to the database CRUD layer: create_project, list_projects, get_project, update_project, delete_project.

## Scope
- Implement all 5 project tool handlers
- Parse parameters, call database layer, return MCP-formatted content
- Proper error messages for missing required params and not-found IDs

## Acceptance Criteria
- [ ] All 5 project tools callable via `tools/call`
- [ ] Create returns the new project with generated ULID
- [ ] List supports status filtering
- [ ] Get includes epic summary
- [ ] Update only modifies provided fields
- [ ] Delete cascades properly
- [ ] Missing required params return InvalidParams error
- [ ] Not-found IDs return meaningful error message

## Technical Context
### Relevant Spec Sections
- PRD.md — Project MCP tools specification

### Related Files/Directories
- `src/mcp/tools.rs` — Tool dispatch
- `src/db/project.rs` — Database CRUD layer

### Dependencies on Other Systems
- Database layer from epic_00

## Implementation Guidance
### Approach
For each tool: parse parameters from JSON, call the corresponding database function, format the result as MCP content (text type with JSON payload). Handle missing required params with InvalidParams error. Handle not-found IDs with meaningful error messages. `get_project` should include an epic count summary in its response.

### Considerations
- All tools return MCP-formatted content (text type with JSON payload)
- Proper error messages for missing required params, not-found IDs

### Anti-patterns to Avoid
- Do not return raw database errors to the client — wrap in user-friendly messages

## Testing Requirements

### Unit Tests
- [ ] Each tool parses parameters correctly
- [ ] Missing required params return InvalidParams
- [ ] Not-found IDs return meaningful error

### Integration Tests
- [ ] Full CRUD cycle via MCP tools

### Manual Tests
- [ ] Test via MCP client

## Notes
TBD
