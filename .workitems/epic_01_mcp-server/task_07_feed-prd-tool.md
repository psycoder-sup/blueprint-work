---
id: TK-0107
title: "Implement feed_prd Tool"
status: TODO
epic: 1
priority: medium
dependencies: [TK-0101]
blockers: []
commits: []
pr: ""
---

# Implement feed_prd Tool

## Objective
Implement the `feed_prd` MCP tool that accepts PRD text, stores it in the database, and returns guidance for the LLM to break it down into epics and blue-tasks.

## Scope
- Required params: project_id, content, title
- Validate project_id exists
- Store PRD in the `prds` table with a generated ULID
- Return confirmation with prd_id and LLM guide text

## Acceptance Criteria
- [ ] PRD content stored in database
- [ ] Response includes prd_id and guide text
- [ ] Guide text is actionable for an LLM to follow
- [ ] Invalid project_id returns error

## Technical Context
### Relevant Spec Sections
- PRD.md — feed_prd tool specification

### Related Files/Directories
- `src/mcp/tools.rs` — Tool dispatch
- `src/db/` — Database layer (prds table)

### Dependencies on Other Systems
- Database layer from epic_00 (prds table)

## Implementation Guidance
### Approach
Parse required params (project_id, content, title). Validate project_id exists. Store PRD in the `prds` table with a generated ULID. Return a response with confirmation message, generated prd_id, and a guide string instructing the LLM to: (1) Analyze the PRD content, (2) Create epics via `create_epic`, (3) Create blue-tasks via `create_task`, (4) Set up dependencies via `add_dependency`.

### Considerations
- The guide text should be clear enough that the LLM can autonomously execute the breakdown
- Include the project_id in the guide so the LLM knows where to create epics

### Anti-patterns to Avoid
- Do not attempt to parse or analyze the PRD content server-side — that's the LLM's job

## Testing Requirements

### Unit Tests
- [ ] PRD stored with valid project_id
- [ ] Invalid project_id returns error
- [ ] Response includes prd_id and guide text

### Integration Tests
- [ ] Full flow: feed_prd → create_epic → create_task → add_dependency

### Manual Tests
- [ ] Test via MCP client with Claude

## Notes
TBD
