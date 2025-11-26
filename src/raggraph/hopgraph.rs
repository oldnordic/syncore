//! HopGraph transformer implementation

use super::config::{RagGraphConfig};
use super::storage::StorageAdapter;
use super::types::{NodeId, RagGraphResult};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

/// HopGraph transformer for multi-hop reasoning
pub struct HopGraphTransformer {
    config: RagGraphConfig,
    storage: Option<Arc<dyn StorageAdapter>>,
}

impl HopGraphTransformer {
    pub fn new(config: RagGraphConfig) -> Self {
        Self {
            config,
            storage: None,
        }
    }

    pub fn with_storage(config: RagGraphConfig, storage: Arc<dyn StorageAdapter>) -> Self {
        Self {
            config,
            storage: Some(storage),
        }
    }

    pub fn multi_hop_reasoning(&self, seed_nodes: &[NodeId]) -> Result<RagGraphResult> {
        use super::attention::compute_attention_scale;
        use super::diffusion::compute_diffusion_scores;
        use super::fusion::fuse_knowledge;

        if seed_nodes.is_empty() {
            anyhow::bail!("Seed nodes cannot be empty");
        }

        // Load adjacency and embeddings based on backend mode
        let (adjacency, embeddings) = if let Some(ref storage) = self.storage {
            // Real mode: use storage adapter
            self.load_real_graph_data(seed_nodes, storage)?
        } else {
            // Mock mode: use deterministic synthetic data
            let adjacency = self.create_mock_adjacency(seed_nodes);
            let embeddings = self.create_mock_embeddings(seed_nodes, &adjacency);
            (adjacency, embeddings)
        };

        // Run diffusion algorithm
        let diffusion_scores = compute_diffusion_scores(
            seed_nodes,
            &adjacency,
            self.config.num_hops,
            self.config.alpha,
        )?;

        // Sort nodes by diffusion score (descending)
        let mut scored_nodes: Vec<(NodeId, f32)> = diffusion_scores.into_iter().collect();
        scored_nodes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Save count before move
        let total_nodes = scored_nodes.len();

        // Take top-k nodes
        let top_k = self.config.top_k.min(total_nodes);
        let top_nodes: Vec<NodeId> = scored_nodes.iter().take(top_k).map(|(id, _)| *id).collect();
        let top_scores: Vec<(NodeId, f32)> = scored_nodes.into_iter().take(top_k).collect();

        // Get graph neighbors (all nodes from diffusion except seeds)
        let graph_neighbors: Vec<NodeId> = top_scores
            .iter()
            .map(|(id, _)| *id)
            .filter(|id| !seed_nodes.contains(id))
            .collect();

        // Fuse knowledge: combine top-k nodes with their embeddings
        let context_embedding = fuse_knowledge(&top_scores, &graph_neighbors, &embeddings)?;

        // Compute attention scale
        let attention_scale =
            compute_attention_scale(&context_embedding, self.config.embedding_dim)?;

        // Build reasoning path (step-by-step description)
        let reasoning_path = vec![
            format!(
                "Started with {} seed nodes: {:?}",
                seed_nodes.len(),
                seed_nodes
            ),
            format!(
                "Ran {} diffusion iterations with alpha={:.2}",
                self.config.num_hops, self.config.alpha
            ),
            format!("Found {} nodes with diffusion scores", total_nodes),
            format!("Selected top {} nodes by score", top_nodes.len()),
            format!("Computed attention scale: {:.3}", attention_scale),
        ];

        Ok(RagGraphResult {
            top_nodes,
            context_embedding,
            reasoning_path,
        })
    }

    /// Load real graph data from storage adapter
    fn load_real_graph_data(
        &self,
        seed_nodes: &[NodeId],
        storage: &Arc<dyn StorageAdapter>,
    ) -> Result<(
        HashMap<NodeId, Vec<(NodeId, f32)>>,
        HashMap<NodeId, Vec<f32>>,
    )> {
        let mut adjacency = HashMap::new();
        let mut embeddings = HashMap::new();
        let mut visited = std::collections::HashSet::new();

        // BFS to build adjacency graph up to num_hops
        let mut current_layer = seed_nodes.to_vec();
        for _hop in 0..self.config.num_hops {
            let mut next_layer = Vec::new();

            for &node_id in &current_layer {
                if visited.contains(&node_id) {
                    continue;
                }
                visited.insert(node_id);

                // Get neighbors from storage
                let neighbors = storage.neighbors_of(node_id).unwrap_or_else(|_| Vec::new());

                // Get embedding from storage
                let embedding = storage
                    .resolve_embedding(node_id)
                    .unwrap_or_else(|_| vec![0.0; self.config.embedding_dim]);

                adjacency.insert(node_id, neighbors.clone());
                embeddings.insert(node_id, embedding);

                // Add neighbors to next layer
                for &(neighbor_id, _weight) in &neighbors {
                    if !visited.contains(&neighbor_id) {
                        next_layer.push(neighbor_id);
                    }
                }
            }

            current_layer = next_layer;
        }

        Ok((adjacency, embeddings))
    }

    /// Create mock adjacency graph for testing
    /// In production, this would load from Neo4j via storage adapter
    fn create_mock_adjacency(&self, seed_nodes: &[NodeId]) -> HashMap<NodeId, Vec<(NodeId, f32)>> {
        let mut adjacency = HashMap::new();

        // Create a small connected graph from seed nodes
        for (i, &node_id) in seed_nodes.iter().enumerate() {
            let mut neighbors = Vec::new();

            // Connect to next seed node (circular)
            let next_idx = (i + 1) % seed_nodes.len();
            let next_node = seed_nodes[next_idx];
            neighbors.push((next_node, 1.0));

            // Add some synthetic neighbor nodes
            let synthetic_base = 1000 + (node_id * 10);
            neighbors.push((synthetic_base, 0.8));
            neighbors.push((synthetic_base + 1, 0.6));

            adjacency.insert(node_id, neighbors);
        }

        adjacency
    }

    /// Create mock embeddings for testing
    /// In production, this would load from HNSW via storage adapter
    fn create_mock_embeddings(
        &self,
        seed_nodes: &[NodeId],
        adjacency: &HashMap<NodeId, Vec<(NodeId, f32)>>,
    ) -> HashMap<NodeId, Vec<f32>> {
        use std::f32::consts::PI;

        let mut embeddings = HashMap::new();
        let dim = self.config.embedding_dim;

        // Collect all unique node IDs
        let mut all_nodes: std::collections::HashSet<NodeId> = seed_nodes.iter().copied().collect();
        for neighbors in adjacency.values() {
            for &(target, _) in neighbors {
                all_nodes.insert(target);
            }
        }

        // Create deterministic embeddings for each node
        for &node_id in &all_nodes {
            let mut embedding = Vec::with_capacity(dim);
            for i in 0..dim {
                // Deterministic embedding based on node_id and dimension
                let val = ((node_id as usize * 7 + i * 13) as f32 * PI / dim as f32).sin();
                embedding.push(val);
            }

            // Normalize to unit length
            let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                for x in &mut embedding {
                    *x /= norm;
                }
            }

            embeddings.insert(node_id, embedding);
        }

        embeddings
    }
}
