//! Graph Backend Trait Tests
//!
//! Tests the GraphBackend trait and Neo4j implementation
//! These tests require a real Neo4j instance running

use anyhow::Result;
use std::env;
use syncore::graph::backend::{GraphBackend, NodeLabel, NodeProperties, RelationType};
use syncore::graph::Neo4jBackend;

#[tokio::test]
#[ignore] // Requires Neo4j instance
async fn test_neo4j_backend_connection() -> Result<()> {
    let uri = env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".to_string());
    let user = env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let pass = env::var("NEO4J_PASS").unwrap_or_else(|_| "password".to_string());
    let namespace = "test_backend";

    let backend = Neo4jBackend::connect(&uri, &user, &pass, namespace).await?;

    assert_eq!(backend.namespace(), namespace);

    Ok(())
}

#[tokio::test]
#[ignore] // Requires Neo4j instance
async fn test_neo4j_backend_entity_operations() -> Result<()> {
    let uri = env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".to_string());
    let user = env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let pass = env::var("NEO4J_PASS").unwrap_or_else(|_| "password".to_string());
    let namespace = "test_entity_ops";

    let backend = Neo4jBackend::connect(&uri, &user, &pass, namespace).await?;

    // Create a test entity
    let props = NodeProperties::full(
        1,
        "test_function".to_string(),
        "src/test.rs".to_string(),
        10,
        20,
        "rust".to_string(),
    );

    backend.upsert_entity(NodeLabel::Function, props.clone()).await?;

    // Retrieve the entity
    let entity = backend.get_entity_by_id(1).await?;
    assert!(entity.is_some());

    let entity = entity.unwrap();
    assert_eq!(entity.name, "test_function");
    assert_eq!(entity.label, "Function");
    assert_eq!(entity.path, Some("src/test.rs".to_string()));

    // Clean up
    backend.delete_entity(1).await?;

    // Verify deletion
    let deleted_entity = backend.get_entity_by_id(1).await?;
    assert!(deleted_entity.is_none());

    Ok(())
}

#[tokio::test]
#[ignore] // Requires Neo4j instance
async fn test_neo4j_backend_relationship_operations() -> Result<()> {
    let uri = env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".to_string());
    let user = env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let pass = env::var("NEO4J_PASS").unwrap_or_else(|_| "password".to_string());
    let namespace = "test_rel_ops";

    let backend = Neo4jBackend::connect(&uri, &user, &pass, namespace).await?;

    // Create two test entities
    let caller_props = NodeProperties::full(
        1,
        "caller_function".to_string(),
        "src/caller.rs".to_string(),
        10,
        20,
        "rust".to_string(),
    );

    let callee_props = NodeProperties::full(
        2,
        "callee_function".to_string(),
        "src/callee.rs".to_string(),
        30,
        40,
        "rust".to_string(),
    );

    backend.upsert_entity(NodeLabel::Function, caller_props).await?;
    backend.upsert_entity(NodeLabel::Function, callee_props).await?;

    // Create relationship
    backend.create_relationship(1, 2, RelationType::Calls).await?;

    // Test callees
    let callees = backend.get_function_callees(1).await?;
    assert_eq!(callees.len(), 1);
    assert_eq!(callees[0].name, "callee_function");

    // Test callers
    let callers = backend.get_function_callers(2).await?;
    assert_eq!(callers.len(), 1);
    assert_eq!(callers[0].name, "caller_function");

    // Clean up
    backend.delete_entity(1).await?;
    backend.delete_entity(2).await?;

    Ok(())
}

#[tokio::test]
#[ignore] // Requires Neo4j instance
async fn test_neo4j_backend_file_operations() -> Result<()> {
    let uri = env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".to_string());
    let user = env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let pass = env::var("NEO4J_PASS").unwrap_or_else(|_| "password".to_string());
    let namespace = "test_file_ops";

    let backend = Neo4jBackend::connect(&uri, &user, &pass, namespace).await?;

    // Create file dependency
    backend.create_file_dependency("src/main.rs", "src/utils.rs").await?;

    // Get file entities
    let main_entities = backend.get_file_entities("src/main.rs").await?;
    let utils_entities = backend.get_file_entities("src/utils.rs").await?;

    // Should have at least the File nodes we created
    assert!(!main_entities.is_empty() || !utils_entities.is_empty());

    Ok(())
}

#[test]
fn test_node_label_and_relation_type_conversions() {
    // Test NodeLabel conversions
    assert_eq!(NodeLabel::Function.as_str(), "Function");
    assert_eq!(NodeLabel::File.as_str(), "File");
    assert_eq!(NodeLabel::Struct.as_str(), "Struct");

    // Test RelationType conversions
    assert_eq!(RelationType::Calls.as_str(), "CALLS");
    assert_eq!(RelationType::Declares.as_str(), "DECLARES");
    assert_eq!(RelationType::DependsOn.as_str(), "DEPENDS_ON");
}

#[test]
fn test_node_properties_creation() {
    // Test minimal properties
    let minimal = NodeProperties::minimal(1, "test".to_string());
    assert_eq!(minimal.id, 1);
    assert_eq!(minimal.name, "test");
    assert!(minimal.path.is_none());
    assert!(minimal.language.is_none());

    // Test full properties
    let full = NodeProperties::full(
        2,
        "test_fn".to_string(),
        "src/test.rs".to_string(),
        10,
        20,
        "rust".to_string(),
    );
    assert_eq!(full.id, 2);
    assert_eq!(full.name, "test_fn");
    assert_eq!(full.path, Some("src/test.rs".to_string()));
    assert_eq!(full.start_line, Some(10));
    assert_eq!(full.end_line, Some(20));
    assert_eq!(full.language, Some("rust".to_string()));
}
