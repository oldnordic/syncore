//! Multi-hop graph reasoning for RAGGraph API
//!
//! Implements multi-hop expansion and reasoning capabilities for deep graph traversal.

use std::sync::Arc;
use crate::graph::GraphBackend;
use anyhow::Result;
use std::collections::HashSet;

/// Configuration for multi-hop expansion
#[derive(Debug, Clone)]
pub struct MultiHopConfig {
    /// Maximum number of hops to explore
    pub max_hops: usize,
    /// Maximum number of entities to visit
    pub max_entities: usize,
    /// Score decay factor per hop
    pub decay_factor: f32,
}

impl Default for MultiHopConfig {
    fn default() -> Self {
        Self {
            max_hops: 3,
            max_entities: 100,
            decay_factor: 0.8,
        }
    }
}

/// Result of multi-hop expansion
#[derive(Debug, Clone)]
pub struct MultiHopResult {
    /// Expanded entity IDs with their scores
    pub entities: Vec<(i64, f32)>,
    /// Total depth explored
    pub depth_reached: usize,
    /// Total entities visited
    pub entities_visited: usize,
}

/// Perform multi-hop expansion from initial seed entities
pub fn multi_hop_expand(
    graph_backend: &Arc<dyn GraphBackend>,
    initial_entities: &[(i64, f32)],
    config: &MultiHopConfig,
) -> Result<MultiHopResult> {
    let mut expanded_entities = initial_entities.to_vec();
    let mut visited: HashSet<i64> = initial_entities.iter().map(|(id, _)| *id).collect();
    let mut current_level = initial_entities.to_vec();
    let mut depth_reached = 0;

    for hop in 1..=config.max_hops {
        if current_level.is_empty() || visited.len() >= config.max_entities {
            break;
        }

        let mut next_level = Vec::new();

        for (entity_id, base_score) in &current_level {
            // Get neighbors from graph backend
            let neighbors = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    graph_backend.get_neighbors(*entity_id).await
                })
            })?;

            for neighbor in neighbors {
                if !visited.contains(&neighbor.id) && visited.len() < config.max_entities {
                    visited.insert(neighbor.id);
                    // Apply decay factor for distance
                    let decayed_score = base_score * config.decay_factor;
                    next_level.push((neighbor.id, decayed_score));
                    expanded_entities.push((neighbor.id, decayed_score));
                }
            }
        }

        current_level = next_level;
        depth_reached = hop;

        if current_level.is_empty() {
            break;
        }
    }

    Ok(MultiHopResult {
        entities: expanded_entities,
        depth_reached,
        entities_visited: visited.len(),
    })
}

/// Compute connectivity score based on multi-hop expansion
pub fn compute_connectivity_score(
    graph_backend: &Arc<dyn GraphBackend>,
    entity_id: i64,
    max_hops: usize,
) -> Result<f32> {
    let config = MultiHopConfig {
        max_hops,
        max_entities: 50,
        decay_factor: 1.0, // No decay for connectivity score
    };

    let initial_entities = [(entity_id, 1.0)];
    let result = multi_hop_expand(graph_backend, &initial_entities, &config)?;

    // Normalize score based on entities reached
    let normalized_score = (result.entities_visited as f32 / 100.0).min(1.0);
    Ok(normalized_score)
}

/// Find shortest path between two entities
pub fn find_shortest_path(
    graph_backend: &Arc<dyn GraphBackend>,
    from_id: i64,
    to_id: i64,
    max_depth: usize,
) -> Result<Option<Vec<i64>>> {
    if from_id == to_id {
        return Ok(Some(vec![from_id]));
    }

    let mut visited: HashSet<i64> = HashSet::new();
    let mut queue: Vec<(i64, Vec<i64>)> = vec![(from_id, vec![from_id])];
    visited.insert(from_id);

    for _depth in 1..=max_depth {
        if queue.is_empty() {
            break;
        }

        let mut next_level = Vec::new();

        for (current_id, path) in queue {
            let neighbors = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    graph_backend.get_neighbors(current_id).await
                })
            })?;

            for neighbor in neighbors {
                if neighbor.id == to_id {
                    let mut final_path = path.clone();
                    final_path.push(neighbor.id);
                    return Ok(Some(final_path));
                }

                if !visited.contains(&neighbor.id) {
                    visited.insert(neighbor.id);
                    let mut new_path = path.clone();
                    new_path.push(neighbor.id);
                    next_level.push((neighbor.id, new_path));
                }
            }
        }

        queue = next_level;
    }

    Ok(None)
}