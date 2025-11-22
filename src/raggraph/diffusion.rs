//! Multi-hop diffusion algorithm
//!
//! Implements PageRank-style diffusion: p = α * A * p + (1 - α) * p0
//! where:
//! - p is the current score vector
//! - p0 is the initial score vector (seed nodes)
//! - A is the normalized adjacency matrix
//! - α is the damping factor (typically 0.85)

use super::types::NodeId;
use anyhow::Result;
use std::collections::HashMap;

/// Compute diffusion scores using PageRank-style propagation
///
/// # Arguments
/// * `seed_nodes` - Initial nodes to start diffusion from
/// * `adjacency` - Sparse adjacency matrix as HashMap<source, Vec<(target, weight)>>
/// * `num_hops` - Number of diffusion iterations to perform
/// * `alpha` - Damping factor (0.0 to 1.0, typically 0.85)
///
/// # Returns
/// HashMap mapping NodeId to diffusion score
pub fn compute_diffusion_scores(
    seed_nodes: &[NodeId],
    adjacency: &HashMap<NodeId, Vec<(NodeId, f32)>>,
    num_hops: usize,
    alpha: f32,
) -> Result<HashMap<NodeId, f32>> {
    if seed_nodes.is_empty() {
        anyhow::bail!("Seed nodes cannot be empty");
    }

    if !(0.0..=1.0).contains(&alpha) {
        anyhow::bail!("Alpha must be between 0.0 and 1.0");
    }

    // Initialize: p0 = uniform distribution over seed nodes
    let mut scores: HashMap<NodeId, f32> = HashMap::new();
    let seed_score = 1.0 / seed_nodes.len() as f32;
    for &node_id in seed_nodes {
        scores.insert(node_id, seed_score);
    }

    // Store initial scores for teleportation
    let initial_scores = scores.clone();

    // Normalize adjacency matrix by out-degree
    let normalized_adj = normalize_adjacency(adjacency);

    // Cumulative scores across all hops (tracks total relevance/reach from seeds)
    let mut cumulative_scores: HashMap<NodeId, f32> = initial_scores.clone();

    // Iterative diffusion: p = α * A * p + (1 - α) * p0
    for _ in 0..num_hops {
        let mut new_scores: HashMap<NodeId, f32> = HashMap::new();

        // Teleportation step: (1 - α) * p0 (seed nodes always have contribution)
        for (&node, &init_score) in &initial_scores {
            *new_scores.entry(node).or_insert(0.0) += (1.0 - alpha) * init_score;
        }

        // Diffusion step: α * A * p
        for (&source, source_score) in &scores {
            if let Some(neighbors) = normalized_adj.get(&source) {
                for &(target, weight) in neighbors {
                    let contribution = alpha * source_score * weight;
                    *new_scores.entry(target).or_insert(0.0) += contribution;
                    // Accumulate for cumulative score
                    *cumulative_scores.entry(target).or_insert(0.0) += contribution;
                }
            } else {
                // Node has no outgoing edges - retains its score (sink node)
                *new_scores.entry(source).or_insert(0.0) += alpha * source_score;
            }
        }

        scores = new_scores;
    }

    Ok(cumulative_scores)
}

/// Normalize adjacency matrix by out-degree
///
/// Converts edge weights to probabilities by dividing by sum of outgoing edge weights
fn normalize_adjacency(
    adjacency: &HashMap<NodeId, Vec<(NodeId, f32)>>,
) -> HashMap<NodeId, Vec<(NodeId, f32)>> {
    let mut normalized: HashMap<NodeId, Vec<(NodeId, f32)>> = HashMap::new();

    for (&source, neighbors) in adjacency {
        // Compute total weight of outgoing edges
        let total_weight: f32 = neighbors.iter().map(|(_, w)| w).sum();

        if total_weight > 0.0 {
            // Normalize each edge weight
            let norm_neighbors: Vec<(NodeId, f32)> = neighbors
                .iter()
                .map(|&(target, weight)| (target, weight / total_weight))
                .collect();

            normalized.insert(source, norm_neighbors);
        } else {
            // No outgoing edges, keep as is
            normalized.insert(source, neighbors.clone());
        }
    }

    normalized
}
