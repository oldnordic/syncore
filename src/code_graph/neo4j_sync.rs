//! Neo4j Relationship Sync Module
//!
//! This module provides post-index synchronization of code edges from SQLite to Neo4j.
//! After indexing code files, this allows batch creation of relationships in the graph database.
//!
//! Features:
//! - Reads all edges from SQLite code_edges table
//! - Creates corresponding Neo4j relationships using MERGE (idempotent)
//! - Supports optional filtering by limit
//! - Returns detailed summary of sync operations
//!
//! Usage:
//! ```rust
//! let summary = sync_relationships_to_neo4j(&db_conn, &neo4j, None, None).await?;
//! println!("Synced {} edges, created {} relationships",
//!          summary.edges_processed, summary.edges_created);
//! ```

use super::neo4j_relationships::create_code_relationship;
use super::types::{CodeEdge, EdgeType};
use crate::graph::Neo4jClient;
use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// Summary of Neo4j sync operation (entities and relationships)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Neo4jSyncSummary {
    /// Total number of entities processed from SQLite
    #[serde(default)]
    pub entities_processed: u64,
    /// Number of entity nodes created in Neo4j
    #[serde(default)]
    pub entities_created: u64,
    /// Number of entities skipped
    #[serde(default)]
    pub entities_skipped: u64,
    /// Total number of edges processed from SQLite
    pub edges_processed: u64,
    /// Number of relationships created in Neo4j
    pub edges_created: u64,
    /// Number of edges skipped (already exist or nodes missing)
    pub edges_skipped: u64,
}

/// Sync entities from SQLite code_entities to Neo4j nodes
///
/// This function reads all entities from the code_entities table and creates
/// corresponding nodes in Neo4j. Should be called BEFORE sync_relationships_to_neo4j
/// to ensure all nodes exist before creating relationships.
///
/// # Arguments
/// * `db_conn` - SQLite database connection (wrapped in Arc<Mutex>)
/// * `neo4j` - Neo4j client connection
/// * `namespace` - Optional namespace filter (currently unused, reserved for future)
/// * `limit` - Optional limit on number of entities to process
///
/// # Returns
/// Summary of sync operation with counts of processed/created/skipped entities
pub async fn sync_entities_to_neo4j(
    db_conn: &Arc<Mutex<Connection>>,
    neo4j: &Neo4jClient,
    _namespace: Option<&str>,
    limit: Option<u64>,
) -> Result<Neo4jSyncSummary> {
    let mut summary = Neo4jSyncSummary {
        entities_processed: 0,
        entities_created: 0,
        entities_skipped: 0,
        edges_processed: 0,
        edges_created: 0,
        edges_skipped: 0,
    };

    // Fetch entities from SQLite
    let entities = fetch_entities_from_sqlite(db_conn, limit)?;

    // Import neo4j_writer to access create_code_entity_node
    use super::neo4j_writer::create_code_entity_node;

    // Process each entity
    for (entity_id, entity) in entities {
        summary.entities_processed += 1;

        // Try to create node in Neo4j
        match create_code_entity_node(neo4j, entity_id, &entity).await {
            Ok(_) => {
                summary.entities_created += 1;
            }
            Err(e) => {
                // Log error but continue processing (best-effort)
                eprintln!(
                    "[Neo4jSync] Failed to create node for entity {}: {}",
                    entity.name, e
                );
                summary.entities_skipped += 1;
            }
        }
    }

    Ok(summary)
}

/// Sync relationships from SQLite code_edges to Neo4j
///
/// This function reads all edges from the code_edges table and creates
/// corresponding relationships in Neo4j using the existing relationship sync logic.
///
/// IMPORTANT: Call sync_entities_to_neo4j() FIRST to ensure all nodes exist
/// before creating relationships between them.
///
/// # Arguments
/// * `db_conn` - SQLite database connection (wrapped in Arc<Mutex>)
/// * `neo4j` - Neo4j client connection
/// * `namespace` - Optional namespace filter (currently unused, reserved for future)
/// * `limit` - Optional limit on number of edges to process
///
/// # Returns
/// Summary of sync operation with counts of processed/created/skipped edges
pub async fn sync_relationships_to_neo4j(
    db_conn: &Arc<Mutex<Connection>>,
    neo4j: &Neo4jClient,
    _namespace: Option<&str>,
    limit: Option<u64>,
) -> Result<Neo4jSyncSummary> {
    let mut summary = Neo4jSyncSummary {
        entities_processed: 0,
        entities_created: 0,
        entities_skipped: 0,
        edges_processed: 0,
        edges_created: 0,
        edges_skipped: 0,
    };

    // Fetch edges from SQLite
    let edges = fetch_edges_from_sqlite(db_conn, limit)?;

    // Process each edge
    for edge in edges {
        summary.edges_processed += 1;

        // Try to create relationship in Neo4j
        match create_code_relationship(neo4j, &edge).await {
            Ok(_) => {
                summary.edges_created += 1;
            }
            Err(e) => {
                // Log error but continue processing
                eprintln!(
                    "[Neo4jSync] Failed to create relationship for edge {:?}: {}",
                    edge, e
                );
                summary.edges_skipped += 1;
            }
        }
    }

    Ok(summary)
}

/// Fetch edges from SQLite code_edges table
///
/// # Arguments
/// * `db_conn` - SQLite database connection
/// * `limit` - Optional limit on number of edges to fetch
///
/// # Returns
/// Vector of CodeEdge structs
fn fetch_edges_from_sqlite(
    db_conn: &Arc<Mutex<Connection>>,
    limit: Option<u64>,
) -> Result<Vec<CodeEdge>> {
    let conn = db_conn
        .lock()
        .map_err(|e| anyhow::anyhow!("Failed to lock database connection: {}", e))?;

    let query = if let Some(lim) = limit {
        format!(
            "SELECT src_entity_id, dst_entity_id, edge_type FROM code_edges LIMIT {}",
            lim
        )
    } else {
        "SELECT src_entity_id, dst_entity_id, edge_type FROM code_edges".to_string()
    };

    let mut stmt = conn.prepare(&query)?;
    let edge_iter = stmt.query_map([], |row| {
        let src_id: i64 = row.get(0)?;
        let dst_id: i64 = row.get(1)?;
        let edge_type_str: String = row.get(2)?;

        let edge_type = match edge_type_str.as_str() {
            "calls" => EdgeType::Calls,
            "imports" => EdgeType::Imports,
            "inherits" => EdgeType::Inherits,
            "references" => EdgeType::References,
            "uses" => EdgeType::Uses,
            "contains" => EdgeType::Contains,
            _ => EdgeType::References, // Default fallback
        };

        Ok(CodeEdge {
            src_entity_id: src_id,
            dst_entity_id: dst_id,
            edge_type,
        })
    })?;

    let mut edges = Vec::new();
    for edge_result in edge_iter {
        edges.push(edge_result?);
    }

    Ok(edges)
}

/// Fetch entities from SQLite code_entities table
///
/// # Arguments
/// * `db_conn` - SQLite database connection
/// * `limit` - Optional limit on number of entities to fetch
///
/// # Returns
/// Vector of (entity_id, CodeEntity) tuples
fn fetch_entities_from_sqlite(
    db_conn: &Arc<Mutex<Connection>>,
    limit: Option<u64>,
) -> Result<Vec<(i64, super::types::CodeEntity)>> {
    use super::types::{CodeEntity, EntityType};

    let conn = db_conn
        .lock()
        .map_err(|e| anyhow::anyhow!("Failed to lock database connection: {}", e))?;

    let query = if let Some(lim) = limit {
        format!(
            "SELECT id, file_path, entity_type, name, signature, line_start, line_end, docstring, language,
                    created_at, last_modified_at, change_count, author_count
             FROM code_entities
             LIMIT {}",
            lim
        )
    } else {
        "SELECT id, file_path, entity_type, name, signature, line_start, line_end, docstring, language,
                created_at, last_modified_at, change_count, author_count
         FROM code_entities".to_string()
    };

    let mut stmt = conn.prepare(&query)?;
    let entity_iter = stmt.query_map([], |row| {
        let id: i64 = row.get(0)?;
        let file_path: String = row.get(1)?;
        let entity_type_str: String = row.get(2)?;
        let name: String = row.get(3)?;
        let signature: Option<String> = row.get(4)?;
        let line_start: i64 = row.get(5)?;
        let line_end: i64 = row.get(6)?;
        let docstring: Option<String> = row.get(7)?;
        let language: String = row.get(8)?;
        let created_at: Option<i64> = row.get(9)?;
        let last_modified_at: Option<i64> = row.get(10)?;
        let change_count: Option<i32> = row.get(11)?;
        let author_count: Option<i32> = row.get(12)?;

        // Parse entity type
        let entity_type = match entity_type_str.as_str() {
            "function" => EntityType::Function,
            "class" => EntityType::Class,
            "method" => EntityType::Method,
            "import" => EntityType::Import,
            "struct" => EntityType::Struct,
            "enum" => EntityType::Enum,
            "trait" => EntityType::Trait,
            _ => EntityType::Function, // Default fallback
        };

        let entity = CodeEntity {
            id: Some(id),
            file_path,
            entity_type,
            name,
            signature,
            line_start: line_start as usize,
            line_end: line_end as usize,
            docstring,
            language,
            body_snippet: None,
            created_at,
            last_modified_at,
            change_count,
            author_count,
        };

        Ok((id, entity))
    })?;

    let mut entities = Vec::new();
    for entity_result in entity_iter {
        entities.push(entity_result?);
    }

    Ok(entities)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_summary_serialization() -> Result<()> {
        let summary = Neo4jSyncSummary {
            entities_processed: 5,
            entities_created: 5,
            entities_skipped: 0,
            edges_processed: 10,
            edges_created: 8,
            edges_skipped: 2,
        };

        let json = serde_json::to_string(&summary)?;
        assert!(json.contains("entities_processed"));
        assert!(json.contains("edges_processed"));
        assert!(json.contains("\"edges_created\":8"));

        let deserialized: Neo4jSyncSummary = serde_json::from_str(&json)?;
        assert_eq!(deserialized.entities_processed, 5);
        assert_eq!(deserialized.entities_created, 5);
        assert_eq!(deserialized.edges_processed, 10);
        assert_eq!(deserialized.edges_created, 8);

        Ok(())
    }
}
