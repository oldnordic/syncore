//! Storage adapter for SQLite + Neo4j integration

use super::types::NodeId;
use anyhow::Result;
use std::collections::HashMap;
use std::f32::consts::PI;

/// Storage adapter for RagGraph data
pub struct StorageAdapter;

impl StorageAdapter {
    pub fn new() -> Self {
        Self
    }

    /// Load graph neighbors from Neo4j
    /// Returns list of (neighbor_id, edge_weight) pairs
    ///
    /// In production, this would:
    /// 1. Connect to Neo4j
    /// 2. Run Cypher query: MATCH (n)-[r]-(m) WHERE id(n) = $node_id RETURN m, r.weight
    /// 3. Return neighbor nodes with edge weights
    ///
    /// For testing, returns mock neighbors
    pub fn load_neighbors(&self, node_id: NodeId) -> Result<Vec<(NodeId, f32)>> {
        // Generate deterministic mock neighbors
        let mut neighbors = Vec::new();

        // Create 2-3 neighbors per node
        let num_neighbors = 2 + (node_id % 2) as usize;
        for i in 0..num_neighbors {
            let neighbor_id = node_id + (i as NodeId) * 100 + 1;
            let weight = 0.5 + (i as f32 * 0.2);
            neighbors.push((neighbor_id, weight));
        }

        Ok(neighbors)
    }

    /// Load embeddings from HNSW vector store
    /// Returns map of node_id -> embedding vector
    ///
    /// In production, this would:
    /// 1. Connect to HNSW index
    /// 2. Fetch embedding vectors for given node IDs
    /// 3. Return HashMap<NodeId, Vec<f32>>
    ///
    /// For testing, returns mock embeddings (384-dim)
    pub fn load_embeddings(&self, node_ids: &[NodeId]) -> Result<HashMap<NodeId, Vec<f32>>> {
        let embedding_dim = 384;
        let mut embeddings = HashMap::new();

        for &node_id in node_ids {
            let embedding = self.generate_mock_embedding(node_id, embedding_dim);
            embeddings.insert(node_id, embedding);
        }

        Ok(embeddings)
    }

    /// Generate deterministic mock embedding for a node
    fn generate_mock_embedding(&self, node_id: NodeId, dim: usize) -> Vec<f32> {
        let mut embedding = Vec::with_capacity(dim);

        for i in 0..dim {
            // Deterministic value based on node_id and dimension
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

        embedding
    }
}
