//! Backend operations for RAGGraph API
//!
//! Handles graph backend interactions, score computation, and data retrieval
//! from both vector store and graph backend.

use std::sync::Arc;
use super::super::fusion_simple;
use crate::graph::GraphBackend;
use anyhow::Result;

/// Compute graph score for an entity based on multi-hop depth using GraphBackend
///
/// This function works with any GraphBackend implementation (SQLiteGraph, Neo4j, etc.)
/// and provides a unified interface for graph-based scoring.
pub fn compute_graph_score(
    graph_backend: &Arc<dyn GraphBackend>,
    entity_id: i64,
) -> Result<f32> {
    // Check if entity exists in graph backend
    let entity_exists = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            graph_backend.entity_exists(entity_id).await
        })
    })?;

    if !entity_exists {
        return Ok(0.0);
    }

    // Perform multi-hop expansion for depth analysis
    let max_depth = 20; // Same limit as original implementation
    let mut visited = std::collections::HashSet::new();
    visited.insert(entity_id);

    // Use simple depth heuristic based on neighbor count and connectivity
    let mut total_neighbors = 0;
    let mut current_level = vec![entity_id];

    for depth in 1..=max_depth {
        if current_level.is_empty() {
            break;
        }

        let mut next_level = Vec::new();

        for &current_id in &current_level {
            let neighbors = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    graph_backend.get_neighbors(current_id).await
                })
            })?;

            for neighbor in neighbors {
                if !visited.contains(&neighbor.id) {
                    visited.insert(neighbor.id);
                    next_level.push(neighbor.id);
                    total_neighbors += 1;
                }
            }
        }

        current_level = next_level;

        // Early exit if we've explored enough
        if visited.len() > 100 {
            break;
        }
    }

    // Compute depth score using the same logic as original
    let depth_score = visited.len();
    let graph_score = super::super::fusion_simple::compute_graph_score(Some(depth_score as i32));

    Ok(graph_score)
}

/// Check if entity exists in the graph backend
pub fn entity_exists(graph_backend: &Arc<dyn GraphBackend>, entity_id: i64) -> Result<bool> {
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            graph_backend.entity_exists(entity_id).await
        })
    })
}

/// Get neighbors for an entity from the graph backend
pub fn get_entity_neighbors(
    graph_backend: &Arc<dyn GraphBackend>,
    entity_id: i64,
) -> Result<Vec<i64>> {
    let neighbor_results = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            graph_backend.get_neighbors(entity_id).await
        })
    })?;

    // Convert EntityResult to entity IDs
    let neighbor_ids: Vec<i64> = neighbor_results
        .into_iter()
        .map(|entity| entity.id)
        .collect();

    Ok(neighbor_ids)
}

/// Get entity details from graph backend
pub fn get_entity_details(
    graph_backend: &Arc<dyn GraphBackend>,
    entity_id: i64,
) -> Result<Option<super::super::types::CodeEntity>> {
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            graph_backend.get_entity(entity_id).await
        })
    })
}