---
id: TK-0101
title: "Implement MCP Protocol Handshake"
status: DONE
epic: 1
priority: medium
dependencies: [TK-0100]
blockers: []
commits: [a9db571]
pr: ""
---

# Implement MCP Protocol Handshake

## Objective
Implement the MCP protocol initialization handshake so the server can register with MCP clients (Claude Desktop, Claude Code, etc.) and advertise its available tools.

## Scope
- Handle `initialize` method with client capabilities and protocol version
- Handle `initialized` notification (no response needed)
- Handle `tools/list` method returning all 19 tool definitions
- Handle `tools/call` method dispatching to tool handlers
- Create `src/mcp/tools.rs` with tool definitions and dispatch

## Acceptance Criteria
- [ ] `initialize` returns valid server info with tools capability
- [ ] `tools/list` returns all 19 tool definitions
- [ ] Each tool has a valid JSON Schema for its inputSchema
- [ ] `tools/call` dispatches to the correct handler
- [ ] Unknown tool names return an appropriate error

## Technical Context
### Relevant Spec Sections
- PRD.md — MCP protocol handshake and tool definitions

### Related Files/Directories
- `src/mcp/tools.rs` — Tool definitions and dispatch
- `src/mcp/mod.rs` — Handler registration

### Dependencies on Other Systems
- MCP protocol specification
- serde_json for JSON Schema generation

## Implementation Guidance
### Approach
Handle `initialize` by receiving client capabilities, returning server info (name: "blueprint", version, capabilities: tools). Handle `initialized` as a no-op notification. For `tools/list`, return all 19 tool definitions with name, description, and inputSchema (JSON Schema). For `tools/call`, dispatch to the appropriate handler. Create `ToolDefinition` struct and `get_tool_definitions()` / `dispatch_tool()` functions.

### Considerations
- Each tool's inputSchema must accurately describe required/optional parameters
- Store client info for the session after initialize

### Anti-patterns to Avoid
- Do not hardcode tool schemas inline — use a structured definition approach

## Testing Requirements

### Unit Tests
- [ ] initialize returns valid server info
- [ ] tools/list returns correct count and format
- [ ] tools/call dispatches to correct handler
- [ ] Unknown tool returns error

### Integration Tests
- [ ] Full handshake sequence: initialize → initialized → tools/list

### Manual Tests
- [ ] Test with Claude Desktop or Claude Code as MCP client

## Notes
Blocks: TK-0102
