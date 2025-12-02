//! Debug test to understand Neo4j bulk operation issue

use anyhow::Result;
use std::sync::Arc;
use syncore::config::{GraphBackend as ConfigBackend, GraphConfig};
use syncore::graph::{create_graph_backend, NodeLabel, NodeProperties};
use tempfile::TempDir;
use tokio;

#[tokio::test]
async fn debug_bulk_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let sqlite_path = temp_dir.path().join("debug.db").to_string_lossy().to_string();

    // SQLiteGraph backend
    let sqlite_config = GraphConfig {
        backend: ConfigBackend::SqliteGraph,
        path: sqlite_path,
        uri: String::new(),
        user: String::new(),
        password: String::new(),
        enabled: true,
    };

    let sqlite_backend = create_graph_backend(&sqlite_config, "debug_test").await?;

    // Create small test dataset
    let entities: Vec<NodeProperties> = (1..=10)
        .map(|i| NodeProperties {
            id: i,
            name: format!("debug_function_{}", i),
            path: Some(format!("/tmp/debug_{}.rs", i)),
            start_line: Some(i),
            end_line: Some(i + 5),
            signature: Some(format!("fn debug_function_{}()", i)),
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

    // Test SQLite bulk upsert
    let sqlite_count =
        sqlite_backend.batch_upsert_entities(NodeLabel::Function, entities.clone(), 5).await?;
    println!("SQLite bulk upsert count: {}", sqlite_count);

    // Verify SQLite results
    let sqlite_results = sqlite_backend.get_entities_by_type(NodeLabel::Function).await?;
    println!("SQLite actual entities: {}", sqlite_results.len());

    Ok(())
}
