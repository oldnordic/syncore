#[cfg(test)]
mod neo4j_label_parity_tests {
    use anyhow::Result;
    use syncore::graph::sqlitegraph_impl::SQLiteGraphBackend;
    use syncore::graph::{GraphBackend, Neo4jBackend, NodeLabel, NodeProperties};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_neo4j_label_parity_fix() -> Result<()> {
        // Skip if Neo4j not available
        let neo4j_uri =
            std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string());
        let neo4j_user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
        let neo4j_pass = std::env::var("NEO4J_PASS").unwrap_or_else(|_| "password".to_string());

        // Test SQLite backend (baseline)
        let temp_dir = TempDir::new()?;
        let sqlite_path = temp_dir.path().join("test.db");
        let mut sqlite_backend =
            SQLiteGraphBackend::new(sqlite_path.to_str().unwrap(), "test_namespace").await?;

        // Test Neo4j backend if available
        let mut neo4j_backend: Option<Neo4jBackend> =
            match Neo4jBackend::connect(&neo4j_uri, &neo4j_user, &neo4j_pass, "test_namespace")
                .await
            {
                Ok(backend) => Some(backend),
                Err(_) => {
                    println!("Neo4j not available, skipping label parity test");
                    return Ok(());
                }
            };

        // Create test entities with identical properties (using correct signature)
        let props = NodeProperties::full(
            1, // id
            "test_function".to_string(),
            "src/test.rs".to_string(),
            10,                 // start_line
            20,                 // end_line
            "rust".to_string(), // language
        );

        // Insert into both backends
        sqlite_backend.upsert_entity(NodeLabel::Function, props.clone()).await?;

        if let Some(ref mut backend) = neo4j_backend {
            backend.upsert_entity(NodeLabel::Function, props.clone()).await?;
        }

        // Query entities from both backends and compare labels
        let sqlite_entities = sqlite_backend.get_entities_by_type(NodeLabel::Function).await?;

        if let Some(ref backend) = neo4j_backend {
            let neo4j_entities = backend.get_entities_by_type(NodeLabel::Function).await?;

            // Compare labels
            assert!(!sqlite_entities.is_empty(), "SQLite should return entities");
            assert!(!neo4j_entities.is_empty(), "Neo4j should return entities");

            // The key test: both should return same label
            let sqlite_label = &sqlite_entities[0].label;
            let neo4j_label = &neo4j_entities[0].label;

            println!("SQLite label: {}", sqlite_label);
            println!("Neo4j label: {}", neo4j_label);

            assert_eq!(
                sqlite_label, neo4j_label,
                "Label parity failed: SQLite returned '{}' but Neo4j returned '{}'",
                sqlite_label, neo4j_label
            );

            // Both should return "Function" not "CodeGraph"
            assert_eq!(sqlite_label, "Function", "SQLite should return 'Function'");
            assert_eq!(neo4j_label, "Function", "Neo4j should return 'Function' after fix");
        }

        Ok(())
    }
}
