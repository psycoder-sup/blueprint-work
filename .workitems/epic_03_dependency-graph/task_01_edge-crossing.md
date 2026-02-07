---
id: TK-0301
title: "Implement Edge-Crossing Minimization"
status: DONE
epic: 3
priority: medium
dependencies: [TK-0300]
blockers: []
commits: []
pr: ""
---

# Implement Edge-Crossing Minimization

## Objective
Implement the barycenter heuristic to minimize edge crossings when positioning nodes within the same layer.

## Scope
- Barycenter computation for each node based on connected nodes in previous layer
- Sort nodes within layers by barycenter value
- Multiple passes (2-3 iterations) for convergence
- Assign x_position based on sorted order

## Acceptance Criteria
- [ ] Nodes within same layer are ordered to minimize crossings
- [ ] Algorithm converges within a few iterations
- [ ] Handles various graph shapes (wide, deep, diamond, fan-out)
- [ ] Unit tests comparing crossing counts before/after

## Technical Context
### Relevant Spec Sections
- PRD.md — Edge-crossing minimization algorithm

### Related Files/Directories
- `src/tui/graph.rs` — Layout algorithm extension

### Dependencies on Other Systems
- DAG layout from TK-0300

## Implementation Guidance
### Approach
For each layer (top to bottom): compute the barycenter (average position) of each node based on its connected nodes in the previous layer. Sort nodes within the layer by their barycenter value. This minimizes the number of crossing edges. Run multiple passes (2-3 iterations) for better results. Assign x_position to each node based on its sorted position. Handle nodes with no connections to previous layer (place at edges).

### Considerations
- Multiple passes improve results but have diminishing returns
- Nodes with no connections to previous layer need special positioning

### Anti-patterns to Avoid
- Do not run excessive iterations — 2-3 passes are sufficient

## Testing Requirements

### Unit Tests
- [ ] Barycenter calculation for simple graphs
- [ ] Crossing count reduction after optimization
- [ ] Various graph topologies

### Integration Tests
- [ ] Full layout pipeline with crossing minimization

### Manual Tests
- TBD

## Notes
Blocks: TK-0306, TK-0307
