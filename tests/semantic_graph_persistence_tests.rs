//! PHASE 2 TDD Tests: Semantic Graph Persistence
//!
//! These tests verify that semantic edges extracted in Phase 1 are correctly
//! persisted to BOTH SQLite (code_edges table) and Neo4j (relationships).
//!
//! Test Strategy:
//! 1. Create test code with specific semantic patterns
//! 2. Extract edges using SemanticExtractor
//! 3. Persist edges using CodeGraph.upsert_*_edge() methods
//! 4. Verify persistence in BOTH SQLite and Neo4j
//! 5. Verify idempotency (can run multiple times)

use anyhow::Result;
use std::sync::{Arc, Mutex};
use syncore::code_graph::CodeGraph;
use syncore::code_graph::{CodeEntity, EdgeType, EntityType};
use syncore::graph::Neo4jClient;
use syncore::vector::{HuggingFaceEmbeddings, VectorStore};

/// Helper: Create temporary test database
async fn setup_test_graph() -> Result<CodeGraph> {
    let db_path = format!("/tmp/syncore_semantic_test_{}.db", std::process::id());
    let _ = std::fs::remove_file(&db_path); // Clean up from previous runs

    // Get Neo4j connection from environment
    let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string());
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let pass = std::env::var("NEO4J_PASS").unwrap_or_else(|_| "testpassword123".to_string());

    let neo4j = Neo4jClient::connect(&uri, &user, &pass).await?;

    // Clean Neo4j test namespace
    neo4j
        .execute_query(
            "MATCH (n {namespace: $ns}) DETACH DELETE n",
            vec![("ns", serde_json::json!(neo4j.namespace()))],
        )
        .await?;

    // Create vector store
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    // Create CodeGraph with Neo4j support
    CodeGraph::new_with_neo4j(&db_path, vector_store, Arc::new(neo4j))
}

/// Helper: Insert test entity and return its ID
async fn insert_test_entity(
    graph: &CodeGraph,
    name: &str,
    entity_type: EntityType,
    line_start: usize,
) -> Result<i64> {
    let entity = CodeEntity::new(
        "test.rs".to_string(),
        entity_type,
        name.to_string(),
        None,
        line_start,
        line_start + 10,
        None,
        "rust".to_string(),
    );

    // Directly insert into SQLite and get ID
    let db_conn = graph.db_conn();
    let conn = db_conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64;

    conn.execute(
        "INSERT INTO code_entities (file_path, entity_type, name, signature, line_start, line_end, docstring, language, indexed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            entity.file_path,
            entity.entity_type.as_str(),
            entity.name,
            entity.signature,
            entity.line_start as i64,
            entity.line_end as i64,
            entity.docstring,
            entity.language,
            now,
        ],
    )?;

    let entity_id = conn.last_insert_rowid();
    drop(conn); // Release lock before Neo4j operation

    // Also create node in Neo4j if available
    if let Ok(neo4j) = graph.neo4j_client() {
        use syncore::code_graph::neo4j_writer::create_code_entity_node;
        create_code_entity_node(neo4j, entity_id, &entity).await?;
    }

    Ok(entity_id)
}

/// Helper: Check if edge exists in SQLite code_edges table
async fn check_sqlite_edge(
    graph: &CodeGraph,
    src_id: i64,
    dst_id: i64,
    edge_type: &str,
) -> Result<bool> {
    let db_conn = graph.db_conn();
    let conn = db_conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM code_edges WHERE src_entity_id = ?1 AND dst_entity_id = ?2 AND edge_type = ?3",
        rusqlite::params![src_id, dst_id, edge_type],
        |row| row.get(0),
    )?;

    Ok(count > 0)
}

/// Helper: Check if relationship exists in Neo4j
async fn check_neo4j_relationship(
    graph: &CodeGraph,
    src_id: i64,
    dst_id: i64,
    rel_type: &str,
) -> Result<bool> {
    let neo4j = graph.neo4j_client()?;
    let cypher = format!(
        "MATCH (a {{id: $src_id, namespace: $ns}})-[:{}]->(b {{id: $dst_id, namespace: $ns}}) RETURN count(*) as cnt",
        rel_type
    );

    let result = neo4j
        .execute_query(
            &cypher,
            vec![
                ("src_id", serde_json::json!(src_id)),
                ("dst_id", serde_json::json!(dst_id)),
                ("ns", serde_json::json!(neo4j.namespace())),
            ],
        )
        .await?;

    // Parse count from result
    if let Some(record) = result.get(0) {
        if let Some(cnt) = record.get("cnt") {
            if let Some(count_val) = cnt.as_i64() {
                return Ok(count_val > 0);
            }
        }
    }

    Ok(false)
}

#[tokio::test]
async fn test_calls_edges_persist_to_sqlite_and_neo4j() -> Result<()> {
    let graph = setup_test_graph().await?;

    // Insert two function entities: caller and callee
    let caller_id = insert_test_entity(&graph, "caller", EntityType::Function, 10).await?;
    let callee_id = insert_test_entity(&graph, "callee", EntityType::Function, 20).await?;

    // Persist CALLS edge using upsert_call_edge
    graph.upsert_call_edge(caller_id, callee_id).await?;

    // Verify edge exists in SQLite
    assert!(
        check_sqlite_edge(&graph, caller_id, callee_id, "calls").await?,
        "CALLS edge not found in SQLite code_edges table"
    );

    // Verify relationship exists in Neo4j
    assert!(
        check_neo4j_relationship(&graph, caller_id, callee_id, "CALLS").await?,
        "CALLS relationship not found in Neo4j"
    );

    // Test idempotency: calling again should not error
    graph.upsert_call_edge(caller_id, callee_id).await?;

    Ok(())
}

#[tokio::test]
async fn test_implements_edges_persist_to_sqlite_and_neo4j() -> Result<()> {
    let graph = setup_test_graph().await?;

    // Insert type entity and trait entity
    let type_id = insert_test_entity(&graph, "MyStruct", EntityType::Struct, 10).await?;
    let trait_id = insert_test_entity(&graph, "MyTrait", EntityType::Trait, 20).await?;

    // Persist IMPLEMENTS edge
    graph.upsert_implements_edge(type_id, trait_id).await?;

    // Verify in SQLite
    assert!(
        check_sqlite_edge(&graph, type_id, trait_id, "implements").await?,
        "IMPLEMENTS edge not found in SQLite"
    );

    // Verify in Neo4j
    assert!(
        check_neo4j_relationship(&graph, type_id, trait_id, "IMPLEMENTS").await?,
        "IMPLEMENTS relationship not found in Neo4j"
    );

    // Test idempotency
    graph.upsert_implements_edge(type_id, trait_id).await?;

    Ok(())
}

#[tokio::test]
async fn test_field_edges_persist_to_graph_stores() -> Result<()> {
    let graph = setup_test_graph().await?;

    // Insert struct entity and field access context (could be a function)
    let struct_id = insert_test_entity(&graph, "MyStruct", EntityType::Struct, 10).await?;
    let accessor_id = insert_test_entity(&graph, "use_field", EntityType::Function, 20).await?;

    // Persist USES_FIELD edge
    graph.upsert_field_edge(accessor_id, struct_id).await?;

    // Verify in SQLite
    assert!(
        check_sqlite_edge(&graph, accessor_id, struct_id, "uses_field").await?,
        "USES_FIELD edge not found in SQLite"
    );

    // Verify in Neo4j
    assert!(
        check_neo4j_relationship(&graph, accessor_id, struct_id, "USES_FIELD").await?,
        "USES_FIELD relationship not found in Neo4j"
    );

    // Test idempotency
    graph.upsert_field_edge(accessor_id, struct_id).await?;

    Ok(())
}

#[tokio::test]
async fn test_type_usage_edges_persist_to_graph_stores() -> Result<()> {
    let graph = setup_test_graph().await?;

    // Insert function entity and type entity
    let function_id = insert_test_entity(&graph, "process_data", EntityType::Function, 10).await?;
    let type_id = insert_test_entity(&graph, "Vec<String>", EntityType::Struct, 20).await?;

    // Persist USES_TYPE edge
    graph.upsert_type_usage_edge(function_id, type_id).await?;

    // Verify in SQLite
    assert!(
        check_sqlite_edge(&graph, function_id, type_id, "uses_type").await?,
        "USES_TYPE edge not found in SQLite"
    );

    // Verify in Neo4j
    assert!(
        check_neo4j_relationship(&graph, function_id, type_id, "USES_TYPE").await?,
        "USES_TYPE relationship not found in Neo4j"
    );

    // Test idempotency
    graph.upsert_type_usage_edge(function_id, type_id).await?;

    Ok(())
}

#[tokio::test]
async fn test_module_child_edges_persist_correctly() -> Result<()> {
    let graph = setup_test_graph().await?;

    // Insert parent module and child module
    let parent_id = insert_test_entity(&graph, "parent_mod", EntityType::Class, 10).await?;
    let child_id = insert_test_entity(&graph, "child_mod", EntityType::Class, 20).await?;

    // Persist MODULE_CHILD edge
    graph.upsert_module_child_edge(parent_id, child_id).await?;

    // Verify in SQLite
    assert!(
        check_sqlite_edge(&graph, parent_id, child_id, "module_child").await?,
        "MODULE_CHILD edge not found in SQLite"
    );

    // Verify in Neo4j
    assert!(
        check_neo4j_relationship(&graph, parent_id, child_id, "MODULE_CHILD").await?,
        "MODULE_CHILD relationship not found in Neo4j"
    );

    // Test idempotency
    graph.upsert_module_child_edge(parent_id, child_id).await?;

    Ok(())
}
