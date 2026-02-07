---
id: TK-0201
title: "Implement Cyberpunk Theme & Color Palette"
status: DONE
epic: 2
priority: medium
dependencies: [TK-0200]
blockers: []
commits: [11057d1]
pr: ""
---

# Implement Cyberpunk Theme & Color Palette

## Objective
Define the cyberpunk neon color palette and visual theme constants used across all TUI panels and widgets.

## Scope
- Create `src/tui/theme.rs` with all color constants and style presets
- Color constants: BG, NEON_CYAN, NEON_MAGENTA, NEON_PINK, NEON_GREEN, NEON_ORANGE, ELECTRIC_BLUE, TEXT_DIM, TEXT_BRIGHT, BORDER_DIM, BORDER_BRIGHT
- Style presets: panel_border, status_style, status_symbol, progress_bar
- ASCII art header for "BLUEPRINT"

## Acceptance Criteria
- [ ] All color constants defined and match PRD spec
- [ ] Style presets return correct ratatui Styles
- [ ] Status symbols render with correct colors
- [ ] Progress bar renders correctly for various ratios
- [ ] ASCII art header looks good in terminal

## Technical Context
### Relevant Spec Sections
- PRD.md — Cyberpunk theme specification

### Related Files/Directories
- `src/tui/theme.rs` — Theme constants and style presets

### Dependencies on Other Systems
- ratatui Color::Rgb for color definitions

## Implementation Guidance
### Approach
Define color constants as ratatui Color::Rgb values. Create style preset functions: `panel_border(focused: bool) -> Style`, `status_style(status: ItemStatus) -> Style`, `status_symbol(status: ItemStatus) -> &str` (returns ◉/◆/▶/■), `progress_bar(done, total, width) -> String` (█░ bar). Design ASCII art header for "BLUEPRINT" in a stylized cyber font.

### Considerations
- Colors should look good on both dark and light terminal backgrounds (primarily dark)
- Progress bar width should be configurable

### Anti-patterns to Avoid
- Do not use ANSI 16 colors — use RGB for consistent appearance

## Testing Requirements

### Unit Tests
- [ ] progress_bar returns correct string for various ratios (0%, 50%, 100%)
- [ ] status_style returns correct color for each status
- [ ] status_symbol returns correct character for each status

### Integration Tests
- TBD

### Manual Tests
- [ ] Visual inspection of all colors in terminal
- [ ] ASCII art header rendering

## Notes
Blocks: TK-0202, TK-0203, TK-0204, TK-0205, TK-0206
