//! Integration Tests for Canonical Neo4j Module
//!
//! TDD: Write tests FIRST before migration
//! Tests verify:
//! 1. Write operations work (upsert, batch, delete)
//! 2. Read operations work (get, find, count)
//! 3. Namespace isolation (only "syncore" namespace)
//! 4. Schema validation (only canonical labels/rels)
//! 5. Idempotency (call twice, same result)

use anyhow::Result;
use syncore::databases::neo4j::{
    create_relationship, upsert_entity, batch_upsert_entities,
    get_entity_by_id, get_file_entities, get_function_callers, get_function_callees,
    count_entities_by_type, validate_structure,
    delete_entity, delete_file_entities,
    NodeLabel, RelationType, NodeProperties, project_namespace,
};
use syncore::graph::Neo4jClient;

/// Helper to get Neo4j connection
async fn get_neo4j_client() -> Result<Neo4jClient> {
    let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string());
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let pass = std::env::var("NEO4J_PASS").unwrap_or_else(|_| "testpassword123".to_string());

    Neo4jClient::connect(&uri, &user, &pass).await
}

/// Helper to clean up ALL test data (all namespaces) to ensure test isolation
async fn cleanup_test_data(client: &Neo4jClient) -> Result<()> {
    // Delete ALL nodes to ensure test isolation
    // This is necessary because previous test runs or existing code may have
    // written to different namespaces
    let query = "MATCH (n) DETACH DELETE n";

    client.execute_query(query, vec![]).await?;

    Ok(())
}

#[tokio::test]
async fn test_namespace_matches_client() -> Result<()> {
    let client = get_neo4j_client().await?;

    // Verify namespace matches client's namespace
    let ns = project_namespace(&client);
    assert!(!ns.is_empty(), "Namespace should not be empty");

    // Verify it matches what Neo4jClient reports
    assert_eq!(ns, client.namespace());

    Ok(())
}

#[tokio::test]
async fn test_upsert_entity_creates_node() -> Result<()> {
    let client = get_neo4j_client().await?;
    cleanup_test_data(&client).await?;

    // Create entity
    let props = NodeProperties::full(
        1001,
        "test_function".to_string(),
        "test.rs".to_string(),
        10,
        20,
        "rust".to_string(),
    );

    upsert_entity(&client, NodeLabel::Function, props).await?;

    // Verify entity exists
    let entity = get_entity_by_id(&client, 1001).await?;
    assert!(entity.is_some());
    let entity = entity.unwrap();
    assert_eq!(entity.name, "test_function");
    assert_eq!(entity.label, "Function");
    assert_eq!(entity.path, Some("test.rs".to_string()));
    assert_eq!(entity.start_line, Some(10));
    assert_eq!(entity.end_line, Some(20));

    cleanup_test_data(&client).await?;
    Ok(())
}

#[tokio::test]
async fn test_upsert_entity_is_idempotent() -> Result<()> {
    let client = get_neo4j_client().await?;
    cleanup_test_data(&client).await?;

    let props = NodeProperties::minimal(1002, "idempotent_test".to_string());

    // Call twice
    upsert_entity(&client, NodeLabel::Function, props.clone()).await?;
    upsert_entity(&client, NodeLabel::Function, props).await?;

    // Should only have 1 node
    let stats = validate_structure(&client).await?;
    assert_eq!(stats.total_nodes, 1);

    cleanup_test_data(&client).await?;
    Ok(())
}

#[tokio::test]
async fn test_create_relationship() -> Result<()> {
    let client = get_neo4j_client().await?;
    cleanup_test_data(&client).await?;

    // Create two entities
    let props1 = NodeProperties::minimal(1003, "caller".to_string());
    let props2 = NodeProperties::minimal(1004, "callee".to_string());

    upsert_entity(&client, NodeLabel::Function, props1).await?;
    upsert_entity(&client, NodeLabel::Function, props2).await?;

    // Create relationship
    create_relationship(&client, 1003, 1004, RelationType::Calls).await?;

    // Verify relationship exists
    let callees = get_function_callees(&client, 1003).await?;
    assert_eq!(callees.len(), 1);
    assert_eq!(callees[0].name, "callee");

    let callers = get_function_callers(&client, 1004).await?;
    assert_eq!(callers.len(), 1);
    assert_eq!(callers[0].name, "caller");

    cleanup_test_data(&client).await?;
    Ok(())
}

#[tokio::test]
async fn test_batch_upsert_entities() -> Result<()> {
    let client = get_neo4j_client().await?;
    cleanup_test_data(&client).await?;

    // Create batch of entities
    let entities = vec![
        NodeProperties::minimal(2001, "func1".to_string()),
        NodeProperties::minimal(2002, "func2".to_string()),
        NodeProperties::minimal(2003, "func3".to_string()),
    ];

    let count = batch_upsert_entities(&client, NodeLabel::Function, entities, 10).await?;
    assert_eq!(count, 3);

    // Verify all entities exist
    let stats = validate_structure(&client).await?;
    assert_eq!(stats.total_nodes, 3);

    cleanup_test_data(&client).await?;
    Ok(())
}

#[tokio::test]
async fn test_get_file_entities() -> Result<()> {
    let client = get_neo4j_client().await?;
    cleanup_test_data(&client).await?;

    // Create entities in same file
    let props1 = NodeProperties::full(3001, "func1".to_string(), "src/main.rs".to_string(), 10, 20, "rust".to_string());
    let props2 = NodeProperties::full(3002, "func2".to_string(), "src/main.rs".to_string(), 30, 40, "rust".to_string());
    let props3 = NodeProperties::full(3003, "func3".to_string(), "src/other.rs".to_string(), 10, 20, "rust".to_string());

    upsert_entity(&client, NodeLabel::Function, props1).await?;
    upsert_entity(&client, NodeLabel::Function, props2).await?;
    upsert_entity(&client, NodeLabel::Function, props3).await?;

    // Query entities in src/main.rs
    let entities = get_file_entities(&client, "src/main.rs").await?;
    assert_eq!(entities.len(), 2);
    assert!(entities.iter().any(|e| e.name == "func1"));
    assert!(entities.iter().any(|e| e.name == "func2"));

    cleanup_test_data(&client).await?;
    Ok(())
}

#[tokio::test]
async fn test_count_entities_by_type() -> Result<()> {
    let client = get_neo4j_client().await?;
    cleanup_test_data(&client).await?;

    // Create entities of different types
    upsert_entity(&client, NodeLabel::Function, NodeProperties::minimal(4001, "func1".to_string())).await?;
    upsert_entity(&client, NodeLabel::Function, NodeProperties::minimal(4002, "func2".to_string())).await?;
    upsert_entity(&client, NodeLabel::Struct, NodeProperties::minimal(4003, "struct1".to_string())).await?;

    let counts = count_entities_by_type(&client).await?;

    let func_count = counts.iter().find(|(label, _)| label == "Function").map(|(_, count)| *count).unwrap_or(0);
    let struct_count = counts.iter().find(|(label, _)| label == "Struct").map(|(_, count)| *count).unwrap_or(0);

    assert_eq!(func_count, 2);
    assert_eq!(struct_count, 1);

    cleanup_test_data(&client).await?;
    Ok(())
}

#[tokio::test]
async fn test_delete_entity() -> Result<()> {
    let client = get_neo4j_client().await?;
    cleanup_test_data(&client).await?;

    // Create entity
    upsert_entity(&client, NodeLabel::Function, NodeProperties::minimal(5001, "to_delete".to_string())).await?;

    // Verify exists
    assert!(get_entity_by_id(&client, 5001).await?.is_some());

    // Delete
    delete_entity(&client, 5001).await?;

    // Verify deleted
    assert!(get_entity_by_id(&client, 5001).await?.is_none());

    cleanup_test_data(&client).await?;
    Ok(())
}

#[tokio::test]
async fn test_delete_file_entities() -> Result<()> {
    let client = get_neo4j_client().await?;
    cleanup_test_data(&client).await?;

    // Create entities in file
    let props1 = NodeProperties::full(6001, "func1".to_string(), "src/delete_me.rs".to_string(), 10, 20, "rust".to_string());
    let props2 = NodeProperties::full(6002, "func2".to_string(), "src/delete_me.rs".to_string(), 30, 40, "rust".to_string());
    let props3 = NodeProperties::full(6003, "func3".to_string(), "src/keep_me.rs".to_string(), 10, 20, "rust".to_string());

    upsert_entity(&client, NodeLabel::Function, props1).await?;
    upsert_entity(&client, NodeLabel::Function, props2).await?;
    upsert_entity(&client, NodeLabel::Function, props3).await?;

    // Delete entities in src/delete_me.rs
    let deleted = delete_file_entities(&client, "src/delete_me.rs").await?;
    assert_eq!(deleted, 2);

    // Verify src/keep_me.rs still exists
    let remaining = get_file_entities(&client, "src/keep_me.rs").await?;
    assert_eq!(remaining.len(), 1);

    cleanup_test_data(&client).await?;
    Ok(())
}

#[tokio::test]
async fn test_namespace_isolation() -> Result<()> {
    let client = get_neo4j_client().await?;
    cleanup_test_data(&client).await?;

    // Create entity in "syncore" namespace
    upsert_entity(&client, NodeLabel::Function, NodeProperties::minimal(7001, "syncore_func".to_string())).await?;

    // Manually create entity in different namespace (for test)
    let query = r#"
        MERGE (e:Function {id: $id, namespace: $ns})
        SET e.name = $name
    "#;
    client.execute_query(query, vec![
        ("id", serde_json::json!(7002)),
        ("ns", serde_json::json!("other_namespace")),
        ("name", serde_json::json!("other_func")),
    ]).await?;

    // Canonical API should only see "syncore" namespace
    let entity = get_entity_by_id(&client, 7001).await?;
    assert!(entity.is_some());

    let entity = get_entity_by_id(&client, 7002).await?;
    assert!(entity.is_none(), "Should not see entities from other namespaces");

    cleanup_test_data(&client).await?;

    // Clean up other namespace
    client.execute_query(
        "MATCH (n {namespace: $ns}) DETACH DELETE n",
        vec![("ns", serde_json::json!("other_namespace"))],
    ).await?;

    Ok(())
}

#[tokio::test]
async fn test_validate_structure() -> Result<()> {
    let client = get_neo4j_client().await?;
    cleanup_test_data(&client).await?;

    // Create graph structure
    upsert_entity(&client, NodeLabel::Function, NodeProperties::minimal(8001, "func1".to_string())).await?;
    upsert_entity(&client, NodeLabel::Function, NodeProperties::minimal(8002, "func2".to_string())).await?;
    upsert_entity(&client, NodeLabel::Function, NodeProperties::minimal(8003, "orphan".to_string())).await?;

    create_relationship(&client, 8001, 8002, RelationType::Calls).await?;

    // Validate
    let stats = validate_structure(&client).await?;
    assert_eq!(stats.total_nodes, 3);
    assert_eq!(stats.total_edges, 1);
    assert_eq!(stats.orphan_count, 1); // orphan node

    cleanup_test_data(&client).await?;
    Ok(())
}
