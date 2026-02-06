---
id: TK-0100
title: "Implement stdio JSON-RPC Transport"
status: TODO
epic: 1
priority: medium
dependencies: []
blockers: []
commits: []
pr: ""
---

# Implement stdio JSON-RPC Transport

## Objective
Build the low-level JSON-RPC 2.0 transport layer that reads requests from stdin and writes responses to stdout. This is the communication backbone of the MCP server.

## Scope
- Create `src/mcp/types.rs` with JSON-RPC request/response/error structs and standard error codes
- Create `src/mcp/mod.rs` with `McpServer` struct and read loop
- Handle malformed requests gracefully with proper error responses
- Use tokio for async stdin/stdout
- Logging goes to stderr (never stdout, which is reserved for JSON-RPC)

## Acceptance Criteria
- [ ] Server reads JSON-RPC requests from stdin line-by-line
- [ ] Valid requests are parsed and dispatched
- [ ] Malformed JSON returns ParseError
- [ ] Unknown methods return MethodNotFound
- [ ] All responses written to stdout as single-line JSON
- [ ] Stderr used for logging/debug output

## Technical Context
### Relevant Spec Sections
- PRD.md — MCP server transport layer

### Related Files/Directories
- `src/mcp/types.rs` — JSON-RPC type definitions
- `src/mcp/mod.rs` — Server struct and main loop

### Dependencies on Other Systems
- tokio for async I/O
- serde_json for JSON parsing

## Implementation Guidance
### Approach
Create `JsonRpcRequest`, `JsonRpcResponse`, and `JsonRpcError` structs. Define standard error codes: ParseError (-32700), InvalidRequest (-32600), MethodNotFound (-32601), InvalidParams (-32602), InternalError (-32603). Build an `McpServer` struct that reads stdin line-by-line, parses JSON-RPC, dispatches to handlers, and writes responses to stdout.

### Considerations
- All logging must go to stderr — stdout is exclusively for JSON-RPC
- Handle malformed requests gracefully without crashing
- Use tokio for async stdin/stdout reading

### Anti-patterns to Avoid
- Do not write any non-JSON-RPC output to stdout
- Do not panic on malformed input

## Testing Requirements

### Unit Tests
- [ ] Parse valid JSON-RPC request
- [ ] Malformed JSON returns ParseError response
- [ ] Unknown method returns MethodNotFound response
- [ ] Response serialization is single-line JSON

### Integration Tests
- [ ] Full request/response cycle via stdin/stdout

### Manual Tests
- [ ] Pipe JSON-RPC requests and verify responses

## Notes
Blocks: TK-0101, TK-0102, TK-0103, TK-0104, TK-0105, TK-0106, TK-0107
