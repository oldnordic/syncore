//! Debug Namespace Mismatch
//!
//! This test investigates the namespace mismatch between API and raw queries

use anyhow::Result;
use syncore::config::{GraphBackend as ConfigBackend, GraphConfig};
use syncore::graph::{create_graph_backend, NodeLabel, NodeProperties};

#[tokio::test]
async fn debug_namespace_mismatch() -> Result<()> {
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

    let neo4j_backend = match create_graph_backend(&neo4j_config, "debug_namespace").await {
        Ok(backend) => backend,
        Err(e) => {
            println!("❌ Neo4j not available: {}", e);
            return Ok(());
        }
    };

    println!("🔍 Debugging namespace mismatch...");

    // Test 1: Check what namespace the backend is actually using
    println!("\n--- Test 1: Check actual namespace ---");

    // Insert a test entity
    let props = NodeProperties {
        id: 999,
        name: "namespace_test".to_string(),
        path: Some("/tmp/namespace_test.rs".to_string()),
        start_line: Some(1),
        end_line: Some(5),
        signature: Some("fn namespace_test()".to_string()),
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

    let api_count = neo4j_backend.get_entities_by_type(NodeLabel::Function).await?.len();
    println!("API count: {}", api_count);

    // Try different namespace patterns
    let namespace_patterns =
        vec!["debug_namespace", "code_debug_namespace", "syncore_default", "code_syncore_default"];

    for pattern in namespace_patterns {
        let raw_count = neo4j_backend
            .execute_query(
                "MATCH (e:Function:SynCore {namespace: $ns}) RETURN count(e) as count",
                vec![("ns", serde_json::json!(pattern))],
            )
            .await?;

        println!("Raw count with namespace '{}': {:?}", pattern, raw_count);
    }

    // Test 2: Get all namespaces in the database
    println!("\n--- Test 2: All namespaces in database ---");

    let all_namespaces = neo4j_backend
        .execute_query(
            "MATCH (e) RETURN DISTINCT e.namespace as namespace ORDER BY e.namespace",
            vec![],
        )
        .await?;

    println!("All namespaces found:");
    for result in &all_namespaces {
        if let Some(ns) = result.get("namespace") {
            println!("  {}", ns);
        }
    }

    // Test 3: Get all Function nodes regardless of namespace
    println!("\n--- Test 3: All Function nodes ---");

    let all_functions = neo4j_backend
        .execute_query(
            "MATCH (e:Function) RETURN e.id as id, e.namespace as namespace ORDER BY e.id",
            vec![],
        )
        .await?;

    println!("All Function nodes:");
    for result in &all_functions {
        if let (Some(id), Some(ns)) = (result.get("id"), result.get("namespace")) {
            println!("  ID: {}, Namespace: {}", id, ns);
        }
    }

    // Clean up
    neo4j_backend.delete_entity(999).await?;

    println!("\n🔍 Namespace mismatch debug complete!");
    Ok(())
}
