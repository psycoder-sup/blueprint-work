---
id: TK-0105
title: "Implement Dependency Tools"
status: DONE
epic: 1
priority: medium
dependencies: [TK-0101]
blockers: []
commits: [01e0ca1]
pr: ""
---

# Implement Dependency Tools

## Objective
Wire up the add_dependency and remove_dependency MCP tools.

## Scope
- Implement add_dependency tool handler with validation
- Implement remove_dependency tool handler
- Validate both referenced items exist, prevent self-references
- Clear error messages for invalid references

## Acceptance Criteria
- [x] add_dependency creates a valid blocks/blocked-by relationship
- [x] Duplicate dependencies handled gracefully
- [x] Self-references rejected
- [x] remove_dependency works for existing deps
- [x] Removing non-existent deps returns informative message

## Technical Context
### Relevant Spec Sections
- PRD.md — Dependency MCP tools specification

### Related Files/Directories
- `src/mcp/tools.rs` — Tool dispatch
- `src/db/dependency.rs` — Database dependency layer

### Dependencies on Other Systems
- Database layer from epic_00

## Implementation Guidance
### Approach
`add_dependency` requires blocker_type, blocker_id, blocked_type, blocked_id. Validate both referenced items exist. Prevent self-references. Return the created dependency. `remove_dependency` takes the same params, removes the relationship, and returns success/failure message. Both tools should give clear error messages for invalid references.

### Considerations
- Duplicate dependencies should be handled gracefully (UNIQUE constraint)
- Self-references must be explicitly rejected with a clear message

### Anti-patterns to Avoid
- Do not silently ignore invalid references — return clear error messages

## Testing Requirements

### Unit Tests
- [x] add_dependency with valid references
- [x] add_dependency rejects self-reference
- [x] add_dependency rejects duplicate
- [x] remove_dependency for existing dep
- [x] remove_dependency for non-existent dep

### Integration Tests
- [x] Full dependency lifecycle via MCP tools

### Manual Tests
- [ ] Test via MCP client

## Notes
TBD
