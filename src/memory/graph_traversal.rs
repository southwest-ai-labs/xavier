//! Memory graph traversal algorithms
//!
//! Provides the implementation and data structures for this module's
//! responsibilities within the Xavier cognitive memory system.
use crate::domain::cycle_breaks::w30_09::NavigationPolicy;
use crate::domain::memory::belief::BeliefEdge;
use crate::memory::belief_graph::BeliefGraph;
use serde::{Deserialize, Serialize};
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};

/// Represents a node affected by a change, including the path taken to reach it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectedNode {
    pub node: String,
    pub relation: String,
    pub depth: usize,
}

/// A utility for traversing the belief graph using various algorithms.
pub struct Pathfinder<'a> {
    _graph: &'a BeliefGraph,
    policy: Option<NavigationPolicy>,
    adjacency_map: HashMap<String, Vec<BeliefEdge>>,
    degrees: HashMap<String, usize>,
}

/// A wrapper for a belief edge with its associated policy-based score.
#[derive(Clone)]
pub struct ScoredEdge {
    pub edge: BeliefEdge,
    pub policy_score: f32,
}

impl<'a> Pathfinder<'a> {
    /// Creates a new Pathfinder instance.
    pub fn new(graph: &'a BeliefGraph) -> Self {
        let relations = graph.get_relations();
        let mut adjacency_map: HashMap<String, Vec<BeliefEdge>> = HashMap::new();
        let mut degrees: HashMap<String, usize> = HashMap::new();

        for relation in relations {
            adjacency_map
                .entry(relation.source.clone())
                .or_default()
                .push(relation.clone());

            // Increment degree for both source and target (undirected for hub detection)
            *degrees.entry(relation.source.clone()).or_default() += 1;
            *degrees.entry(relation.target.clone()).or_default() += 1;
        }

        Self {
            _graph: graph,
            policy: None,
            adjacency_map,
            degrees,
        }
    }

    /// Creates a new Pathfinder instance with a specific navigation policy.
    pub fn with_policy(graph: &'a BeliefGraph, policy: NavigationPolicy) -> Self {
        let mut pathfinder = Self::new(graph);
        pathfinder.policy = Some(policy);
        pathfinder
    }

    /// Finds the shortest path between two concepts using BFS.
    /// Returns a list of relations forming the path.
    pub fn shortest_path(&self, start: &str, end: &str) -> Vec<BeliefEdge> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        queue.push_back((start.to_string(), Vec::new()));
        visited.insert(start.to_string());

        while let Some((current, path)) = queue.pop_front() {
            if current == end {
                return path;
            }

            if let Some(relations) = self.adjacency_map.get(&current) {
                for relation in relations {
                    if !visited.contains(&relation.target) {
                        visited.insert(relation.target.clone());
                        let mut new_path = path.clone();
                        new_path.push(relation.clone());
                        queue.push_back((relation.target.clone(), new_path));
                    }
                }
            }
        }

        Vec::new()
    }

    /// Performs a k-hop expansion from a start concept.
    /// Returns all relations within k hops.
    pub fn k_hop_expansion(&self, start: &str, k: usize) -> Vec<BeliefEdge> {
        let mut result = Vec::new();
        let mut visited_nodes = HashSet::new();
        let mut visited_relations = HashSet::new();
        let mut current_layer = HashSet::new();

        current_layer.insert(start.to_string());
        visited_nodes.insert(start.to_string());

        for _ in 0..k {
            let mut next_layer = HashSet::new();
            for current in current_layer {
                if let Some(relations) = self.adjacency_map.get(&current) {
                    for relation in relations {
                        if visited_relations.insert(relation.id.clone()) {
                            result.push(relation.clone());
                        }
                        if visited_nodes.insert(relation.target.clone()) {
                            next_layer.insert(relation.target.clone());
                        }
                    }
                }
            }
            if next_layer.is_empty() {
                break;
            }
            current_layer = next_layer;
        }

        result
    }

    /// Finds all possible paths from start to end up to max_depth.
    pub fn all_paths(&self, start: &str, end: &str, max_depth: usize) -> Vec<Vec<BeliefEdge>> {
        let mut results = Vec::new();
        self.find_all_paths_recursive(start, end, max_depth, Vec::new(), &mut results);
        results
    }

    fn find_all_paths_recursive(
        &self,
        current: &str,
        end: &str,
        depth_left: usize,
        current_path: Vec<BeliefEdge>,
        results: &mut Vec<Vec<BeliefEdge>>,
    ) {
        if current == end {
            if !current_path.is_empty() {
                results.push(current_path);
            }
            return;
        }

        if depth_left == 0 {
            return;
        }

        if let Some(relations) = self.adjacency_map.get(current) {
            for relation in relations {
                if !current_path.iter().any(|r| r.target == relation.target) {
                    let mut next_path = current_path.clone();
                    next_path.push(relation.clone());
                    self.find_all_paths_recursive(
                        &relation.target,
                        end,
                        depth_left - 1,
                        next_path,
                        results,
                    );
                }
            }
        }
    }

    /// Performs a guided search from a start concept using the navigation policy.
    /// Uses a priority queue to explore paths with higher cumulative scores first.
    pub fn guided_search(&self, start: &str, query: &str, max_depth: usize) -> Vec<ScoredEdge> {
        let mut result = Vec::new();
        let mut visited_nodes = HashSet::new();
        let mut visited_relations = HashSet::new();
        let mut priority_queue = BinaryHeap::new();

        let now = chrono::Utc::now();
        let policy = self
            .policy
            .as_ref()
            .cloned()
            .unwrap_or_else(NavigationPolicy::default);

        #[derive(PartialEq)]
        struct NodeState {
            score: f32,
            concept: String,
            depth: usize,
        }
        impl Eq for NodeState {}
        impl PartialOrd for NodeState {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }
        impl Ord for NodeState {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.score
                    .partial_cmp(&other.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }
        }

        priority_queue.push(NodeState {
            score: 1.0,
            concept: start.to_string(),
            depth: 0,
        });
        visited_nodes.insert(start.to_string());

        while let Some(NodeState {
            score: current_score,
            concept: current,
            depth,
        }) = priority_queue.pop()
        {
            if depth >= max_depth {
                continue;
            }

            if let Some(relations) = self.adjacency_map.get(&current) {
                let source_degree = *self.degrees.get(&current).unwrap_or(&0);

                for relation in relations {
                    if !visited_relations.contains(&relation.id) {
                        visited_relations.insert(relation.id.clone());
                        let target_degree = *self.degrees.get(&relation.target).unwrap_or(&0);

                        let transition_score = policy.score_transition(
                            query,
                            relation,
                            now,
                            source_degree,
                            target_degree,
                        );
                        let combined_score = current_score * transition_score;

                        // Threshold to prune low-relevance paths
                        if combined_score > 0.1 && !visited_nodes.contains(&relation.target) {
                            visited_nodes.insert(relation.target.clone());
                            result.push(ScoredEdge {
                                edge: relation.clone(),
                                policy_score: combined_score,
                            });
                            priority_queue.push(NodeState {
                                score: combined_score,
                                concept: relation.target.clone(),
                                depth: depth + 1,
                            });
                        }
                    }
                }
            }
        }

        // Sort final results by policy score descending
        result.sort_by(|a, b| {
            b.policy_score
                .partial_cmp(&a.policy_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        result
    }

    /// Performs a depth-limited BFS to find nodes affected by a change at `start_node`.
    pub fn affected_bfs(&self, start_node: &str, max_depth: usize) -> Vec<AffectedNode> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        queue.push_back((start_node.to_string(), "root".to_string(), 0));
        visited.insert(start_node.to_string());

        while let Some((current, relation_type, depth)) = queue.pop_front() {
            if depth > 0 {
                result.push(AffectedNode {
                    node: current.clone(),
                    relation: relation_type,
                    depth,
                });
            }

            if depth < max_depth {
                if let Some(relations) = self.adjacency_map.get(&current) {
                    for relation in relations {
                        if !visited.contains(&relation.target) {
                            visited.insert(relation.target.clone());
                            queue.push_back((
                                relation.target.clone(),
                                relation.relation_type.clone(),
                                depth + 1,
                            ));
                        }
                    }
                }
            }
        }

        result
    }

    /// Computes Personalized PageRank (PPR) with restart vector u focused on seed_nodes.
    /// Follows the power iteration: p_{k+1} = (1 - d) u + d * M * p_k
    /// where d is the damping factor (typically 0.85).
    pub fn personalized_pagerank(
        &self,
        seed_nodes: &[String],
        damping_factor: f32,
        max_iterations: usize,
        tolerance: f32,
    ) -> HashMap<String, f32> {
        let mut ranks: HashMap<String, f32> = HashMap::new();
        // Match seeds against adjacency map and degrees (handling lowercase normalization)
        let valid_seeds: Vec<String> = seed_nodes
            .iter()
            .map(|s| s.to_lowercase())
            .filter(|s| self.adjacency_map.contains_key(s) || self.degrees.contains_key(s))
            .collect();

        if valid_seeds.is_empty() {
            return ranks;
        }

        // Initialize uniform restart vector u over valid seed nodes
        let seed_prob = 1.0 / (valid_seeds.len() as f32);
        let mut restart_vector: HashMap<String, f32> = HashMap::new();
        for seed in &valid_seeds {
            restart_vector.insert(seed.clone(), seed_prob);
            ranks.insert(seed.clone(), seed_prob);
        }

        let d = damping_factor.clamp(0.01, 0.99);

        for _ in 0..max_iterations {
            let mut next_ranks: HashMap<String, f32> = HashMap::new();

            // Distribute probability through outgoing edges
            for (node, &prob) in &ranks {
                if prob <= 0.0 {
                    continue;
                }
                if let Some(edges) = self.adjacency_map.get(node) {
                    if !edges.is_empty() {
                        let out_share = (prob * d) / (edges.len() as f32);
                        for edge in edges {
                            *next_ranks.entry(edge.target.clone()).or_insert(0.0) += out_share;
                        }
                    } else {
                        // Dangling node: redirect to restart vector
                        for (seed, &u_prob) in &restart_vector {
                            *next_ranks.entry(seed.clone()).or_insert(0.0) += prob * d * u_prob;
                        }
                    }
                } else {
                    for (seed, &u_prob) in &restart_vector {
                        *next_ranks.entry(seed.clone()).or_insert(0.0) += prob * d * u_prob;
                    }
                }
            }

            // Add restart teleportation component: (1 - d) * u
            for (seed, &u_prob) in &restart_vector {
                *next_ranks.entry(seed.clone()).or_insert(0.0) += (1.0 - d) * u_prob;
            }

            // Check convergence
            let mut max_diff: f32 = 0.0;
            let all_keys: HashSet<String> =
                ranks.keys().chain(next_ranks.keys()).cloned().collect();
            for key in all_keys {
                let p_old = ranks.get(&key).copied().unwrap_or(0.0);
                let p_new = next_ranks.get(&key).copied().unwrap_or(0.0);
                max_diff = max_diff.max((p_new - p_old).abs());
            }

            ranks = next_ranks;

            if max_diff < tolerance {
                break;
            }
        }

        ranks
    }

    pub fn associate_and_rerank(
        &self,
        initial_candidates: &[(String, f32)],
        alpha: f32,
        damping_factor: f32,
        max_iterations: usize,
        tolerance: f32,
    ) -> Vec<(String, f32)> {
        if initial_candidates.is_empty() {
            return Vec::new();
        }

        let mut seeds: Vec<String> = initial_candidates
            .iter()
            .filter(|(_, s)| *s > 0.0)
            .map(|(id, _)| id.clone())
            .collect();

        if seeds.is_empty() {
            seeds = initial_candidates
                .iter()
                .map(|(id, _)| id.clone())
                .collect();
        }

        let ppr_scores =
            self.personalized_pagerank(&seeds, damping_factor, max_iterations, tolerance);
        let max_ppr = ppr_scores.values().fold(0.0f32, |acc, &val| acc.max(val));

        let a = alpha.clamp(0.0, 1.0);
        let mut results: Vec<(String, f32)> = Vec::new();

        // Track candidates added
        let mut seen = HashSet::new();

        for (id, direct_score) in initial_candidates {
            let ppr_raw = ppr_scores.get(&id.to_lowercase()).copied().unwrap_or(0.0);
            let ppr_norm = if max_ppr > 0.0 {
                ppr_raw / max_ppr
            } else {
                0.0
            };
            let composite = a * direct_score + (1.0 - a) * ppr_norm;
            results.push((id.clone(), composite));
            seen.insert(id.to_lowercase());
        }

        // Associative Expansion (HippoRAG discovery of unindexed bridge nodes)
        for (node_id, &ppr_raw) in &ppr_scores {
            if !seen.contains(&node_id.to_lowercase()) {
                let ppr_norm = if max_ppr > 0.0 {
                    ppr_raw / max_ppr
                } else {
                    0.0
                };
                if ppr_norm >= 0.1 && a < 1.0 {
                    let composite = (1.0 - a) * ppr_norm;
                    results.push((node_id.clone(), composite));
                }
            }
        }

        results.sort_by(|a_val, b_val| {
            b_val
                .1
                .partial_cmp(&a_val.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a_val.0.cmp(&b_val.0))
        });

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_associate_and_rerank_empty_candidates() {
        let graph = BeliefGraph::new();
        let pathfinder = Pathfinder::new(&graph);
        let results = pathfinder.associate_and_rerank(&[], 0.5, 0.85, 100, 1e-4);
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_associate_and_rerank_combines_vector_and_graph() {
        let graph = BeliefGraph::new();
        graph.add_node("a".to_string(), 1.0, None);
        graph.add_node("b".to_string(), 1.0, None);
        graph.add_node("c".to_string(), 1.0, None);

        graph
            .add_relation(
                "a".to_string(),
                "b".to_string(),
                "relates".to_string(),
                None,
                None,
            )
            .await
            .unwrap();
        graph
            .add_relation(
                "b".to_string(),
                "c".to_string(),
                "relates".to_string(),
                None,
                None,
            )
            .await
            .unwrap();

        let pathfinder = Pathfinder::new(&graph);

        let initial_candidates = vec![("a".to_string(), 0.9), ("c".to_string(), 0.2)];

        let results = pathfinder.associate_and_rerank(&initial_candidates, 0.5, 0.85, 100, 1e-4);

        assert_eq!(results.len(), 3); // a, c, and discovered b
        assert_eq!(results[0].0, "a");
        // We know 'b' should get a decent PPR score due to 'a' being the seed.
        // It might even surpass 'c' in total score.
        let has_b = results.iter().any(|(id, _)| id == "b");
        assert!(has_b, "node b should be discovered");
    }

    #[tokio::test]
    async fn test_associate_and_rerank_discovers_multihop_neighbors() {
        let graph = BeliefGraph::new();
        graph.add_node("seed_node".to_string(), 1.0, None);
        graph.add_node("hop_1".to_string(), 1.0, None);
        graph.add_node("hop_2".to_string(), 1.0, None);

        graph
            .add_relation(
                "seed_node".to_string(),
                "hop_1".to_string(),
                "relates".to_string(),
                None,
                None,
            )
            .await
            .unwrap();
        graph
            .add_relation(
                "hop_1".to_string(),
                "hop_2".to_string(),
                "relates".to_string(),
                None,
                None,
            )
            .await
            .unwrap();

        let pathfinder = Pathfinder::new(&graph);
        let candidates = vec![("seed_node".to_string(), 1.0)];

        // With high damping factor, pagerank will propagate to hop_1 and hop_2
        let results = pathfinder.associate_and_rerank(&candidates, 0.0, 0.9, 100, 1e-4);

        let has_hop_1 = results.iter().any(|(id, _)| id == "hop_1");
        assert!(has_hop_1, "hop_1 should be discovered");
    }

    #[tokio::test]
    async fn test_associate_and_rerank_alpha_boundary() {
        let graph = BeliefGraph::new();
        graph.add_node("a".to_string(), 1.0, None);
        graph.add_node("b".to_string(), 1.0, None);
        graph
            .add_relation(
                "a".to_string(),
                "b".to_string(),
                "relates".to_string(),
                None,
                None,
            )
            .await
            .unwrap();

        let pathfinder = Pathfinder::new(&graph);
        let candidates = vec![("a".to_string(), 0.8), ("b".to_string(), 0.2)];

        // Alpha = 1.0 (pure direct score)
        let results_alpha_1 = pathfinder.associate_and_rerank(&candidates, 1.0, 0.85, 100, 1e-4);
        assert_eq!(results_alpha_1[0].0, "a");
        assert_eq!(results_alpha_1[0].1, 0.8);
        assert_eq!(results_alpha_1[1].0, "b");
        assert_eq!(results_alpha_1[1].1, 0.2);

        // Alpha = 0.0 (pure PPR score)
        let results_alpha_0 = pathfinder.associate_and_rerank(&candidates, 0.0, 0.85, 100, 1e-4);
        assert_eq!(results_alpha_0[0].0, "a");
        assert_eq!(results_alpha_0[0].1, 1.0); // max_ppr normalization means top node is 1.0
    }

    #[test]
    fn test_personalized_pagerank_empty_seeds_returns_empty() {
        let graph = BeliefGraph::new();
        let pathfinder = Pathfinder::new(&graph);
        let ranks = pathfinder.personalized_pagerank(&[], 0.85, 100, 1e-4);
        assert!(ranks.is_empty());
    }

    #[tokio::test]
    async fn test_personalized_pagerank_propagates_to_neighbors() {
        let graph = BeliefGraph::new();
        graph.add_node("a".to_string(), 1.0, None);
        graph.add_node("b".to_string(), 1.0, None);
        graph.add_node("c".to_string(), 1.0, None);

        graph
            .add_relation(
                "a".to_string(),
                "b".to_string(),
                "relates".to_string(),
                None,
                None,
            )
            .await
            .unwrap();
        graph
            .add_relation(
                "b".to_string(),
                "c".to_string(),
                "relates".to_string(),
                None,
                None,
            )
            .await
            .unwrap();

        let pathfinder = Pathfinder::new(&graph);
        let seeds = vec!["a".to_string()];
        let ranks = pathfinder.personalized_pagerank(&seeds, 0.85, 100, 1e-4);

        let rank_a = *ranks.get("a").unwrap_or(&0.0);
        let rank_b = *ranks.get("b").unwrap_or(&0.0);
        let rank_c = *ranks.get("c").unwrap_or(&0.0);

        assert!(rank_a > rank_b, "a should have higher rank than b");
        assert!(rank_b > rank_c, "b should have higher rank than c");
        assert!(rank_c > 0.0, "c should have non-zero rank");
    }

    #[tokio::test]
    async fn test_personalized_pagerank_damping_boundary() {
        let graph = BeliefGraph::new();
        graph.add_node("a".to_string(), 1.0, None);
        graph.add_node("b".to_string(), 1.0, None);

        graph
            .add_relation(
                "a".to_string(),
                "b".to_string(),
                "relates".to_string(),
                None,
                None,
            )
            .await
            .unwrap();

        let pathfinder = Pathfinder::new(&graph);
        let seeds = vec!["a".to_string()];

        // Pass out-of-bounds damping factor (1.5)
        let ranks = pathfinder.personalized_pagerank(&seeds, 1.5, 100, 1e-4);

        let sum: f32 = ranks.values().sum();
        assert!(
            (sum - 1.0).abs() < 0.05,
            "Probabilities should sum to approximately 1.0, got {}",
            sum
        );
    }

    #[tokio::test]
    async fn test_pathfinding() {
        let graph = BeliefGraph::new();
        graph.add_node("A".to_string(), 1.0, None);
        graph.add_node("B".to_string(), 1.0, None);
        graph.add_node("C".to_string(), 1.0, None);
        graph.add_node("D".to_string(), 1.0, None);

        graph
            .add_relation(
                "A".to_string(),
                "B".to_string(),
                "related_to".to_string(),
                Some("mem1".to_string()),
                None,
            )
            .await
            .unwrap();
        graph
            .add_relation(
                "B".to_string(),
                "C".to_string(),
                "related_to".to_string(),
                Some("mem2".to_string()),
                None,
            )
            .await
            .unwrap();
        graph
            .add_relation(
                "A".to_string(),
                "D".to_string(),
                "related_to".to_string(),
                Some("mem3".to_string()),
                None,
            )
            .await
            .unwrap();
        graph
            .add_relation(
                "D".to_string(),
                "C".to_string(),
                "related_to".to_string(),
                Some("mem4".to_string()),
                None,
            )
            .await
            .unwrap();

        let pathfinder = Pathfinder::new(&graph);

        let shortest = pathfinder.shortest_path("a", "c");
        assert_eq!(shortest.len(), 2);
        assert_eq!(shortest[0].source, "a");
        assert_eq!(shortest[0].target, "b");
        assert_eq!(shortest[1].source, "b");
        assert_eq!(shortest[1].target, "c");

        let expansion = pathfinder.k_hop_expansion("a", 1);
        assert_eq!(expansion.len(), 2);

        let all = pathfinder.all_paths("a", "c", 3);
        assert_eq!(all.len(), 2);

        let affected = pathfinder.affected_bfs("a", 2);
        assert_eq!(affected.len(), 3); // B, D, C (via B or D)
        assert!(affected.iter().any(|a| a.node == "b" && a.depth == 1));
        assert!(affected.iter().any(|a| a.node == "d" && a.depth == 1));
        assert!(affected.iter().any(|a| a.node == "c" && a.depth == 2));
    }

    #[tokio::test]
    async fn test_guided_search() {
        let graph = BeliefGraph::new();
        graph.add_node("a".to_string(), 1.0, None);
        graph.add_node("b".to_string(), 1.0, None);
        graph.add_node("d".to_string(), 1.0, None);

        graph
            .add_relation(
                "a".to_string(),
                "b".to_string(),
                "rust_expert".to_string(),
                Some("mem1".to_string()),
                None,
            )
            .await
            .unwrap();
        graph
            .add_relation(
                "a".to_string(),
                "d".to_string(),
                "cooks_pasta".to_string(),
                Some("mem2".to_string()),
                None,
            )
            .await
            .unwrap();

        let pathfinder = Pathfinder::new(&graph);
        let results = pathfinder.guided_search("a", "rust", 1);

        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.edge.target == "b"));
        assert!(results[0].edge.target == "b");
        assert!(results[0].policy_score > 0.5);
    }
}
