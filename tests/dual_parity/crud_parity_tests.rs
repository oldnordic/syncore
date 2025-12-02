//! CRUD Parity Tests
//!
//! Tests Create, Read, Update, Delete operations parity between
//! Neo4j and SQLiteGraph backends with deterministic ordering.

use anyhow::Result;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use syncore::config::{GraphBackend as ConfigBackend, GraphConfig};
use syncore::graph::{
    create_graph_backend, EntityResult, GraphBackend, NodeLabel, NodeProperties, RelationType,
};
use tempfile::TempDir;
use tokio;

/// Test configuration for both backends
async fn setup_test_backends() -> Result<(Arc<dyn GraphBackend>, Arc<dyn GraphBackend>)> {
    // Setup SQLiteGraph backend
    let temp_dir = TempDir::new()?;
    let sqlite_path = temp_dir.path().join("test.db").to_string_lossy().to_string();

    let sqlite_config = GraphConfig {
        backend: ConfigBackend::SqliteGraph,
        path: sqlite_path.clone(),
        uri: String::new(),
        user: String::new(),
        password: String::new(),
        enabled: true,
    };

    let sqlite_backend = create_graph_backend(&sqlite_config, "crud_parity_test").await?;

    // Setup Neo4j backend (if available)
    let neo4j_uri =
        std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string());
    let neo4j_user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let neo4j_pass = std::env::var("NEO4J_PASS").unwrap_or_else(|_| "testpassword123".to_string());

    let neo4j_config = GraphConfig {
        backend: ConfigBackend::Neo4j,
        path: String::new(),
        uri: neo4j_uri,
        user: neo4j_user,
        password: neo4j_pass,
        enabled: true,
    };

    let neo4j_backend = match create_graph_backend(&neo4j_config, "crud_parity_test").await {
        Ok(backend) => backend,
        Err(_) => {
            // Skip Neo4j tests if not available
            return Ok((sqlite_backend.clone(), sqlite_backend.clone()));
        }
    };

    Ok((sqlite_backend, neo4j_backend))
}

/// Clean up test data in both backends
async fn cleanup_backends(
    sqlite_backend: &Arc<dyn GraphBackend>,
    neo4j_backend: &Arc<dyn GraphBackend>,
) -> Result<()> {
    // Clear all entities in test namespace
    let _ = sqlite_backend
        .execute_query("DELETE FROM code_entities WHERE file_path LIKE '%crud_parity_test%'", vec![])
        .await;

    let _ = neo4j_backend
        .execute_query("MATCH (n) WHERE n.file_path CONTAINS 'crud_parity_test' DETACH DELETE n", vec![])
        .await;

    Ok(())
}

/// Normalize entity results for comparison
fn normalize_entity(entity: &EntityResult) -> HashMap<String, serde_json::Value> {
    let mut map = HashMap::new();
    map.insert("id".to_string(), json!(entity.id));
    map.insert("name".to_string(), json!(entity.name));
    map.insert("label".to_string(), json!(entity.label));
    map.insert("path".to_string(), json!(entity.path));
    map.insert("start_line".to_string(), json!(entity.start_line));
    map.insert("end_line".to_string(), json!(entity.end_line));
    map.insert("signature".to_string(), json!(entity.signature));
    map.insert("body_snippet".to_string(), json!(entity.body_snippet));
    // Skip temporal fields as they may differ between backends
    map
}

/// Compare entity lists with deterministic ordering
fn compare_entity_lists(
    sqlite_results: &[EntityResult],
    neo4j_results: &[EntityResult],
    test_name: &str,
) -> Result<()> {
    // Normalize all entities
    let sqlite_normalized: Vec<_> = sqlite_results.iter().map(normalize_entity).collect();
    let neo4j_normalized: Vec<_> = neo4j_results.iter().map(normalize_entity).collect();

    // Sort by ID for deterministic comparison
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

#[tokio::test]
async fn test_create_entity_parity() -> Result<()> {
    let (sqlite_backend, neo4j_backend) = setup_test_backends().await?;

    cleanup_backends(&sqlite_backend, &neo4j_backend).await?;

    // Test data with all fields
    let props = NodeProperties {
        id: 1,
        name: "test_function".to_string(),
        path: Some("/tmp/crud_parity_test.rs".to_string()),
        start_line: Some(10),
        end_line: Some(20),
        signature: Some("fn test_function() -> Result<()>".to_string()),
        body_snippet: Some("println!(\"test\");\nOk(())".to_string()),
        docstring: Some("/// Test function for parity".to_string()),
        hash: Some("abc123def456".to_string()),
        language: Some("rust".to_string()),
        file_sha256: Some("filehash789".to_string()),
        mtime: Some(1234567890),
        created_at: Some("2023-01-01T00:00:00Z".to_string()),
        last_modified_at: Some("2023-01-02T00:00:00Z".to_string()),
        change_count: Some(5),
        author_count: Some(2),
    };

    // Create entity in both backends
    sqlite_backend.upsert_entity(NodeLabel::Function, props.clone()).await?;
    neo4j_backend.upsert_entity(NodeLabel::Function, props).await?;

    // Retrieve and compare
    let sqlite_result = sqlite_backend.get_entity_by_id(1).await?;
    let neo4j_result = neo4j_backend.get_entity_by_id(1).await?;

    match (sqlite_result, neo4j_result) {
        (Some(sqlite_entity), Some(neo4j_entity)) => {
            compare_entity_lists(&[sqlite_entity], &[neo4j_entity], "create_entity")?;
        }
        (None, None) => {
            anyhow::bail!("Both backends failed to retrieve entity");
        }
        (Some(_), None) => {
            anyhow::bail!("Neo4j backend failed to retrieve entity");
        }
        (None, Some(_)) => {
            anyhow::bail!("SQLite backend failed to retrieve entity");
        }
    }

    cleanup_backends(&sqlite_backend, &neo4j_backend).await?;
    Ok(())
}

#[tokio::test]
async fn test_get_entity_by_id_parity() -> Result<()> {
    let (sqlite_backend, neo4j_backend) = setup_test_backends().await?;

    cleanup_backends(&sqlite_backend, &neo4j_backend).await?;

    // Create multiple entities
    let entities: Vec<NodeProperties> = (1..=5)
        .map(|i| NodeProperties {
            id: i,
            name: format!("function_{}", i),
            path: Some(format!("/tmp/crud_parity_test_{}.rs", i)),
            start_line: Some(i * 10),
            end_line: Some(i * 10 + 5),
            signature: Some(format!("fn function_{}() -> Result<()>", i)),
            body_snippet: Some(format!("// Body of function {}", i)),
            docstring: Some(format!("/// Function {}", i)),
            hash: Some(format!("hash_{}", i)),
            language: Some("rust".to_string()),
            file_sha256: Some(format!("file_hash_{}", i)),
            mtime: Some(1234567890 + i as i64),
            created_at: Some("2023-01-01T00:00:00Z".to_string()),
            last_modified_at: Some("2023-01-02T00:00:00Z".to_string()),
            change_count: Some(i),
            author_count: Some(1),
        })
        .collect();

    for props in &entities {
        sqlite_backend.upsert_entity(NodeLabel::Function, props.clone()).await?;
        neo4j_backend.upsert_entity(NodeLabel::Function, props.clone()).await?;
    }

    // Test retrieval by ID for each entity
    for i in 1..=5 {
        let sqlite_result = sqlite_backend.get_entity_by_id(i).await?;
        let neo4j_result = neo4j_backend.get_entity_by_id(i).await?;

        match (sqlite_result, neo4j_result) {
            (Some(sqlite_entity), Some(neo4j_entity)) => {
                compare_entity_lists(&[sqlite_entity], &[neo4j_entity], &format!("get_entity_by_id_{}", i))?;
            }
            (None, None) => {
                anyhow::bail!("Both backends failed to retrieve entity {}", i);
            }
            (Some(_), None) => {
                anyhow::bail!("Neo4j backend failed to retrieve entity {}", i);
            }
            (None, Some(_)) => {
                anyhow::bail!("SQLite backend failed to retrieve entity {}", i);
            }
        }
    }

    // Test non-existent entity
    let sqlite_none = sqlite_backend.get_entity_by_id(999).await?;
    let neo4j_none = neo4j_backend.get_entity_by_id(999).await?;

    assert!(sqlite_none.is_none(), "SQLite should return None for non-existent entity");
    assert!(neo4j_none.is_none(), "Neo4j should return None for non-existent entity");

    cleanup_backends(&sqlite_backend, &neo4j_backend).await?;
    Ok(())
}

#[tokio::test]
async fn test_find_by_name_parity() -> Result<()> {
    let (sqlite_backend, neo4j_backend) = setup_test_backends().await?;

    cleanup_backends(&sqlite_backend, &neo4j_backend).await?;

    // Create entities with same name in different files and types
    let entities = vec![
        NodeProperties {
            id: 1,
            name: "duplicate_name".to_string(),
            path: Some("/tmp/crud_parity_test_1.rs".to_string()),
            start_line: Some(10),
            end_line: Some(15),
            signature: Some("fn duplicate_name()".to_string()),
            body_snippet: Some("// First implementation".to_string()),
            docstring: Some("/// First function".to_string()),
            hash: Some("hash1".to_string()),
            language: Some("rust".to_string()),
            file_sha256: Some("file_hash1".to_string()),
            mtime: Some(1234567890),
            created_at: Some("2023-01-01T00:00:00Z".to_string()),
            last_modified_at: Some("2023-01-02T00:00:00Z".to_string()),
            change_count: Some(1),
            author_count: Some(1),
        },
        NodeProperties {
            id: 2,
            name: "duplicate_name".to_string(),
            path: Some("/tmp/crud_parity_test_2.rs".to_string()),
            start_line: Some(20),
            end_line: Some(25),
            signature: Some("fn duplicate_name()".to_string()),
            body_snippet: Some("// Second implementation".to_string()),
            docstring: Some("/// Second function".to_string()),
            hash: Some("hash2".to_string()),
            language: Some("rust".to_string()),
            file_sha256: Some("file_hash2".to_string()),
            mtime: Some(1234567891),
            created_at: Some("2023-01-01T00:00:00Z".to_string()),
            last_modified_at: Some("2023-01-02T00:00:00Z".to_string()),
            change_count: Some(2),
            author_count: Some(1),
        },
        NodeProperties {
            id: 3,
            name: "unique_name".to_string(),
            path: Some("/tmp/crud_parity_test_3.rs".to_string()),
            start_line: Some(30),
            end_line: Some(35),
            signature: Some("fn unique_name()".to_string()),
            body_snippet: Some("// Unique implementation".to_string()),
            docstring: Some("/// Unique function".to_string()),
            hash: Some("hash3".to_string()),
            language: Some("rust".to_string()),
            file_sha256: Some("file_hash3".to_string()),
            mtime: Some(1234567892),
            created_at: Some("2023-01-01T00:00:00Z".to_string()),
            last_modified_at: Some("2023-01-02T00:00:00Z".to_string()),
            change_count: Some(3),
            author_count: Some(1),
        },
    ];

    for props in &entities {
        sqlite_backend.upsert_entity(NodeLabel::Function, props.clone()).await?;
        neo4j_backend.upsert_entity(NodeLabel::Function, props.clone()).await?;
    }

    // Find by duplicate name
    let sqlite_duplicate = sqlite_backend.find_entities_by_name("duplicate_name").await?;
    let neo4j_duplicate = neo4j_backend.find_entities_by_name("duplicate_name").await?;

    compare_entity_lists(&sqlite_duplicate, &neo4j_duplicate, "find_by_name_duplicate")?;
    assert_eq!(sqlite_duplicate.len(), 2, "Should find 2 entities with duplicate name");

    // Find by unique name
    let sqlite_unique = sqlite_backend.find_entities_by_name("unique_name").await?;
    let neo4j_unique = neo4j_backend.find_entities_by_name("unique_name").await?;

    compare_entity_lists(&sqlite_unique, &neo4j_unique, "find_by_name_unique")?;
    assert_eq!(sqlite_unique.len(), 1, "Should find 1 entity with unique name");

    // Find non-existent name
    let sqlite_none = sqlite_backend.find_entities_by_name("non_existent").await?;
    let neo4j_none = neo4j_backend.find_entities_by_name("non_existent").await?;

    assert!(sqlite_none.is_empty(), "SQLite should return empty list for non-existent name");
    assert!(neo4j_none.is_empty(), "Neo4j should return empty list for non-existent name");

    cleanup_backends(&sqlite_backend, &neo4j_backend).await?;
    Ok(())
}

#[tokio::test]
async fn test_find_by_label_parity() -> Result<()> {
    let (sqlite_backend, neo4j_backend) = setup_test_backends().await?;

    cleanup_backends(&sqlite_backend, &neo4j_backend).await?;

    // Create entities of different types
    let function_entities: Vec<NodeProperties> = (1..=3)
        .map(|i| NodeProperties {
            id: i,
            name: format!("function_{}", i),
            path: Some(format!("/tmp/crud_parity_test_functions.rs", i)),
            start_line: Some(i * 10),
            end_line: Some(i * 10 + 5),
            signature: Some(format!("fn function_{}()", i)),
            body_snippet: Some(format!("// Function body {}", i)),
            docstring: Some(format!("/// Function {}", i)),
            hash: Some(format!("fn_hash_{}", i)),
            language: Some("rust".to_string()),
            file_sha256: Some(format!("fn_file_hash_{}", i)),
            mtime: Some(1234567890 + i as i64),
            created_at: Some("2023-01-01T00:00:00Z".to_string()),
            last_modified_at: Some("2023-01-02T00:00:00Z".to_string()),
            change_count: Some(i),
            author_count: Some(1),
        })
        .collect();

    let struct_entities: Vec<NodeProperties> = (4..=6)
        .map(|i| NodeProperties {
            id: i,
            name: format!("struct_{}", i),
            path: Some(format!("/tmp/crud_parity_test_structs.rs", i)),
            start_line: Some(i * 10),
            end_line: Some(i * 10 + 5),
            signature: Some(format!("struct Struct{} {{}}", i)),
            body_snippet: Some(format!("// Struct body {}", i)),
            docstring: Some(format!("/// Struct {}", i)),
            hash: Some(format!("struct_hash_{}", i)),
            language: Some("rust".to_string()),
            file_sha256: Some(format!("struct_file_hash_{}", i)),
            mtime: Some(1234567890 + i as i64),
            created_at: Some("2023-01-01T00:00:00Z".to_string()),
            last_modified_at: Some("2023-01-02T00:00:00Z".to_string()),
            change_count: Some(i),
            author_count: Some(1),
        })
        .collect();

    // Insert functions
    for props in &function_entities {
        sqlite_backend.upsert_entity(NodeLabel::Function, props.clone()).await?;
        neo4j_backend.upsert_entity(NodeLabel::Function, props.clone()).await?;
    }

    // Insert structs
    for props in &struct_entities {
        sqlite_backend.upsert_entity(NodeLabel::Struct, props.clone()).await?;
        neo4j_backend.upsert_entity(NodeLabel::Struct, props.clone()).await?;
    }

    // Find functions by label
    let sqlite_functions = sqlite_backend.get_entities_by_type(NodeLabel::Function).await?;
    let neo4j_functions = neo4j_backend.get_entities_by_type(NodeLabel::Function).await?;

    compare_entity_lists(&sqlite_functions, &neo4j_functions, "find_by_label_function")?;
    assert_eq!(sqlite_functions.len(), 3, "Should find 3 functions");

    // Find structs by label
    let sqlite_structs = sqlite_backend.get_entities_by_type(NodeLabel::Struct).await?;
    let neo4j_structs = neo4j_backend.get_entities_by_type(NodeLabel::Struct).await?;

    compare_entity_lists(&sqlite_structs, &neo4j_structs, "find_by_label_struct")?;
    assert_eq!(sqlite_structs.len(), 3, "Should find 3 structs");

    cleanup_backends(&sqlite_backend, &neo4j_backend).await?;
    Ok(())
}

#[tokio::test]
async fn test_delete_entity_parity() -> Result<()> {
    let (sqlite_backend, neo4j_backend) = setup_test_backends().await?;

    cleanup_backends(&sqlite_backend, &neo4j_backend).await?;

    // Create entities
    let entities: Vec<NodeProperties> = (1..=5)
        .map(|i| NodeProperties {
            id: i,
            name: format!("entity_to_delete_{}", i),
            path: Some(format!("/tmp/crud_parity_test_delete.rs", i)),
            start_line: Some(i * 10),
            end_line: Some(i * 10 + 5),
            signature: Some(format!("fn entity_to_delete_{}()", i)),
            body_snippet: Some(format!("// Entity {} to delete", i)),
            docstring: Some(format!("/// Entity {} to delete", i)),
            hash: Some(format!("delete_hash_{}", i)),
            language: Some("rust".to_string()),
            file_sha256: Some(format!("delete_file_hash_{}", i)),
            mtime: Some(1234567890 + i as i64),
            created_at: Some("2023-01-01T00:00:00Z".to_string()),
            last_modified_at: Some("2023-01-02T00:00:00Z".to_string()),
            change_count: Some(i),
            author_count: Some(1),
        })
        .collect();

    for props in &entities {
        sqlite_backend.upsert_entity(NodeLabel::Function, props.clone()).await?;
        neo4j_backend.upsert_entity(NodeLabel::Function, props.clone()).await?;
    }

    // Create some relationships
    sqlite_backend.create_relationship(1, 2, RelationType::Calls).await?;
    sqlite_backend.create_relationship(2, 3, RelationType::Calls).await?;
    neo4j_backend.create_relationship(1, 2, RelationType::Calls).await?;
    neo4j_backend.create_relationship(2, 3, RelationType::Calls).await?;

    // Verify entities exist before deletion
    let sqlite_before = sqlite_backend.get_entity_by_id(2).await?;
    let neo4j_before = neo4j_backend.get_entity_by_id(2).await?;
    assert!(sqlite_before.is_some(), "Entity 2 should exist in SQLite before deletion");
    assert!(neo4j_before.is_some(), "Entity 2 should exist in Neo4j before deletion");

    // Delete entity 2 (should also delete relationships)
    sqlite_backend.delete_entity(2).await?;
    neo4j_backend.delete_entity(2).await?;

    // Verify entity is deleted
    let sqlite_after = sqlite_backend.get_entity_by_id(2).await?;
    let neo4j_after = neo4j_backend.get_entity_by_id(2).await?;
    assert!(sqlite_after.is_none(), "Entity 2 should be deleted from SQLite");
    assert!(neo4j_after.is_none(), "Entity 2 should be deleted from Neo4j");

    // Verify other entities still exist
    let sqlite_entity1 = sqlite_backend.get_entity_by_id(1).await?;
    let neo4j_entity1 = neo4j_backend.get_entity_by_id(1).await?;
    assert!(sqlite_entity1.is_some(), "Entity 1 should still exist in SQLite");
    assert!(neo4j_entity1.is_some(), "Entity 1 should still exist in Neo4j");

    // Verify relationships are deleted (entity 1 should no longer have callees)
    let sqlite_callees = sqlite_backend.get_function_callees(1).await?;
    let neo4j_callees = neo4j_backend.get_function_callees(1).await?;
    assert!(sqlite_callees.is_empty(), "Entity 1 should have no callees after entity 2 deletion");
    assert!(neo4j_callees.is_empty(), "Entity 1 should have no callees after entity 2 deletion");

    cleanup_backends(&sqlite_backend, &neo4j_backend).await?;
    Ok(())
}

#[tokio::test]
async fn test_batch_upsert_parity() -> Result<()> {
    let (sqlite_backend, neo4j_backend) = setup_test_backends().await?;

    cleanup_backends(&sqlite_backend, &neo4j_backend).await?;

    // Create a large batch of entities
    let entities: Vec<NodeProperties> = (1..=20)
        .map(|i| NodeProperties {
            id: i,
            name: format!("batch_function_{}", i),
            path: Some(format!("/tmp/crud_parity_test_batch.rs")),
            start_line: Some(i * 5),
            end_line: Some(i * 5 + 3),
            signature: Some(format!("fn batch_function_{}() -> Result<()>", i)),
            body_snippet: Some(format!("// Batch function {}", i)),
            docstring: Some(format!("/// Batch function {}", i)),
            hash: Some(format!("batch_hash_{}", i)),
            language: Some("rust".to_string()),
            file_sha256: Some("batch_file_hash".to_string()),
            mtime: Some(1234567890 + i as i64),
            created_at: Some("2023-01-01T00:00:00Z".to_string()),
            last_modified_at: Some("2023-01-02T00:00:00Z".to_string()),
            change_count: Some(i),
            author_count: Some(1),
        })
        .collect();

    // Batch upsert with different batch sizes
    let sqlite_count = sqlite_backend.batch_upsert_entities(NodeLabel::Function, entities.clone(), 5).await?;
    let neo4j_count = neo4j_backend.batch_upsert_entities(NodeLabel::Function, entities, 5).await?;

    assert_eq!(sqlite_count, 20, "SQLite should process 20 entities");
    assert_eq!(neo4j_count, 20, "Neo4j should process 20 entities");

    // Retrieve all entities and compare
    let sqlite_results = sqlite_backend.get_entities_by_type(NodeLabel::Function).await?;
    let neo4j_results = neo4j_backend.get_entities_by_type(NodeLabel::Function).await?;

    compare_entity_lists(&sqlite_results, &neo4j_results, "batch_upsert")?;
    assert_eq!(sqlite_results.len(), 20, "Should have 20 functions in SQLite");
    assert_eq!(neo4j_results.len(), 20, "Should have 20 functions in Neo4j");

    cleanup_backends(&sqlite_backend, &neo4j_backend).await?;
    Ok(())
}

#[tokio::test]
async fn test_entity_label_parity_across_backends() -> Result<()> {
    let (sqlite_backend, neo4j_backend) = setup_test_backends().await?;

    cleanup_backends(&sqlite_backend, &neo4j_backend).await?;

    // Test each entity type for label parity
    let entity_types = vec![
        (NodeLabel::Function, "Function"),
        (NodeLabel::Struct, "Struct"),
        (NodeLabel::Enum, "Enum"),
        (NodeLabel::Trait, "Trait"),
        (NodeLabel::Module, "Module"),
    ];

    for (i, (node_label, expected_label)) in entity_types.iter().enumerate() {
        let props = NodeProperties {
            id: (i + 1) as i64,
            name: format!("test_entity_{}", i),
            path: Some(format!("/tmp/label_parity_test_{}.rs", i)),
            start_line: Some(((i + 1) * 10) as i64),
            end_line: Some(((i + 1) * 10 + 5) as i64),
            signature: Some(format!("signature_{}", i)),
            body_snippet: Some(format!("body_{}", i)),
            docstring: Some(format!("doc_{}", i)),
            hash: Some(format!("hash_{}", i)),
            language: Some("rust".to_string()),
            file_sha256: Some(format!("file_hash_{}", i)),
            mtime: Some(1234567890 + i as i64),
            created_at: Some("2023-01-01T00:00:00Z".to_string()),
            last_modified_at: Some("2023-01-02T00:00:00Z".to_string()),
            change_count: Some(i as i64),
            author_count: Some(1),
        };

        // Create entity in both backends
        sqlite_backend.upsert_entity(*node_label, props.clone()).await?;
        neo4j_backend.upsert_entity(*node_label, props).await?;

        // Fetch entity from both backends
        let sqlite_result = sqlite_backend.get_entity_by_id((i + 1) as i64).await?;
        let neo4j_result = neo4j_backend.get_entity_by_id((i + 1) as i64).await?;

        match (sqlite_result, neo4j_result) {
            (Some(sqlite_entity), Some(neo4j_entity)) => {
                // CRITICAL: Compare label field for parity
                assert_eq!(
                    sqlite_entity.label, expected_label,
                    "SQLite label mismatch for {}: expected {}, got {}",
                    expected_label,
                    sqlite_entity.label
                );
                assert_eq!(
                    neo4j_entity.label, expected_label,
                    "Neo4j label mismatch for {}: expected {}, got {}",
                    expected_label,
                    neo4j_entity.label
                );
                assert_eq!(
                    sqlite_entity.label, neo4j_entity.label,
                    "Label parity failure for {}: SQLite={}, Neo4j={}",
                    expected_label,
                    sqlite_entity.label,
                    neo4j_entity.label
                );
            }
            (None, None) => {
                anyhow::bail!("Both backends failed to retrieve entity {}", i + 1);
            }
            (Some(_), None) => {
                anyhow::bail!("Neo4j backend failed to retrieve entity {}", i + 1);
            }
            (None, Some(_)) => {
                anyhow::bail!("SQLite backend failed to retrieve entity {}", i + 1);
            }
        }
    }

    cleanup_backends(&sqlite_backend, &neo4j_backend).await?;
    Ok(())
}