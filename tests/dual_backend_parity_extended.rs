//! Extended Dual-Backend Parity Tests
//!
//! This file extends the existing dual_backend_parity_tests.rs with comprehensive
//! coverage of all GraphBackend trait methods to ensure complete parity between
//! SQLiteGraph and Neo4j backends.
//!
//! REQUIREMENT: Real Neo4j instance must be running (no mocks allowed)

use anyhow::Result;
use chrono;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use syncore::config::{GraphBackend as ConfigBackend, GraphConfig};
use syncore::db::DbManager;
use syncore::graph::backend::{Neo4jBackend, SQLiteGraphBackend};
use syncore::graph::types::{
    Entity, EntityType, Relationship, RelationshipType as NewRelationType,
};
use syncore::graph::{
    create_graph_backend, EntityResult, GraphBackend, NodeLabel, NodeProperties, RelationType,
};
use tempfile::TempDir;
use tokio::sync::Mutex;

/// Test configuration for both backends using the new backend types
async fn setup_extended_test_backends(
) -> Result<(Arc<Mutex<SQLiteGraphBackend>>, Option<Arc<Mutex<Neo4jBackend>>>)> {
    // Setup SQLiteGraph backend
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_extended_parity.db");
    let db_manager = DbManager::new(db_path.to_str().unwrap())?;
    let sqlite_backend = SQLiteGraphBackend::new(db_manager).await?;
    let sqlite_arc = Arc::new(Mutex::new(sqlite_backend));

    // Setup Neo4j backend (if available)
    let neo4j_backend = match Neo4jBackend::new("bolt://127.0.0.1:7687", "neo4j", "password").await
    {
        Ok(backend) => Some(Arc::new(Mutex::new(backend))),
        Err(_) => {
            println!("Warning: Neo4j not available, some tests will be skipped");
            None
        }
    };

    // Keep temp_dir alive
    std::mem::forget(temp_dir);

    Ok((sqlite_arc, neo4j_backend))
}

/// Normalize entity results for comparison using new Entity type
fn normalize_new_entity(entity: &Entity) -> HashMap<String, serde_json::Value> {
    let mut map = HashMap::new();
    map.insert("id".to_string(), json!(entity.id));
    map.insert("name".to_string(), json!(entity.name));
    map.insert("entity_type".to_string(), json!(format!("{:?}", entity.entity_type)));
    map.insert("file_path".to_string(), json!(entity.file_path));
    map.insert("language".to_string(), json!(entity.language));
    // Skip temporal fields as they may differ between backends
    map
}

/// Normalize relationship results for comparison
fn normalize_relationship(rel: &Relationship) -> HashMap<String, serde_json::Value> {
    let mut map = HashMap::new();
    map.insert("id".to_string(), json!(rel.id));
    map.insert("from_id".to_string(), json!(rel.from_id));
    map.insert("to_id".to_string(), json!(rel.to_id));
    map.insert("relationship_type".to_string(), json!(format!("{:?}", rel.relationship_type)));
    // Skip temporal fields as they may differ between backends
    map
}

/// Compare entity lists with tolerance for ordering differences
fn compare_new_entity_lists(
    sqlite_results: &[Entity],
    neo4j_results: &[Entity],
    test_name: &str,
) -> Result<()> {
    // Normalize all entities
    let sqlite_normalized: Vec<_> = sqlite_results.iter().map(normalize_new_entity).collect();
    let neo4j_normalized: Vec<_> = neo4j_results.iter().map(normalize_new_entity).collect();

    // Sort by ID for comparison
    let mut sqlite_sorted = sqlite_normalized.clone();
    let mut neo4j_sorted = neo4j_normalized.clone();
    sqlite_sorted.sort_by(|a, b| a["id"].as_i64().cmp(&b["id"].as_i64()));
    neo4j_sorted.sort_by(|a, b| a["id"].as_i64().cmp(&b["id"].as_i64()));

    // Compare counts
    if sqlite_sorted.len() != neo4j_sorted.len() {
        anyhow::bail!(
            "{}: Entity count mismatch - SQLite: {}, Neo4j: {}",
            test_name,
            sqlite_sorted.len(),
            neo4j_sorted.len()
        );
    }

    // Compare each entity
    for (i, (sqlite_entity, neo4j_entity)) in
        sqlite_sorted.iter().zip(neo4j_sorted.iter()).enumerate()
    {
        if sqlite_entity != neo4j_entity {
            anyhow::bail!(
                "{}: Entity {} mismatch\nSQLite: {:?}\nNeo4j: {:?}",
                test_name,
                i + 1,
                sqlite_entity,
                neo4j_entity
            );
        }
    }

    println!("✓ {}: {} entities match", test_name, sqlite_sorted.len());
    Ok(())
}

/// Compare relationship lists
fn compare_relationship_lists(
    sqlite_results: &[Relationship],
    neo4j_results: &[Relationship],
    test_name: &str,
) -> Result<()> {
    // Normalize all relationships
    let sqlite_normalized: Vec<_> = sqlite_results.iter().map(normalize_relationship).collect();
    let neo4j_normalized: Vec<_> = neo4j_results.iter().map(normalize_relationship).collect();

    // Sort by ID for comparison
    let mut sqlite_sorted = sqlite_normalized.clone();
    let mut neo4j_sorted = neo4j_normalized.clone();
    sqlite_sorted.sort_by(|a, b| a["id"].as_i64().cmp(&b["id"].as_i64()));
    neo4j_sorted.sort_by(|a, b| a["id"].as_i64().cmp(&b["id"].as_i64()));

    // Compare counts
    if sqlite_sorted.len() != neo4j_sorted.len() {
        anyhow::bail!(
            "{}: Relationship count mismatch - SQLite: {}, Neo4j: {}",
            test_name,
            sqlite_sorted.len(),
            neo4j_sorted.len()
        );
    }

    // Compare each relationship
    for (i, (sqlite_rel, neo4j_rel)) in sqlite_sorted.iter().zip(neo4j_sorted.iter()).enumerate() {
        if sqlite_rel != neo4j_rel {
            anyhow::bail!(
                "{}: Relationship {} mismatch\nSQLite: {:?}\nNeo4j: {:?}",
                test_name,
                i + 1,
                sqlite_rel,
                neo4j_rel
            );
        }
    }

    println!("✓ {}: {} relationships match", test_name, sqlite_sorted.len());
    Ok(())
}

// ===== CRUD PARITY TESTS =====

#[tokio::test]
async fn test_extended_entity_crud_operations() -> Result<()> {
    let (sqlite_backend, neo4j_backend) = setup_extended_test_backends().await?;

    // Test entity creation
    let entity = Entity {
        id: None,
        name: "extended_test_function".to_string(),
        entity_type: EntityType::Function,
        file_path: "/tmp/extended_test.rs".to_string(),
        language: "rust".to_string(),
        properties: HashMap::new(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    // Create in SQLite
    let sqlite_id = {
        let backend = sqlite_backend.lock().await;
        backend.upsert_entity(&entity).await?
    };

    // Create in Neo4j if available
    let neo4j_id = if let Some(neo4j_backend) = &neo4j_backend {
        let backend = neo4j_backend.lock().await;
        backend.upsert_entity(&entity).await?
    } else {
        sqlite_id // Use SQLite ID for comparison if Neo4j not available
    };

    // Retrieve and compare
    let sqlite_entity = {
        let backend = sqlite_backend.lock().await;
        backend.get_entity_by_id(sqlite_id).await?
    };

    if let (Some(sqlite_ent), Some(neo4j_backend)) = (&sqlite_entity, &neo4j_backend) {
        let neo4j_entity = {
            let backend = neo4j_backend.lock().await;
            backend.get_entity_by_id(neo4j_id).await?
        };

        if let Some(neo4j_ent) = neo4j_entity {
            compare_new_entity_lists(&[sqlite_ent.clone()], &[neo4j_ent], "entity_crud_create")?;
        }
    }

    // Test entity update
    let mut updated_entity = entity.clone();
    updated_entity.name = "updated_function".to_string();
    updated_entity.id = Some(sqlite_id);

    let sqlite_update_id = {
        let backend = sqlite_backend.lock().await;
        backend.upsert_entity(&updated_entity).await?
    };

    assert_eq!(sqlite_id, sqlite_update_id, "Update should return same ID");

    // Test entity deletion
    {
        let backend = sqlite_backend.lock().await;
        backend.delete_entity(sqlite_id).await?;
    }

    let deleted_entity = {
        let backend = sqlite_backend.lock().await;
        backend.get_entity_by_id(sqlite_id).await?
    };

    assert!(deleted_entity.is_none(), "Entity should be deleted");

    println!("✓ Extended CRUD operations completed successfully");
    Ok(())
}

#[tokio::test]
async fn test_extended_batch_operations() -> Result<()> {
    let (sqlite_backend, neo4j_backend) = setup_extended_test_backends().await?;

    // Create multiple entities
    let mut entities = Vec::new();
    for i in 0..10 {
        let entity = Entity {
            id: None,
            name: format!("batch_function_{}", i),
            entity_type: EntityType::Function,
            file_path: format!("/tmp/batch_test_{}.rs", i),
            language: "rust".to_string(),
            properties: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        entities.push(entity);
    }

    // Batch upsert in SQLite
    let sqlite_ids = {
        let backend = sqlite_backend.lock().await;
        backend.batch_upsert_entities(&entities).await?
    };

    assert_eq!(sqlite_ids.len(), 10, "Should create 10 entities");

    // Batch upsert in Neo4j if available
    if let Some(neo4j_backend) = &neo4j_backend {
        let neo4j_ids = {
            let backend = neo4j_backend.lock().await;
            backend.batch_upsert_entities(&entities).await?
        };

        assert_eq!(neo4j_ids.len(), 10, "Should create 10 entities");

        // Retrieve and compare all entities
        let sqlite_entities = {
            let backend = sqlite_backend.lock().await;
            let mut all_entities = Vec::new();
            for &id in &sqlite_ids {
                if let Some(entity) = backend.get_entity_by_id(id).await? {
                    all_entities.push(entity);
                }
            }
            all_entities
        };

        let neo4j_entities = {
            let backend = neo4j_backend.lock().await;
            let mut all_entities = Vec::new();
            for &id in &neo4j_ids {
                if let Some(entity) = backend.get_entity_by_id(id).await? {
                    all_entities.push(entity);
                }
            }
            all_entities
        };

        compare_new_entity_lists(&sqlite_entities, &neo4j_entities, "batch_operations")?;
    }

    println!("✓ Extended batch operations completed successfully");
    Ok(())
}

// ===== RELATIONSHIP PARITY TESTS =====

#[tokio::test]
async fn test_extended_relationship_crud_operations() -> Result<()> {
    let (sqlite_backend, neo4j_backend) = setup_extended_test_backends().await?;

    // Create source and target entities
    let source_entity = Entity {
        id: None,
        name: "source_function".to_string(),
        entity_type: EntityType::Function,
        file_path: "/tmp/relationship_test.rs".to_string(),
        language: "rust".to_string(),
        properties: HashMap::new(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let target_entity = Entity {
        id: None,
        name: "target_function".to_string(),
        entity_type: EntityType::Function,
        file_path: "/tmp/relationship_test.rs".to_string(),
        language: "rust".to_string(),
        properties: HashMap::new(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    // Create entities in SQLite
    let sqlite_source_id = {
        let backend = sqlite_backend.lock().await;
        backend.upsert_entity(&source_entity).await?
    };

    let sqlite_target_id = {
        let backend = sqlite_backend.lock().await;
        backend.upsert_entity(&target_entity).await?
    };

    // Create relationship
    let relationship = Relationship {
        id: None,
        from_id: sqlite_source_id,
        to_id: sqlite_target_id,
        relationship_type: NewRelationType::Calls,
        properties: HashMap::new(),
        created_at: chrono::Utc::now(),
    };

    let sqlite_rel_id = {
        let backend = sqlite_backend.lock().await;
        backend.create_relationship(&relationship).await?
    };

    // Verify relationship exists
    let sqlite_relationship = {
        let backend = sqlite_backend.lock().await;
        backend.get_relationship_by_id(sqlite_rel_id).await?
    };

    assert!(sqlite_relationship.is_some(), "Relationship should exist in SQLite");

    // Test with Neo4j if available
    if let Some(neo4j_backend) = &neo4j_backend {
        let neo4j_source_id = {
            let backend = neo4j_backend.lock().await;
            backend.upsert_entity(&source_entity).await?
        };

        let neo4j_target_id = {
            let backend = neo4j_backend.lock().await;
            backend.upsert_entity(&target_entity).await?
        };

        let mut neo4j_relationship = relationship.clone();
        neo4j_relationship.from_id = neo4j_source_id;
        neo4j_relationship.to_id = neo4j_target_id;

        let neo4j_rel_id = {
            let backend = neo4j_backend.lock().await;
            backend.create_relationship(&neo4j_relationship).await?
        };

        let neo4j_relationship = {
            let backend = neo4j_backend.lock().await;
            backend.get_relationship_by_id(neo4j_rel_id).await?
        };

        assert!(neo4j_relationship.is_some(), "Relationship should exist in Neo4j");

        // Compare relationships
        if let (Some(sqlite_rel), Some(neo4j_rel)) = (&sqlite_relationship, &neo4j_relationship) {
            compare_relationship_lists(
                &[sqlite_rel.clone()],
                &[neo4j_rel.clone()],
                "relationship_crud",
            )?;
        }
    }

    println!("✓ Extended relationship CRUD operations completed successfully");
    Ok(())
}

#[tokio::test]
async fn test_extended_batch_relationship_operations() -> Result<()> {
    let (sqlite_backend, neo4j_backend) = setup_extended_test_backends().await?;

    // Create entities for relationships
    let mut entity_ids = Vec::new();
    for i in 0..6 {
        let entity = Entity {
            id: None,
            name: format!("rel_entity_{}", i),
            entity_type: EntityType::Function,
            file_path: format!("/tmp/rel_test_{}.rs", i),
            language: "rust".to_string(),
            properties: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let id = {
            let backend = sqlite_backend.lock().await;
            backend.upsert_entity(&entity).await?
        };
        entity_ids.push(id);
    }

    // Create relationships
    let mut relationships = Vec::new();
    for i in 0..5 {
        let relationship = Relationship {
            id: None,
            from_id: entity_ids[i],
            to_id: entity_ids[i + 1],
            relationship_type: NewRelationType::Calls,
            properties: HashMap::new(),
            created_at: chrono::Utc::now(),
        };
        relationships.push(relationship);
    }

    // Batch create relationships in SQLite
    let sqlite_rel_ids = {
        let backend = sqlite_backend.lock().await;
        backend.batch_create_relationships(&relationships).await?
    };

    assert_eq!(sqlite_rel_ids.len(), 5, "Should create 5 relationships");

    // Test with Neo4j if available
    if let Some(neo4j_backend) = &neo4j_backend {
        let mut neo4j_entity_ids = Vec::new();
        for i in 0..6 {
            let entity = Entity {
                id: None,
                name: format!("neo4j_rel_entity_{}", i),
                entity_type: EntityType::Function,
                file_path: format!("/tmp/neo4j_rel_test_{}.rs", i),
                language: "rust".to_string(),
                properties: HashMap::new(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };

            let id = {
                let backend = neo4j_backend.lock().await;
                backend.upsert_entity(&entity).await?
            };
            neo4j_entity_ids.push(id);
        }

        let mut neo4j_relationships = Vec::new();
        for i in 0..5 {
            let relationship = Relationship {
                id: None,
                from_id: neo4j_entity_ids[i],
                to_id: neo4j_entity_ids[i + 1],
                relationship_type: NewRelationType::Calls,
                properties: HashMap::new(),
                created_at: chrono::Utc::now(),
            };
            neo4j_relationships.push(relationship);
        }

        let neo4j_rel_ids = {
            let backend = neo4j_backend.lock().await;
            backend.batch_create_relationships(&neo4j_relationships).await?
        };

        assert_eq!(neo4j_rel_ids.len(), 5, "Should create 5 relationships");

        // Retrieve and compare relationships
        let sqlite_relationships = {
            let backend = sqlite_backend.lock().await;
            let mut all_rels = Vec::new();
            for &id in &sqlite_rel_ids {
                if let Some(rel) = backend.get_relationship_by_id(id).await? {
                    all_rels.push(rel);
                }
            }
            all_rels
        };

        let neo4j_relationships = {
            let backend = neo4j_backend.lock().await;
            let mut all_rels = Vec::new();
            for &id in &neo4j_rel_ids {
                if let Some(rel) = backend.get_relationship_by_id(id).await? {
                    all_rels.push(rel);
                }
            }
            all_rels
        };

        compare_relationship_lists(
            &sqlite_relationships,
            &neo4j_relationships,
            "batch_relationships",
        )?;
    }

    println!("✓ Extended batch relationship operations completed successfully");
    Ok(())
}

// ===== QUERY PARITY TESTS =====

#[tokio::test]
async fn test_extended_query_operations() -> Result<()> {
    let (sqlite_backend, neo4j_backend) = setup_extended_test_backends().await?;

    // Create test entities with different types
    let mut entities = Vec::new();
    let entity_types = vec![
        EntityType::Function,
        EntityType::Struct,
        EntityType::Enum,
        EntityType::Trait,
        EntityType::Module,
    ];

    for (i, &entity_type) in entity_types.iter().enumerate() {
        let entity = Entity {
            id: None,
            name: format!("query_entity_{}", i),
            entity_type,
            file_path: format!("/tmp/query_test_{}.rs", i),
            language: "rust".to_string(),
            properties: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        entities.push(entity);
    }

    // Insert entities
    let mut entity_ids = Vec::new();
    for entity in &entities {
        let id = {
            let backend = sqlite_backend.lock().await;
            backend.upsert_entity(entity).await?
        };
        entity_ids.push(id);
    }

    // Test queries by entity type
    for (i, &entity_type) in entity_types.iter().enumerate() {
        let sqlite_results = {
            let backend = sqlite_backend.lock().await;
            backend.find_entities_by_type(entity_type).await?
        };

        assert_eq!(sqlite_results.len(), 1, "Should find 1 entity for type {:?}", entity_type);

        // Test with Neo4j if available
        if let Some(neo4j_backend) = &neo4j_backend {
            let neo4j_results = {
                let backend = neo4j_backend.lock().await;
                backend.find_entities_by_type(entity_type).await?
            };

            compare_new_entity_lists(
                &sqlite_results,
                &neo4j_results,
                &format!("query_by_type_{:?}", entity_type),
            )?;
        }
    }

    // Test name search
    let sqlite_name_results = {
        let backend = sqlite_backend.lock().await;
        backend.find_entities_by_name("query_entity").await?
    };

    assert_eq!(sqlite_name_results.len(), 5, "Should find all 5 entities");

    if let Some(neo4j_backend) = &neo4j_backend {
        let neo4j_name_results = {
            let backend = neo4j_backend.lock().await;
            backend.find_entities_by_name("query_entity").await?
        };

        compare_new_entity_lists(&sqlite_name_results, &neo4j_name_results, "query_by_name")?;
    }

    println!("✓ Extended query operations completed successfully");
    Ok(())
}

#[tokio::test]
async fn test_extended_neighbor_operations() -> Result<()> {
    let (sqlite_backend, neo4j_backend) = setup_extended_test_backends().await?;

    // Create a star pattern: center -> multiple neighbors
    let center_entity = Entity {
        id: None,
        name: "center_function".to_string(),
        entity_type: EntityType::Function,
        file_path: "/tmp/neighbor_test.rs".to_string(),
        language: "rust".to_string(),
        properties: HashMap::new(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let center_id = {
        let backend = sqlite_backend.lock().await;
        backend.upsert_entity(&center_entity).await?
    };

    // Create neighbor entities
    let mut neighbor_ids = Vec::new();
    for i in 0..5 {
        let neighbor = Entity {
            id: None,
            name: format!("neighbor_function_{}", i),
            entity_type: EntityType::Function,
            file_path: format!("/tmp/neighbor_test_{}.rs", i),
            language: "rust".to_string(),
            properties: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let id = {
            let backend = sqlite_backend.lock().await;
            backend.upsert_entity(&neighbor).await?
        };
        neighbor_ids.push(id);
    }

    // Create relationships from center to neighbors
    for &neighbor_id in &neighbor_ids {
        let relationship = Relationship {
            id: None,
            from_id: center_id,
            to_id: neighbor_id,
            relationship_type: NewRelationType::Calls,
            properties: HashMap::new(),
            created_at: chrono::Utc::now(),
        };

        {
            let backend = sqlite_backend.lock().await;
            backend.create_relationship(&relationship).await?;
        }
    }

    // Test neighbor queries
    let sqlite_neighbors = {
        let backend = sqlite_backend.lock().await;
        backend.get_neighbors(center_id).await?
    };

    assert_eq!(sqlite_neighbors.len(), 5, "Should find 5 neighbors");

    if let Some(neo4j_backend) = &neo4j_backend {
        // Recreate structure in Neo4j
        let neo4j_center_id = {
            let backend = neo4j_backend.lock().await;
            backend.upsert_entity(&center_entity).await?
        };

        let mut neo4j_neighbor_ids = Vec::new();
        for i in 0..5 {
            let neighbor = Entity {
                id: None,
                name: format!("neighbor_function_{}", i),
                entity_type: EntityType::Function,
                file_path: format!("/tmp/neighbor_test_{}.rs", i),
                language: "rust".to_string(),
                properties: HashMap::new(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };

            let id = {
                let backend = neo4j_backend.lock().await;
                backend.upsert_entity(&neighbor).await?
            };
            neo4j_neighbor_ids.push(id);
        }

        for &neighbor_id in &neo4j_neighbor_ids {
            let relationship = Relationship {
                id: None,
                from_id: neo4j_center_id,
                to_id: neighbor_id,
                relationship_type: NewRelationType::Calls,
                properties: HashMap::new(),
                created_at: chrono::Utc::now(),
            };

            {
                let backend = neo4j_backend.lock().await;
                backend.create_relationship(&relationship).await?;
            }
        }

        let neo4j_neighbors = {
            let backend = neo4j_backend.lock().await;
            backend.get_neighbors(neo4j_center_id).await?
        };

        compare_new_entity_lists(&sqlite_neighbors, &neo4j_neighbors, "neighbor_operations")?;
    }

    println!("✓ Extended neighbor operations completed successfully");
    Ok(())
}

// ===== ERROR HANDLING PARITY TESTS =====

#[tokio::test]
async fn test_extended_error_handling() -> Result<()> {
    let (sqlite_backend, neo4j_backend) = setup_extended_test_backends().await?;

    // Test invalid entity ID
    let sqlite_result = {
        let backend = sqlite_backend.lock().await;
        backend.get_entity_by_id(999999).await
    };

    assert!(sqlite_result.is_ok());
    assert!(sqlite_result.unwrap().is_none(), "Should return None for invalid ID");

    if let Some(neo4j_backend) = &neo4j_backend {
        let neo4j_result = {
            let backend = neo4j_backend.lock().await;
            backend.get_entity_by_id(999999).await
        };

        assert!(neo4j_result.is_ok());
        assert!(neo4j_result.unwrap().is_none(), "Should return None for invalid ID");
    }

    // Test invalid relationship ID
    let sqlite_rel_result = {
        let backend = sqlite_backend.lock().await;
        backend.get_relationship_by_id(999999).await
    };

    assert!(sqlite_rel_result.is_ok());
    assert!(sqlite_rel_result.unwrap().is_none(), "Should return None for invalid relationship ID");

    if let Some(neo4j_backend) = &neo4j_backend {
        let neo4j_rel_result = {
            let backend = neo4j_backend.lock().await;
            backend.get_relationship_by_id(999999).await
        };

        assert!(neo4j_rel_result.is_ok());
        assert!(
            neo4j_rel_result.unwrap().is_none(),
            "Should return None for invalid relationship ID"
        );
    }

    // Test empty search queries
    let sqlite_search_result = {
        let backend = sqlite_backend.lock().await;
        backend.find_entities_by_name("").await
    };

    assert!(sqlite_search_result.is_ok());
    assert!(
        sqlite_search_result.unwrap().is_empty(),
        "Should return empty results for empty query"
    );

    if let Some(neo4j_backend) = &neo4j_backend {
        let neo4j_search_result = {
            let backend = neo4j_backend.lock().await;
            backend.find_entities_by_name("").await
        };

        assert!(neo4j_search_result.is_ok());
        assert!(
            neo4j_search_result.unwrap().is_empty(),
            "Should return empty results for empty query"
        );
    }

    println!("✓ Extended error handling completed successfully");
    Ok(())
}

// ===== ORDERING PARITY TESTS =====

#[tokio::test]
async fn test_extended_ordering_consistency() -> Result<()> {
    let (sqlite_backend, neo4j_backend) = setup_extended_test_backends().await?;

    // Create entities with known ordering
    let mut entities = Vec::new();
    for i in 0..10 {
        let entity = Entity {
            id: None,
            name: format!("ordering_entity_{:02}", i),
            entity_type: EntityType::Function,
            file_path: format!("/tmp/ordering_test_{:02}.rs", i),
            language: "rust".to_string(),
            properties: HashMap::new(),
            created_at: chrono::Utc::now() + chrono::Duration::milliseconds(i as i64),
            updated_at: chrono::Utc::now(),
        };
        entities.push(entity);
    }

    // Insert entities in reverse order
    let mut entity_ids = Vec::new();
    for entity in entities.iter().rev() {
        let id = {
            let backend = sqlite_backend.lock().await;
            backend.upsert_entity(entity).await?
        };
        entity_ids.push(id);
    }

    // Retrieve all entities
    let sqlite_entities = {
        let backend = sqlite_backend.lock().await;
        let mut all_entities = Vec::new();
        for &id in &entity_ids {
            if let Some(entity) = backend.get_entity_by_id(id).await? {
                all_entities.push(entity);
            }
        }
        all_entities
    };

    // Sort by ID for deterministic comparison
    let mut sqlite_sorted = sqlite_entities.clone();
    sqlite_sorted.sort_by(|a, b| a.id.cmp(&b.id));

    assert_eq!(sqlite_sorted.len(), 10, "Should have 10 entities");

    if let Some(neo4j_backend) = &neo4j_backend {
        // Recreate in Neo4j
        let mut neo4j_entity_ids = Vec::new();
        for entity in entities.iter().rev() {
            let id = {
                let backend = neo4j_backend.lock().await;
                backend.upsert_entity(entity).await?
            };
            neo4j_entity_ids.push(id);
        }

        let neo4j_entities = {
            let backend = neo4j_backend.lock().await;
            let mut all_entities = Vec::new();
            for &id in &neo4j_entity_ids {
                if let Some(entity) = backend.get_entity_by_id(id).await? {
                    all_entities.push(entity);
                }
            }
            all_entities
        };

        let mut neo4j_sorted = neo4j_entities.clone();
        neo4j_sorted.sort_by(|a, b| a.id.cmp(&b.id));

        compare_new_entity_lists(&sqlite_sorted, &neo4j_sorted, "ordering_consistency")?;
    }

    println!("✓ Extended ordering consistency completed successfully");
    Ok(())
}

// ===== RAGGRAPH PARITY TESTS =====

#[tokio::test]
async fn test_extended_raggraph_operations() -> Result<()> {
    let (sqlite_backend, neo4j_backend) = setup_extended_test_backends().await?;

    // Test task node creation
    let task_entity = Entity {
        id: None,
        name: "test_task".to_string(),
        entity_type: EntityType::Task,
        file_path: "/tmp/task_test.rs".to_string(),
        language: "rust".to_string(),
        properties: {
            let mut props = HashMap::new();
            props.insert("priority".to_string(), "high".to_string());
            props.insert("status".to_string(), "pending".to_string());
            props
        },
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let sqlite_task_id = {
        let backend = sqlite_backend.lock().await;
        backend.create_task_node(&task_entity).await?
    };

    // Verify task node
    let sqlite_task = {
        let backend = sqlite_backend.lock().await;
        backend.get_entity_by_id(sqlite_task_id).await?
    };

    assert!(sqlite_task.is_some(), "Task should exist in SQLite");

    if let Some(neo4j_backend) = &neo4j_backend {
        let neo4j_task_id = {
            let backend = neo4j_backend.lock().await;
            backend.create_task_node(&task_entity).await?
        };

        let neo4j_task = {
            let backend = neo4j_backend.lock().await;
            backend.get_entity_by_id(neo4j_task_id).await?
        };

        assert!(neo4j_task.is_some(), "Task should exist in Neo4j");

        if let (Some(sqlite_task), Some(neo4j_task)) = (&sqlite_task, &neo4j_task) {
            compare_new_entity_lists(
                &[sqlite_task.clone()],
                &[neo4j_task.clone()],
                "raggraph_task",
            )?;
        }
    }

    // Test memory node creation
    let memory_entity = Entity {
        id: None,
        name: "test_memory".to_string(),
        entity_type: EntityType::Memory,
        file_path: "/tmp/memory_test.rs".to_string(),
        language: "rust".to_string(),
        properties: {
            let mut props = HashMap::new();
            props.insert("content".to_string(), "test memory content".to_string());
            props.insert("namespace".to_string(), "test".to_string());
            props
        },
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let sqlite_memory_id = {
        let backend = sqlite_backend.lock().await;
        backend.create_memory_node(&memory_entity).await?
    };

    let sqlite_memory = {
        let backend = sqlite_backend.lock().await;
        backend.get_entity_by_id(sqlite_memory_id).await?
    };

    assert!(sqlite_memory.is_some(), "Memory should exist in SQLite");

    if let Some(neo4j_backend) = &neo4j_backend {
        let neo4j_memory_id = {
            let backend = neo4j_backend.lock().await;
            backend.create_memory_node(&memory_entity).await?
        };

        let neo4j_memory = {
            let backend = neo4j_backend.lock().await;
            backend.get_entity_by_id(neo4j_memory_id).await?
        };

        assert!(neo4j_memory.is_some(), "Memory should exist in Neo4j");

        if let (Some(sqlite_memory), Some(neo4j_memory)) = (&sqlite_memory, &neo4j_memory) {
            compare_new_entity_lists(
                &[sqlite_memory.clone()],
                &[neo4j_memory.clone()],
                "raggraph_memory",
            )?;
        }
    }

    // Test embedding node creation
    let embedding_entity = Entity {
        id: None,
        name: "test_embedding".to_string(),
        entity_type: EntityType::Embedding,
        file_path: "/tmp/embedding_test.rs".to_string(),
        language: "rust".to_string(),
        properties: {
            let mut props = HashMap::new();
            props.insert("vector_id".to_string(), "12345".to_string());
            props.insert("dimension".to_string(), "384".to_string());
            props
        },
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let sqlite_embedding_id = {
        let backend = sqlite_backend.lock().await;
        backend.create_embedding_node(&embedding_entity).await?
    };

    let sqlite_embedding = {
        let backend = sqlite_backend.lock().await;
        backend.get_entity_by_id(sqlite_embedding_id).await?
    };

    assert!(sqlite_embedding.is_some(), "Embedding should exist in SQLite");

    if let Some(neo4j_backend) = &neo4j_backend {
        let neo4j_embedding_id = {
            let backend = neo4j_backend.lock().await;
            backend.create_embedding_node(&embedding_entity).await?
        };

        let neo4j_embedding = {
            let backend = neo4j_backend.lock().await;
            backend.get_entity_by_id(neo4j_embedding_id).await?
        };

        assert!(neo4j_embedding.is_some(), "Embedding should exist in Neo4j");

        if let (Some(sqlite_embedding), Some(neo4j_embedding)) =
            (&sqlite_embedding, &neo4j_embedding)
        {
            compare_new_entity_lists(
                &[sqlite_embedding.clone()],
                &[neo4j_embedding.clone()],
                "raggraph_embedding",
            )?;
        }
    }

    println!("✓ Extended RAGGraph operations completed successfully");
    Ok(())
}
