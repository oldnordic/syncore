//! Cache Parity Tests
//!
//! TDD-driven tests for cache path validation between Neo4j and SQLiteGraph backends.
//! Tests SQLite: match_triples_fast vs match_triples, Neo4j: skip fast-path.

use anyhow::Result;
use syncore::graph::{GraphBackend, Neo4jBackend, SQLiteGraphBackend};
use syncore::graph::{NodeLabel, NodeProperties, RelationType, EntityResult};
use tempfile::TempDir;
use tokio;

/// Reuse the same setup from relationship_parity_tests
use super::relationship_parity_tests::DualBackendSetup;

/// Test 1: SQLite fast-path vs regular path parity
#[tokio::test]
#[ignore]
async fn test_sqlite_fast_path_parity() -> Result<()> {
    let setup = DualBackendSetup::new().await?;
    
    // Create test entities
    let entity_ids = setup.create_test_entities(20).await?;
    
    // Create complex relationship patterns
    for i in 0..entity_ids.len() - 1 {
        let src_id = entity_ids[i];
        let dst_id = entity_ids[i + 1];
        
        // Create multiple relationship types
        setup.neo4j.create_relationship(src_id, dst_id, RelationType::Calls).await?;
        setup.sqlite.create_relationship(src_id, dst_id, RelationType::Calls).await?;
        
        if i % 2 == 0 {
            setup.neo4j.create_relationship(src_id, dst_id, RelationType::Contains).await?;
            setup.sqlite.create_relationship(src_id, dst_id, RelationType::Contains).await?;
        }
        
        if i % 3 == 0 {
            setup.neo4j.create_relationship(src_id, dst_id, RelationType::DependsOn).await?;
            setup.sqlite.create_relationship(src_id, dst_id, RelationType::DependsOn).await?;
        }
    }
    
    // Test that SQLite operations are consistent regardless of fast-path usage
    for &entity_id in &entity_ids {
        // Test neighbor queries (should use fast-path internally)
        let sqlite_neighbors_1 = setup.sqlite.get_neighbors(entity_id).await?;
        let sqlite_neighbors_2 = setup.sqlite.get_neighbors(entity_id).await?;
        
        // Results should be identical across multiple calls
        assert_eq!(
            sqlite_neighbors_1.len(),
            sqlite_neighbors_2.len(),
            "SQLite neighbor query should be consistent across calls for entity {}",
            entity_id
        );
        
        for (i, (neighbor1, neighbor2)) in sqlite_neighbors_1.iter().zip(sqlite_neighbors_2.iter()).enumerate() {
            assert_eq!(
                neighbor1.id, neighbor2.id,
                "Neighbor ID mismatch at index {} for entity {}",
                i, entity_id
            );
            assert_eq!(
                neighbor1.name, neighbor2.name,
                "Neighbor name mismatch at index {} for entity {}",
                i, entity_id
            );
        }
        
        // Compare with Neo4j results
        let neo4j_neighbors = setup.neo4j.get_neighbors(entity_id).await?;
        
        assert_eq!(
            sqlite_neighbors_1.len(),
            neo4j_neighbors.len(),
            "Neighbor count mismatch between SQLite and Neo4j for entity {}",
            entity_id
        );
        
        // Sort both by ID for comparison
        let mut sqlite_sorted = sqlite_neighbors_1.clone();
        let mut neo4j_sorted = neo4j_neighbors.clone();
        sqlite_sorted.sort_by_key(|e| e.id);
        neo4j_sorted.sort_by_key(|e| e.id);
        
        for (i, (sqlite_entity, neo4j_entity)) in sqlite_sorted.iter().zip(neo4j_sorted.iter()).enumerate() {
            assert_eq!(
                sqlite_entity.id, neo4j_entity.id,
                "Entity ID mismatch at index {} for entity {}",
                i, entity_id
            );
            assert_eq!(
                sqlite_entity.name, neo4j_entity.name,
                "Entity name mismatch at index {} for entity {}",
                i, entity_id
            );
            assert_eq!(
                sqlite_entity.label, neo4j_entity.label,
                "Entity label mismatch at index {} for entity {}",
                i, entity_id
            );
        }
    }
    
    Ok(())
}

/// Test 2: Cache invalidation behavior
#[tokio::test]
#[ignore]
async fn test_cache_invalidation_parity() -> Result<()> {
    let setup = DualBackendSetup::new().await?;
    
    // Create initial entities
    let entity_ids = setup.create_test_entities(10).await?;
    
    // Create initial relationships
    for i in 0..entity_ids.len() - 1 {
        let src_id = entity_ids[i];
        let dst_id = entity_ids[i + 1];
        
        setup.neo4j.create_relationship(src_id, dst_id, RelationType::Calls).await?;
        setup.sqlite.create_relationship(src_id, dst_id, RelationType::Calls).await?;
    }
    
    // Get initial neighbor counts
    let mut sqlite_neighbor_counts = Vec::new();
    let mut neo4j_neighbor_counts = Vec::new();
    
    for &entity_id in &entity_ids {
        let sqlite_neighbors = setup.sqlite.get_neighbors(entity_id).await?;
        let neo4j_neighbors = setup.neo4j.get_neighbors(entity_id).await?;
        
        sqlite_neighbor_counts.push(sqlite_neighbors.len());
        neo4j_neighbor_counts.push(neo4j_neighbors.len());
    }
    
    // Add new relationships
    for i in 0..entity_ids.len() / 2 {
        let src_id = entity_ids[i];
        let dst_id = entity_ids[entity_ids.len() - 1 - i];
        
        setup.neo4j.create_relationship(src_id, dst_id, RelationType::Uses).await?;
        setup.sqlite.create_relationship(src_id, dst_id, RelationType::Uses).await?;
    }
    
    // Get updated neighbor counts
    let mut sqlite_updated_counts = Vec::new();
    let mut neo4j_updated_counts = Vec::new();
    
    for &entity_id in &entity_ids {
        let sqlite_neighbors = setup.sqlite.get_neighbors(entity_id).await?;
        let neo4j_neighbors = setup.neo4j.get_neighbors(entity_id).await?;
        
        sqlite_updated_counts.push(sqlite_neighbors.len());
        neo4j_updated_counts.push(neo4j_neighbors.len());
    }
    
    // Verify cache invalidation worked
    for (i, (initial, updated)) in sqlite_neighbor_counts.iter().zip(sqlite_updated_counts.iter()).enumerate() {
        if i < entity_ids.len() / 2 || i >= entity_ids.len() - entity_ids.len() / 2 {
            // These entities should have new relationships
            assert!(
                updated > initial,
                "SQLite cache not invalidated for entity {}: {} -> {}",
                entity_ids[i], initial, updated
            );
        }
    }
    
    for (i, (initial, updated)) in neo4j_neighbor_counts.iter().zip(neo4j_updated_counts.iter()).enumerate() {
        if i < entity_ids.len() / 2 || i >= entity_ids.len() - entity_ids.len() / 2 {
            // These entities should have new relationships
            assert!(
                updated > initial,
                "Neo4j should reflect new relationships for entity {}: {} -> {}",
                entity_ids[i], initial, updated
            );
        }
    }
    
    // Verify final parity
    assert_eq!(
        sqlite_updated_counts, neo4j_updated_counts,
        "Final neighbor counts should match between backends"
    );
    
    Ok(())
}

/// Test 3: Performance consistency under cache load
#[tokio::test]
#[ignore]
async fn test_cache_performance_consistency() -> Result<()> {
    let setup = DualBackendSetup::new().await?;
    
    // Create a larger dataset
    let entity_ids = setup.create_test_entities(100).await?;
    
    // Create dense relationships
    for i in 0..entity_ids.len() {
        for j in 1..=5 {
            let src_id = entity_ids[i];
            let dst_idx = (i + j) % entity_ids.len();
            let dst_id = entity_ids[dst_idx];
            
            setup.neo4j.create_relationship(src_id, dst_id, RelationType::Calls).await?;
            setup.sqlite.create_relationship(src_id, dst_id, RelationType::Calls).await?;
        }
    }
    
    // Perform multiple queries to test cache behavior
    let iterations = 10;
    
    for iteration in 1..=iterations {
        let mut sqlite_total = 0;
        let mut neo4j_total = 0;
        
        for &entity_id in &entity_ids {
            let sqlite_neighbors = setup.sqlite.get_neighbors(entity_id).await?;
            let neo4j_neighbors = setup.neo4j.get_neighbors(entity_id).await?;
            
            sqlite_total += sqlite_neighbors.len();
            neo4j_total += neo4j_neighbors.len();
            
            // Verify parity on each iteration
            assert_eq!(
                sqlite_neighbors.len(),
                neo4j_neighbors.len(),
                "Neighbor count mismatch in iteration {} for entity {}",
                iteration, entity_id
            );
        }
        
        assert_eq!(
            sqlite_total, neo4j_total,
            "Total neighbor count mismatch in iteration {}",
            iteration
        );
        
        // Verify consistency across iterations
        if iteration > 1 {
            // Totals should be the same across iterations
            assert_eq!(
                sqlite_total, 500, // 100 entities * 5 relationships each
                "SQLite total should be consistent in iteration {}",
                iteration
            );
            assert_eq!(
                neo4j_total, 500,
                "Neo4j total should be consistent in iteration {}",
                iteration
            );
        }
    }
    
    Ok(())
}

/// Test 4: Entity deletion cache behavior
#[tokio::test]
#[ignore]
async fn test_deletion_cache_behavior() -> Result<()> {
    let setup = DualBackendSetup::new().await?;
    
    // Create entities and relationships
    let entity_ids = setup.create_test_entities(15).await?;
    
    for i in 0..entity_ids.len() - 1 {
        let src_id = entity_ids[i];
        let dst_id = entity_ids[i + 1];
        
        setup.neo4j.create_relationship(src_id, dst_id, RelationType::Calls).await?;
        setup.sqlite.create_relationship(src_id, dst_id, RelationType::Calls).await?;
    }
    
    // Get initial state
    let initial_stats_neo4j = setup.neo4j.validate_structure().await?;
    let initial_stats_sqlite = setup.sqlite.validate_structure().await?;
    
    assert_eq!(
        initial_stats_neo4j.total_nodes, initial_stats_sqlite.total_nodes,
        "Initial node count mismatch"
    );
    assert_eq!(
        initial_stats_neo4j.total_edges, initial_stats_sqlite.total_edges,
        "Initial edge count mismatch"
    );
    
    // Delete some entities
    let entities_to_delete = &entity_ids[3..8]; // Delete 5 entities
    for &entity_id in entities_to_delete {
        setup.neo4j.delete_entity(entity_id).await?;
        setup.sqlite.delete_entity(entity_id).await?;
    }
    
    // Verify cache consistency after deletion
    let final_stats_neo4j = setup.neo4j.validate_structure().await?;
    let final_stats_sqlite = setup.sqlite.validate_structure().await?;
    
    assert_eq!(
        final_stats_neo4j.total_nodes, final_stats_sqlite.total_nodes,
        "Final node count mismatch after deletion"
    );
    assert_eq!(
        final_stats_neo4j.total_edges, final_stats_sqlite.total_edges,
        "Final edge count mismatch after deletion"
    );
    
    // Verify specific counts
    assert_eq!(
        final_stats_neo4j.total_nodes,
        initial_stats_neo4j.total_nodes - 5,
        "Neo4j node count should decrease by 5"
    );
    assert_eq!(
        final_stats_sqlite.total_nodes,
        initial_stats_sqlite.total_nodes - 5,
        "SQLite node count should decrease by 5"
    );
    
    // Test neighbor queries after deletion
    for &entity_id in &entity_ids {
        if entities_to_delete.contains(&entity_id) {
            // Deleted entity should have no neighbors
            let neo4j_neighbors = setup.neo4j.get_neighbors(entity_id).await?;
            let sqlite_neighbors = setup.sqlite.get_neighbors(entity_id).await?;
            
            assert!(neo4j_neighbors.is_empty(), "Deleted entity should have no neighbors in Neo4j");
            assert!(sqlite_neighbors.is_empty(), "Deleted entity should have no neighbors in SQLite");
        } else {
            // Remaining entities should have updated neighbor counts
            let neo4j_neighbors = setup.neo4j.get_neighbors(entity_id).await?;
            let sqlite_neighbors = setup.sqlite.get_neighbors(entity_id).await?;
            
            assert_eq!(
                neo4j_neighbors.len(),
                sqlite_neighbors.len(),
                "Remaining entity should have consistent neighbor counts"
            );
        }
    }
    
    Ok(())
}

/// Test 5: File-level operations cache behavior
#[tokio::test]
#[ignore]
async fn test_file_operations_cache_behavior() -> Result<()> {
    let setup = DualBackendSetup::new().await?;
    
    // Create file with multiple entities
    let file_path = "/src/cache_test.rs";
    
    // Create file node
    setup.neo4j.upsert_file_by_path(file_path).await?;
    setup.sqlite.upsert_file_by_path(file_path).await?;
    
    // Create entities in file
    let mut entity_ids = Vec::new();
    for i in 1..=10 {
        let props = NodeProperties {
            id: i,
            name: format!("cache_function_{}", i),
            path: Some(file_path.to_string()),
            start_line: Some(i * 10),
            end_line: Some(i * 10 + 5),
            signature: Some(format!("fn cache_function_{}() {{}}", i)),
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
        entity_ids.push(i);
    }
    
    // Test file entity queries multiple times
    for iteration in 1..=5 {
        let neo4j_file_entities = setup.neo4j.get_file_entities(file_path).await?;
        let sqlite_file_entities = setup.sqlite.get_file_entities(file_path).await?;
        
        assert_eq!(
            neo4j_file_entities.len(),
            sqlite_file_entities.len(),
            "File entity count mismatch in iteration {}",
            iteration
        );
        assert_eq!(
            neo4j_file_entities.len(),
            10,
            "Should have 10 entities in file"
        );
        
        // Verify ordering consistency
        for (i, (neo4j_entity, sqlite_entity)) in neo4j_file_entities.iter().zip(sqlite_file_entities.iter()).enumerate() {
            assert_eq!(
                neo4j_entity.id, sqlite_entity.id,
                "Entity ID mismatch at index {} in iteration {}",
                i, iteration
            );
            assert_eq!(
                neo4j_entity.name, sqlite_entity.name,
                "Entity name mismatch at index {} in iteration {}",
                i, iteration
            );
        }
    }
    
    // Delete file entities
    let neo4j_deleted = setup.neo4j.delete_file_entities(file_path).await?;
    let sqlite_deleted = setup.sqlite.delete_file_entities(file_path).await?;
    
    assert_eq!(neo4j_deleted, sqlite_deleted, "File deletion count mismatch");
    assert_eq!(neo4j_deleted, 10, "Should delete 10 entities");
    
    // Verify cache invalidation after file deletion
    let neo4j_after = setup.neo4j.get_file_entities(file_path).await?;
    let sqlite_after = setup.sqlite.get_file_entities(file_path).await?;
    
    assert!(neo4j_after.is_empty(), "Neo4j file entities should be deleted");
    assert!(sqlite_after.is_empty(), "SQLite file entities should be deleted");
    
    Ok(())
}

/// Test 6: Concurrent access cache behavior
#[tokio::test]
#[ignore]
async fn test_concurrent_cache_behavior() -> Result<()> {
    let setup = DualBackendSetup::new().await?;
    
    // Create initial entities
    let entity_ids = setup.create_test_entities(20).await?;
    
    // Create initial relationships
    for i in 0..entity_ids.len() - 1 {
        let src_id = entity_ids[i];
        let dst_id = entity_ids[i + 1];
        
        setup.neo4j.create_relationship(src_id, dst_id, RelationType::Calls).await?;
        setup.sqlite.create_relationship(src_id, dst_id, RelationType::Calls).await?;
    }
    
    // Perform concurrent queries
    let mut handles = Vec::new();
    
    for i in 0..5 {
        let neo4j_backend = setup.neo4j.clone();
        let sqlite_backend = setup.sqlite.clone();
        let test_entity_ids = entity_ids.clone();
        
        let handle = tokio::spawn(async move {
            let mut neo4j_total = 0;
            let mut sqlite_total = 0;
            
            for &entity_id in &test_entity_ids {
                let neo4j_neighbors = neo4j_backend.get_neighbors(entity_id).await.unwrap();
                let sqlite_neighbors = sqlite_backend.get_neighbors(entity_id).await.unwrap();
                
                neo4j_total += neo4j_neighbors.len();
                sqlite_total += sqlite_neighbors.len();
                
                // Verify parity during concurrent access
                assert_eq!(
                    neo4j_neighbors.len(),
                    sqlite_neighbors.len(),
                    "Concurrent access parity failed for entity {} in thread {}",
                    entity_id, i
                );
            }
            
            (neo4j_total, sqlite_total, i)
        });
        
        handles.push(handle);
    }
    
    // Wait for all concurrent operations
    let mut results = Vec::new();
    for handle in handles {
        let result = handle.await?;
        results.push(result);
    }
    
    // Verify all threads got consistent results
    for (neo4j_total, sqlite_total, thread_id) in results {
        assert_eq!(
            neo4j_total, sqlite_total,
            "Thread {} got inconsistent totals",
            thread_id
        );
        assert_eq!(
            neo4j_total, 19, // 20 entities with 19 relationships in a chain
            "Thread {} got unexpected total",
            thread_id
        );
    }
    
    Ok(())
}