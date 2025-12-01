//! Tests for Neo4j CodeEntity node creation
//!
//! These tests verify that CodeEntity nodes are correctly created in Neo4j
//! with proper labels, properties, and idempotency guarantees.
//!
//! REQUIREMENT: Real Neo4j instance must be running (no mocks allowed)

use anyhow::Result;
use syncore::code_graph::neo4j_writer::create_code_entity_node;
use syncore::code_graph::{CodeEntity, EntityType};
use syncore::graph::Neo4jClient;

/// Helper to get Neo4j connection (same pattern as other tests)
async fn get_neo4j_client() -> Result<Neo4jClient> {
    let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string());
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let pass = std::env::var("NEO4J_PASS").unwrap_or_else(|_| "testpassword123".to_string());

    Neo4jClient::connect(&uri, &user, &pass).await
}

/// Helper to clear all CodeEntity-related nodes in test namespace
async fn clear_code_entities(neo4j: &Neo4jClient) -> Result<()> {
    let labels = vec!["Function", "Class", "Method", "Import", "Struct", "Enum", "Trait"];

    for label in labels {
        let cypher = format!("MATCH (n:{} {{namespace: $ns}}) DETACH DELETE n", label);
        neo4j.execute_query(&cypher, vec![("ns", serde_json::json!(neo4j.namespace()))]).await?;
    }

    Ok(())
}

#[tokio::test]
async fn test_create_function_node() -> Result<()> {
    let neo4j = get_neo4j_client().await?;
    clear_code_entities(&neo4j).await?;

    // Create a sample function entity
    let entity = CodeEntity {
        id: Some(1),
        file_path: "/tmp/test.rs".to_string(),
        entity_type: EntityType::Function,
        name: "my_function".to_string(),
        signature: Some("my_function(a: i32, b: i32)".to_string()),
        line_start: 10,
        line_end: 15,
        docstring: Some("/// Test function".to_string()),
        language: "rust".to_string(),
        body_snippet: None,
        created_at: None,
        last_modified_at: None,
        change_count: None,
        author_count: None,
    };

    // Create node
    create_code_entity_node(&neo4j, 1, &entity).await?;

    // Verify node exists with correct properties
    let result = neo4j.execute_query(
        "MATCH (f:Function {id: $id, namespace: $ns}) RETURN f.name as name, f.file_path as file_path, f.signature as signature, f.line_start as line_start, f.line_end as line_end, f.language as language",
        vec![
            ("id", serde_json::json!(1)),
            ("ns", serde_json::json!(neo4j.namespace())),
        ],
    ).await?;

    assert_eq!(result.len(), 1, "Expected exactly one Function node");
    let node = &result[0];
    assert_eq!(node["name"].as_str(), Some("my_function"));
    assert_eq!(node["file_path"].as_str(), Some("/tmp/test.rs"));
    assert_eq!(node["signature"].as_str(), Some("my_function(a: i32, b: i32)"));
    assert_eq!(node["line_start"].as_i64(), Some(10));
    assert_eq!(node["line_end"].as_i64(), Some(15));
    assert_eq!(node["language"].as_str(), Some("rust"));

    Ok(())
}

#[tokio::test]
async fn test_create_class_node() -> Result<()> {
    let neo4j = get_neo4j_client().await?;
    clear_code_entities(&neo4j).await?;

    let entity = CodeEntity {
        id: Some(2),
        file_path: "/tmp/test.rs".to_string(),
        entity_type: EntityType::Class,
        name: "MyClass".to_string(),
        signature: None,
        line_start: 20,
        line_end: 50,
        docstring: None,
        language: "rust".to_string(),
        body_snippet: None,
        created_at: None,
        last_modified_at: None,
        change_count: None,
        author_count: None,
    };

    create_code_entity_node(&neo4j, 2, &entity).await?;

    // Verify Class node exists
    let result = neo4j
        .execute_query(
            "MATCH (c:Class {id: $id, namespace: $ns}) RETURN c.name as name",
            vec![("id", serde_json::json!(2)), ("ns", serde_json::json!(neo4j.namespace()))],
        )
        .await?;

    assert_eq!(result.len(), 1);
    assert_eq!(result[0]["name"].as_str(), Some("MyClass"));

    Ok(())
}

#[tokio::test]
async fn test_create_method_node() -> Result<()> {
    let neo4j = get_neo4j_client().await?;
    clear_code_entities(&neo4j).await?;

    let entity = CodeEntity {
        id: Some(3),
        file_path: "/tmp/test.rs".to_string(),
        entity_type: EntityType::Method,
        name: "MyClass.my_method".to_string(),
        signature: Some("my_method(self, x: i32)".to_string()),
        line_start: 30,
        line_end: 35,
        docstring: None,
        language: "rust".to_string(),
        body_snippet: None,
        created_at: None,
        last_modified_at: None,
        change_count: None,
        author_count: None,
    };

    create_code_entity_node(&neo4j, 3, &entity).await?;

    // Verify Method node exists
    let result = neo4j.execute_query(
        "MATCH (m:Method {id: $id, namespace: $ns}) RETURN m.name as name, m.signature as signature",
        vec![
            ("id", serde_json::json!(3)),
            ("ns", serde_json::json!(neo4j.namespace())),
        ],
    ).await?;

    assert_eq!(result.len(), 1);
    assert_eq!(result[0]["name"].as_str(), Some("MyClass.my_method"));
    assert_eq!(result[0]["signature"].as_str(), Some("my_method(self, x: i32)"));

    Ok(())
}

#[tokio::test]
async fn test_idempotency_merge() -> Result<()> {
    let neo4j = get_neo4j_client().await?;
    clear_code_entities(&neo4j).await?;

    let entity = CodeEntity {
        id: Some(4),
        file_path: "/tmp/test.rs".to_string(),
        entity_type: EntityType::Function,
        name: "test_function".to_string(),
        signature: Some("test_function()".to_string()),
        line_start: 100,
        line_end: 110,
        docstring: None,
        language: "rust".to_string(),
        body_snippet: None,
        created_at: None,
        last_modified_at: None,
        change_count: None,
        author_count: None,
    };

    // Create node first time
    create_code_entity_node(&neo4j, 4, &entity).await?;

    // Create same node second time (should update, not duplicate)
    let mut updated_entity = entity.clone();
    updated_entity.docstring = Some("/// Updated docstring".to_string());
    create_code_entity_node(&neo4j, 4, &updated_entity).await?;

    // Verify only ONE node exists
    let result = neo4j.execute_query(
        "MATCH (f:Function {id: $id, namespace: $ns}) RETURN f.name as name, f.docstring as docstring",
        vec![
            ("id", serde_json::json!(4)),
            ("ns", serde_json::json!(neo4j.namespace())),
        ],
    ).await?;

    assert_eq!(result.len(), 1, "Should have exactly one node (MERGE idempotency)");
    assert_eq!(result[0]["name"].as_str(), Some("test_function"));
    assert_eq!(result[0]["docstring"].as_str(), Some("/// Updated docstring"));

    Ok(())
}

#[tokio::test]
async fn test_all_entity_types() -> Result<()> {
    let neo4j = get_neo4j_client().await?;
    clear_code_entities(&neo4j).await?;

    // Create one node for each entity type
    let entity_types = vec![
        (EntityType::Function, "Function", "my_func"),
        (EntityType::Class, "Class", "MyClass"),
        (EntityType::Method, "Method", "MyClass.method"),
        (EntityType::Import, "Import", "std::collections"),
        (EntityType::Struct, "Struct", "MyStruct"),
        (EntityType::Enum, "Enum", "MyEnum"),
        (EntityType::Trait, "Trait", "MyTrait"),
    ];

    for (i, (entity_type, _label, name)) in entity_types.iter().enumerate() {
        let entity = CodeEntity {
            id: Some(i as i64 + 10),
            file_path: "/tmp/test.rs".to_string(),
            entity_type: entity_type.clone(),
            name: name.to_string(),
            signature: None,
            line_start: i * 10,
            line_end: i * 10 + 5,
            docstring: None,
            language: "rust".to_string(),
            body_snippet: None,
            created_at: None,
            last_modified_at: None,
            change_count: None,
            author_count: None,
        };

        create_code_entity_node(&neo4j, i as i64 + 10, &entity).await?;
    }

    // Verify all nodes exist with correct labels
    for (i, (_, label, name)) in entity_types.iter().enumerate() {
        let cypher =
            format!("MATCH (n:{} {{id: $id, namespace: $ns}}) RETURN n.name as name", label);
        let result = neo4j
            .execute_query(
                &cypher,
                vec![
                    ("id", serde_json::json!(i as i64 + 10)),
                    ("ns", serde_json::json!(neo4j.namespace())),
                ],
            )
            .await?;

        assert_eq!(result.len(), 1, "Node for {} not found", label);
        assert_eq!(result[0]["name"].as_str(), Some(*name));
    }

    Ok(())
}

#[tokio::test]
async fn test_namespace_isolation() -> Result<()> {
    let neo4j = get_neo4j_client().await?;
    clear_code_entities(&neo4j).await?;

    let entity = CodeEntity {
        id: Some(100),
        file_path: "/tmp/test.rs".to_string(),
        entity_type: EntityType::Function,
        name: "isolated_function".to_string(),
        signature: None,
        line_start: 1,
        line_end: 10,
        docstring: None,
        language: "rust".to_string(),
        body_snippet: None,
        created_at: None,
        last_modified_at: None,
        change_count: None,
        author_count: None,
    };

    create_code_entity_node(&neo4j, 100, &entity).await?;

    // Query with correct namespace - should find node
    let result1 = neo4j
        .execute_query(
            "MATCH (f:Function {id: $id, namespace: $ns}) RETURN f.name as name",
            vec![("id", serde_json::json!(100)), ("ns", serde_json::json!(neo4j.namespace()))],
        )
        .await?;
    assert_eq!(result1.len(), 1);

    // Query with WRONG namespace - should find nothing
    let result2 = neo4j
        .execute_query(
            "MATCH (f:Function {id: $id, namespace: $ns}) RETURN f.name as name",
            vec![("id", serde_json::json!(100)), ("ns", serde_json::json!("wrong_namespace"))],
        )
        .await?;
    assert_eq!(result2.len(), 0, "Should not find node in different namespace");

    Ok(())
}
