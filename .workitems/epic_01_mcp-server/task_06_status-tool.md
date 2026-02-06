---
id: TK-0106
title: "Implement get_status Tool"
status: DONE
epic: 1
priority: medium
dependencies: [TK-0101]
blockers: []
commits: []
pr: ""
---

# Implement get_status Tool

## Objective
Implement the `get_status` MCP tool that returns a project-wide status overview with aggregate counts and blocked item details.

## Scope
- Optional `project_id` param — scoped to project or all projects
- Response includes: project name, epic/task counts by status, blocked items list
- Efficient queries using GROUP BY and JOIN

## Acceptance Criteria
- [ ] Returns correct aggregate counts
- [ ] Status breakdown matches actual data
- [ ] Blocked items correctly identified (blockers not `done`)
- [ ] Works with and without project_id filter

## Technical Context
### Relevant Spec Sections
- PRD.md — get_status tool specification

### Related Files/Directories
- `src/mcp/tools.rs` — Tool dispatch
- `src/db/` — Database query layer

### Dependencies on Other Systems
- Database layer from epic_00

## Implementation Guidance
### Approach
Accept optional `project_id` param. Query total_epics, epics_by_status breakdown, total_tasks, tasks_by_status breakdown. Query blocked_items array with type, id, title, and blocked_by list. Use GROUP BY for status counts, JOIN for blocked items. Return comprehensive status overview.

### Considerations
- Query efficiency: use GROUP BY for status counts, JOIN for blocked items
- If no project_id, aggregate across all projects

### Anti-patterns to Avoid
- Do not issue N+1 queries — batch blocked item lookups

## Testing Requirements

### Unit Tests
- [ ] Aggregate counts match actual data
- [ ] Status breakdown is correct
- [ ] Blocked items identified correctly

### Integration Tests
- [ ] Status with and without project_id filter

### Manual Tests
- [ ] Test via MCP client

## Notes
TBD
