---
id: TK-0302
title: "Build ASCII Box-Node Renderer"
status: TODO
epic: 3
priority: medium
dependencies: [TK-0300]
blockers: []
commits: []
pr: ""
---

# Build ASCII Box-Node Renderer

## Objective
Render each node in the dependency graph as an ASCII box with the item's title, status, and progress info. Border style and color vary by status.

## Scope
- Node box format with fixed width (~20 chars)
- Border styles by status: DONE (solid green), TODO (solid gray), IN_PROGRESS (dotted cyan), BLOCKED (solid pulsing red)
- Status symbol inside with appropriate colors
- Progress bar for epic nodes
- 2D character buffer for compositing
- Node positioning at assigned (x, y) coordinates

## Acceptance Criteria
- [ ] Nodes render as proper ASCII boxes
- [ ] Border style matches status
- [ ] Colors are correct per theme
- [ ] Title truncated if too long (with ellipsis)
- [ ] Progress bar renders inside epic nodes
- [ ] Nodes positioned correctly on canvas

## Technical Context
### Relevant Spec Sections
- PRD.md — Node rendering specification

### Related Files/Directories
- `src/tui/graph.rs` — Node rendering logic

### Dependencies on Other Systems
- Theme colors from TK-0201

## Implementation Guidance
### Approach
Node box format: `╔══════════════════╗ / ║ ◉ {title} ║ / ║ [{progress}] ║ / ╚══════════════════╝`. Border styles by status: DONE = solid double-line neon green, TODO = solid double-line dim gray, IN_PROGRESS = dotted border neon cyan (animated by TK-0303), BLOCKED = solid double-line pulsing red/orange (by TK-0304). Status symbol inside: ◉/◆/▶/■. Progress line for epics. Render on 2D character buffer.

### Considerations
- Title truncation with ellipsis for long titles
- 2D character buffer needed for compositing nodes and edges

### Anti-patterns to Avoid
- Do not hardcode node width — define as a constant for easy adjustment

## Testing Requirements

### Unit Tests
- [ ] Node box rendering for each status
- [ ] Title truncation
- [ ] Progress bar inside node

### Integration Tests
- [ ] Multiple nodes positioned on canvas

### Manual Tests
- [ ] Visual inspection of rendered nodes

## Notes
Blocks: TK-0303, TK-0304, TK-0306, TK-0307
