//! Relationship Parity Tests
//!
//! TDD-driven tests for CRUD operations parity between Neo4j and SQLiteGraph backends.
//! Tests create N entities + M relationships and verify identical behavior.

use anyhow::Result;
use std::collections::HashMap;
use syncore::graph::{GraphBackend, Neo4jBackend, SQLiteGraphBackend};
use syncore::graph::{NodeLabel, NodeProperties, RelationType, EntityResult};
use tempfile::TempDir;
use tokio;

/// Test setup for dual backend comparison
struct DualBackendSetup {
    neo4j: Neo4jBackend,
    sqlite: SQLiteGraphBackend,
    _temp_dir: TempDir,
}

impl DualBackendSetup {
    async fn new() -> Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        let db_path = temp_dir.path().join("test.db");
        
        // Setup SQLite backend
        let sqlite = SQLiteGraphBackend::connect(
            db_path.to_str().unwrap(),
            "",
            "",
            "code_syncore_default"
        ).await?;
        
        // Setup Neo4j backend (using test configuration)
        let neo4j = Neo4jBackend::connect(
            "bolt://127.0.0.1:7687",
            "neo4j",
            "test_password",
            "code_syncore_default"
        ).await?;
        
        Ok(Self {
            neo4j,
            sqlite,
            _temp_dir: temp_dir,
        })
    }
    
    /// Create test entities on both backends
    async fn create_test_entities(&self, count: usize) -> Result<Vec<i64>> {
        let mut entity_ids = Vec::new();
        
        for i in 1..=count {
            let props = NodeProperties {
                id: i as i64,
                name: format!("entity_{}", i),
                path: Some(format!("/src/test_{}.rs", i)),
                start_line: Some(i as i64),
                end_line: Some((i + 10) as i64),
                signature: Some(format!("fn entity_{}() {{}}", i)),
                body_snippet: Some(format!("// Entity {} body", i)),
                docstring: Some(format!("Entity {} documentation", i)),
                hash: Some(format!("hash_{}", i)),
                language: Some("rust".to_string()),
                file_sha256: Some(format!("file_hash_{}", i)),
                mtime: Some((i * 1000) as i64),
                created_at: Some(format!("2024-01-{:02}T00:00:00Z", i % 28 + 1)),
                last_modified_at: Some(format!("2024-12-{:02}T00:00:00Z", i % 28 + 1)),
                change_count: Some(i as i64),
                author_count: Some((i % 3 + 1) as i64),
            };
            
            // Create on both backends
            self.neo4j.upsert_entity(NodeLabel::Function, props.clone()).await?;
            self.sqlite.upsert_entity(NodeLabel::Function, props).await?;
            
            entity_ids.push(i as i64);
        }
        
        Ok(entity_ids)
    }
    
    /// Create test relationships on both backends
    async fn create_test_relationships(
        &self,
        entity_ids: &[i64],
        relationship_count: usize,
    ) -> Result<Vec<(i64, i64, RelationType)>> {
        let mut relationships = Vec::new();
        
        for i in 0..relationship_count {
            let src_idx = i % entity_ids.len();
            let dst_idx = (i + 1) % entity_ids.len();
            let rel_type = match i % 4 {
                0 => RelationType::Calls,
                1 => RelationType::Contains,
                2 => RelationType::DependsOn,
                _ => RelationType::Uses,
            };
            
            let src_id = entity_ids[src_idx];
            let dst_id = entity_ids[dst_idx];
            
            // Create on both backends
            self.neo4j.create_relationship(src_id, dst_id, rel_type).await?;
            self.sqlite.create_relationship(src_id, dst_id, rel_type).await?;
            
            relationships.push((src_id, dst_id, rel_type));
        }
        
        Ok(relationships)
    }
}

/// Compare EntityResults from both backends for exact parity
fn compare_entity_results(
    neo4j_results: &[EntityResult],
    sqlite_results: &[EntityResult],
) -> Result<bool> {
    if neo4j_results.len() != sqlite_results.len() {
        println!(
            "Count mismatch: Neo4j={}, SQLite={}",
            neo4j_results.len(),
            sqlite_results.len()
        );
        return Ok(false);
    }
    
    for (i, (neo4j_entity, sqlite_entity)) in neo4j_results.iter().zip(sqlite_results.iter()).enumerate() {
        if neo4j_entity.id != sqlite_entity.id {
            println!("ID mismatch at index {}: Neo4j={}, SQLite={}", i, neo4j_entity.id, sqlite_entity.id);
            return Ok(false);
        }
        
        if neo4j_entity.name != sqlite_entity.name {
            println!("Name mismatch at index {}: Neo4j={}, SQLite={}", i, neo4j_entity.name, sqlite_entity.name);
            return Ok(false);
        }
        
        if neo4j_entity.label != sqlite_entity.label {
            println!("Label mismatch at index {}: Neo4j={}, SQLite={}", i, neo4j_entity.label, sqlite_entity.label);
            return Ok(false);
        }
        
        if neo4j_entity.path != sqlite_entity.path {
            println!("Path mismatch at index {}: Neo4j={:?}, SQLite={:?}", i, neo4j_entity.path, sqlite_entity.path);
            return Ok(false);
        }
    }
    
    Ok(true)
}

/// Test 1: Basic CRUD operations parity
#[tokio::test]
#[ignore] // Ignore until Neo4j is available for CI
async fn test_basic_crud_parity() -> Result<()> {
    let setup = DualBackendSetup::new().await?;
    
    // Create entities
    let entity_ids = setup.create_test_entities(5).await?;
    
    // Verify entity retrieval parity
    for &id in &entity_ids {
        let neo4j_entity = setup.neo4j.get_entity_by_id(id).await?;
        let sqlite_entity = setup.sqlite.get_entity_by_id(id).await?;
        
        match (neo4j_entity, sqlite_entity) {
            (Some(neo4j), Some(sqlite)) => {
                assert_eq!(neo4j.id, sqlite.id, "Entity ID mismatch for ID {}", id);
                assert_eq!(neo4j.name, sqlite.name, "Entity name mismatch for ID {}", id);
                assert_eq!(neo4j.label, sqlite.label, "Entity label mismatch for ID {}", id);
            }
            (None, None) => {} // Both missing - OK
            _ => panic!("Entity presence mismatch for ID {}", id),
        }
    }
    
    // Create relationships
    let relationships = setup.create_test_relationships(&entity_ids, 8).await?;
    
    // Verify relationship operations through neighbor queries
    for &id in &entity_ids {
        let neo4j_neighbors = setup.neo4j.get_neighbors(id).await?;
        let sqlite_neighbors = setup.sqlite.get_neighbors(id).await?;
        
        assert!(
            compare_entity_results(&neo4j_neighbors, &sqlite_neighbors)?,
            "Neighbor parity failed for entity ID {}",
            id
        );
    }
    
    // Test deletion parity
    let delete_id = entity_ids[0];
    setup.neo4j.delete_entity(delete_id).await?;
    setup.sqlite.delete_entity(delete_id).await?;
    
    // Verify deletion
    let neo4j_deleted = setup.neo4j.get_entity_by_id(delete_id).await?;
    let sqlite_deleted = setup.sqlite.get_entity_by_id(delete_id).await?;
    
    assert!(neo4j_deleted.is_none(), "Neo4j entity should be deleted");
    assert!(sqlite_deleted.is_none(), "SQLite entity should be deleted");
    
    Ok(())
}

/// Test 2: Large-scale operations parity
#[tokio::test]
#[ignore]
async fn test_large_scale_operations_parity() -> Result<()> {
    let setup = DualBackendSetup::new().await?;
    
    // Create 100 entities
    let entity_ids = setup.create_test_entities(100).await?;
    
    // Create 500 relationships
    let relationships = setup.create_test_relationships(&entity_ids, 500).await?;
    
    // Verify entity counts by type
    let neo4j_counts = setup.neo4j.count_entities_by_type().await?;
    let sqlite_counts = setup.sqlite.count_entities_by_type().await?;
    
    assert_eq!(
        neo4j_counts.len(),
        sqlite_counts.len(),
        "Entity type count mismatch"
    );
    
    for (neo4j_type, neo4j_count) in neo4j_counts {
        let sqlite_count = sqlite_counts
            .iter()
            .find(|(t, _)| t == &neo4j_type)
            .map(|(_, c)| *c)
            .unwrap_or(0);
        
        assert_eq!(
            neo4j_count, sqlite_count,
            "Count mismatch for type {}: Neo4j={}, SQLite={}",
            neo4j_type, neo4j_count, sqlite_count
        );
    }
    
    // Verify graph structure validation
    let neo4j_stats = setup.neo4j.validate_structure().await?;
    let sqlite_stats = setup.sqlite.validate_structure().await?;
    
    assert_eq!(
        neo4j_stats.total_nodes, sqlite_stats.total_nodes,
        "Total nodes mismatch"
    );
    assert_eq!(
        neo4j_stats.total_edges, sqlite_stats.total_edges,
        "Total edges mismatch"
    );
    assert_eq!(
        neo4j_stats.orphan_count, sqlite_stats.orphan_count,
        "Orphan count mismatch"
    );
    
    Ok(())
}

/// Test 3: Edge case handling parity
#[tokio::test]
#[ignore]
async fn test_edge_case_handling_parity() -> Result<()> {
    let setup = DualBackendSetup::new().await?;
    
    // Test 1: Self-referencing relationships
    let self_id = 1i64;
    let props = NodeProperties {
        id: self_id,
        name: "self_ref".to_string(),
        path: Some("/src/self_ref.rs".to_string()),
        start_line: Some(1),
        end_line: Some(10),
        signature: Some("fn self_ref()".to_string()),
        body_snippet: None,
        docstring: None,
        hash: None,
        language: Some("rust".to_string()),
        file_sha256: None,
        mtime: None,
        created_at: None,
        last_modified_at: None,
        change_count: None,
        author_count: None,
    };
    
    setup.neo4j.upsert_entity(NodeLabel::Function, props.clone()).await?;
    setup.sqlite.upsert_entity(NodeLabel::Function, props).await?;
    
    // Create self-referencing relationship
    setup.neo4j.create_relationship(self_id, self_id, RelationType::Calls).await?;
    setup.sqlite.create_relationship(self_id, self_id, RelationType::Calls).await?;
    
    // Verify self-reference handling
    let neo4j_neighbors = setup.neo4j.get_neighbors(self_id).await?;
    let sqlite_neighbors = setup.sqlite.get_neighbors(self_id).await?;
    
    assert!(
        compare_entity_results(&neo4j_neighbors, &sqlite_neighbors)?,
        "Self-reference neighbor parity failed"
    );
    
    // Test 2: Duplicate relationships (idempotency)
    setup.neo4j.create_relationship(self_id, self_id, RelationType::Calls).await?;
    setup.sqlite.create_relationship(self_id, self_id, RelationType::Calls).await?;
    
    // Should still have same number of neighbors (idempotent)
    let neo4j_neighbors_dup = setup.neo4j.get_neighbors(self_id).await?;
    let sqlite_neighbors_dup = setup.sqlite.get_neighbors(self_id).await?;
    
    assert_eq!(
        neo4j_neighbors.len(),
        neo4j_neighbors_dup.len(),
        "Neo4j duplicate relationship not idempotent"
    );
    assert_eq!(
        sqlite_neighbors.len(),
        sqlite_neighbors_dup.len(),
        "SQLite duplicate relationship not idempotent"
    );
    
    // Test 3: Non-existent entity handling
    let non_existent_id = 999999i64;
    let neo4j_missing = setup.neo4j.get_entity_by_id(non_existent_id).await?;
    let sqlite_missing = setup.sqlite.get_entity_by_id(non_existent_id).await?;
    
    assert!(neo4j_missing.is_none(), "Neo4j should return None for non-existent entity");
    assert!(sqlite_missing.is_none(), "SQLite should return None for non-existent entity");
    
    // Test 4: Relationship to non-existent entity (should fail gracefully)
    let result_neo4j = setup.neo4j.create_relationship(non_existent_id, self_id, RelationType::Calls).await;
    let result_sqlite = setup.sqlite.create_relationship(non_existent_id, self_id, RelationType::Calls).await;
    
    // Both should handle gracefully (either succeed with dangling reference or fail cleanly)
    match (result_neo4j, result_sqlite) {
        (Ok(_), Ok(_)) => {} // Both succeeded
        (Err(_), Err(_)) => {} // Both failed
        _ => panic!("Inconsistent error handling for relationship to non-existent entity"),
    }
    
    Ok(())
}

/// Test 4: Batch operations parity
#[tokio::test]
#[ignore]
async fn test_batch_operations_parity() -> Result<()> {
    let setup = DualBackendSetup::new().await?;
    
    // Prepare batch entities
    let mut batch_entities = Vec::new();
    for i in 1..=50 {
        batch_entities.push(NodeProperties {
            id: i,
            name: format!("batch_entity_{}", i),
            path: Some(format!("/src/batch_{}.rs", i)),
            start_line: Some(i),
            end_line: Some(i + 5),
            signature: Some(format!("fn batch_{}() {{}}", i)),
            body_snippet: None,
            docstring: None,
            hash: None,
            language: Some("rust".to_string()),
            file_sha256: None,
            mtime: None,
            created_at: None,
            last_modified_at: None,
            change_count: None,
            author_count: None,
        });
    }
    
    // Execute batch upserts
    let neo4j_count = setup.neo4j.batch_upsert_entities(NodeLabel::Function, batch_entities.clone(), 10).await?;
    let sqlite_count = setup.sqlite.batch_upsert_entities(NodeLabel::Function, batch_entities, 10).await?;
    
    assert_eq!(neo4j_count, sqlite_count, "Batch entity count mismatch");
    assert_eq!(neo4j_count, 50, "Expected 50 entities to be created");
    
    // Prepare batch relationships
    let mut batch_relationships = Vec::new();
    for i in 1..=30 {
        batch_relationships.push((i, i + 1, RelationType::Calls));
    }
    
    // Execute batch relationship creation
    let neo4j_rel_count = setup.neo4j.batch_create_relationships(batch_relationships.clone(), 5).await?;
    let sqlite_rel_count = setup.sqlite.batch_create_relationships(batch_relationships, 5).await?;
    
    assert_eq!(neo4j_rel_count, sqlite_rel_count, "Batch relationship count mismatch");
    assert_eq!(neo4j_rel_count, 30, "Expected 30 relationships to be created");
    
    // Verify final state
    let neo4j_stats = setup.neo4j.validate_structure().await?;
    let sqlite_stats = setup.sqlite.validate_structure().await?;
    
    assert_eq!(neo4j_stats.total_nodes, sqlite_stats.total_nodes, "Final node count mismatch");
    assert_eq!(neo4j_stats.total_edges, sqlite_stats.total_edges, "Final edge count mismatch");
    
    Ok(())
}

/// Test 5: File-level operations parity
#[tokio::test]
#[ignore]
async fn test_file_operations_parity() -> Result<()> {
    let setup = DualBackendSetup::new().await?;
    
    // Create file entities
    let file_path = "/src/test_file.rs";
    setup.neo4j.upsert_file_by_path(file_path).await?;
    setup.sqlite.upsert_file_by_path(file_path).await?;
    
    // Create entities in file
    let props1 = NodeProperties {
        id: 1,
        name: "function1".to_string(),
        path: Some(file_path.to_string()),
        start_line: Some(1),
        end_line: Some(10),
        signature: Some("fn function1()".to_string()),
        body_snippet: None,
        docstring: None,
        hash: None,
        language: Some("rust".to_string()),
        file_sha256: None,
        mtime: None,
        created_at: None,
        last_modified_at: None,
        change_count: None,
        author_count: None,
    };
    
    let props2 = NodeProperties {
        id: 2,
        name: "function2".to_string(),
        path: Some(file_path.to_string()),
        start_line: Some(11),
        end_line: Some(20),
        signature: Some("fn function2()".to_string()),
        body_snippet: None,
        docstring: None,
        hash: None,
        language: Some("rust".to_string()),
        file_sha256: None,
        mtime: None,
        created_at: None,
        last_modified_at: None,
        change_count: None,
        author_count: None,
    };
    
    setup.neo4j.upsert_entity(NodeLabel::Function, props1.clone()).await?;
    setup.sqlite.upsert_entity(NodeLabel::Function, props1).await?;
    
    setup.neo4j.upsert_entity(NodeLabel::Function, props2.clone()).await?;
    setup.sqlite.upsert_entity(NodeLabel::Function, props2).await?;
    
    // Verify file entity retrieval
    let neo4j_file_entities = setup.neo4j.get_file_entities(file_path).await?;
    let sqlite_file_entities = setup.sqlite.get_file_entities(file_path).await?;
    
    assert!(
        compare_entity_results(&neo4j_file_entities, &sqlite_file_entities)?,
        "File entities parity failed"
    );
    
    // Test file deletion
    let neo4j_deleted = setup.neo4j.delete_file_entities(file_path).await?;
    let sqlite_deleted = setup.sqlite.delete_file_entities(file_path).await?;
    
    assert_eq!(neo4j_deleted, sqlite_deleted, "File deletion count mismatch");
    
    // Verify deletion
    let neo4j_after = setup.neo4j.get_file_entities(file_path).await?;
    let sqlite_after = setup.sqlite.get_file_entities(file_path).await?;
    
    assert!(neo4j_after.is_empty(), "Neo4j file entities should be deleted");
    assert!(sqlite_after.is_empty(), "SQLite file entities should be deleted");
    
    Ok(())
}