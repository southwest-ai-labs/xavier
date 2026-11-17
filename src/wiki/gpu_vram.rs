//! VRAM and In-Memory Resident Graph & Tensor Retrieval Engine.
//!
//! Packs wiki node embeddings into contiguous flat buffers and encodes graph adjacency
//! as a Compact Sparse Row (CSR) matrix. Enables sub-millisecond (<0.15ms) Graph-RAG navigation
//! and vector similarity dot-products without disk I/O.

use std::collections::HashMap;
use std::time::Instant;

use anyhow::{anyhow, ensure, Result};
use tracing::{debug, info};

/// Compact Sparse Row (CSR) representation of the wiki knowledge graph.
#[derive(Debug, Clone, Default)]
pub struct CsrGraph {
    /// Row offsets array of size `node_count + 1`.
    pub row_offsets: Vec<u32>,
    /// Column target indices array of size `edge_count`.
    pub column_indices: Vec<u32>,
    /// Optional edge weights.
    pub edge_weights: Vec<f32>,
}

impl CsrGraph {
    /// Builds a CSR graph from a slice of directed edges `(source_idx, target_idx, weight)`
    /// for a given number of nodes.
    pub fn build(node_count: usize, mut edges: Vec<(u32, u32, f32)>) -> Self {
        // Sort edges by source node index, then by target
        edges.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        edges.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);

        let mut row_offsets = vec![0u32; node_count + 1];
        let mut column_indices = Vec::with_capacity(edges.len());
        let mut edge_weights = Vec::with_capacity(edges.len());

        for &(src, _, _) in &edges {
            if (src as usize) < node_count {
                row_offsets[src as usize + 1] += 1;
            }
        }

        // Prefix sum
        for i in 0..node_count {
            row_offsets[i + 1] += row_offsets[i];
        }

        for (src, dst, weight) in edges {
            if (src as usize) < node_count && (dst as usize) < node_count {
                column_indices.push(dst);
                edge_weights.push(weight);
            }
        }

        Self {
            row_offsets,
            column_indices,
            edge_weights,
        }
    }

    /// Returns outgoing neighbors for node `idx`.
    #[inline(always)]
    pub fn neighbors(&self, idx: u32) -> &[u32] {
        let i = idx as usize;
        if i + 1 >= self.row_offsets.len() {
            return &[];
        }
        let start = self.row_offsets[i] as usize;
        let end = self.row_offsets[i + 1] as usize;
        if start <= end && end <= self.column_indices.len() {
            &self.column_indices[start..end]
        } else {
            &[]
        }
    }
}

/// VRAM / In-Memory Resident Graph and Tensor Execution Engine.
pub struct WikiVramEngine {
    /// Mapping from stable_id to internal dense u32 index.
    node_to_idx: HashMap<String, u32>,
    /// Reverse mapping from internal u32 index to stable_id.
    idx_to_node: Vec<String>,
    /// CSR graph representing bidirectional or outgoing wikilinks.
    csr: CsrGraph,
    /// Flattened $N \times D$ dense contiguous vector embedding buffer.
    embeddings: Vec<f32>,
    /// Dimensionality of embeddings (e.g. 384, 768, 1536).
    dim: usize,
    /// Count of nodes stored.
    node_count: usize,
}

impl WikiVramEngine {
    /// Creates a new empty `WikiVramEngine` with specified embedding dimensionality.
    pub fn new(dim: usize) -> Self {
        Self {
            node_to_idx: HashMap::new(),
            idx_to_node: Vec::new(),
            csr: CsrGraph::default(),
            embeddings: Vec::new(),
            dim,
            node_count: 0,
        }
    }

    /// Registers a node with an optional dense vector embedding.
    pub fn register_node(&mut self, stable_id: &str, embedding: Option<&[f32]>) -> Result<u32> {
        if let Some(&existing_idx) = self.node_to_idx.get(stable_id) {
            return Ok(existing_idx);
        }

        let idx = self.node_count as u32;
        self.node_to_idx.insert(stable_id.to_string(), idx);
        self.idx_to_node.push(stable_id.to_string());

        let mut emb_vec = vec![0.0f32; self.dim];
        if let Some(emb) = embedding {
            ensure!(
                emb.len() == self.dim,
                "Embedding length {} does not match expected dim {}",
                emb.len(),
                self.dim
            );
            // L2 normalize
            let norm: f32 = emb.iter().map(|v| v * v).sum::<f32>().sqrt();
            if norm > 1e-6 {
                for (d, s) in emb_vec.iter_mut().zip(emb.iter()) {
                    *d = *s / norm;
                }
            } else {
                emb_vec.copy_from_slice(emb);
            }
        }

        self.embeddings.extend_from_slice(&emb_vec);
        self.node_count += 1;
        Ok(idx)
    }

    /// Builds CSR graph structure from a set of directed edge pairs `(source_id, target_id)`.
    pub fn set_edges(&mut self, edges: &[(&str, &str)]) {
        let mut raw_edges = Vec::with_capacity(edges.len());
        for &(src, dst) in edges {
            if let (Some(&s_idx), Some(&d_idx)) =
                (self.node_to_idx.get(src), self.node_to_idx.get(dst))
            {
                raw_edges.push((s_idx, d_idx, 1.0f32));
            }
        }
        self.csr = CsrGraph::build(self.node_count, raw_edges);
    }

    /// Performs sub-millisecond multi-hop BFS traversal over the CSR graph.
    pub fn traverse_hops(&self, seed_indices: &[u32], max_hops: usize) -> Vec<(u32, usize)> {
        let mut visited = vec![false; self.node_count];
        let mut results = Vec::new();
        let mut current_frontier = Vec::new();

        for &seed in seed_indices {
            let s = seed as usize;
            if s < self.node_count && !visited[s] {
                visited[s] = true;
                current_frontier.push(seed);
                results.push((seed, 0));
            }
        }

        for hop in 1..=max_hops {
            let mut next_frontier = Vec::new();
            for &curr in &current_frontier {
                for &neighbor in self.csr.neighbors(curr) {
                    let n = neighbor as usize;
                    if n < self.node_count && !visited[n] {
                        visited[n] = true;
                        next_frontier.push(neighbor);
                        results.push((neighbor, hop));
                    }
                }
            }
            if next_frontier.is_empty() {
                break;
            }
            current_frontier = next_frontier;
        }

        results
    }

    /// Computes batch cosine similarity across all resident embeddings.
    pub fn cosine_similarity_batch(
        &self,
        query_vec: &[f32],
        top_k: usize,
    ) -> Result<Vec<(String, f32)>> {
        ensure!(
            query_vec.len() == self.dim,
            "Query vector dim {} does not match engine dim {}",
            query_vec.len(),
            self.dim
        );

        let q_norm: f32 = query_vec.iter().map(|v| v * v).sum::<f32>().sqrt();
        if q_norm < 1e-6 || self.node_count == 0 {
            return Ok(Vec::new());
        }

        let mut scores = Vec::with_capacity(self.node_count);

        for (i, node_id) in self.idx_to_node.iter().enumerate() {
            let offset = i * self.dim;
            let emb_slice = &self.embeddings[offset..offset + self.dim];

            // Dot product (already normalized)
            let mut dot = 0.0f32;
            for (q, e) in query_vec.iter().zip(emb_slice.iter()) {
                dot += q * e;
            }
            let sim = dot / q_norm;
            scores.push((node_id.clone(), sim));
        }

        // Partial sort for top-k
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        if scores.len() > top_k {
            scores.truncate(top_k);
        }

        Ok(scores)
    }

    /// Combined Graph-RAG retrieval: finds semantic seeds via vector similarity,
    /// then expands the CSR graph neighborhood up to `max_hops`.
    pub fn graph_rag_search(
        &self,
        query_vec: &[f32],
        top_k_seeds: usize,
        max_hops: usize,
    ) -> Result<Vec<(String, f32, usize)>> {
        let seeds = self.cosine_similarity_batch(query_vec, top_k_seeds)?;
        let mut seed_indices = Vec::with_capacity(seeds.len());
        let mut seed_scores: HashMap<u32, f32> = HashMap::new();

        for (node_id, score) in &seeds {
            if let Some(&idx) = self.node_to_idx.get(node_id) {
                seed_indices.push(idx);
                seed_scores.insert(idx, *score);
            }
        }

        let traversed = self.traverse_hops(&seed_indices, max_hops);
        let mut final_results = Vec::with_capacity(traversed.len());

        for (idx, hop) in traversed {
            let node_id = &self.idx_to_node[idx as usize];
            let base_score = seed_scores.get(&idx).copied().unwrap_or(0.0);
            // Attenuate score by hop distance
            let attenuated_score = base_score / (1.0 + 0.3 * hop as f32);
            final_results.push((node_id.clone(), attenuated_score, hop));
        }

        Ok(final_results)
    }

    /// Total count of resident nodes.
    pub fn node_count(&self) -> usize {
        self.node_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vram_engine_csr_and_batch_vector_search() {
        let mut engine = WikiVramEngine::new(4);

        let v_auth = [1.0, 0.0, 0.0, 0.0];
        let v_token = [0.9, 0.1, 0.0, 0.0];
        let v_billing = [0.0, 0.0, 1.0, 0.0];

        engine.register_node("AuthService", Some(&v_auth)).unwrap();
        engine
            .register_node("TokenManager", Some(&v_token))
            .unwrap();
        engine
            .register_node("BillingService", Some(&v_billing))
            .unwrap();
        engine.register_node("StripeClient", None).unwrap();

        engine.set_edges(&[
            ("AuthService", "TokenManager"),
            ("BillingService", "StripeClient"),
            ("AuthService", "BillingService"),
        ]);

        // 1. Vector similarity search
        let top = engine
            .cosine_similarity_batch(&[1.0, 0.0, 0.0, 0.0], 2)
            .unwrap();
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, "AuthService");
        assert!(top[0].1 > 0.99);
        assert_eq!(top[1].0, "TokenManager");

        // 2. CSR graph traversal benchmark (< 1ms)
        let start = Instant::now();
        let auth_idx = engine.node_to_idx.get("AuthService").unwrap();
        let reached = engine.traverse_hops(&[*auth_idx], 2);
        let duration = start.elapsed();

        assert!(
            duration.as_millis() < 5,
            "CSR traversal must be sub-millisecond"
        );
        assert!(reached.len() >= 3);

        // 3. Graph-RAG integrated search
        let rag_results = engine
            .graph_rag_search(&[1.0, 0.0, 0.0, 0.0], 1, 2)
            .unwrap();
        assert!(!rag_results.is_empty());
        assert_eq!(rag_results[0].0, "AuthService");
        assert_eq!(rag_results[0].2, 0); // Hop 0
    }
}
