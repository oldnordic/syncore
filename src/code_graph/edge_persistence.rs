//! PHASE 2: Semantic Edge Persistence
//!
//! This module implements dual-write persistence of semantic edges to both:
//! 1. SQLite code_edges table (for SQL queries and join operations)
//! 2. Neo4j relationships (for graph traversal and multi-hop queries)
//!
//! All upsert methods are idempotent and can be called multiple times safely.

use super::graph::CodeGraph;
use super::types::{CodeEdge, EdgeType};
use anyhow::{anyhow, Result};

impl CodeGraph {
    /// Upsert a CALLS edge between two functions
    ///
    /// Persists to BOTH SQLite and Neo4j (if available).
    /// Idempotent: can be called multiple times for the same edge.
    ///
    /// # Arguments
    /// * `caller_id` - Entity ID of the calling function
    /// * `callee_id` - Entity ID of the called function
    #[allow(clippy::similar_names)]
    pub async fn upsert_call_edge(&self, caller_id: i64, callee_id: i64) -> Result<()> {
        self.upsert_edge(caller_id, callee_id, EdgeType::Calls).await
    }

    /// Upsert an IMPLEMENTS edge between a type and a trait
    ///
    /// Persists to BOTH SQLite and Neo4j (if available).
    /// Idempotent: can be called multiple times for the same edge.
    ///
    /// # Arguments
    /// * `type_id` - Entity ID of the implementing type/struct
    /// * `trait_id` - Entity ID of the trait being implemented
    pub async fn upsert_implements_edge(&self, type_id: i64, trait_id: i64) -> Result<()> {
        self.upsert_edge(type_id, trait_id, EdgeType::Implements).await
    }

    /// Upsert a USES_FIELD edge for struct field access
    ///
    /// Persists to BOTH SQLite and Neo4j (if available).
    /// Idempotent: can be called multiple times for the same edge.
    ///
    /// # Arguments
    /// * `accessor_id` - Entity ID of the function/method accessing the field
    /// * `struct_id` - Entity ID of the struct containing the field
    pub async fn upsert_field_edge(&self, accessor_id: i64, struct_id: i64) -> Result<()> {
        self.upsert_edge(accessor_id, struct_id, EdgeType::UsesField).await
    }

    /// Upsert a USES_TYPE edge for type usage
    ///
    /// Persists to BOTH SQLite and Neo4j (if available).
    /// Idempotent: can be called multiple times for the same edge.
    ///
    /// # Arguments
    /// * `user_id` - Entity ID of the function/struct using the type
    /// * `type_id` - Entity ID of the type being used
    pub async fn upsert_type_usage_edge(&self, user_id: i64, type_id: i64) -> Result<()> {
        self.upsert_edge(user_id, type_id, EdgeType::UsesType).await
    }

    /// Upsert a MODULE_CHILD edge for module hierarchy
    ///
    /// Persists to BOTH SQLite and Neo4j (if available).
    /// Idempotent: can be called multiple times for the same edge.
    ///
    /// # Arguments
    /// * `parent_id` - Entity ID of the parent module
    /// * `child_id` - Entity ID of the child module
    pub async fn upsert_module_child_edge(&self, parent_id: i64, child_id: i64) -> Result<()> {
        self.upsert_edge(parent_id, child_id, EdgeType::ModuleChild).await
    }

    /// Internal helper: Upsert edge to both SQLite and Neo4j
    ///
    /// This is the core dual-write implementation that all public upsert methods use.
    ///
    /// # Steps:
    /// 1. Insert/update edge in SQLite code_edges table
    /// 2. If Neo4j is available, create relationship in Neo4j
    async fn upsert_edge(
        &self,
        src_entity_id: i64,
        dst_entity_id: i64,
        edge_type: EdgeType,
    ) -> Result<()> {
        // Step 1: Upsert to SQLite
        self.upsert_edge_sqlite(src_entity_id, dst_entity_id, &edge_type)?;

        // Step 2: If Neo4j available, create relationship
        if let Ok(neo4j) = self.neo4j_client() {
            let edge = CodeEdge {
                src_entity_id,
                dst_entity_id,
                edge_type,
            };
            super::neo4j_relationships::create_code_relationship(neo4j, &edge).await?;
        }

        Ok(())
    }

    /// Upsert edge to SQLite code_edges table
    ///
    /// Uses INSERT OR REPLACE for idempotency.
    fn upsert_edge_sqlite(
        &self,
        src_entity_id: i64,
        dst_entity_id: i64,
        edge_type: &EdgeType,
    ) -> Result<()> {
        let db = self.db.lock().map_err(|e| anyhow!("Failed to lock database: {}", e))?;

        let edge_type_str = match edge_type {
            EdgeType::Calls => "calls",
            EdgeType::Imports => "imports",
            EdgeType::Inherits => "inherits",
            EdgeType::References => "references",
            EdgeType::Uses => "uses",
            EdgeType::Contains => "contains",
            EdgeType::UsesField => "uses_field",
            EdgeType::Implements => "implements",
            EdgeType::UsesType => "uses_type",
            EdgeType::ModuleChild => "module_child",
        };

        db.execute(
            "INSERT OR REPLACE INTO code_edges (src_entity_id, dst_entity_id, edge_type)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![src_entity_id, dst_entity_id, edge_type_str],
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector::{HuggingFaceEmbeddings, VectorStore};
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_upsert_edge_sqlite_only() -> Result<()> {
        let db_path = "/tmp/test_edge_persistence.db";
        let _ = std::fs::remove_file(db_path);

        let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
        let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

        let graph = CodeGraph::new(db_path, vector_store)?;

        // Insert edge (without Neo4j)
        graph.upsert_edge_sqlite(1, 2, &EdgeType::Calls)?;

        // Verify edge exists
        let db = graph.db.lock().unwrap();
        let count: i64 = db.query_row(
            "SELECT COUNT(*) FROM code_edges WHERE src_entity_id = 1 AND dst_entity_id = 2 AND edge_type = 'calls'",
            [],
            |row| row.get(0),
        )?;

        assert_eq!(count, 1, "Edge should be persisted to SQLite");

        Ok(())
    }
}
