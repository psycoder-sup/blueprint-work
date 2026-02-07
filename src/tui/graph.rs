use std::collections::{BTreeSet, HashMap, VecDeque};

use crate::models::ItemStatus;

/// A node in the dependency DAG.
#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub status: ItemStatus,
    pub layer: Option<usize>,
    pub x_position: usize,
}

/// A directed edge: `from` (blocker) → `to` (blocked).
#[derive(Debug, Clone)]
pub struct Edge {
    pub from: String,
    pub to: String,
}

/// DAG layout computed via Kahn's topological sort and longest-path layer assignment.
#[derive(Debug)]
pub struct DagLayout {
    pub nodes: HashMap<String, Node>,
    pub edges: Vec<Edge>,
    pub layers: Vec<Vec<String>>,
    pub orphans: Vec<String>,
    pub cycle_nodes: Vec<String>,
}

impl DagLayout {
    /// Build a new layout from a set of nodes and edges.
    ///
    /// Edges referencing unknown node IDs are silently filtered out.
    /// Orphan nodes (not referenced by any valid edge) are placed in `orphans`
    /// with `layer = None`. Cycle participants are detected and placed in a
    /// fallback layer at the end.
    pub fn new(nodes: Vec<Node>, edges: Vec<Edge>) -> Self {
        let mut node_map: HashMap<String, Node> = HashMap::new();
        for node in nodes {
            node_map.insert(node.id.clone(), node);
        }

        // Filter edges to only those whose endpoints exist.
        let valid_edges: Vec<Edge> = edges
            .into_iter()
            .filter(|e| node_map.contains_key(&e.from) && node_map.contains_key(&e.to))
            .collect();

        // Build adjacency: children[from] = set of `to` IDs, parents[to] = set of `from` IDs.
        let mut children: HashMap<String, BTreeSet<String>> = HashMap::new();
        let mut parents: HashMap<String, BTreeSet<String>> = HashMap::new();
        let mut connected: BTreeSet<String> = BTreeSet::new();

        for edge in &valid_edges {
            children
                .entry(edge.from.clone())
                .or_default()
                .insert(edge.to.clone());
            parents
                .entry(edge.to.clone())
                .or_default()
                .insert(edge.from.clone());
            connected.insert(edge.from.clone());
            connected.insert(edge.to.clone());
        }

        // Identify orphans: nodes not in any valid edge.
        let mut orphans: Vec<String> = node_map
            .keys()
            .filter(|id| !connected.contains(*id))
            .cloned()
            .collect();
        orphans.sort();

        // Mark orphan nodes with layer = None.
        for id in &orphans {
            if let Some(node) = node_map.get_mut(id) {
                node.layer = None;
            }
        }

        // Kahn's topological sort on connected nodes.
        let topo_order = Self::topological_sort(&connected, &children, &parents);

        // Detect cycle participants: connected nodes not in topo_order.
        let topo_set: BTreeSet<&String> = topo_order.iter().collect();
        let mut cycle_nodes: Vec<String> = connected
            .iter()
            .filter(|id| !topo_set.contains(id))
            .cloned()
            .collect();
        cycle_nodes.sort();

        if !cycle_nodes.is_empty() {
            eprintln!(
                "Warning: dependency cycle detected among nodes: {}",
                cycle_nodes.join(", ")
            );
        }

        // Assign layers via longest path.
        let layers = Self::assign_layers(&topo_order, &parents, &cycle_nodes, &mut node_map);

        let mut layout = DagLayout {
            nodes: node_map,
            edges: valid_edges,
            layers,
            orphans,
            cycle_nodes,
        };

        layout.minimize_crossings();
        layout
    }

    /// Kahn's algorithm (BFS-based topological sort).
    ///
    /// Returns nodes in topological order. Any connected node NOT in the result
    /// is a cycle participant.
    fn topological_sort(
        connected: &BTreeSet<String>,
        children: &HashMap<String, BTreeSet<String>>,
        parents: &HashMap<String, BTreeSet<String>>,
    ) -> Vec<String> {
        // Compute in-degrees for connected nodes.
        let mut in_degree: HashMap<&String, usize> = HashMap::new();
        for id in connected {
            let deg = parents.get(id).map_or(0, |p| p.len());
            in_degree.insert(id, deg);
        }

        // Seed queue with zero in-degree nodes, sorted for determinism.
        let mut queue: VecDeque<&String> = in_degree
            .iter()
            .filter(|&(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();

        let mut order: Vec<String> = Vec::new();

        while let Some(node) = queue.pop_front() {
            order.push(node.clone());

            if let Some(kids) = children.get(node) {
                // Process children in sorted order for determinism.
                for child in kids {
                    if let Some(deg) = in_degree.get_mut(child) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(child);
                        }
                    }
                }
            }
        }

        order
    }

    /// Assign layers via longest-path from roots.
    ///
    /// Processes nodes in topological order. Each node's layer is
    /// `max(parent_layers) + 1`, with roots at layer 0.
    /// Cycle nodes are placed in a fallback layer at the end.
    fn assign_layers(
        topo_order: &[String],
        parents: &HashMap<String, BTreeSet<String>>,
        cycle_nodes: &[String],
        node_map: &mut HashMap<String, Node>,
    ) -> Vec<Vec<String>> {
        let mut layer_map: HashMap<String, usize> = HashMap::new();

        for id in topo_order {
            let layer = match parents.get(id) {
                Some(pars) => {
                    pars.iter()
                        .filter_map(|p| layer_map.get(p))
                        .max()
                        .map_or(0, |max_parent| max_parent + 1)
                }
                None => 0,
            };
            layer_map.insert(id.clone(), layer);
            if let Some(node) = node_map.get_mut(id) {
                node.layer = Some(layer);
            }
        }

        // Group nodes into layers.
        let max_layer = layer_map.values().copied().max().unwrap_or(0);
        let mut layers: Vec<Vec<String>> = (0..=max_layer)
            .map(|l| {
                let mut ids: Vec<String> = layer_map
                    .iter()
                    .filter(|&(_, &layer)| layer == l)
                    .map(|(id, _)| id.clone())
                    .collect();
                ids.sort();
                ids
            })
            .collect();

        // Remove trailing empty layers (shouldn't happen, but defensive).
        while layers.last().is_some_and(|l| l.is_empty()) {
            layers.pop();
        }

        // Cycle nodes go into a fallback layer at the end.
        if !cycle_nodes.is_empty() {
            let fallback_layer = layers.len();
            let mut sorted_cycle = cycle_nodes.to_vec();
            sorted_cycle.sort();
            for id in &sorted_cycle {
                if let Some(node) = node_map.get_mut(id) {
                    node.layer = Some(fallback_layer);
                }
            }
            layers.push(sorted_cycle);
        }

        layers
    }

    /// Minimize edge crossings using the barycenter heuristic.
    ///
    /// Performs multiple forward (top-to-bottom) and backward (bottom-to-top)
    /// passes. In each pass, nodes within a layer are sorted by the average
    /// x-position (barycenter) of their connected nodes in the adjacent layer.
    /// Nodes with no connections to the adjacent layer retain their current
    /// position. After reordering, each node's `x_position` is updated.
    fn minimize_crossings(&mut self) {
        if self.layers.len() < 2 {
            self.assign_x_positions();
            return;
        }

        // Build parent and child adjacency maps from edges.
        let mut parents_map: HashMap<String, Vec<String>> = HashMap::new();
        let mut children_map: HashMap<String, Vec<String>> = HashMap::new();
        for edge in &self.edges {
            children_map
                .entry(edge.from.clone())
                .or_default()
                .push(edge.to.clone());
            parents_map
                .entry(edge.to.clone())
                .or_default()
                .push(edge.from.clone());
        }

        // Run 3 iterations of forward + backward passes.
        for _ in 0..3 {
            // Forward pass: top-to-bottom (layer 1, 2, ..., n-1).
            for layer_idx in 1..self.layers.len() {
                self.sort_layer_by_barycenter(layer_idx, layer_idx - 1, &parents_map);
            }

            // Backward pass: bottom-to-top (layer n-2, n-3, ..., 0).
            for layer_idx in (0..self.layers.len() - 1).rev() {
                self.sort_layer_by_barycenter(layer_idx, layer_idx + 1, &children_map);
            }
        }

        self.assign_x_positions();
    }

    /// Sort a layer's nodes by the barycenter of their neighbors in the
    /// reference layer.
    fn sort_layer_by_barycenter(
        &mut self,
        target_layer: usize,
        ref_layer: usize,
        adjacency: &HashMap<String, Vec<String>>,
    ) {
        // Build a position lookup for the reference layer.
        let ref_positions: HashMap<&str, usize> = self.layers[ref_layer]
            .iter()
            .enumerate()
            .map(|(pos, id)| (id.as_str(), pos))
            .collect();

        // Compute barycenters for each node in the target layer.
        let mut barycenters: Vec<(String, f64)> = self.layers[target_layer]
            .iter()
            .enumerate()
            .map(|(original_pos, node_id)| {
                let bc = Self::compute_barycenter(node_id, adjacency, &ref_positions);
                // Nodes with no connections keep their current position as barycenter.
                let effective_bc = bc.unwrap_or(original_pos as f64);
                (node_id.clone(), effective_bc)
            })
            .collect();

        // Stable sort by barycenter to preserve relative order for equal values.
        barycenters.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        // Update the layer with the new ordering.
        self.layers[target_layer] = barycenters.into_iter().map(|(id, _)| id).collect();
    }

    /// Compute the barycenter (average position) of a node's neighbors in the
    /// reference layer. Returns `None` if the node has no neighbors in that layer.
    fn compute_barycenter(
        node_id: &str,
        adjacency: &HashMap<String, Vec<String>>,
        ref_positions: &HashMap<&str, usize>,
    ) -> Option<f64> {
        let neighbors = adjacency.get(node_id)?;
        let positions: Vec<f64> = neighbors
            .iter()
            .filter_map(|n| ref_positions.get(n.as_str()))
            .map(|&pos| pos as f64)
            .collect();

        if positions.is_empty() {
            return None;
        }

        Some(positions.iter().sum::<f64>() / positions.len() as f64)
    }

    /// Assign `x_position` to each node based on its index within its layer.
    fn assign_x_positions(&mut self) {
        for layer in &self.layers {
            for (pos, node_id) in layer.iter().enumerate() {
                if let Some(node) = self.nodes.get_mut(node_id) {
                    node.x_position = pos;
                }
            }
        }
    }

    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn has_cycles(&self) -> bool {
        !self.cycle_nodes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str) -> Node {
        Node {
            id: id.to_string(),
            label: id.to_string(),
            status: ItemStatus::Todo,
            layer: None,
            x_position: 0,
        }
    }

    fn edge(from: &str, to: &str) -> Edge {
        Edge {
            from: from.to_string(),
            to: to.to_string(),
        }
    }

    #[test]
    fn linear_chain() {
        // A → B → C → 3 layers
        let layout = DagLayout::new(
            vec![node("A"), node("B"), node("C")],
            vec![edge("A", "B"), edge("B", "C")],
        );

        assert_eq!(layout.layer_count(), 3);
        assert_eq!(layout.layers[0], vec!["A"]);
        assert_eq!(layout.layers[1], vec!["B"]);
        assert_eq!(layout.layers[2], vec!["C"]);
        assert!(layout.orphans.is_empty());
        assert!(!layout.has_cycles());
    }

    #[test]
    fn diamond() {
        // A → B, A → C, B → D, C → D
        // Longest path: A(0) → B(1) → D(2), A(0) → C(1) → D(2)
        let layout = DagLayout::new(
            vec![node("A"), node("B"), node("C"), node("D")],
            vec![
                edge("A", "B"),
                edge("A", "C"),
                edge("B", "D"),
                edge("C", "D"),
            ],
        );

        assert_eq!(layout.layer_count(), 3);
        assert_eq!(layout.layers[0], vec!["A"]);
        assert_eq!(layout.layers[1], vec!["B", "C"]);
        assert_eq!(layout.layers[2], vec!["D"]);
        assert!(!layout.has_cycles());
    }

    #[test]
    fn fan_out() {
        // A → B, A → C, A → D
        let layout = DagLayout::new(
            vec![node("A"), node("B"), node("C"), node("D")],
            vec![edge("A", "B"), edge("A", "C"), edge("A", "D")],
        );

        assert_eq!(layout.layer_count(), 2);
        assert_eq!(layout.layers[0], vec!["A"]);
        assert_eq!(layout.layers[1], vec!["B", "C", "D"]);
    }

    #[test]
    fn orphans_mixed_with_connected() {
        let layout = DagLayout::new(
            vec![node("A"), node("B"), node("X"), node("Y")],
            vec![edge("A", "B")],
        );

        assert_eq!(layout.layer_count(), 2);
        assert_eq!(layout.layers[0], vec!["A"]);
        assert_eq!(layout.layers[1], vec!["B"]);
        assert_eq!(layout.orphans, vec!["X", "Y"]);
        assert!(layout.nodes["X"].layer.is_none());
        assert!(layout.nodes["Y"].layer.is_none());
    }

    #[test]
    fn all_orphans() {
        let layout = DagLayout::new(vec![node("A"), node("B"), node("C")], vec![]);

        assert_eq!(layout.layer_count(), 0);
        assert_eq!(layout.orphans, vec!["A", "B", "C"]);
        assert_eq!(layout.edge_count(), 0);
    }

    #[test]
    fn empty_graph() {
        let layout = DagLayout::new(vec![], vec![]);

        assert_eq!(layout.layer_count(), 0);
        assert_eq!(layout.node_count(), 0);
        assert_eq!(layout.edge_count(), 0);
        assert!(layout.orphans.is_empty());
        assert!(!layout.has_cycles());
    }

    #[test]
    fn single_node() {
        let layout = DagLayout::new(vec![node("A")], vec![]);

        assert_eq!(layout.node_count(), 1);
        assert_eq!(layout.orphans, vec!["A"]);
        assert_eq!(layout.layer_count(), 0);
    }

    #[test]
    fn cycle_detection_full() {
        // A → B → C → A (full cycle)
        let layout = DagLayout::new(
            vec![node("A"), node("B"), node("C")],
            vec![edge("A", "B"), edge("B", "C"), edge("C", "A")],
        );

        assert!(layout.has_cycles());
        assert_eq!(layout.cycle_nodes, vec!["A", "B", "C"]);
        // Cycle nodes placed in a fallback layer.
        assert_eq!(layout.layer_count(), 1);
        assert_eq!(layout.layers[0], vec!["A", "B", "C"]);
    }

    #[test]
    fn cycle_detection_partial() {
        // X → A, B → C → B (partial cycle, X and A are fine)
        let layout = DagLayout::new(
            vec![node("A"), node("B"), node("C"), node("X")],
            vec![edge("X", "A"), edge("B", "C"), edge("C", "B")],
        );

        assert!(layout.has_cycles());
        assert_eq!(layout.cycle_nodes, vec!["B", "C"]);
        // X(0) → A(1) are normal layers; B,C in fallback layer 2.
        assert_eq!(layout.layer_count(), 3);
        assert_eq!(layout.layers[0], vec!["X"]);
        assert_eq!(layout.layers[1], vec!["A"]);
        assert_eq!(layout.layers[2], vec!["B", "C"]);
    }

    #[test]
    fn longest_path_wins() {
        // A → D (direct, short path = layer 1)
        // A → B → C → D (long path = layer 3)
        // Longest path should win: D at layer 3.
        let layout = DagLayout::new(
            vec![node("A"), node("B"), node("C"), node("D")],
            vec![
                edge("A", "B"),
                edge("B", "C"),
                edge("C", "D"),
                edge("A", "D"),
            ],
        );

        assert_eq!(layout.layer_count(), 4);
        assert_eq!(layout.nodes["A"].layer, Some(0));
        assert_eq!(layout.nodes["B"].layer, Some(1));
        assert_eq!(layout.nodes["C"].layer, Some(2));
        assert_eq!(layout.nodes["D"].layer, Some(3));
    }

    #[test]
    fn edges_with_unknown_nodes_filtered() {
        let layout = DagLayout::new(
            vec![node("A"), node("B")],
            vec![
                edge("A", "B"),
                edge("A", "GHOST"),
                edge("PHANTOM", "B"),
            ],
        );

        assert_eq!(layout.edge_count(), 1);
        assert_eq!(layout.layer_count(), 2);
        assert_eq!(layout.layers[0], vec!["A"]);
        assert_eq!(layout.layers[1], vec!["B"]);
    }

    #[test]
    fn deterministic_ordering_within_layers() {
        // Multiple roots and children — ordering determined by barycenter.
        let layout = DagLayout::new(
            vec![node("C"), node("A"), node("B"), node("D"), node("E"), node("F")],
            vec![
                edge("C", "F"),
                edge("A", "D"),
                edge("B", "E"),
                edge("A", "E"),
            ],
        );

        // Roots: A, B, C (all have in-degree 0). Layer 1: D, E, F.
        assert_eq!(layout.layer_count(), 2);
        assert_eq!(layout.layers[0], vec!["A", "B", "C"]);
        assert_eq!(layout.layers[1], vec!["D", "E", "F"]);
    }

    // ==================== Edge-crossing minimization tests ====================

    /// Count the number of edge crossings between two adjacent layers.
    fn count_crossings(layout: &DagLayout) -> usize {
        let mut total = 0;

        for layer_idx in 0..layout.layers.len().saturating_sub(1) {
            let upper = &layout.layers[layer_idx];
            let lower = &layout.layers[layer_idx + 1];

            let upper_pos: HashMap<&str, usize> = upper
                .iter()
                .enumerate()
                .map(|(i, id)| (id.as_str(), i))
                .collect();
            let lower_pos: HashMap<&str, usize> = lower
                .iter()
                .enumerate()
                .map(|(i, id)| (id.as_str(), i))
                .collect();

            let mut edge_pairs: Vec<(usize, usize)> = Vec::new();
            for e in &layout.edges {
                if let (Some(&up), Some(&lp)) =
                    (upper_pos.get(e.from.as_str()), lower_pos.get(e.to.as_str()))
                {
                    edge_pairs.push((up, lp));
                }
            }

            for i in 0..edge_pairs.len() {
                for j in (i + 1)..edge_pairs.len() {
                    let (a, b) = edge_pairs[i];
                    let (c, d) = edge_pairs[j];
                    if (a < c && b > d) || (a > c && b < d) {
                        total += 1;
                    }
                }
            }
        }

        total
    }

    #[test]
    fn barycenter_reduces_crossings() {
        // Layer 0: [A, B], Layer 1: [C, D]
        // Edges: A->D, B->C
        // Alphabetical order in layer 1 = [C, D] causes 1 crossing.
        // Barycenter: C parent B (pos 1), D parent A (pos 0).
        // After sort: [D, C] -> 0 crossings.
        let layout = DagLayout::new(
            vec![node("A"), node("B"), node("C"), node("D")],
            vec![edge("A", "D"), edge("B", "C")],
        );

        assert_eq!(layout.layer_count(), 2);
        assert_eq!(layout.layers[0], vec!["A", "B"]);
        assert_eq!(layout.layers[1], vec!["D", "C"]);
        assert_eq!(count_crossings(&layout), 0);
    }

    #[test]
    fn barycenter_diamond_no_crossings() {
        let layout = DagLayout::new(
            vec![node("A"), node("B"), node("C"), node("D")],
            vec![
                edge("A", "B"),
                edge("A", "C"),
                edge("B", "D"),
                edge("C", "D"),
            ],
        );

        assert_eq!(count_crossings(&layout), 0);
    }

    #[test]
    fn barycenter_fan_out_no_crossings() {
        let layout = DagLayout::new(
            vec![node("A"), node("B"), node("C"), node("D")],
            vec![edge("A", "B"), edge("A", "C"), edge("A", "D")],
        );

        assert_eq!(count_crossings(&layout), 0);
    }

    #[test]
    fn barycenter_wide_graph_crossing_reduction() {
        // Layer 0: [A, B, C], Layer 1: [D, E, F]
        // Edges: A->F, B->E, C->D — maximum crossings alphabetically (3).
        // After barycenter: [F, E, D] -> 0 crossings.
        let layout = DagLayout::new(
            vec![
                node("A"), node("B"), node("C"),
                node("D"), node("E"), node("F"),
            ],
            vec![edge("A", "F"), edge("B", "E"), edge("C", "D")],
        );

        assert_eq!(layout.layer_count(), 2);
        assert_eq!(layout.layers[0], vec!["A", "B", "C"]);
        assert_eq!(layout.layers[1], vec!["F", "E", "D"]);
        assert_eq!(count_crossings(&layout), 0);
    }

    #[test]
    fn x_positions_assigned_after_minimization() {
        let layout = DagLayout::new(
            vec![node("A"), node("B"), node("C"), node("D")],
            vec![edge("A", "D"), edge("B", "C")],
        );

        assert_eq!(layout.nodes["A"].x_position, 0);
        assert_eq!(layout.nodes["B"].x_position, 1);
        assert_eq!(layout.nodes["D"].x_position, 0);
        assert_eq!(layout.nodes["C"].x_position, 1);
    }

    #[test]
    fn crossing_count_comparison() {
        // P->U, Q->T, R->S — maximum crossings alphabetically (3).
        // After barycenter: [U, T, S] -> 0 crossings.
        let layout = DagLayout::new(
            vec![
                node("P"), node("Q"), node("R"),
                node("S"), node("T"), node("U"),
            ],
            vec![edge("P", "U"), edge("Q", "T"), edge("R", "S")],
        );

        assert_eq!(count_crossings(&layout), 0);
        assert_eq!(layout.layers[1], vec!["U", "T", "S"]);
    }

    #[test]
    fn multi_layer_crossing_reduction() {
        // Three layers: A->D, B->C, C->F, D->E
        let layout = DagLayout::new(
            vec![
                node("A"), node("B"), node("C"),
                node("D"), node("E"), node("F"),
            ],
            vec![
                edge("A", "D"),
                edge("B", "C"),
                edge("C", "F"),
                edge("D", "E"),
            ],
        );

        assert_eq!(layout.layer_count(), 3);
        assert_eq!(count_crossings(&layout), 0);
    }

    #[test]
    fn no_connection_nodes_retain_position() {
        let layout = DagLayout::new(
            vec![
                node("A"), node("B"),
                node("C"), node("D"), node("E"),
            ],
            vec![
                edge("A", "C"),
                edge("A", "D"),
                edge("A", "E"),
                edge("B", "D"),
            ],
        );

        for layer in &layout.layers {
            for (pos, node_id) in layer.iter().enumerate() {
                assert_eq!(layout.nodes[node_id].x_position, pos);
            }
        }
    }

    #[test]
    fn backward_pass_improves_upper_layers() {
        let layout = DagLayout::new(
            vec![
                node("A"), node("B"), node("C"),
                node("D"),
                node("E"), node("F"), node("G"),
            ],
            vec![
                edge("A", "D"),
                edge("B", "D"),
                edge("C", "D"),
                edge("D", "E"),
                edge("D", "F"),
                edge("D", "G"),
            ],
        );

        assert_eq!(layout.layer_count(), 3);
        assert_eq!(count_crossings(&layout), 0);
    }
}
