---
id: TK-0304
title: "Implement Pulsing Border for Blocked Nodes"
status: DONE
epic: 3
priority: medium
dependencies: [TK-0302]
blockers: []
commits: []
pr: ""
---

# Implement Pulsing Border for Blocked Nodes

## Objective
Implement a pulsing brightness effect on blocked node borders. The border alternates between bright and dim red/orange, creating a warning pulse effect.

## Scope
- Solid double-line border with pulsing color
- Alternate between bright (neon orange/red) and dim (dark red/orange) colors
- ~800ms pulse cycle (alternate every 400ms)
- Uses same global animation counter as marching border

## Acceptance Criteria
- [ ] Blocked nodes have pulsing red/orange borders
- [ ] Pulse is visible and not too fast/slow
- [ ] Only blocked nodes pulse (others are static or marching)
- [ ] Smooth transition between bright and dim

## Technical Context
### Relevant Spec Sections
- PRD.md — Blocked node pulsing effect

### Related Files/Directories
- `src/tui/graph.rs` — Node border rendering
- `src/tui/mod.rs` — Animation frame counter

### Dependencies on Other Systems
- Node renderer from TK-0302
- Global animation counter (shared with TK-0303)

## Implementation Guidance
### Approach
Blocked nodes use solid double-line border (`╔═╗║╚═╝`). Color alternates between bright (neon orange #ff6e27 or bright red #ff2d6f) and dim (dark red #661122 or dark orange #663311). Pulse cycle: ~800ms (alternate every 400ms). Uses the same global animation counter as the marching border. Border style is always solid — only color pulses.

### Considerations
- Pulse rate should be distinguishable from marching animation rate
- Color transitions should be clearly visible

### Anti-patterns to Avoid
- Do not use a separate timer — share the global animation counter

## Testing Requirements

### Unit Tests
- [ ] Color selection based on animation frame
- [ ] Pulse timing logic

### Integration Tests
- [ ] Pulsing renders correctly alongside marching borders

### Manual Tests
- [ ] Visual inspection of pulse effect

## Notes
TBD
