//! Debug Neo4j Transaction Issues - Fixed
//!
//! This test investigates if there are transaction or connection issues causing the 100-entity limit

use anyhow::Result;
use syncore::config::{GraphBackend as ConfigBackend, GraphConfig};
use syncore::graph::{create_graph_backend, NodeLabel, NodeProperties};

#[tokio::test]
async fn debug_neo4j_transaction_fixed() -> Result<()> {
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

    let neo4j_backend = match create_graph_backend(&neo4j_config, "debug_transaction_fixed").await {
        Ok(backend) => backend,
        Err(e) => {
            println!("❌ Neo4j not available: {}", e);
            return Ok(());
        }
    };

    println!("🔍 Debugging Neo4j transaction issues (fixed)...");

    // Test: Raw Neo4j queries with correct namespace
    println!("\n--- Test: Raw Neo4j query test with correct namespace ---");

    // Clear any existing entities first
    for i in 1..=50 {
        neo4j_backend.delete_entity(i).await.ok();
    }

    // Insert 50 test entities
    for i in 1..=50 {
        let props = NodeProperties {
            id: i,
            name: format!("raw_test_{}", i),
            path: Some(format!("/tmp/raw_test_{}.rs", i)),
            start_line: Some(i),
            end_line: Some(i + 5),
            signature: Some(format!("fn raw_test_{}()", i)),
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

    // Use execute_query directly to count entities with correct namespace format
    let count_results = neo4j_backend
        .execute_query(
            "MATCH (e:Function:SynCore {namespace: $ns}) RETURN count(e) as count",
            vec![("ns", serde_json::json!("code_debug_transaction_fixed"))],
        )
        .await?;

    println!("Raw count query result: {:?}", count_results);

    // Get first 20 entities directly
    let entity_results = neo4j_backend.execute_query(
        "MATCH (e:Function:SynCore {namespace: $ns}) RETURN e.id as id, e.name as name ORDER BY e.id LIMIT 20",
        vec![("ns", serde_json::json!("code_debug_transaction_fixed"))],
    ).await?;

    println!("First 20 entities from raw query:");
    for (i, result) in entity_results.iter().enumerate() {
        if let Some(id) = result.get("id") {
            if let Some(name) = result.get("name") {
                println!("  {}: ID={}, Name={}", i + 1, id, name);
            }
        }
    }

    // Compare with get_entities_by_type
    let api_entities = neo4j_backend.get_entities_by_type(NodeLabel::Function).await?;
    println!("API get_entities_by_type count: {}", api_entities.len());

    println!("\n🔍 Transaction debug complete!");
    Ok(())
}
