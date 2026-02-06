---
id: TK-0403
title: "Write README & Usage Documentation"
status: TODO
epic: 4
priority: medium
dependencies: []
blockers: []
commits: []
pr: ""
---

# Write README & Usage Documentation

## Objective
Write comprehensive README documentation covering installation, MCP server configuration, CLI usage, and TUI guide.

## Scope
- Sections: Overview, Installation, MCP Server Setup, CLI Usage, MCP Tools Reference, TUI Guide, Environment Variables, Development
- Include placeholder screenshots/recordings
- Keep concise — link to PRD for full specs

## Acceptance Criteria
- [ ] README covers all sections listed above
- [ ] MCP config example is copy-pasteable
- [ ] Keybindings table is complete
- [ ] Clear installation instructions

## Technical Context
### Relevant Spec Sections
- PRD.md — Full project specification

### Related Files/Directories
- `README.md` — Project README

### Dependencies on Other Systems
- All other epics should be near-complete before writing final docs

## Implementation Guidance
### Approach
Write sections: Overview (what Blueprint is, Project → Epic → Blue-Task hierarchy), Installation (cargo install, building from source), MCP Server Setup (configuration JSON for Claude Desktop / Claude Code), CLI Usage (blueprint serve, tui, status), MCP Tools Reference (brief description of all 19 tools), TUI Guide (keybindings, panels, dependency graph view), Environment Variables (BLUEPRINT_DB), Development (building, testing, contributing). Include placeholder screenshots. Keep concise.

### Considerations
- MCP config example must be copy-pasteable and correct
- All keybindings should match the actual implementation

### Anti-patterns to Avoid
- Do not write documentation before features are implemented — keep in sync

## Testing Requirements

### Unit Tests
- TBD

### Integration Tests
- TBD

### Manual Tests
- [ ] Follow installation instructions from scratch
- [ ] Copy-paste MCP config and verify it works

## Notes
Dependencies note: "All other epics should be near-complete" — this is a soft dependency, not a hard task blocker.
