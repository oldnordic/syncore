//! Knowledge fusion from vector and graph sources
//!
//! Fuses HNSW vector search results with Neo4j graph neighbor embeddings
//! using weighted averaging based on similarity/diffusion scores.

use super::types::NodeId;
use anyhow::Result;
use std::collections::HashMap;

/// Fuse HNSW vector results with graph neighbor embeddings
///
/// Combines vector search results (NodeId, similarity_score) with graph neighbors
/// using their diffusion scores as weights. Produces a unified context embedding.
///
/// # Arguments
/// * `vector_results` - Top-k vector search results with similarity scores
/// * `graph_neighbors` - Graph neighbors from multi-hop diffusion
/// * `embeddings` - Lookup map from NodeId to embedding vectors
///
/// # Returns
/// Fused embedding vector (weighted average of vector + graph embeddings)
///
/// # Algorithm
/// 1. Collect vector results with weights = similarity scores (high weight)
/// 2. Collect graph neighbors with weights = uniform (lower weight)
/// 3. Normalize all weights to sum to 1.0
/// 4. Compute weighted average of embeddings
pub fn fuse_knowledge(
    vector_results: &[(NodeId, f32)],
    graph_neighbors: &[NodeId],
    embeddings: &HashMap<NodeId, Vec<f32>>,
) -> Result<Vec<f32>> {
    if vector_results.is_empty() && graph_neighbors.is_empty() {
        anyhow::bail!("Cannot fuse knowledge with no vector results and no graph neighbors");
    }

    // Determine embedding dimension from first available embedding
    let embedding_dim = embeddings
        .values()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No embeddings available"))?
        .len();

    // Collect weighted nodes
    let mut weighted_nodes: Vec<(NodeId, f32)> = Vec::new();

    // Vector results get high weight (similarity scores)
    // Use higher multiplier to prioritize vector search results
    const VECTOR_WEIGHT_MULTIPLIER: f32 = 3.0;
    for &(node_id, similarity) in vector_results {
        if embeddings.contains_key(&node_id) {
            weighted_nodes.push((node_id, similarity * VECTOR_WEIGHT_MULTIPLIER));
        }
    }

    // Graph neighbors get uniform weight
    // Lower weight to complement (not dominate) vector results
    let graph_weight = if graph_neighbors.is_empty() {
        0.0
    } else {
        1.0 / graph_neighbors.len() as f32
    };

    for &node_id in graph_neighbors {
        // Skip if already included from vector results
        if !weighted_nodes.iter().any(|(id, _)| *id == node_id) && embeddings.contains_key(&node_id)
        {
            weighted_nodes.push((node_id, graph_weight));
        }
    }

    if weighted_nodes.is_empty() {
        anyhow::bail!("No valid embeddings found for any nodes");
    }

    // Normalize weights to sum to 1.0
    let total_weight: f32 = weighted_nodes.iter().map(|(_, w)| w).sum();
    let normalized_nodes: Vec<(NodeId, f32)> = weighted_nodes
        .iter()
        .map(|&(id, w)| (id, w / total_weight))
        .collect();

    // Compute weighted average of embeddings
    let mut fused = vec![0.0f32; embedding_dim];
    for (node_id, weight) in normalized_nodes {
        if let Some(embedding) = embeddings.get(&node_id) {
            for (i, &val) in embedding.iter().enumerate() {
                fused[i] += weight * val;
            }
        }
    }

    Ok(fused)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_only_fusion() {
        let vector_results = vec![(1, 0.9), (2, 0.7)];
        let graph_neighbors = vec![];
        let mut embeddings = HashMap::new();
        embeddings.insert(1, vec![1.0, 0.0]);
        embeddings.insert(2, vec![0.0, 1.0]);

        let fused = fuse_knowledge(&vector_results, &graph_neighbors, &embeddings).unwrap();
        assert_eq!(fused.len(), 2);
        // Should be weighted toward node 1 (higher similarity)
        assert!(fused[0] > fused[1]);
    }

    #[test]
    fn test_graph_only_fusion() {
        let vector_results = vec![];
        let graph_neighbors = vec![1, 2];
        let mut embeddings = HashMap::new();
        embeddings.insert(1, vec![1.0, 0.0]);
        embeddings.insert(2, vec![0.0, 1.0]);

        let fused = fuse_knowledge(&vector_results, &graph_neighbors, &embeddings).unwrap();
        assert_eq!(fused.len(), 2);
        // Should be equal average since uniform weights
        assert!((fused[0] - 0.5).abs() < 0.01);
        assert!((fused[1] - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_combined_fusion() {
        let vector_results = vec![(1, 1.0)];
        let graph_neighbors = vec![2, 3];
        let mut embeddings = HashMap::new();
        embeddings.insert(1, vec![1.0, 0.0, 0.0]);
        embeddings.insert(2, vec![0.0, 1.0, 0.0]);
        embeddings.insert(3, vec![0.0, 0.0, 1.0]);

        let fused = fuse_knowledge(&vector_results, &graph_neighbors, &embeddings).unwrap();
        assert_eq!(fused.len(), 3);
        // Vector result should dominate due to higher weight multiplier
        assert!(fused[0] > fused[1]);
        assert!(fused[0] > fused[2]);
    }

    #[test]
    fn test_missing_embeddings_skipped() {
        let vector_results = vec![(1, 0.9), (999, 0.8)]; // 999 has no embedding
        let graph_neighbors = vec![];
        let mut embeddings = HashMap::new();
        embeddings.insert(1, vec![1.0, 0.0]);

        let fused = fuse_knowledge(&vector_results, &graph_neighbors, &embeddings).unwrap();
        assert_eq!(fused.len(), 2);
        // Should only use node 1
        assert!((fused[0] - 1.0).abs() < 0.01);
        assert!((fused[1] - 0.0).abs() < 0.01);
    }
}
