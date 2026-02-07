---
id: TK-0303
title: "Implement Animated Marching Dotted Border"
status: DONE
epic: 3
priority: medium
dependencies: [TK-0302]
blockers: []
commits: []
pr: ""
---

# Implement Animated Marching Dotted Border

## Objective
Implement the animated "marching ants" dotted border effect for in-progress nodes. The dashed border cycles through 4 frames, creating an "electricity flowing through wires" cyberpunk effect.

## Scope
- 4-frame animation cycle at ~500ms per frame
- Alternating dash characters (╌ and ┄) for horizontal, (╎ and ┆) for vertical
- Global animation frame counter in App state
- Node renderer reads current frame for border selection
- Border color: neon cyan (#00fff5)

## Acceptance Criteria
- [ ] In-progress nodes display dotted borders
- [ ] Border pattern visibly shifts every ~500ms
- [ ] Animation is smooth (no stuttering)
- [ ] All 4 frames render correctly
- [ ] Only in-progress nodes animate (others have static borders)

## Technical Context
### Relevant Spec Sections
- PRD.md — Marching border animation

### Related Files/Directories
- `src/tui/graph.rs` — Node border rendering
- `src/tui/mod.rs` — Animation frame counter

### Dependencies on Other Systems
- Node renderer from TK-0302

## Implementation Guidance
### Approach
4-frame animation cycle: Frame 1: `┌╌╌╌╌╌╌┐` / `╎` / `└╌╌╌╌╌╌┘`, Frame 2-4 alternate between `╌`/`┄` and `╎`/`┆`. Track a global animation frame counter in App state, incremented every ~500ms. Node renderer reads the current frame and selects the appropriate border characters. Border color: neon cyan (#00fff5). Unicode characters used: ╌ (U+254C), ┄ (U+2504), ╎ (U+254E), ┆ (U+2506).

### Considerations
- Animation timer should be tied to the event loop tick, not wall clock
- Only in-progress nodes should use this border

### Anti-patterns to Avoid
- Do not create per-node timers — use a single global animation counter

## Testing Requirements

### Unit Tests
- [ ] Each frame generates correct border characters
- [ ] Frame cycling wraps around correctly

### Integration Tests
- [ ] Animation renders smoothly in graph view

### Manual Tests
- [ ] Visual inspection of animation smoothness

## Notes
TBD
