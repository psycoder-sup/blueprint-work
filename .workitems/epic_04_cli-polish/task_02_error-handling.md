---
id: TK-0402
title: "Implement Error Handling & User-Friendly Messages"
status: TODO
epic: 4
priority: medium
dependencies: []
blockers: []
commits: []
pr: ""
---

# Implement Error Handling & User-Friendly Messages

## Objective
Audit all error paths and ensure they produce helpful, user-friendly messages instead of raw panics or cryptic errors.

## Scope
- Audit: DB open/creation, migration, MCP JSON-RPC, tool parameters, not-found errors, dependency violations, TUI terminal setup
- Use `anyhow` for error context chaining
- User-facing errors: describe what went wrong, suggest fix, include relevant context
- MCP errors follow JSON-RPC error codes
- CLI/TUI errors to stderr with color

## Acceptance Criteria
- [ ] No raw panics in normal operation
- [ ] All error messages are human-readable
- [ ] Error messages include actionable suggestions
- [ ] MCP errors follow JSON-RPC format
- [ ] Edge cases (empty DB, missing file, etc.) handled gracefully

## Technical Context
### Relevant Spec Sections
- PRD.md — Error handling requirements

### Related Files/Directories
- All `src/` files — error paths audit
- `src/mcp/` — JSON-RPC error formatting
- `src/db/` — Database error handling

### Dependencies on Other Systems
- anyhow crate for error context

## Implementation Guidance
### Approach
Audit all error paths across: database open/creation failures, migration failures, MCP JSON-RPC errors, invalid tool parameters, not-found errors, dependency violations, TUI terminal setup failures. Use `anyhow` for error context chaining. All user-facing errors should describe what went wrong, suggest what to do, and include relevant IDs/names. MCP errors follow JSON-RPC error codes. CLI/TUI errors printed to stderr with color.

### Considerations
- Error messages should be helpful to both end users and LLM clients
- MCP errors must strictly follow JSON-RPC error code conventions

### Anti-patterns to Avoid
- Do not use `.unwrap()` or `.expect()` in production paths
- Do not expose internal error details (file paths, SQL queries) to MCP clients

## Testing Requirements

### Unit Tests
- [ ] Each error path produces a human-readable message
- [ ] MCP errors have correct JSON-RPC error codes

### Integration Tests
- [ ] Error scenarios (missing DB, invalid input, etc.)

### Manual Tests
- [ ] Trigger various error conditions and verify messages

## Notes
TBD
