# Epic 01: MCP Server

## Description

Implement the stdio-based MCP (Model Context Protocol) server. This is the core interface that LLMs use to interact with Blueprint — creating projects, epics, blue-tasks, managing dependencies, and feeding PRDs for breakdown.

## Status

`todo`

## Dependencies

- epic_00_foundation (all CRUD operations must be in place)

## Blocked By

- epic_00_foundation

## Blocks

(none — can be developed in parallel with epic_02 once foundation is done)

## Tasks

| # | Task | Status |
|---|------|--------|
| 00 | Implement stdio JSON-RPC transport | done |
| 01 | Implement MCP protocol handshake | done |
| 02 | Implement Project CRUD tools (5 tools) | done |
| 03 | Implement Epic CRUD tools (5 tools) | todo |
| 04 | Implement BlueTask CRUD tools (5 tools) | todo |
| 05 | Implement dependency tools (add/remove) | todo |
| 06 | Implement get_status tool | todo |
| 07 | Implement feed_prd tool | todo |

## Acceptance Criteria

- MCP server starts via `blueprint serve` and communicates over stdin/stdout
- All 19 tools respond correctly to JSON-RPC calls
- MCP handshake (initialize, tools/list) works with Claude Desktop / Claude Code
- Error responses follow JSON-RPC error format
- Integration test: full flow of create project -> create epics -> create tasks -> add deps -> get status
