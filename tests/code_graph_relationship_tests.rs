//! Tests for Neo4j relationship creation in CodeGraph
//!
//! These tests verify that CodeEdge relationships are correctly synced to Neo4j
//! with proper relationship types, properties, and idempotency guarantees.
//!
//! REQUIREMENT: Real Neo4j instance must be running (no mocks allowed)

use anyhow::Result;
use std::io::Write;
use std::sync::{Arc, Mutex};
use syncore::code_graph::{CodeEdge, CodeEntity, CodeGraph, EdgeType, EntityType};
use syncore::graph::Neo4jClient;
use syncore::vector::{HuggingFaceEmbeddings, VectorStore};
use tempfile::Builder;

/// Helper to get Neo4j connection
async fn get_neo4j_client() -> Result<Neo4jClient> {
    let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string());
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let pass = std::env::var("NEO4J_PASS").unwrap_or_else(|_| "testpassword123".to_string());

    Neo4jClient::connect(&uri, &user, &pass).await
}

/// Helper to clear all CodeEntity and relationship data
async fn clear_code_graph(neo4j: &Neo4jClient) -> Result<()> {
    // First, delete all nodes and their relationships in this namespace
    neo4j
        .execute_query(
            "MATCH (n {namespace: $ns}) DETACH DELETE n",
            vec![("ns", serde_json::json!(neo4j.namespace()))],
        )
        .await?;

    Ok(())
}

#[tokio::test]
async fn test_create_calls_relationship_neo4j() -> Result<()> {
    let neo4j = get_neo4j_client().await?;
    clear_code_graph(&neo4j).await?;

    // Create two function entities
    let func_a = CodeEntity {
        id: Some(1),
        file_path: "/tmp/test.rs".to_string(),
        entity_type: EntityType::Function,
        name: "function_a".to_string(),
        signature: Some("function_a()".to_string()),
        line_start: 1,
        line_end: 5,
        docstring: None,
        language: "rust".to_string(),
        created_at: None,
        last_modified_at: None,
        change_count: None,
        author_count: None,
    };

    let func_b = CodeEntity {
        id: Some(2),
        file_path: "/tmp/test.rs".to_string(),
        entity_type: EntityType::Function,
        name: "function_b".to_string(),
        signature: Some("function_b()".to_string()),
        line_start: 7,
        line_end: 10,
        docstring: None,
        language: "rust".to_string(),
        created_at: None,
        last_modified_at: None,
        change_count: None,
        author_count: None,
    };

    // Create entities in Neo4j
    syncore::code_graph::neo4j_writer::create_code_entity_node(&neo4j, 1, &func_a).await?;
    syncore::code_graph::neo4j_writer::create_code_entity_node(&neo4j, 2, &func_b).await?;

    // Create CALLS relationship
    let edge = CodeEdge {
        src_entity_id: 1,
        dst_entity_id: 2,
        edge_type: EdgeType::Calls,
    };

    syncore::code_graph::neo4j_relationships::create_code_relationship(&neo4j, &edge).await?;

    // Verify relationship exists in Neo4j
    let result = neo4j.execute_query(
        "MATCH (a:Function {id: $src, namespace: $ns})-[r:CALLS]->(b:Function {id: $dst, namespace: $ns}) RETURN count(r) as count",
        vec![
            ("src", serde_json::json!(1)),
            ("dst", serde_json::json!(2)),
            ("ns", serde_json::json!(neo4j.namespace())),
        ],
    ).await?;

    assert_eq!(
        result[0]["count"].as_i64(),
        Some(1),
        "CALLS relationship should exist"
    );

    Ok(())
}

#[tokio::test]
async fn test_imports_relationship_neo4j() -> Result<()> {
    let neo4j = get_neo4j_client().await?;
    clear_code_graph(&neo4j).await?;

    // Create import entity
    let import_entity = CodeEntity {
        id: Some(10),
        file_path: "/tmp/main.rs".to_string(),
        entity_type: EntityType::Import,
        name: "std::collections::HashMap".to_string(),
        signature: None,
        line_start: 1,
        line_end: 1,
        docstring: None,
        language: "rust".to_string(),
        created_at: None,
        last_modified_at: None,
        change_count: None,
        author_count: None,
    };

    // Create function that uses the import
    let func = CodeEntity {
        id: Some(11),
        file_path: "/tmp/main.rs".to_string(),
        entity_type: EntityType::Function,
        name: "my_func".to_string(),
        signature: Some("my_func()".to_string()),
        line_start: 3,
        line_end: 10,
        docstring: None,
        language: "rust".to_string(),
        created_at: None,
        last_modified_at: None,
        change_count: None,
        author_count: None,
    };

    syncore::code_graph::neo4j_writer::create_code_entity_node(&neo4j, 10, &import_entity).await?;
    syncore::code_graph::neo4j_writer::create_code_entity_node(&neo4j, 11, &func).await?;

    // Create IMPORTS relationship
    let edge = CodeEdge {
        src_entity_id: 11,
        dst_entity_id: 10,
        edge_type: EdgeType::Imports,
    };

    syncore::code_graph::neo4j_relationships::create_code_relationship(&neo4j, &edge).await?;

    // Verify relationship
    let result = neo4j.execute_query(
        "MATCH (f:Function {id: $src, namespace: $ns})-[r:IMPORTS]->(i:Import {id: $dst, namespace: $ns}) RETURN count(r) as count",
        vec![
            ("src", serde_json::json!(11)),
            ("dst", serde_json::json!(10)),
            ("ns", serde_json::json!(neo4j.namespace())),
        ],
    ).await?;

    assert_eq!(result[0]["count"].as_i64(), Some(1));

    Ok(())
}

#[tokio::test]
async fn test_relationship_idempotency() -> Result<()> {
    let neo4j = get_neo4j_client().await?;
    clear_code_graph(&neo4j).await?;

    let func_a = CodeEntity {
        id: Some(20),
        file_path: "/tmp/test.rs".to_string(),
        entity_type: EntityType::Function,
        name: "func_a".to_string(),
        signature: Some("func_a()".to_string()),
        line_start: 1,
        line_end: 5,
        docstring: None,
        language: "rust".to_string(),
        created_at: None,
        last_modified_at: None,
        change_count: None,
        author_count: None,
    };

    let func_b = CodeEntity {
        id: Some(21),
        file_path: "/tmp/test.rs".to_string(),
        entity_type: EntityType::Function,
        name: "func_b".to_string(),
        signature: Some("func_b()".to_string()),
        line_start: 7,
        line_end: 10,
        docstring: None,
        language: "rust".to_string(),
        created_at: None,
        last_modified_at: None,
        change_count: None,
        author_count: None,
    };

    syncore::code_graph::neo4j_writer::create_code_entity_node(&neo4j, 20, &func_a).await?;
    syncore::code_graph::neo4j_writer::create_code_entity_node(&neo4j, 21, &func_b).await?;

    let edge = CodeEdge {
        src_entity_id: 20,
        dst_entity_id: 21,
        edge_type: EdgeType::Calls,
    };

    // Create relationship twice
    syncore::code_graph::neo4j_relationships::create_code_relationship(&neo4j, &edge).await?;
    syncore::code_graph::neo4j_relationships::create_code_relationship(&neo4j, &edge).await?;

    // Should only have one relationship (MERGE idempotency)
    let result = neo4j.execute_query(
        "MATCH (a {id: $src, namespace: $ns})-[r:CALLS]->(b {id: $dst, namespace: $ns}) RETURN count(r) as count",
        vec![
            ("src", serde_json::json!(20)),
            ("dst", serde_json::json!(21)),
            ("ns", serde_json::json!(neo4j.namespace())),
        ],
    ).await?;

    assert_eq!(
        result[0]["count"].as_i64(),
        Some(1),
        "Should have exactly one relationship (MERGE)"
    );

    Ok(())
}

#[tokio::test]
async fn test_relationship_namespace_isolation() -> Result<()> {
    let neo4j_alpha = get_neo4j_client().await?;

    // Clear alpha namespace
    clear_code_graph(&neo4j_alpha).await?;

    let func_a = CodeEntity {
        id: Some(30),
        file_path: "/tmp/test.rs".to_string(),
        entity_type: EntityType::Function,
        name: "func_alpha".to_string(),
        signature: Some("func_alpha()".to_string()),
        line_start: 1,
        line_end: 5,
        docstring: None,
        language: "rust".to_string(),
        created_at: None,
        last_modified_at: None,
        change_count: None,
        author_count: None,
    };

    let func_b = CodeEntity {
        id: Some(31),
        file_path: "/tmp/test.rs".to_string(),
        entity_type: EntityType::Function,
        name: "func_beta".to_string(),
        signature: Some("func_beta()".to_string()),
        line_start: 7,
        line_end: 10,
        docstring: None,
        language: "rust".to_string(),
        created_at: None,
        last_modified_at: None,
        change_count: None,
        author_count: None,
    };

    syncore::code_graph::neo4j_writer::create_code_entity_node(&neo4j_alpha, 30, &func_a).await?;
    syncore::code_graph::neo4j_writer::create_code_entity_node(&neo4j_alpha, 31, &func_b).await?;

    let edge = CodeEdge {
        src_entity_id: 30,
        dst_entity_id: 31,
        edge_type: EdgeType::Calls,
    };

    syncore::code_graph::neo4j_relationships::create_code_relationship(&neo4j_alpha, &edge).await?;

    // Query with correct namespace
    let result1 = neo4j_alpha.execute_query(
        "MATCH (a {id: $src, namespace: $ns})-[r:CALLS]->(b {id: $dst, namespace: $ns}) RETURN count(r) as count",
        vec![
            ("src", serde_json::json!(30)),
            ("dst", serde_json::json!(31)),
            ("ns", serde_json::json!(neo4j_alpha.namespace())),
        ],
    ).await?;

    assert_eq!(result1[0]["count"].as_i64(), Some(1));

    // Query with wrong namespace
    let result2 = neo4j_alpha.execute_query(
        "MATCH (a {id: $src, namespace: $ns})-[r:CALLS]->(b {id: $dst, namespace: $ns}) RETURN count(r) as count",
        vec![
            ("src", serde_json::json!(30)),
            ("dst", serde_json::json!(31)),
            ("ns", serde_json::json!("wrong_namespace")),
        ],
    ).await?;

    assert_eq!(
        result2[0]["count"].as_i64(),
        Some(0),
        "Should not find relationship in different namespace"
    );

    Ok(())
}

#[tokio::test]
async fn test_backwards_compatibility_no_neo4j() -> Result<()> {
    // Create CodeGraph without Neo4j
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let mut code_graph = CodeGraph::new(":memory:", vector_store)?;

    // Create temp file
    let mut temp_file = Builder::new().prefix("test_").suffix(".rs").tempfile()?;

    writeln!(temp_file, "fn test_func() {{}}")?;
    temp_file.flush()?;

    // Index without Neo4j - should not panic
    let result = code_graph.index_file(temp_file.path());

    assert!(result.is_ok(), "Indexing should work without Neo4j");
    assert!(result.unwrap() >= 1);

    Ok(())
}

#[tokio::test]
async fn test_all_relationship_types() -> Result<()> {
    let neo4j = get_neo4j_client().await?;
    clear_code_graph(&neo4j).await?;

    // Create entities for different relationship types
    let entities: Vec<(i64, CodeEntity)> = vec![
        (
            40,
            CodeEntity {
                id: Some(40),
                file_path: "/tmp/test.rs".to_string(),
                entity_type: EntityType::Function,
                name: "caller".to_string(),
                signature: Some("caller()".to_string()),
                line_start: 1,
                line_end: 5,
                docstring: None,
                language: "rust".to_string(),
                created_at: None,
                last_modified_at: None,
                change_count: None,
                author_count: None,
            },
        ),
        (
            41,
            CodeEntity {
                id: Some(41),
                file_path: "/tmp/test.rs".to_string(),
                entity_type: EntityType::Function,
                name: "callee".to_string(),
                signature: Some("callee()".to_string()),
                line_start: 7,
                line_end: 10,
                docstring: None,
                language: "rust".to_string(),
                created_at: None,
                last_modified_at: None,
                change_count: None,
                author_count: None,
            },
        ),
        (
            42,
            CodeEntity {
                id: Some(42),
                file_path: "/tmp/test.rs".to_string(),
                entity_type: EntityType::Class,
                name: "BaseClass".to_string(),
                signature: None,
                line_start: 12,
                line_end: 20,
                docstring: None,
                language: "rust".to_string(),
                created_at: None,
                last_modified_at: None,
                change_count: None,
                author_count: None,
            },
        ),
        (
            43,
            CodeEntity {
                id: Some(43),
                file_path: "/tmp/test.rs".to_string(),
                entity_type: EntityType::Class,
                name: "DerivedClass".to_string(),
                signature: None,
                line_start: 22,
                line_end: 30,
                docstring: None,
                language: "rust".to_string(),
                created_at: None,
                last_modified_at: None,
                change_count: None,
                author_count: None,
            },
        ),
    ];

    for (id, entity) in &entities {
        syncore::code_graph::neo4j_writer::create_code_entity_node(&neo4j, *id, entity).await?;
    }

    // Test all relationship types
    let relationship_types = vec![
        (EdgeType::Calls, "CALLS", 40, 41),
        (EdgeType::Inherits, "INHERITS", 43, 42),
        (EdgeType::Uses, "USES", 40, 42),
        (EdgeType::References, "REFERENCES", 41, 43),
    ];

    for (edge_type, cypher_type, src, dst) in relationship_types {
        let edge = CodeEdge {
            src_entity_id: src,
            dst_entity_id: dst,
            edge_type: edge_type.clone(),
        };

        syncore::code_graph::neo4j_relationships::create_code_relationship(&neo4j, &edge).await?;

        let cypher = format!(
            "MATCH (a {{id: $src, namespace: $ns}})-[r:{}]->(b {{id: $dst, namespace: $ns}}) RETURN count(r) as count",
            cypher_type
        );

        let result = neo4j
            .execute_query(
                &cypher,
                vec![
                    ("src", serde_json::json!(src)),
                    ("dst", serde_json::json!(dst)),
                    ("ns", serde_json::json!(neo4j.namespace())),
                ],
            )
            .await?;

        assert_eq!(
            result[0]["count"].as_i64(),
            Some(1),
            "Relationship type {} should exist",
            cypher_type
        );
    }

    Ok(())
}
