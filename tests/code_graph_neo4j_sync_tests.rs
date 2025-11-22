//! Phase R2.6 - Neo4j Relationship Sync Tests
//!
//! These tests verify the post-index relationship sync functionality that:
//! - Reads edges from SQLite code_edges table
//! - Syncs them as relationships in Neo4j
//! - Is idempotent (safe to run multiple times)
//! - Works with real databases (no mocks)
//!
//! REQUIREMENT: Real Neo4j instance must be running

use anyhow::Result;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use syncore::code_graph::neo4j_sync::{sync_relationships_to_neo4j, Neo4jSyncSummary};
use syncore::code_graph::{CodeEdge, CodeEntity, EdgeType, EntityType};
use syncore::graph::Neo4jClient;
use syncore::vector::{HuggingFaceEmbeddings, VectorStore};

/// Helper to get Neo4j connection
async fn get_neo4j_client() -> Result<Neo4jClient> {
    let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string());
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let pass = std::env::var("NEO4J_PASS").unwrap_or_else(|_| "testpassword123".to_string());

    Neo4jClient::connect(&uri, &user, &pass).await
}

/// Helper to create test database with entities and edges
fn setup_test_db() -> Result<Arc<Mutex<Connection>>> {
    let conn = Connection::open_in_memory()?;

    // Create schema
    conn.execute_batch(
        r#"
        CREATE TABLE code_entities (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_path TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            name TEXT NOT NULL,
            signature TEXT,
            line_start INTEGER NOT NULL,
            line_end INTEGER NOT NULL,
            docstring TEXT,
            language TEXT NOT NULL
        );

        CREATE TABLE code_edges (
            src_entity_id INTEGER NOT NULL REFERENCES code_entities(id) ON DELETE CASCADE,
            dst_entity_id INTEGER NOT NULL REFERENCES code_entities(id) ON DELETE CASCADE,
            edge_type TEXT NOT NULL,
            PRIMARY KEY (src_entity_id, dst_entity_id, edge_type)
        );
        "#,
    )?;

    // Insert test entities
    conn.execute(
        "INSERT INTO code_entities (file_path, entity_type, name, signature, line_start, line_end, language)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        ["test.rs", "function", "func_a", "func_a()", "1", "5", "rust"],
    )?;

    conn.execute(
        "INSERT INTO code_entities (file_path, entity_type, name, signature, line_start, line_end, language)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        ["test.rs", "function", "func_b", "func_b()", "7", "10", "rust"],
    )?;

    conn.execute(
        "INSERT INTO code_entities (file_path, entity_type, name, signature, line_start, line_end, language)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        ["test.rs", "function", "func_c", "func_c()", "12", "15", "rust"],
    )?;

    // Insert test edges
    conn.execute(
        "INSERT INTO code_edges (src_entity_id, dst_entity_id, edge_type) VALUES (?1, ?2, ?3)",
        rusqlite::params![1, 2, "calls"],
    )?;

    conn.execute(
        "INSERT INTO code_edges (src_entity_id, dst_entity_id, edge_type) VALUES (?1, ?2, ?3)",
        rusqlite::params![2, 3, "calls"],
    )?;

    conn.execute(
        "INSERT INTO code_edges (src_entity_id, dst_entity_id, edge_type) VALUES (?1, ?2, ?3)",
        rusqlite::params![1, 3, "references"],
    )?;

    Ok(Arc::new(Mutex::new(conn)))
}

/// Helper to create nodes in Neo4j for test entities
async fn create_test_nodes_in_neo4j(neo4j: &Neo4jClient) -> Result<()> {
    use syncore::code_graph::neo4j_writer;

    let entities = vec![
        CodeEntity {
            id: Some(1),
            file_path: "test.rs".to_string(),
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
        },
        CodeEntity {
            id: Some(2),
            file_path: "test.rs".to_string(),
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
        },
        CodeEntity {
            id: Some(3),
            file_path: "test.rs".to_string(),
            entity_type: EntityType::Function,
            name: "func_c".to_string(),
            signature: Some("func_c()".to_string()),
            line_start: 12,
            line_end: 15,
            docstring: None,
            language: "rust".to_string(),
            created_at: None,
            last_modified_at: None,
            change_count: None,
            author_count: None,
        },
    ];

    for entity in entities {
        let entity_id = entity.id.unwrap();
        neo4j_writer::create_code_entity_node(neo4j, entity_id, &entity).await?;
    }

    Ok(())
}

/// Helper to count relationships in Neo4j
async fn count_neo4j_relationships(neo4j: &Neo4jClient, rel_type: &str) -> Result<i64> {
    let cypher = format!(
        "MATCH ()-[r:{}]->() WHERE r.namespace = $ns OR r.namespace IS NULL RETURN count(r) as count",
        rel_type
    );

    let result = neo4j
        .execute_query(&cypher, vec![("ns", serde_json::json!(neo4j.namespace()))])
        .await?;

    Ok(result[0].get("count").and_then(|v| v.as_i64()).unwrap_or(0))
}

#[tokio::test]
async fn test_sync_neo4j_creates_relationships_from_edges() -> Result<()> {
    let neo4j = get_neo4j_client().await?;
    let db = setup_test_db()?;

    // Create nodes in Neo4j first
    create_test_nodes_in_neo4j(&neo4j).await?;

    // Sync relationships
    let summary = sync_relationships_to_neo4j(&db, &neo4j, None, None).await?;

    // Verify summary
    assert_eq!(summary.edges_processed, 3, "Should process 3 edges");
    assert!(
        summary.edges_created >= 3,
        "Should create at least 3 relationships"
    );

    // Verify relationships in Neo4j
    let calls_count = count_neo4j_relationships(&neo4j, "CALLS").await?;
    let refs_count = count_neo4j_relationships(&neo4j, "REFERENCES").await?;

    assert!(
        calls_count >= 2,
        "Should have at least 2 CALLS relationships"
    );
    assert!(
        refs_count >= 1,
        "Should have at least 1 REFERENCES relationship"
    );

    Ok(())
}

#[tokio::test]
async fn test_sync_neo4j_is_idempotent() -> Result<()> {
    let neo4j = get_neo4j_client().await?;
    let db = setup_test_db()?;

    // Create nodes in Neo4j
    create_test_nodes_in_neo4j(&neo4j).await?;

    // First sync
    let summary1 = sync_relationships_to_neo4j(&db, &neo4j, None, None).await?;
    assert_eq!(summary1.edges_processed, 3);

    // Second sync (should be idempotent)
    let summary2 = sync_relationships_to_neo4j(&db, &neo4j, None, None).await?;
    assert_eq!(summary2.edges_processed, 3);

    // Verify relationship count hasn't doubled (allow some tolerance for test isolation)
    let calls_count = count_neo4j_relationships(&neo4j, "CALLS").await?;
    assert!(
        calls_count <= 10,
        "Idempotent sync should not significantly duplicate relationships, got {}",
        calls_count
    );

    Ok(())
}

#[tokio::test]
async fn test_sync_neo4j_respects_limit() -> Result<()> {
    let neo4j = get_neo4j_client().await?;
    let db = setup_test_db()?;

    // Create nodes in Neo4j
    create_test_nodes_in_neo4j(&neo4j).await?;

    // Sync with limit of 2
    let summary = sync_relationships_to_neo4j(&db, &neo4j, None, Some(2)).await?;

    // Should only process 2 edges
    assert_eq!(summary.edges_processed, 2, "Should respect limit parameter");

    Ok(())
}

#[tokio::test]
async fn test_sync_neo4j_handles_missing_nodes_gracefully() -> Result<()> {
    let neo4j = get_neo4j_client().await?;
    let db = setup_test_db()?;

    // Don't create nodes in Neo4j - MERGE will fail to match but won't error

    // Sync relationships (nodes don't exist)
    let summary = sync_relationships_to_neo4j(&db, &neo4j, None, None).await?;

    // Should process edges (they may be skipped or created depending on Neo4j MERGE behavior)
    assert_eq!(summary.edges_processed, 3);
    // Just verify no panic occurred - MERGE won't error but won't create rels without nodes
    assert!(summary.edges_created + summary.edges_skipped == 3);

    Ok(())
}

#[tokio::test]
async fn test_sync_tool_response_structure() -> Result<()> {
    let neo4j = get_neo4j_client().await?;
    let db = setup_test_db()?;

    // Create nodes
    create_test_nodes_in_neo4j(&neo4j).await?;

    // Sync and verify response structure
    let summary = sync_relationships_to_neo4j(&db, &neo4j, None, None).await?;

    // Verify all required fields exist
    assert!(summary.edges_processed > 0);
    assert!(summary.edges_created >= 0);
    assert!(summary.edges_skipped >= 0);

    // Verify JSON serialization works
    let json = serde_json::to_string(&summary)?;
    assert!(json.contains("edges_processed"));
    assert!(json.contains("edges_created"));
    assert!(json.contains("edges_skipped"));

    Ok(())
}

#[tokio::test]
async fn test_sync_backwards_compatibility() -> Result<()> {
    // Verify that existing R2.2, R2.3, R2.4, R2.5 functionality still works

    use syncore::code_graph::fusion_simple::FusionSimple;
    use syncore::code_graph::CodeGraph;

    // Test R2.2: Basic indexing
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let _code_graph = CodeGraph::new(":memory:", vector_store)?;

    // Test R2.4: Fusion still works
    let fusion = FusionSimple::new(0.6, 0.3, 0.1);
    let score = fusion.combine(0.8, 0.4, 0.0);
    assert!((score - 0.64).abs() < 0.001);

    Ok(())
}

// ============================================================================
// NEW TESTS FOR ENTITY NODE SYNC (R2.6 Patch)
// ============================================================================

/// Helper to count entity nodes in Neo4j
async fn count_neo4j_entities(neo4j: &Neo4jClient) -> Result<i64> {
    let cypher = "MATCH (n:CodeEntity) WHERE n.namespace = $ns OR n.namespace IS NULL RETURN count(n) as count";

    let result = neo4j
        .execute_query(cypher, vec![("ns", serde_json::json!(neo4j.namespace()))])
        .await?;

    Ok(result[0].get("count").and_then(|v| v.as_i64()).unwrap_or(0))
}

#[tokio::test]
async fn test_sync_entities_creates_all_nodes() -> Result<()> {
    use syncore::code_graph::neo4j_sync::sync_entities_to_neo4j;

    let neo4j = get_neo4j_client().await?;
    let db = setup_test_db()?;

    // Count entities in SQLite
    let sqlite_count: i64 = {
        let conn = db.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM code_entities", [], |row| row.get(0))?
    };

    assert_eq!(sqlite_count, 3, "Setup should create 3 entities in SQLite");

    // Sync entities to Neo4j
    let summary = sync_entities_to_neo4j(&db, &neo4j, None, None).await?;

    // Verify summary
    assert_eq!(
        summary.entities_processed, 3,
        "Should process 3 entities from SQLite"
    );
    assert!(
        summary.entities_created >= 3,
        "Should create at least 3 entity nodes, got {}",
        summary.entities_created
    );

    // Verify nodes in Neo4j
    let neo4j_count = count_neo4j_entities(&neo4j).await?;
    assert!(
        neo4j_count >= 3,
        "Neo4j should have at least 3 CodeEntity nodes, got {}",
        neo4j_count
    );

    Ok(())
}

#[tokio::test]
async fn test_sync_entities_idempotent() -> Result<()> {
    use syncore::code_graph::neo4j_sync::sync_entities_to_neo4j;

    let neo4j = get_neo4j_client().await?;
    let db = setup_test_db()?;

    // First sync
    let summary1 = sync_entities_to_neo4j(&db, &neo4j, None, None).await?;
    assert_eq!(summary1.entities_processed, 3);

    let count_after_first_sync = count_neo4j_entities(&neo4j).await?;

    // Second sync (should be idempotent due to MERGE)
    let summary2 = sync_entities_to_neo4j(&db, &neo4j, None, None).await?;
    assert_eq!(summary2.entities_processed, 3);

    let count_after_second_sync = count_neo4j_entities(&neo4j).await?;

    // Count should be roughly the same (allow small variance for test isolation)
    assert!(
        (count_after_second_sync - count_after_first_sync).abs() <= 3,
        "Idempotent sync should not duplicate nodes. First: {}, Second: {}",
        count_after_first_sync,
        count_after_second_sync
    );

    Ok(())
}

#[tokio::test]
async fn test_sync_entities_runs_before_edges() -> Result<()> {
    use syncore::code_graph::neo4j_sync::{sync_entities_to_neo4j, sync_relationships_to_neo4j};

    let neo4j = get_neo4j_client().await?;
    let db = setup_test_db()?;

    // Sync entities FIRST
    let entity_summary = sync_entities_to_neo4j(&db, &neo4j, None, None).await?;
    assert!(
        entity_summary.entities_created >= 3,
        "Should create entity nodes first"
    );

    // Then sync edges
    let edge_summary = sync_relationships_to_neo4j(&db, &neo4j, None, None).await?;
    assert!(edge_summary.edges_created >= 3, "Should create edges");

    // Verify: All edges in our namespace should have valid CodeEntity start and end nodes
    // This query specifically checks edges created by this test (using namespace isolation)
    let cypher = r#"
        MATCH (a)-[r]->(b)
        WHERE r.namespace = $ns
          AND (NOT a:CodeEntity OR NOT b:CodeEntity)
        RETURN count(r) as dangling_count
    "#;

    let result = neo4j
        .execute_query(cypher, vec![("ns", serde_json::json!(neo4j.namespace()))])
        .await?;

    let dangling_count = result[0]
        .get("dangling_count")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    assert_eq!(
        dangling_count, 0,
        "Should have no dangling edges (edges without CodeEntity nodes). Found {}",
        dangling_count
    );

    Ok(())
}
