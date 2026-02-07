---
id: TK-0305
title: "Implement Edge Routing"
status: DONE
epic: 3
priority: medium
dependencies: [TK-0300, TK-0302]
blockers: []
commits: []
pr: ""
---

# Implement Edge Routing

## Objective
Draw edges (dependency arrows) between nodes on the graph canvas, routing around other nodes with directional arrows.

## Scope
- Edge characters: │, ─, ╰, ╯, ┬, ┐, ▼, ▶, ◄
- Routing strategy: bottom-center of blocker to top-center of blocked
- Simple vertical lines for adjacent layers, L/Z-shaped for others
- Edge merging with tee characters
- Color coding: cyan for normal, red for blocked targets

## Acceptance Criteria
- [ ] Edges render between connected nodes with correct direction
- [ ] Arrow heads point in the correct direction
- [ ] L-shaped routing for non-adjacent nodes works
- [ ] Merged edges use tee characters
- [ ] Edge colors match status of target node
- [ ] No edge overwriting node content

## Technical Context
### Relevant Spec Sections
- PRD.md — Edge routing specification

### Related Files/Directories
- `src/tui/graph.rs` — Edge rendering logic

### Dependencies on Other Systems
- DAG layout from TK-0300
- Node renderer from TK-0302 (for position data)

## Implementation Guidance
### Approach
Edge characters: vertical `│`, horizontal `─`, corners `╰`/`╯`, tee `┬`/`┐`, arrows `▼`/`▶`/`◄`. Routing: edges go from bottom-center of blocker to top-center of blocked. Adjacent layers: simple vertical line with `▼`. Same layer or across multiple layers: L-shaped or Z-shaped routing. Merge edges where multiple deps point to same target (use `┬` tee). Colors: normal = cyan, into blocked nodes = red. Render on 2D buffer after nodes are placed.

### Considerations
- Edges must not overwrite node content on the buffer
- Handle overlapping edges gracefully (allow character merging)

### Anti-patterns to Avoid
- Do not render edges before nodes — nodes take priority on the buffer

## Testing Requirements

### Unit Tests
- [ ] Simple vertical edge between adjacent layers
- [ ] L-shaped routing
- [ ] Edge merging with tee characters
- [ ] Color selection based on target status

### Integration Tests
- [ ] Full graph with nodes and edges

### Manual Tests
- [ ] Visual inspection of edge routing

## Notes
Blocks: TK-0306, TK-0307
