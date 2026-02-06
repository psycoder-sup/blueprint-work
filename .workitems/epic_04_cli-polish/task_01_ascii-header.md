---
id: TK-0401
title: "Create ASCII Art Header & Branding"
status: TODO
epic: 4
priority: medium
dependencies: []
blockers: []
commits: []
pr: ""
---

# Create ASCII Art Header & Branding

## Objective
Design and implement a cyberpunk-styled ASCII art header for "BLUEPRINT" used in the TUI header and CLI status output.

## Scope
- Stylized "BLUEPRINT" logo in ASCII art with glitch/cyber aesthetic
- Must fit within 80 columns
- Neon cyan primary color with magenta accents
- Header rendering function usable from both TUI and CLI
- Optional: subtle "glitch" effect for TUI animated header

## Acceptance Criteria
- [ ] ASCII header looks distinctly cyberpunk
- [ ] Fits within 80 columns
- [ ] Renders correctly in both TUI and CLI contexts
- [ ] Colors applied when terminal supports them

## Technical Context
### Relevant Spec Sections
- PRD.md — Branding and visual identity

### Related Files/Directories
- `src/tui/theme.rs` — Theme constants (shared with TUI)
- `src/cli/` — CLI rendering

### Dependencies on Other Systems
- crossterm for CLI colors
- ratatui for TUI colors

## Implementation Guidance
### Approach
Create a stylized "BLUEPRINT" logo in ASCII art. Consider fonts like ANSI Shadow, Cyberlarge, Electronic, or a custom design with block characters. Should fit within 80 columns. Neon cyan primary color with magenta accents. Include a header rendering function usable from both TUI and CLI contexts. Optional: subtle "glitch" effect (randomized block chars) for TUI animated header.

### Considerations
- Must look good in both TUI (ratatui) and CLI (crossterm) contexts
- Consider terminal width constraints (80 columns)

### Anti-patterns to Avoid
- Do not make the header too tall — keep it compact (5-7 lines max)

## Testing Requirements

### Unit Tests
- [ ] Header string fits within 80 columns
- [ ] Header rendering function works

### Integration Tests
- TBD

### Manual Tests
- [ ] Visual inspection in terminal
- [ ] Test in both TUI and CLI contexts

## Notes
TBD
