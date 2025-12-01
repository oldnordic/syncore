//! TDD Tests for Graph Suite and Mapping Suite Bug Fixes
//!
//! Tests for 4 specific bugs:
//! - Bug 1: graph_suite query returns null for entity properties
//! - Bug 2: graph_suite relationship query fails with "Failed to execute Cypher query"
//! - Bug 3: mapping_suite search returns 0 results for indexed files
//! - Bug 4: mapping_suite get returns found=false for existing files
//!
//! APEX Standards:
//! - TDD-first: These tests MUST fail before fixes
//! - Real operations: No mocks for IO-bound components
//! - Deterministic: Tests must be repeatable
//! - Cleanup: Each test cleans up its data
//!
//! Prerequisites:
//! - Neo4j running at localhost:7687 (neo4j/testpassword123)
//! - SQLite databases in /home/feanor/.config/syncore/

use anyhow::Result;
use std::sync::{Arc, Mutex};
use syncore::graph::Neo4jClient;
use syncore::memory::Memory;
use syncore::router::SynCoreState;
use syncore::tasks::Tasks;
use syncore::vector::{RealEmbeddings, VectorStore};

/// Helper to create isolated test state for mapping tests
fn create_test_state() -> Result<SynCoreState> {
    let id = std::process::id();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let mem_path = format!("/tmp/syncore_bugfix_mem_{}_{}.db", id, ts);
    let task_path = format!("/tmp/syncore_bugfix_task_{}_{}.db", id, ts);

    let memory = Memory::new(&mem_path)?;
    let tasks = Tasks::new(&task_path)?;
    let embeddings = Box::new(RealEmbeddings::new(384)?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    Ok(SynCoreState::new(memory, tasks, vector_store))
}

// ============================================================================
// BUG 1: graph_suite query returns null for entity properties
// ============================================================================

#[tokio::test]
async fn test_bug1_graph_suite_query_returns_entity_properties() -> Result<()> {
    // Setup: Create a test entity in Neo4j with Function label
    let client = Neo4jClient::connect("bolt://localhost:7687", "neo4j", "testpassword123").await?;

    // Use actual label pattern from syncore (:Function:SynCore)
    let cypher_create = r#"
        CREATE (f:Function:SynCore {
            id: 999001,
            namespace: 'SynCore',
            name: 'test_function',
            path: '/tmp/test.rs',
            entity_type: 'function',
            start_line: 10,
            end_line: 20,
            signature: 'fn test_function()',
            language: 'rust'
        })
    "#;

    client.execute_query(cypher_create, vec![]).await?;

    // Test: Query the entity using :Function label and extract properties
    let cypher_query = r#"
        MATCH (n:Function:SynCore {id: 999001, namespace: 'SynCore'})
        RETURN n.id as id,
               n.name as name,
               n.path as path,
               n.entity_type as entity_type,
               n.start_line as start_line,
               n.signature as signature
    "#;

    let result = client.execute_query(cypher_query, vec![]).await?;

    // Assert: Properties should NOT be null
    assert_eq!(result.len(), 1, "Should return exactly 1 result");

    let row = &result[0];
    assert_eq!(row["id"], serde_json::json!(999001), "id should be 999001");
    assert_eq!(
        row["name"],
        serde_json::json!("test_function"),
        "name should not be null"
    );
    assert_eq!(
        row["path"],
        serde_json::json!("/tmp/test.rs"),
        "path should not be null"
    );
    assert_eq!(
        row["entity_type"],
        serde_json::json!("function"),
        "entity_type should not be null"
    );
    assert_eq!(
        row["start_line"],
        serde_json::json!(10),
        "start_line should be 10"
    );
    assert_eq!(
        row["signature"],
        serde_json::json!("fn test_function()"),
        "signature should not be null"
    );

    // Cleanup
    client
        .execute_query(
            "MATCH (n:Function:SynCore {id: 999001, namespace: 'SynCore'}) DELETE n",
            vec![],
        )
        .await?;

    Ok(())
}

#[tokio::test]
async fn test_bug1_returns_node_object_with_properties() -> Result<()> {
    // Setup: Create test entity
    let client = Neo4jClient::connect("bolt://localhost:7687", "neo4j", "testpassword123").await?;

    // Cleanup any existing test data first
    client
        .execute_query(
            "MATCH (n:Struct:SynCore {id: 999002, namespace: 'SynCore'}) DELETE n",
            vec![],
        )
        .await?;

    let cypher_create = r#"
        CREATE (s:Struct:SynCore {
            id: 999002,
            namespace: 'SynCore',
            name: 'TestStruct',
            path: '/tmp/test_struct.rs',
            entity_type: 'struct'
        })
    "#;

    client.execute_query(cypher_create, vec![]).await?;

    // Test: Return the node object itself (not individual properties)
    let cypher_query = "MATCH (n:Struct:SynCore {id: 999002, namespace: 'SynCore'}) RETURN n";

    let result = client.execute_query(cypher_query, vec![]).await?;

    // Assert: Node object should contain properties, not be null
    assert_eq!(result.len(), 1, "Should return exactly 1 result");

    let row = &result[0];
    let node_obj = &row["n"];

    // Node should be an object (not null)
    assert!(
        node_obj.is_object(),
        "Node 'n' should be an object, not null: {:?}",
        node_obj
    );

    // Properties should be extractable from node object
    assert_eq!(
        node_obj["name"],
        serde_json::json!("TestStruct"),
        "node.name should be accessible"
    );
    assert_eq!(
        node_obj["path"],
        serde_json::json!("/tmp/test_struct.rs"),
        "node.path should be accessible"
    );

    // Cleanup
    client
        .execute_query(
            "MATCH (n:Struct:SynCore {id: 999002, namespace: 'SynCore'}) DELETE n",
            vec![],
        )
        .await?;

    Ok(())
}

// ============================================================================
// BUG 2: graph_suite relationship query fails
// ============================================================================

#[tokio::test]
async fn test_bug2_graph_suite_relationship_query_succeeds() -> Result<()> {
    // Setup: Create two entities with a CALLS relationship
    let client = Neo4jClient::connect("bolt://localhost:7687", "neo4j", "testpassword123").await?;

    let cypher_setup = r#"
        CREATE (f1:Function:SynCore {
            id: 999003,
            namespace: 'SynCore',
            name: 'caller_fn',
            path: '/tmp/caller.rs'
        })
        CREATE (f2:Function:SynCore {
            id: 999004,
            namespace: 'SynCore',
            name: 'callee_fn',
            path: '/tmp/callee.rs'
        })
        CREATE (f1)-[:CALLS]->(f2)
    "#;

    client.execute_query(cypher_setup, vec![]).await?;

    // Test: Query relationships using :Function label
    let cypher_query = r#"
        MATCH (a:Function:SynCore {namespace: 'SynCore'})-[r:CALLS]->(b:Function:SynCore {namespace: 'SynCore'})
        WHERE a.id = 999003 AND b.id = 999004
        RETURN a.name as caller,
               type(r) as rel_type,
               b.name as callee
    "#;

    let result = client.execute_query(cypher_query, vec![]).await?;

    // Assert: Query should succeed and return relationship
    assert_eq!(result.len(), 1, "Should return exactly 1 relationship");

    let row = &result[0];
    assert_eq!(
        row["caller"],
        serde_json::json!("caller_fn"),
        "caller should be caller_fn"
    );
    assert_eq!(
        row["rel_type"],
        serde_json::json!("CALLS"),
        "relationship type should be CALLS"
    );
    assert_eq!(
        row["callee"],
        serde_json::json!("callee_fn"),
        "callee should be callee_fn"
    );

    // Cleanup
    client.execute_query(
        "MATCH (a:Function:SynCore)-[r:CALLS]->(b:Function:SynCore) WHERE a.id = 999003 AND b.id = 999004 DELETE r, a, b",
        vec![]
    ).await?;

    Ok(())
}

#[tokio::test]
async fn test_bug2_relationship_query_with_count_aggregation() -> Result<()> {
    // Setup: Create multiple relationships
    let client = Neo4jClient::connect("bolt://localhost:7687", "neo4j", "testpassword123").await?;

    let cypher_setup = r#"
        CREATE (f1:Function:SynCore {id: 999005, namespace: 'SynCore', name: 'main'})
        CREATE (f2:Function:SynCore {id: 999006, namespace: 'SynCore', name: 'helper1'})
        CREATE (f3:Function:SynCore {id: 999007, namespace: 'SynCore', name: 'helper2'})
        CREATE (f1)-[:CALLS]->(f2)
        CREATE (f1)-[:CALLS]->(f3)
    "#;

    client.execute_query(cypher_setup, vec![]).await?;

    // Test: Relationship query with GROUP BY and count()
    let cypher_query = r#"
        MATCH (a:Function:SynCore {namespace: 'SynCore'})-[r]->(b:Function:SynCore {namespace: 'SynCore'})
        WHERE a.id = 999005
        WITH type(r) as rel_type, count(r) as count
        RETURN rel_type, count
        ORDER BY count DESC
    "#;

    let result = client.execute_query(cypher_query, vec![]).await?;

    // Assert: Query should succeed and return aggregated counts
    assert_eq!(result.len(), 1, "Should return 1 aggregated result");

    let row = &result[0];
    assert_eq!(
        row["rel_type"],
        serde_json::json!("CALLS"),
        "rel_type should be CALLS"
    );
    assert_eq!(row["count"], serde_json::json!(2), "count should be 2");

    // Cleanup
    client.execute_query(
        "MATCH (n:Function:SynCore {namespace: 'SynCore'}) WHERE n.id IN [999005, 999006, 999007] DETACH DELETE n",
        vec![]
    ).await?;

    Ok(())
}

// ============================================================================
// BUG 3: mapping_suite search returns 0 results for indexed files
// ============================================================================

#[tokio::test]
async fn test_bug3_mapping_suite_search_finds_indexed_files() -> Result<()> {
    use syncore::portfolio::mapping_tool::{FileNode, MappingTool};

    // Setup: Create SynCoreState and populate file_nodes
    let state = create_test_state()?;
    let mapping = MappingTool::new(state.clone());

    // Record a test file node
    let test_node = FileNode {
        path: "/tmp/test_search_file.rs".to_string(),
        kind: "module".to_string(),
        language: Some("rust".to_string()),
        imports: vec!["std::io".to_string(), "anyhow::Result".to_string()],
        exports: vec!["MyStruct".to_string(), "my_function".to_string()],
        dependencies: vec![],
    };

    mapping.record_file(&test_node)?;

    // Test: Search for the file using semantic keywords
    let results = mapping.search_related("test_search_file rust")?;

    // Assert: Search should find the indexed file
    assert!(
        !results.is_empty(),
        "Search should return at least 1 result, got 0"
    );
    assert!(
        results.iter().any(|n| n.path == "/tmp/test_search_file.rs"),
        "Search results should include /tmp/test_search_file.rs"
    );

    // Cleanup: Remove from file_nodes table
    state.tasks.with_db(|conn| {
        conn.execute(
            "DELETE FROM file_nodes WHERE path = '/tmp/test_search_file.rs'",
            [],
        )?;
        Ok(())
    })?;

    Ok(())
}

// ============================================================================
// BUG 4: mapping_suite get returns found=false for existing files
// ============================================================================

#[tokio::test]
async fn test_bug4_mapping_suite_get_finds_indexed_file() -> Result<()> {
    use syncore::portfolio::mapping_tool::{FileNode, MappingTool};

    // Setup: Create SynCoreState and populate file_nodes
    let state = create_test_state()?;
    let mapping = MappingTool::new(state.clone());

    // Record a test file node
    let test_node = FileNode {
        path: "/tmp/test_get_file.rs".to_string(),
        kind: "module".to_string(),
        language: Some("rust".to_string()),
        imports: vec!["std::collections::HashMap".to_string()],
        exports: vec!["Config".to_string()],
        dependencies: vec![],
    };

    mapping.record_file(&test_node)?;

    // Test: Get the file by exact path
    let result = mapping.get_file("/tmp/test_get_file.rs")?;

    // Assert: File should be found
    assert!(
        result.is_some(),
        "get_file() should return Some(node), got None (found=false)"
    );

    let node = result.unwrap();
    assert_eq!(node.path, "/tmp/test_get_file.rs", "Path should match");
    assert_eq!(node.kind, "module", "Kind should be 'module'");
    assert_eq!(
        node.language,
        Some("rust".to_string()),
        "Language should be rust"
    );
    assert_eq!(
        node.imports,
        vec!["std::collections::HashMap"],
        "Imports should match"
    );
    assert_eq!(node.exports, vec!["Config"], "Exports should match");

    // Cleanup: Remove from file_nodes table
    state.tasks.with_db(|conn| {
        conn.execute(
            "DELETE FROM file_nodes WHERE path = '/tmp/test_get_file.rs'",
            [],
        )?;
        Ok(())
    })?;

    Ok(())
}

#[tokio::test]
async fn test_bug4_mapping_suite_get_returns_none_for_missing_file() -> Result<()> {
    use syncore::portfolio::mapping_tool::MappingTool;

    // Setup: Create empty MappingTool
    let state = create_test_state()?;
    let mapping = MappingTool::new(state);

    // Test: Query non-existent file
    let result = mapping.get_file("/nonexistent/path/file.rs")?;

    // Assert: Should correctly return None (found=false is correct here)
    assert!(
        result.is_none(),
        "get_file() should return None for non-existent files"
    );

    Ok(())
}
