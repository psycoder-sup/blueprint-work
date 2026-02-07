---
id: TK-0300
title: "Implement DAG Topological Sort & Layer Assignment"
status: DONE
epic: 3
priority: medium
dependencies: []
blockers: []
commits: []
pr: ""
---

# Implement DAG Topological Sort & Layer Assignment

## Objective
Implement the core graph layout algorithm: topological sort of the dependency DAG and assignment of nodes to layers (depth levels) for rendering.

## Scope
- Create `src/tui/graph.rs` with `DagLayout`, `Node`, and `Edge` structs
- Topological sort using Kahn's algorithm or DFS-based approach
- Layer assignment based on longest path from root nodes
- Handle orphan nodes and cycle detection

## Acceptance Criteria
- [ ] Topological sort produces correct ordering
- [ ] Layer assignment places roots at top, leaves at bottom
- [ ] Orphan nodes handled separately
- [ ] Cycle detection doesn't crash (logs warning)
- [ ] Unit tests with various graph topologies

## Technical Context
### Relevant Spec Sections
- PRD.md — Dependency graph layout algorithm

### Related Files/Directories
- `src/tui/graph.rs` — DAG layout logic

### Dependencies on Other Systems
- Dependency data from database layer

## Implementation Guidance
### Approach
Create `DagLayout` struct holding nodes, edges, layers. Create `Node` struct: id, label, status, layer, x_position. Create `Edge` struct: from_node, to_node. Algorithm: (1) Build adjacency list from dependencies, (2) Compute topological ordering (Kahn's algorithm or DFS-based), (3) Assign layers based on longest path from root nodes (sources), (4) Root nodes at layer 0, dependents at layer 1+. Handle orphan nodes (no edges) in a special "orphan" layer. Detect and handle cycles gracefully.

### Considerations
- Cycle detection is defensive — proper dependencies shouldn't have cycles
- Orphan nodes need special handling to not disrupt layout

### Anti-patterns to Avoid
- Do not crash on cycles — log a warning and break the cycle

## Testing Requirements

### Unit Tests
- [ ] Linear chain graph: A → B → C
- [ ] Diamond graph: A → B, A → C, B → D, C → D
- [ ] Fan-out graph: A → B, A → C, A → D
- [ ] Orphan nodes mixed with connected nodes
- [ ] Cycle detection

### Integration Tests
- [ ] Layout with real dependency data from database

### Manual Tests
- TBD

## Notes
Blocks: TK-0301, TK-0302, TK-0305
