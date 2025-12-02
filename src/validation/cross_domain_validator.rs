//! Cross-Domain Consistency Validator
//!
//! Phase 5: Cross-domain consistency validation layer
//! Detects and prevents desynchronization between:
//! - CodeGraph (SQLite: code_entities + code_edges)
//! - VectorStore (HNSW indices + embeddings table)
//! - MemoryStore (memory table, sled cache)
//! - Neo4j graph (optional: skip gracefully if unavailable)
//!
//! This validator ONLY REPORTS issues - it does NOT fix them.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::HashSet;
use std::path::Path;

use crate::db::manager::DbManager;
use crate::graph::Neo4jClient;
use crate::memory::Memory;
use crate::vector::VectorStore;

/// Cross-domain consistency report
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CrossDomainReport {
    /// Code entities that exist but have no corresponding vector embedding
    pub missing_nodes: Vec<String>,

    /// Vector embeddings that exist but have no corresponding code entity
    pub orphan_vectors: Vec<String>,

    /// Memory entries that exist but have no corresponding vector
    pub memory_without_vectors: Vec<String>,

    /// Vectors that exist but have no corresponding memory entry
    pub vectors_without_memory: Vec<String>,

    /// Edges referencing missing entities (SQLite)
    pub dangling_edges: Vec<String>,

    /// Corrupted HNSW snapshot files
    pub corrupted_snapshots: Vec<String>,

    /// Checksum mismatches between file content and stored hashes
    pub mismatched_checksums: Vec<String>,

    /// Neo4j nodes missing compared to SQLite (if Neo4j available)
    pub neo4j_missing_nodes: Vec<String>,

    /// Neo4j relationships missing compared to SQLite (if Neo4j available)
    pub neo4j_missing_relationships: Vec<String>,
}

impl CrossDomainReport {
    /// Check if any consistency issues were found
    pub fn has_issues(&self) -> bool {
        !self.missing_nodes.is_empty()
            || !self.orphan_vectors.is_empty()
            || !self.memory_without_vectors.is_empty()
            || !self.vectors_without_memory.is_empty()
            || !self.dangling_edges.is_empty()
            || !self.corrupted_snapshots.is_empty()
            || !self.mismatched_checksums.is_empty()
            || !self.neo4j_missing_nodes.is_empty()
            || !self.neo4j_missing_relationships.is_empty()
    }

    /// Get total count of all issues
    pub fn total_issues(&self) -> usize {
        self.missing_nodes.len()
            + self.orphan_vectors.len()
            + self.memory_without_vectors.len()
            + self.vectors_without_memory.len()
            + self.dangling_edges.len()
            + self.corrupted_snapshots.len()
            + self.mismatched_checksums.len()
            + self.neo4j_missing_nodes.len()
            + self.neo4j_missing_relationships.len()
    }
}

/// Cross-domain consistency validator
pub struct CrossDomainValidator<'a> {
    db_manager: &'a DbManager,
    vector_store: &'a VectorStore,
    memory: Option<&'a Memory>,
    neo4j: Option<&'a Neo4jClient>,
}

impl<'a> CrossDomainValidator<'a> {
    /// Create new validator
    pub fn new(
        db_manager: &'a DbManager,
        vector_store: &'a VectorStore,
        memory: Option<&'a Memory>,
    ) -> Self {
        Self {
            db_manager,
            vector_store,
            memory,
            neo4j: None, // TODO: Add Neo4j client when available
        }
    }

    /// Create validator with Neo4j client
    pub fn with_neo4j(
        db_manager: &'a DbManager,
        vector_store: &'a VectorStore,
        memory: Option<&'a Memory>,
        neo4j: &'a Neo4jClient,
    ) -> Self {
        Self {
            db_manager,
            vector_store,
            memory,
            neo4j: Some(neo4j),
        }
    }

    /// Validate code entities vs vector embeddings
    pub fn validate_code_vs_vector(&self) -> Result<(Vec<String>, Vec<String>)> {
        let mut missing_entities = Vec::new();
        let mut orphan_vectors = Vec::new();

        // Get all code entities
        let conn = self.db_manager.code_graph_conn();
        let conn_lock = conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock code_graph database: {}", e))?;

        let mut stmt = conn_lock.prepare("SELECT id, file_path, name FROM code_entities")?;

        let entity_rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        })?;

        let mut entity_ids = HashSet::new();
        for row_result in entity_rows {
            let (id, _file_path, _name) = row_result?;
            entity_ids.insert(id);
        }

        // Get all vector IDs from vector store
        let vector_ids: HashSet<i64> =
            self.vector_store.get_vectors().iter().map(|(id, _, _, _)| *id).collect();

        // Find missing entities (vectors without entities)
        for &vector_id in &vector_ids {
            if !entity_ids.contains(&vector_id) {
                orphan_vectors.push(format!("vector_id={} has no code entity", vector_id));
            }
        }

        // Find missing vectors (entities without vectors)
        for &entity_id in &entity_ids {
            if !vector_ids.contains(&entity_id) {
                missing_entities.push(format!("entity_id={} has no vector embedding", entity_id));
            }
        }

        Ok((missing_entities, orphan_vectors))
    }

    /// Validate vector embeddings vs memory entries
    pub fn validate_vector_vs_memory(&self) -> Result<(Vec<String>, Vec<String>)> {
        let mut memory_without_vectors = Vec::new();
        let mut vectors_without_memory = Vec::new();

        if let Some(_memory) = self.memory {
            // Check if embedding_id column exists
            let conn = self.db_manager.main_conn();
            let conn_lock =
                conn.lock().map_err(|e| anyhow::anyhow!("Failed to lock main database: {}", e))?;

            // Check if embedding_id column exists
            let has_embedding_id: bool = conn_lock
                .query_row(
                    "SELECT COUNT(*) FROM pragma_table_info('memory') WHERE name = 'embedding_id'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap_or(0)
                > 0;

            if has_embedding_id {
                // Get all memory entries with embedding_id
                let mut stmt =
                    conn_lock.prepare("SELECT id, k FROM memory WHERE embedding_id IS NOT NULL")?;

                let memory_rows = stmt
                    .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))?;

                let mut memory_ids = HashSet::new();
                for row_result in memory_rows {
                    let (id, _key) = row_result?;
                    memory_ids.insert(id);
                }

                // Get all vector IDs from vector store
                let vector_ids: HashSet<i64> =
                    self.vector_store.get_vectors().iter().map(|(id, _, _, _)| *id).collect();

                // Find memory entries without vectors
                for &memory_id in &memory_ids {
                    if !vector_ids.contains(&memory_id) {
                        memory_without_vectors
                            .push(format!("memory_id={} has no vector", memory_id));
                    }
                }

                // Find vectors without memory entries
                for &vector_id in &vector_ids {
                    if !memory_ids.contains(&vector_id) {
                        vectors_without_memory
                            .push(format!("vector_id={} has no memory entry", vector_id));
                    }
                }
            } else {
                // embedding_id column doesn't exist, skip this validation
                memory_without_vectors
                    .push("embedding_id column not found in memory table".to_string());
            }
        }

        Ok((memory_without_vectors, vectors_without_memory))
    }

    /// Validate SQLite vs Neo4j (if available)
    pub async fn validate_sqlite_vs_neo4j(&self) -> Result<(Vec<String>, Vec<String>)> {
        let mut missing_nodes = Vec::new();
        let mut missing_relationships = Vec::new();

        if let Some(neo4j) = self.neo4j {
            // Get SQLite entity count
            let sqlite_count: i64 = {
                let conn = self.db_manager.code_graph_conn();
                let conn_lock = conn
                    .lock()
                    .map_err(|e| anyhow::anyhow!("Failed to lock code_graph database: {}", e))?;

                conn_lock.query_row("SELECT COUNT(*) FROM code_entities", [], |row| row.get(0))?
            };

            // Get Neo4j node count
            let neo4j_query = "MATCH (n:CodeEntity) RETURN count(n) as count";
            let neo4j_result = neo4j.execute_query(neo4j_query, vec![]).await;

            match neo4j_result {
                Ok(rows) => {
                    if let Some(row) = rows.first() {
                        if let Some(count_value) = row.get("count") {
                            if let Some(neo4j_count) = count_value.as_i64() {
                                if neo4j_count < sqlite_count {
                                    missing_nodes.push(format!(
                                        "Neo4j has {} nodes, SQLite has {} entities",
                                        neo4j_count, sqlite_count
                                    ));
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    missing_nodes.push(format!("Failed to query Neo4j: {}", e));
                }
            }

            // Get SQLite edge count
            let sqlite_edge_count: i64 = {
                let conn = self.db_manager.code_graph_conn();
                let conn_lock = conn
                    .lock()
                    .map_err(|e| anyhow::anyhow!("Failed to lock code_graph database: {}", e))?;

                conn_lock.query_row("SELECT COUNT(*) FROM code_edges", [], |row| row.get(0))?
            };

            // Get Neo4j relationship count
            let neo4j_rel_query = "MATCH ()-[r:CODE_RELATION]->() RETURN count(r) as count";
            let neo4j_rel_result = neo4j.execute_query(neo4j_rel_query, vec![]).await;

            match neo4j_rel_result {
                Ok(rows) => {
                    if let Some(row) = rows.first() {
                        if let Some(count_value) = row.get("count") {
                            if let Some(neo4j_rel_count) = count_value.as_i64() {
                                if neo4j_rel_count < sqlite_edge_count {
                                    missing_relationships.push(format!(
                                        "Neo4j has {} relationships, SQLite has {} edges",
                                        neo4j_rel_count, sqlite_edge_count
                                    ));
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    missing_relationships
                        .push(format!("Failed to query Neo4j relationships: {}", e));
                }
            }
        }

        Ok((missing_nodes, missing_relationships))
    }

    /// Validate HNSW snapshot integrity
    pub fn validate_hnsw_snapshot(&self) -> Result<Vec<String>> {
        let mut snapshot_issues = Vec::new();

        // Check snapshot files exist and are valid
        let index_path = self.vector_store.index_path();
        let vectors_path = format!("{}.vectors", index_path);
        let meta_path = format!("{}.meta", index_path);

        // Check vectors file
        if Path::new(&vectors_path).exists() {
            match std::fs::read(&vectors_path) {
                Ok(data) => {
                    // Try to deserialize
                    match bincode::deserialize::<Vec<(i64, Option<i64>, Vec<f32>, String)>>(&data) {
                        Ok(_) => {
                            // Valid vectors file
                        }
                        Err(e) => {
                            snapshot_issues.push(format!("Corrupted vectors file: {}", e));
                        }
                    }
                }
                Err(e) => {
                    snapshot_issues.push(format!("Cannot read vectors file: {}", e));
                }
            }
        }

        // Check meta file
        if Path::new(&meta_path).exists() {
            match std::fs::read(&meta_path) {
                Ok(data) => {
                    // Try to deserialize
                    match bincode::deserialize::<crate::vector::VectorMeta>(&data) {
                        Ok(_) => {
                            // Valid meta file
                        }
                        Err(e) => {
                            snapshot_issues.push(format!("Corrupted meta file: {}", e));
                        }
                    }
                }
                Err(e) => {
                    snapshot_issues.push(format!("Cannot read meta file: {}", e));
                }
            }
        }

        // Check HNSW index files
        let hnsw_data_path = format!("{}.data", index_path);
        let hnsw_graph_path = format!("{}.graph", index_path);

        for path in [&hnsw_data_path, &hnsw_graph_path] {
            if Path::new(path).exists() {
                match std::fs::metadata(path) {
                    Ok(metadata) => {
                        if metadata.len() == 0 {
                            snapshot_issues.push(format!("Empty HNSW file: {}", path));
                        }
                    }
                    Err(e) => {
                        snapshot_issues.push(format!("Cannot access HNSW file {}: {}", path, e));
                    }
                }
            }
        }

        Ok(snapshot_issues)
    }

    /// Validate checksum consistency by comparing file content with stored hashes
    pub fn validate_checksum_consistency(&self) -> Result<Vec<String>> {
        let mut checksum_issues = Vec::new();

        // Get all entities with file paths and checksums
        let conn = self.db_manager.code_graph_conn();
        let conn_lock = conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock code_graph database: {}", e))?;

        // Check if checksum column exists
        let has_checksum_column: bool = conn_lock
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('code_entities') WHERE name = 'checksum'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0;

        if !has_checksum_column {
            // Checksum column doesn't exist yet, skip validation
            return Ok(checksum_issues);
        }

        let mut stmt = conn_lock.prepare(
            "SELECT id, file_path, checksum FROM code_entities 
             WHERE file_path IS NOT NULL AND checksum IS NOT NULL",
        )?;

        let entity_rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<String>>(2)?))
        })?;

        for row_result in entity_rows {
            let (entity_id, file_path, stored_checksum) = row_result?;

            // Try to read file and compute SHA256
            match std::fs::read_to_string(&file_path) {
                Ok(content) => {
                    use sha2::Sha256;

                    let mut hasher = Sha256::new();
                    hasher.update(content.as_bytes());
                    let computed_checksum = format!("{:x}", hasher.finalize());

                    if let Some(stored) = stored_checksum {
                        if computed_checksum != stored {
                            checksum_issues.push(format!(
                                "Entity {}: checksum mismatch. Stored: {}, Computed: {}",
                                entity_id, stored, computed_checksum
                            ));
                        }
                    }
                }
                Err(e) => {
                    checksum_issues.push(format!(
                        "Entity {}: cannot read file '{}': {}",
                        entity_id, file_path, e
                    ));
                }
            }
        }

        Ok(checksum_issues)
    }

    /// Run full consistency scan across all domains
    pub async fn run_full_consistency_scan(&self) -> Result<CrossDomainReport> {
        let mut report = CrossDomainReport::default();

        // Validate code vs vector
        let (missing_entities, orphan_vectors) = self.validate_code_vs_vector()?;
        report.missing_nodes = missing_entities;
        report.orphan_vectors = orphan_vectors;

        // Validate vector vs memory
        let (memory_without_vectors, vectors_without_memory) = self.validate_vector_vs_memory()?;
        report.memory_without_vectors = memory_without_vectors;
        report.vectors_without_memory = vectors_without_memory;

        // Validate dangling edges
        report.dangling_edges = self.validate_dangling_edges()?;

        // Validate HNSW snapshot
        report.corrupted_snapshots = self.validate_hnsw_snapshot()?;

        // Validate checksum consistency
        report.mismatched_checksums = self.validate_checksum_consistency()?;

        // Validate SQLite vs Neo4j (if available)
        let (neo4j_missing_nodes, neo4j_missing_relationships) =
            self.validate_sqlite_vs_neo4j().await?;
        report.neo4j_missing_nodes = neo4j_missing_nodes;
        report.neo4j_missing_relationships = neo4j_missing_relationships;

        Ok(report)
    }

    /// Validate dangling edges in SQLite
    pub fn validate_dangling_edges(&self) -> Result<Vec<String>> {
        let mut dangling_edges = Vec::new();

        let conn = self.db_manager.code_graph_conn();
        let conn_lock = conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock code_graph database: {}", e))?;

        // Get all entity IDs
        let mut stmt = conn_lock.prepare("SELECT id FROM code_entities")?;
        let entity_rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;

        let mut entity_ids = HashSet::new();
        for row_result in entity_rows {
            entity_ids.insert(row_result?);
        }

        // Check all edges
        let mut stmt =
            conn_lock.prepare("SELECT src_entity_id, dst_entity_id, edge_type FROM code_edges")?;

        let edge_rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?))
        })?;

        for row_result in edge_rows {
            let (src_id, dst_id, edge_type) = row_result?;

            if !entity_ids.contains(&src_id) {
                dangling_edges.push(format!(
                    "Edge {}->{} ({}) references missing source entity {}",
                    src_id, dst_id, edge_type, src_id
                ));
            }

            if !entity_ids.contains(&dst_id) {
                dangling_edges.push(format!(
                    "Edge {}->{} ({}) references missing destination entity {}",
                    src_id, dst_id, edge_type, dst_id
                ));
            }
        }

        Ok(dangling_edges)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross_domain_report_default() {
        let report = CrossDomainReport::default();
        assert!(!report.has_issues());
        assert_eq!(report.total_issues(), 0);
    }

    #[test]
    fn test_cross_domain_report_has_issues() {
        let mut report = CrossDomainReport::default();
        report.missing_nodes.push("test".to_string());
        assert!(report.has_issues());
        assert_eq!(report.total_issues(), 1);
    }

    #[test]
    fn test_cross_domain_report_total_issues() {
        let mut report = CrossDomainReport::default();
        report.missing_nodes.push("issue1".to_string());
        report.orphan_vectors.push("issue2".to_string());
        report.dangling_edges.push("issue3".to_string());
        assert_eq!(report.total_issues(), 3);
    }
}
