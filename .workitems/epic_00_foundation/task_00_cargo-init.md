---
id: TK-0000
title: "Initialize Cargo Project with Clap CLI"
status: TODO
epic: 0
priority: medium
dependencies: []
blockers: []
commits: []
pr: ""
---

# Initialize Cargo Project with Clap CLI

## Objective
Bootstrap the Rust project with Cargo. Set up the binary crate with `clap` for CLI argument parsing. Define the three subcommands: `serve`, `tui`, and `status`. Each should initially print a placeholder message.

## Scope
- Initialize with `cargo init --name blueprint`
- Add dependencies to Cargo.toml: clap (derive), serde, serde_json, rusqlite (bundled), tokio, ratatui, crossterm, ulid, chrono, anyhow
- Create `src/main.rs` with clap derive-based CLI
- Set up module structure: `mcp/`, `db/`, `models/`, `tui/`, `cli/`
- Excluded: actual implementation of subcommands (handled by later tasks)

## Acceptance Criteria
- [ ] `cargo build` succeeds with no warnings
- [ ] `blueprint --help` shows subcommands
- [ ] `blueprint serve`, `blueprint tui`, `blueprint status` print placeholder messages
- [ ] Module directory structure exists (even if mod.rs files are minimal)

## Technical Context
### Relevant Spec Sections
- PRD.md — Project setup and CLI structure

### Related Files/Directories
- `Cargo.toml` — Project manifest
- `src/main.rs` — Entry point
- `src/mcp/`, `src/db/`, `src/models/`, `src/tui/`, `src/cli/` — Module directories

### Dependencies on Other Systems
- None

## Implementation Guidance
### Approach
Use `clap` derive macros for CLI definition. Define three subcommands: `serve`, `tui`, `status`. Create minimal `mod.rs` files in each module directory to establish the project structure.

### Considerations
- Ensure all listed dependencies compile together without conflicts
- Use `bundled` feature for rusqlite to avoid system SQLite dependency

### Anti-patterns to Avoid
- Do not implement actual functionality in subcommands yet — placeholder messages only

## Testing Requirements

### Unit Tests
- [ ] CLI argument parsing works for all subcommands

### Integration Tests
- [ ] `cargo build` produces a working binary

### Manual Tests
- [ ] Run `blueprint --help` and verify output
- [ ] Run each subcommand and verify placeholder output

## Notes
Blocks: TK-0001, TK-0002
