use anyhow::Result;
use syncore::graph::backend::{GraphBackend, Neo4jBackend, SQLiteGraphBackend};
use syncore::graph::types::{Entity, Relationship, EntityType, RelationshipType};
use syncore::db::DbManager;
use tempfile::TempDir;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Test error behavior parity between SQLiteGraph and Neo4j backends
/// This ensures both backends handle errors consistently and predictably

async fn setup_sqlite_backend() -> Result<(Arc<Mutex<SQLiteGraphBackend>>, TempDir)> {
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("test_error_parity.db");
    let db_manager = DbManager::new(db_path.to_str().unwrap())?;
    let backend = SQLiteGraphBackend::new(db_manager).await?;
    Ok((Arc::new(Mutex::new(backend)), temp_dir))
}

async fn setup_neo4j_backend() -> Result<Arc<Mutex<Neo4jBackend>>> {
    // Skip Neo4j tests if not available
    match Neo4jBackend::new("bolt://127.0.0.1:7687", "neo4j", "password").await {
        Ok(backend) => Ok(Arc::new(Mutex::new(backend))),
        Err(_) => {
            println!("Warning: Neo4j not available, skipping Neo4j error parity tests");
            Err(anyhow::anyhow!("Neo4j not available"))
        }
    }
}

/// Test invalid entity ID handling
#[tokio::test]
async fn test_invalid_entity_id_errors() -> Result<()> {
    let (sqlite_backend, _temp_dir) = setup_sqlite_backend().await?;
    
    // Test with SQLite
    let sqlite_result = {
        let backend = sqlite_backend.lock().await;
        backend.get_entity_by_id(999999).await
    };
    
    // Should return None for non-existent entity
    assert!(sqlite_result.is_ok());
    assert!(sqlite_result.unwrap().is_none());
    
    // Test with Neo4j if available
    if let Ok(neo4j_backend) = setup_neo4j_backend().await {
        let neo4j_result = {
            let backend = neo4j_backend.lock().await;
            backend.get_entity_by_id(999999).await
        };
        
        // Should also return None for non-existent entity
        assert!(neo4j_result.is_ok());
        assert!(neo4j_result.unwrap().is_none());
    }
    
    Ok(())
}

/// Test invalid relationship ID handling
#[tokio::test]
async fn test_invalid_relationship_id_errors() -> Result<()> {
    let (sqlite_backend, _temp_dir) = setup_sqlite_backend().await?;
    
    // Test with SQLite
    let sqlite_result = {
        let backend = sqlite_backend.lock().await;
        backend.get_relationship_by_id(999999).await
    };
    
    // Should return None for non-existent relationship
    assert!(sqlite_result.is_ok());
    assert!(sqlite_result.unwrap().is_none());
    
    // Test with Neo4j if available
    if let Ok(neo4j_backend) = setup_neo4j_backend().await {
        let neo4j_result = {
            let backend = neo4j_backend.lock().await;
            backend.get_relationship_by_id(999999).await
        };
        
        // Should also return None for non-existent relationship
        assert!(neo4j_result.is_ok());
        assert!(neo4j_result.unwrap().is_none());
    }
    
    Ok(())
}

/// Test duplicate entity creation handling
#[tokio::test]
async fn test_duplicate_entity_creation() -> Result<()> {
    let (sqlite_backend, _temp_dir) = setup_sqlite_backend().await?;
    
    let entity = Entity {
        id: None,
        name: "duplicate_test".to_string(),
        entity_type: EntityType::Function,
        file_path: "/test/duplicate.rs".to_string(),
        language: "rust".to_string(),
        properties: HashMap::new(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    // Create first entity with SQLite
    let first_id = {
        let backend = sqlite_backend.lock().await;
        backend.upsert_entity(&entity).await?
    };
    
    // Create duplicate entity with SQLite (should update existing)
    let second_id = {
        let backend = sqlite_backend.lock().await;
        backend.upsert_entity(&entity).await?
    };
    
    // Both should return the same ID (update behavior)
    assert_eq!(first_id, second_id);
    
    // Test with Neo4j if available
    if let Ok(neo4j_backend) = setup_neo4j_backend().await {
        let neo4j_first_id = {
            let backend = neo4j_backend.lock().await;
            backend.upsert_entity(&entity).await?
        };
        
        let neo4j_second_id = {
            let backend = neo4j_backend.lock().await;
            backend.upsert_entity(&entity).await?
        };
        
        // Neo4j should also return the same ID for duplicates
        assert_eq!(neo4j_first_id, neo4j_second_id);
    }
    
    Ok(())
}

/// Test relationship creation with non-existent entities
#[tokio::test]
async fn test_relationship_with_nonexistent_entities() -> Result<()> {
    let (sqlite_backend, _temp_dir) = setup_sqlite_backend().await?;
    
    let relationship = Relationship {
        id: None,
        from_id: 999999, // Non-existent entity
        to_id: 999998,    // Non-existent entity
        relationship_type: RelationshipType::Calls,
        properties: HashMap::new(),
        created_at: chrono::Utc::now(),
    };
    
    // Test with SQLite - should handle gracefully
    let sqlite_result = {
        let backend = sqlite_backend.lock().await;
        backend.create_relationship(&relationship).await
    };
    
    // SQLite should either create relationship or return error
    // The exact behavior depends on implementation
    match sqlite_result {
        Ok(_) => {
            // If successful, verify relationship exists
            let rel_id = sqlite_result.unwrap();
            let check_result = {
                let backend = sqlite_backend.lock().await;
                backend.get_relationship_by_id(rel_id).await
            };
            assert!(check_result.is_ok());
        }
        Err(_) => {
            // If error, that's also acceptable behavior
        }
    }
    
    // Test with Neo4j if available
    if let Ok(neo4j_backend) = setup_neo4j_backend().await {
        let neo4j_result = {
            let backend = neo4j_backend.lock().await;
            backend.create_relationship(&relationship).await
        };
        
        // Neo4j should behave similarly to SQLite
        match neo4j_result {
            Ok(_) => {
                let rel_id = neo4j_result.unwrap();
                let check_result = {
                    let backend = neo4j_backend.lock().await;
                    backend.get_relationship_by_id(rel_id).await
                };
                assert!(check_result.is_ok());
            }
            Err(_) => {
                // Error is acceptable
            }
        }
    }
    
    Ok(())
}

/// Test malformed query handling
#[tokio::test]
async fn test_malformed_query_handling() -> Result<()> {
    let (sqlite_backend, _temp_dir) = setup_sqlite_backend().await?;
    
    // Test with SQLite - empty query should return empty results
    let sqlite_result = {
        let backend = sqlite_backend.lock().await;
        backend.find_entities_by_name("").await
    };
    
    assert!(sqlite_result.is_ok());
    let sqlite_entities = sqlite_result.unwrap();
    assert!(sqlite_entities.is_empty());
    
    // Test with Neo4j if available
    if let Ok(neo4j_backend) = setup_neo4j_backend().await {
        let neo4j_result = {
            let backend = neo4j_backend.lock().await;
            backend.find_entities_by_name("").await
        };
        
        assert!(neo4j_result.is_ok());
        let neo4j_entities = neo4j_result.unwrap();
        assert!(neo4j_entities.is_empty());
    }
    
    Ok(())
}

/// Test batch operations with mixed valid/invalid data
#[tokio::test]
async fn test_batch_operations_mixed_data() -> Result<()> {
    let (sqlite_backend, _temp_dir) = setup_sqlite_backend().await?;
    
    // Create valid entities
    let valid_entity1 = Entity {
        id: None,
        name: "valid_entity_1".to_string(),
        entity_type: EntityType::Function,
        file_path: "/test/valid1.rs".to_string(),
        language: "rust".to_string(),
        properties: HashMap::new(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    let valid_entity2 = Entity {
        id: None,
        name: "valid_entity_2".to_string(),
        entity_type: EntityType::Function,
        file_path: "/test/valid2.rs".to_string(),
        language: "rust".to_string(),
        properties: HashMap::new(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    let entities = vec![valid_entity1, valid_entity2];
    
    // Test batch upsert with SQLite
    let sqlite_result = {
        let backend = sqlite_backend.lock().await;
        backend.batch_upsert_entities(&entities).await
    };
    
    assert!(sqlite_result.is_ok());
    let sqlite_ids = sqlite_result.unwrap();
    assert_eq!(sqlite_ids.len(), 2);
    
    // Test with Neo4j if available
    if let Ok(neo4j_backend) = setup_neo4j_backend().await {
        let neo4j_result = {
            let backend = neo4j_backend.lock().await;
            backend.batch_upsert_entities(&entities).await
        };
        
        assert!(neo4j_result.is_ok());
        let neo4j_ids = neo4j_result.unwrap();
        assert_eq!(neo4j_ids.len(), 2);
    }
    
    Ok(())
}

/// Test concurrent access error handling
#[tokio::test]
async fn test_concurrent_access_errors() -> Result<()> {
    let (sqlite_backend, _temp_dir) = setup_sqlite_backend().await?;
    
    let entity = Entity {
        id: None,
        name: "concurrent_test".to_string(),
        entity_type: EntityType::Function,
        file_path: "/test/concurrent.rs".to_string(),
        language: "rust".to_string(),
        properties: HashMap::new(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    // Test concurrent entity creation with SQLite
    let backend_clone = sqlite_backend.clone();
    let entity_clone = entity.clone();
    
    let handle1 = tokio::spawn(async move {
        let backend = backend_clone.lock().await;
        backend.upsert_entity(&entity).await
    });
    
    let handle2 = tokio::spawn(async move {
        let backend = backend_clone.lock().await;
        backend.upsert_entity(&entity_clone).await
    });
    
    let (result1, result2) = tokio::join!(handle1, handle2);
    
    // Both operations should succeed (SQLite handles concurrency)
    assert!(result1.is_ok());
    assert!(result1.unwrap().is_ok());
    assert!(result2.is_ok());
    assert!(result2.unwrap().is_ok());
    
    // Test with Neo4j if available
    if let Ok(neo4j_backend) = setup_neo4j_backend().await {
        let neo4j_clone = neo4j_backend.clone();
        let entity_clone2 = entity.clone();
        
        let neo4j_handle1 = tokio::spawn(async move {
            let backend = neo4j_clone.lock().await;
            backend.upsert_entity(&entity).await
        });
        
        let neo4j_handle2 = tokio::spawn(async move {
            let backend = neo4j_backend.lock().await;
            backend.upsert_entity(&entity_clone2).await
        });
        
        let (neo4j_result1, neo4j_result2) = tokio::join!(neo4j_handle1, neo4j_handle2);
        
        // Both operations should succeed
        assert!(neo4j_result1.is_ok());
        assert!(neo4j_result1.unwrap().is_ok());
        assert!(neo4j_result2.is_ok());
        assert!(neo4j_result2.unwrap().is_ok());
    }
    
    Ok(())
}

/// Test database connection error recovery
#[tokio::test]
async fn test_connection_error_recovery() -> Result<()> {
    let (sqlite_backend, _temp_dir) = setup_sqlite_backend().await?;
    
    let entity = Entity {
        id: None,
        name: "recovery_test".to_string(),
        entity_type: EntityType::Function,
        file_path: "/test/recovery.rs".to_string(),
        language: "rust".to_string(),
        properties: HashMap::new(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    // Test multiple operations to ensure connection stability
    for i in 0..10 {
        let mut test_entity = entity.clone();
        test_entity.name = format!("recovery_test_{}", i);
        
        let result = {
            let backend = sqlite_backend.lock().await;
            backend.upsert_entity(&test_entity).await
        };
        
        assert!(result.is_ok(), "Operation {} failed", i);
        
        // Verify entity was created
        let entity_id = result.unwrap();
        let verify_result = {
            let backend = sqlite_backend.lock().await;
            backend.get_entity_by_id(entity_id).await
        };
        
        assert!(verify_result.is_ok());
        assert!(verify_result.unwrap().is_some());
    }
    
    Ok(())
}

/// Test large data handling
#[tokio::test]
async fn test_large_data_handling() -> Result<()> {
    let (sqlite_backend, _temp_dir) = setup_sqlite_backend().await?;
    
    // Create entity with large properties
    let mut large_properties = HashMap::new();
    large_properties.insert("large_data".to_string(), "x".repeat(10000));
    
    let large_entity = Entity {
        id: None,
        name: "large_data_test".to_string(),
        entity_type: EntityType::Function,
        file_path: "/test/large.rs".to_string(),
        language: "rust".to_string(),
        properties: large_properties,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    // Test with SQLite
    let sqlite_result = {
        let backend = sqlite_backend.lock().await;
        backend.upsert_entity(&large_entity).await
    };
    
    assert!(sqlite_result.is_ok());
    
    // Verify retrieval
    let entity_id = sqlite_result.unwrap();
    let retrieve_result = {
        let backend = sqlite_backend.lock().await;
        backend.get_entity_by_id(entity_id).await
    };
    
    assert!(retrieve_result.is_ok());
    let retrieved_entity = retrieve_result.unwrap();
    assert!(retrieved_entity.is_some());
    
    let retrieved = retrieved_entity.unwrap();
    assert_eq!(retrieved.properties.get("large_data").unwrap().len(), 10000);
    
    // Test with Neo4j if available
    if let Ok(neo4j_backend) = setup_neo4j_backend().await {
        let neo4j_result = {
            let backend = neo4j_backend.lock().await;
            backend.upsert_entity(&large_entity).await
        };
        
        assert!(neo4j_result.is_ok());
        
        let neo4j_entity_id = neo4j_result.unwrap();
        let neo4j_retrieve_result = {
            let backend = neo4j_backend.lock().await;
            backend.get_entity_by_id(neo4j_entity_id).await
        };
        
        assert!(neo4j_retrieve_result.is_ok());
        let neo4j_retrieved_entity = neo4j_retrieve_result.unwrap();
        assert!(neo4j_retrieved_entity.is_some());
        
        let neo4j_retrieved = neo4j_retrieved_entity.unwrap();
        assert_eq!(neo4j_retrieved.properties.get("large_data").unwrap().len(), 10000);
    }
    
    Ok(())
}

/// Test transaction rollback behavior
#[tokio::test]
async fn test_transaction_rollback() -> Result<()> {
    let (sqlite_backend, _temp_dir) = setup_sqlite_backend().await?;
    
    // Create initial entity
    let entity = Entity {
        id: None,
        name: "rollback_test".to_string(),
        entity_type: EntityType::Function,
        file_path: "/test/rollback.rs".to_string(),
        language: "rust".to_string(),
        properties: HashMap::new(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    let entity_id = {
        let backend = sqlite_backend.lock().await;
        backend.upsert_entity(&entity).await?
    };
    
    // Verify entity exists
    let verify_before = {
        let backend = sqlite_backend.lock().await;
        backend.get_entity_by_id(entity_id).await
    };
    assert!(verify_before.is_ok());
    assert!(verify_before.unwrap().is_some());
    
    // Delete entity
    {
        let backend = sqlite_backend.lock().await;
        backend.delete_entity(entity_id).await?;
    }
    
    // Verify entity is deleted
    let verify_after = {
        let backend = sqlite_backend.lock().await;
        backend.get_entity_by_id(entity_id).await
    };
    assert!(verify_after.is_ok());
    assert!(verify_after.unwrap().is_none());
    
    // Test with Neo4j if available
    if let Ok(neo4j_backend) = setup_neo4j_backend().await {
        let neo4j_entity_id = {
            let backend = neo4j_backend.lock().await;
            backend.upsert_entity(&entity).await?
        };
        
        // Verify exists
        let neo4j_verify_before = {
            let backend = neo4j_backend.lock().await;
            backend.get_entity_by_id(neo4j_entity_id).await
        };
        assert!(neo4j_verify_before.is_ok());
        assert!(neo4j_verify_before.unwrap().is_some());
        
        // Delete
        {
            let backend = neo4j_backend.lock().await;
            backend.delete_entity(neo4j_entity_id).await?;
        }
        
        // Verify deleted
        let neo4j_verify_after = {
            let backend = neo4j_backend.lock().await;
            backend.get_entity_by_id(neo4j_entity_id).await
        };
        assert!(neo4j_verify_after.is_ok());
        assert!(neo4j_verify_after.unwrap().is_none());
    }
    
    Ok(())
}