//! Debug Delete Operations
//!
//! This test specifically investigates delete operation issues

use anyhow::Result;
use syncore::config::{GraphBackend as ConfigBackend, GraphConfig};
use syncore::graph::{create_graph_backend, NodeLabel, NodeProperties};

#[tokio::test]
async fn debug_delete_operations() -> Result<()> {
    // Setup Neo4j backend
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

    let neo4j_backend = match create_graph_backend(&neo4j_config, "debug_delete").await {
        Ok(backend) => backend,
        Err(e) => {
            println!("❌ Neo4j not available: {}", e);
            return Ok(());
        }
    };

    println!("🔍 Debugging delete operations...");

    // Test 1: Insert 20 entities, then delete every other one
    println!("\n--- Test 1: Insert 20, delete odd IDs ---");

    // Insert 20 entities
    for i in 1..=20 {
        let props = NodeProperties {
            id: i,
            name: format!("delete_test_{}", i),
            path: Some(format!("/tmp/delete_test_{}.rs", i)),
            start_line: Some(i),
            end_line: Some(i + 5),
            signature: Some(format!("fn delete_test_{}()", i)),
            body_snippet: None,
            docstring: None,
            hash: None,
            language: Some("rust".to_string()),
            file_sha256: None,
            mtime: None,
            created_at: None,
            last_modified_at: None,
            change_count: None,
            author_count: None,
        };

        neo4j_backend.upsert_entity(NodeLabel::Function, props).await?;
    }

    let count_before = neo4j_backend.get_entities_by_type(NodeLabel::Function).await?.len();
    println!("After inserting 20 entities: {} entities", count_before);

    // Delete odd IDs (1, 3, 5, ..., 19) - should be 10 deletions
    let mut delete_errors = 0;
    for i in (1..=20).step_by(2) {
        match neo4j_backend.delete_entity(i).await {
            Ok(_) => println!("  ✓ Deleted entity {}", i),
            Err(e) => {
                delete_errors += 1;
                println!("  ❌ Error deleting entity {}: {}", i, e);
            }
        }
    }

    let count_after = neo4j_backend.get_entities_by_type(NodeLabel::Function).await?.len();
    println!("After deleting 10 entities: {} entities (expected: 10)", count_after);
    println!("Delete errors: {}", delete_errors);

    // List remaining entities to verify
    let remaining_entities = neo4j_backend.get_entities_by_type(NodeLabel::Function).await?;
    println!(
        "Remaining entity IDs: {:?}",
        remaining_entities.iter().map(|e| e.id).collect::<Vec<_>>()
    );

    // Test 2: Raw query to verify deletion
    println!("\n--- Test 2: Raw query verification ---");

    let raw_count = neo4j_backend
        .execute_query(
            "MATCH (e:Function:SynCore {namespace: $ns}) RETURN count(e) as count",
            vec![("ns", serde_json::json!("code_debug_delete"))],
        )
        .await?;

    println!("Raw count query result: {:?}", raw_count);

    let raw_entities = neo4j_backend
        .execute_query(
            "MATCH (e:Function:SynCore {namespace: $ns}) RETURN e.id as id ORDER BY e.id",
            vec![("ns", serde_json::json!("code_debug_delete"))],
        )
        .await?;

    println!("Raw entities count: {}", raw_entities.len());
    for (i, result) in raw_entities.iter().enumerate() {
        if let Some(id) = result.get("id") {
            if i < 10 {
                // Show first 10
                println!("  Raw entity {}: {}", i + 1, id);
            }
        }
    }

    // Test 3: Delete remaining entities one by one
    println!("\n--- Test 3: Delete remaining entities ---");

    for entity in &remaining_entities {
        match neo4j_backend.delete_entity(entity.id).await {
            Ok(_) => println!("  ✓ Deleted remaining entity {}", entity.id),
            Err(e) => println!("  ❌ Error deleting remaining entity {}: {}", entity.id, e),
        }
    }

    let final_count = neo4j_backend.get_entities_by_type(NodeLabel::Function).await?.len();
    let final_raw_count = neo4j_backend
        .execute_query(
            "MATCH (e:Function:SynCore {namespace: $ns}) RETURN count(e) as count",
            vec![("ns", serde_json::json!("code_debug_delete"))],
        )
        .await?;

    println!("Final API count: {}", final_count);
    println!("Final raw count: {:?}", final_raw_count);

    println!("\n🔍 Delete operations debug complete!");
    Ok(())
}
