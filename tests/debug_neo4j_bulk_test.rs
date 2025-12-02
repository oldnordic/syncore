//! Debug test to understand Neo4j bulk operation issue

use anyhow::Result;
use syncore::config::{GraphBackend as ConfigBackend, GraphConfig};
use syncore::graph::{create_graph_backend, NodeLabel, NodeProperties};
use tokio;

#[tokio::test]
async fn debug_neo4j_bulk_operations() -> Result<()> {
    // Neo4j backend (if available)
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

    let neo4j_backend = match create_graph_backend(&neo4j_config, "debug_bulk_test").await {
        Ok(backend) => backend,
        Err(e) => {
            println!("⚠️  Neo4j not available: {}", e);
            return Ok(());
        }
    };

    // Create small test dataset
    let entities: Vec<NodeProperties> = (1..=10)
        .map(|i| NodeProperties {
            id: i,
            name: format!("debug_neo4j_function_{}", i),
            path: Some(format!("/tmp/debug_neo4j_{}.rs", i)),
            start_line: Some(i),
            end_line: Some(i + 5),
            signature: Some(format!("fn debug_neo4j_function_{}()", i)),
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
        })
        .collect();

    println!("Created {} test entities", entities.len());

    // Test Neo4j bulk upsert
    let neo4j_count =
        neo4j_backend.batch_upsert_entities(NodeLabel::Function, entities.clone(), 5).await?;
    println!("Neo4j bulk upsert count: {}", neo4j_count);

    // Verify Neo4j results
    let neo4j_results = neo4j_backend.get_entities_by_type(NodeLabel::Function).await?;
    println!("Neo4j actual entities: {}", neo4j_results.len());

    Ok(())
}
