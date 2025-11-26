//! PHASE 4: Multi-Hop Graph Reasoning
//!
//! This module implements recursive graph traversal across SQLite and Neo4j,
//! enabling 1–N hop structural scoring for RagGraph fusion.
//!
//! Core features:
//! - BFS traversal with depth limiting
//! - Cycle detection using visited set
//! - Branch limiting (max 20 neighbors per node)
//! - Dual-mode: SQLite-only or SQLite+Neo4j union
//! - Deterministic ordering by entity_id

use super::types::EdgeType;
use crate::graph::Neo4jClient;
use anyhow::Result;
use rusqlite::Connection;
use std::collections::{HashSet, VecDeque};

/// A node discovered during multi-hop traversal
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiHopNode {
    /// Entity ID from code_entities table
    pub id: i64,
    /// Depth from starting node (0 = start node, 1 = direct neighbor, etc.)
    pub depth: usize,
    /// Type of edge that led to this node (None for start node)
    pub edge_type: Option<EdgeType>,
}

/// Result of multi-hop traversal
#[derive(Debug, Clone)]
pub struct MultiHopResult {
    /// All nodes discovered during traversal, ordered by depth then by id
    pub nodes: Vec<MultiHopNode>,
}

impl MultiHopResult {
    /// Create empty result
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Add a node to the result
    pub fn add_node(&mut self, node: MultiHopNode) {
        self.nodes.push(node);
    }

    /// Sort nodes by depth then by id for deterministic ordering
    pub fn sort(&mut self) {
        self.nodes.sort_by_key(|n| (n.depth, n.id));
    }
}

/// Maximum number of neighbors to explore per node (branch limit)
const MAX_NEIGHBORS_PER_NODE: usize = 20;

/// Get direct neighbors of an entity from SQLite
///
/// Returns deduplicated list of (neighbor_id, edge_type) pairs.
/// Sorted by neighbor_id for deterministic ordering.
///
/// # Arguments
/// * `db` - SQLite connection
/// * `entity_id` - Starting entity ID
pub fn neighbors_sqlite(db: &Connection, entity_id: i64) -> Result<Vec<(i64, EdgeType)>> {
    let mut stmt = db.prepare(
        "SELECT dst_entity_id, edge_type FROM code_edges
         WHERE src_entity_id = ?
         ORDER BY dst_entity_id",
    )?;

    let neighbors: Vec<(i64, EdgeType)> = stmt
        .query_map([entity_id], |row| {
            let dst_id: i64 = row.get(0)?;
            let edge_type_str: String = row.get(1)?;
            let edge_type = EdgeType::from_str(&edge_type_str);
            Ok((dst_id, edge_type))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(neighbors)
}

/// Get direct neighbors of an entity from Neo4j
///
/// Returns deduplicated list of (neighbor_id, edge_type) pairs.
/// Sorted by neighbor_id for deterministic ordering.
///
/// # Specialized Multi-Hop Traversal Query
///
/// This query is intentionally NOT migrated to the canonical Neo4j module because:
/// 1. Multi-hop traversal requires edge type information for BFS algorithm
/// 2. Canonical `get_neighbors()` returns full `EntityResult` without edge types
/// 3. This is a specialized graph algorithm, not general-purpose CRUD
/// 4. Query already follows best practices: namespace filtering, :SynCore label, parameterization
///
/// For normal neighbor queries (without edge types), use canonical `get_neighbors()`.
///
/// # Arguments
/// * `neo4j` - Neo4j client
/// * `entity_id` - Starting entity ID
pub async fn neighbors_neo4j(neo4j: &Neo4jClient, entity_id: i64) -> Result<Vec<(i64, EdgeType)>> {
    // TASK C: Restrict to :SynCore label for project isolation
    let query = r#"
        MATCH (a:SynCore {namespace: $ns})-[r]->(b:SynCore {namespace: $ns})
        WHERE a.id = $entity_id
        RETURN b.id as dst_id, type(r) as edge_type
        ORDER BY b.id
    "#;

    let results = neo4j
        .execute_query(
            query,
            vec![
                ("ns", serde_json::json!(neo4j.namespace())),
                ("entity_id", serde_json::json!(entity_id)),
            ],
        )
        .await?;

    let mut neighbors = Vec::new();
    for record in results {
        if let (Some(dst_id), Some(edge_type_str)) = (
            record.get("dst_id").and_then(|v| v.as_i64()),
            record.get("edge_type").and_then(|v| v.as_str()),
        ) {
            let edge_type = EdgeType::from_str(edge_type_str);
            neighbors.push((dst_id, edge_type));
        }
    }

    Ok(neighbors)
}

/// Perform multi-hop BFS traversal using SQLite only
///
/// # Arguments
/// * `db` - SQLite connection
/// * `entity_id` - Starting entity ID
/// * `max_depth` - Maximum traversal depth (0 = just start node, 1 = direct neighbors, etc.)
///
/// # Returns
/// MultiHopResult with all discovered nodes, annotated with depth and edge type
///
/// # Safety
/// - Uses visited set to prevent cycles
/// - Applies branch limit of MAX_NEIGHBORS_PER_NODE per node
/// - Stops exactly at max_depth
pub fn multi_hop_sqlite(
    db: &Connection,
    entity_id: i64,
    max_depth: usize,
) -> Result<MultiHopResult> {
    let mut result = MultiHopResult::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    // Start with the initial node
    let start_node = MultiHopNode {
        id: entity_id,
        depth: 0,
        edge_type: None,
    };
    result.add_node(start_node);
    visited.insert(entity_id);
    queue.push_back((entity_id, 0));

    while let Some((current_id, current_depth)) = queue.pop_front() {
        // Stop if we've reached max depth
        if current_depth >= max_depth {
            continue;
        }

        // Get neighbors from SQLite
        let neighbors = neighbors_sqlite(db, current_id)?;

        // Apply branch limit: only take first MAX_NEIGHBORS_PER_NODE neighbors
        let neighbors_to_explore = neighbors
            .into_iter()
            .take(MAX_NEIGHBORS_PER_NODE)
            .collect::<Vec<_>>();

        for (neighbor_id, edge_type) in neighbors_to_explore {
            // Skip if already visited (cycle detection)
            if visited.contains(&neighbor_id) {
                continue;
            }

            visited.insert(neighbor_id);

            let neighbor_node = MultiHopNode {
                id: neighbor_id,
                depth: current_depth + 1,
                edge_type: Some(edge_type),
            };
            result.add_node(neighbor_node);
            queue.push_back((neighbor_id, current_depth + 1));
        }
    }

    result.sort();
    Ok(result)
}

/// Perform multi-hop BFS traversal using SQLite + Neo4j union (if available)
///
/// If Neo4j is None, falls back to SQLite-only traversal.
///
/// # Arguments
/// * `db` - SQLite connection
/// * `neo4j_opt` - Optional Neo4j client
/// * `entity_id` - Starting entity ID
/// * `max_depth` - Maximum traversal depth
///
/// # Returns
/// MultiHopResult with all discovered nodes from both data sources
///
/// # Safety
/// - Uses visited set to prevent cycles
/// - Applies branch limit of MAX_NEIGHBORS_PER_NODE per node
/// - Stops exactly at max_depth
/// - Deduplicates neighbors from SQLite and Neo4j
pub async fn multi_hop(
    db: &Connection,
    neo4j_opt: Option<&Neo4jClient>,
    entity_id: i64,
    max_depth: usize,
) -> Result<MultiHopResult> {
    let mut result = MultiHopResult::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    // Start with the initial node
    let start_node = MultiHopNode {
        id: entity_id,
        depth: 0,
        edge_type: None,
    };
    result.add_node(start_node);
    visited.insert(entity_id);
    queue.push_back((entity_id, 0));

    while let Some((current_id, current_depth)) = queue.pop_front() {
        // Stop if we've reached max depth
        if current_depth >= max_depth {
            continue;
        }

        // Get neighbors from SQLite
        let mut all_neighbors = neighbors_sqlite(db, current_id)?;

        // Union with Neo4j neighbors if available
        if let Some(neo4j) = neo4j_opt {
            let neo4j_neighbors = neighbors_neo4j(neo4j, current_id).await?;

            // Add Neo4j neighbors that aren't already in SQLite neighbors
            let sqlite_ids: HashSet<i64> = all_neighbors.iter().map(|(id, _)| *id).collect();
            for (neighbor_id, edge_type) in neo4j_neighbors {
                if !sqlite_ids.contains(&neighbor_id) {
                    all_neighbors.push((neighbor_id, edge_type));
                }
            }

            // Re-sort after union for deterministic ordering
            all_neighbors.sort_by_key(|(id, _)| *id);
        }

        // Apply branch limit: only take first MAX_NEIGHBORS_PER_NODE neighbors
        let neighbors_to_explore = all_neighbors
            .into_iter()
            .take(MAX_NEIGHBORS_PER_NODE)
            .collect::<Vec<_>>();

        for (neighbor_id, edge_type) in neighbors_to_explore {
            // Skip if already visited (cycle detection)
            if visited.contains(&neighbor_id) {
                continue;
            }

            visited.insert(neighbor_id);

            let neighbor_node = MultiHopNode {
                id: neighbor_id,
                depth: current_depth + 1,
                edge_type: Some(edge_type),
            };
            result.add_node(neighbor_node);
            queue.push_back((neighbor_id, current_depth + 1));
        }
    }

    result.sort();
    Ok(result)
}

impl EdgeType {
    /// Parse EdgeType from string (for database queries)
    fn from_str(s: &str) -> Self {
        match s {
            "calls" => EdgeType::Calls,
            "imports" => EdgeType::Imports,
            "contains" => EdgeType::Contains,
            "references" => EdgeType::References,
            "uses" => EdgeType::Uses,
            "inherits" => EdgeType::Inherits,
            "implements" => EdgeType::Implements,
            "uses_field" => EdgeType::UsesField,
            "uses_type" => EdgeType::UsesType,
            "module_child" => EdgeType::ModuleChild,
            _ => EdgeType::Uses, // Default fallback
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_hop_result_sort() {
        let mut result = MultiHopResult::new();
        result.add_node(MultiHopNode {
            id: 3,
            depth: 1,
            edge_type: Some(EdgeType::Calls),
        });
        result.add_node(MultiHopNode {
            id: 1,
            depth: 0,
            edge_type: None,
        });
        result.add_node(MultiHopNode {
            id: 2,
            depth: 1,
            edge_type: Some(EdgeType::Uses),
        });

        result.sort();

        assert_eq!(result.nodes[0].id, 1); // depth 0
        assert_eq!(result.nodes[1].id, 2); // depth 1, id 2
        assert_eq!(result.nodes[2].id, 3); // depth 1, id 3
    }
}
