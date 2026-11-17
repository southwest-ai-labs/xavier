//! Bi-directional Knowledge Graph Engine for Living Wiki.
//!
//! Provides forward link and backlink exploration, orphan page detection,
//! and multi-hop neighborhood graph extractions.

use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::wiki::storage::WikiStorageEngine;

/// Node in a wiki neighborhood subgraph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WikiGraphNode {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub depth: usize,
}

/// Directed edge in a wiki neighborhood subgraph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WikiGraphEdge {
    pub source: String,
    pub target: String,
    pub edge_type: String,
}

/// Subgraph centered on a root entity up to a maximum traversal depth.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WikiSubGraph {
    pub root_id: String,
    pub max_depth: usize,
    pub nodes: Vec<WikiGraphNode>,
    pub edges: Vec<WikiGraphEdge>,
}

/// High-level Graph navigator over the living wiki storage.
#[derive(Clone)]
pub struct WikiGraph {
    storage: Arc<WikiStorageEngine>,
}

impl WikiGraph {
    /// Creates a new `WikiGraph` backed by a `WikiStorageEngine`.
    pub fn new(storage: Arc<WikiStorageEngine>) -> Self {
        Self { storage }
    }

    /// Returns all outgoing forward links from `page_id`.
    pub fn get_forward_links(&self, page_id: &str) -> Result<Vec<String>> {
        self.storage.get_forward_links(page_id)
    }

    /// Returns all incoming backlinks to `page_id`.
    pub fn get_backlinks(&self, page_id: &str) -> Result<Vec<String>> {
        self.storage.get_backlinks(page_id)
    }

    /// Finds orphan pages (pages with 0 incoming backlinks and 0 outgoing links).
    pub fn find_orphan_pages(&self) -> Result<Vec<String>> {
        self.storage.find_orphan_nodes()
    }

    /// Extracts an N-hop neighborhood subgraph centered on `page_id`.
    pub fn get_neighborhood(&self, page_id: &str, depth: usize) -> Result<WikiSubGraph> {
        let mut visited = HashSet::new();
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut queue = VecDeque::new();

        queue.push_back((page_id.to_string(), 0));
        visited.insert(page_id.to_string());

        let root_title = self
            .storage
            .get_node(page_id)?
            .map(|n| n.title)
            .unwrap_or_else(|| page_id.to_string());

        nodes.push(WikiGraphNode {
            id: page_id.to_string(),
            title: root_title,
            kind: "root".to_string(),
            depth: 0,
        });

        while let Some((current_id, current_depth)) = queue.pop_front() {
            if current_depth >= depth {
                continue;
            }

            // 1. Traverse forward links (calls / references)
            let forward = self.storage.get_forward_links(&current_id)?;
            for target in forward {
                edges.push(WikiGraphEdge {
                    source: current_id.clone(),
                    target: target.clone(),
                    edge_type: "forward".to_string(),
                });

                if visited.insert(target.clone()) {
                    let title = self
                        .storage
                        .get_node(&target)?
                        .map(|n| n.title)
                        .unwrap_or_else(|| target.clone());

                    nodes.push(WikiGraphNode {
                        id: target.clone(),
                        title,
                        kind: "forward".to_string(),
                        depth: current_depth + 1,
                    });

                    queue.push_back((target, current_depth + 1));
                }
            }

            // 2. Traverse backlinks (callers / referrers)
            let backlinks = self.storage.get_backlinks(&current_id)?;
            for source in backlinks {
                edges.push(WikiGraphEdge {
                    source: source.clone(),
                    target: current_id.clone(),
                    edge_type: "backlink".to_string(),
                });

                if visited.insert(source.clone()) {
                    let title = self
                        .storage
                        .get_node(&source)?
                        .map(|n| n.title)
                        .unwrap_or_else(|| source.clone());

                    nodes.push(WikiGraphNode {
                        id: source.clone(),
                        title,
                        kind: "backlink".to_string(),
                        depth: current_depth + 1,
                    });

                    queue.push_back((source, current_depth + 1));
                }
            }
        }

        Ok(WikiSubGraph {
            root_id: page_id.to_string(),
            max_depth: depth,
            nodes,
            edges,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wiki_graph_traversal_and_neighborhood() {
        let storage = Arc::new(WikiStorageEngine::in_memory().expect("in-memory db"));
        let graph = WikiGraph::new(storage.clone());

        storage
            .upsert_node(
                "A",
                "Node A",
                "entity",
                "src/a.rs",
                "# A",
                &["B".to_string()],
            )
            .expect("A");

        storage
            .upsert_node(
                "B",
                "Node B",
                "entity",
                "src/b.rs",
                "# B",
                &["C".to_string()],
            )
            .expect("B");

        storage
            .upsert_node("C", "Node C", "entity", "src/c.rs", "# C", &[])
            .expect("C");

        storage
            .upsert_node("D_orphan", "Node D", "entity", "src/d.rs", "# D", &[])
            .expect("D");

        // Forward links and backlinks
        assert_eq!(graph.get_forward_links("A").unwrap(), vec!["B".to_string()]);
        assert_eq!(graph.get_backlinks("B").unwrap(), vec!["A".to_string()]);

        // Orphan detection
        let orphans = graph.find_orphan_pages().unwrap();
        assert!(orphans.contains(&"D_orphan".to_string()));
        assert!(!orphans.contains(&"A".to_string()));
        assert!(!orphans.contains(&"B".to_string()));

        // Neighborhood extraction (depth 2 from A should find B and C)
        let sub = graph.get_neighborhood("A", 2).unwrap();
        assert_eq!(sub.nodes.len(), 3);
        let ids: Vec<&str> = sub.nodes.iter().map(|n| n.id.as_str()).collect();
        assert!(ids.contains(&"A"));
        assert!(ids.contains(&"B"));
        assert!(ids.contains(&"C"));
    }
}
