#[cfg(test)]
mod neo4j_label_debug_tests {
    use anyhow::Result;
    use syncore::graph::sqlitegraph_impl::SQLiteGraphBackend;
    use syncore::graph::{GraphBackend, Neo4jBackend, NodeLabel, NodeProperties};
    use tempfile::TempDir;

    #[tokio::test]
    async fn debug_neo4j_labels() -> Result<()> {
        // Skip if Neo4j not available
        let neo4j_uri =
            std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string());
        let neo4j_user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
        let neo4j_pass = std::env::var("NEO4J_PASS").unwrap_or_else(|_| "password".to_string());

        let temp_dir = TempDir::new()?;
        let sqlite_path = temp_dir.path().join("test.db");
        let mut sqlite_backend =
            SQLiteGraphBackend::new(sqlite_path.to_str().unwrap(), "test_namespace").await?;

        let mut neo4j_backend: Option<Neo4jBackend> =
            match Neo4jBackend::connect(&neo4j_uri, &neo4j_user, &neo4j_pass, "test_namespace")
                .await
            {
                Ok(backend) => Some(backend),
                Err(_) => {
                    println!("Neo4j not available, skipping debug test");
                    return Ok(());
                }
            };

        let props = NodeProperties::full(
            1,
            "test_function".to_string(),
            "src/test.rs".to_string(),
            10,
            20,
            "rust".to_string(),
        );

        // Insert into both backends
        sqlite_backend.upsert_entity(NodeLabel::Function, props.clone()).await?;

        if let Some(ref mut backend) = neo4j_backend {
            backend.upsert_entity(NodeLabel::Function, props.clone()).await?;

            // Debug: Query raw labels from Neo4j
            let debug_query = r#"
                MATCH (e {namespace: $ns, graph_domain: $graph_domain})
                WHERE e.name = $name
                RETURN labels(e) as all_labels
            "#;

            let results = backend
                .execute_query(
                    debug_query,
                    vec![
                        ("ns", serde_json::json!("test_namespace")),
                        ("graph_domain", serde_json::json!("code")),
                        ("name", serde_json::json!("test_function")),
                    ],
                )
                .await?;

            println!("DEBUG: Raw Neo4j labels: {:?}", results);

            // Also test the reader query directly
            let reader_query = r#"
                MATCH (e {namespace: $ns, graph_domain: $graph_domain})
                WHERE e.name = $name
                RETURN e.id as id,
                       e.name as name,
                       CASE 
                           WHEN size(labels(e)) >= 2 THEN labels(e)[1]
                           ELSE labels(e)[0]
                       END as label,
                       e.path as path,
                       e.start_line as start_line,
                       e.end_line as end_line
            "#;

            let reader_results = backend
                .execute_query(
                    reader_query,
                    vec![
                        ("ns", serde_json::json!("test_namespace")),
                        ("graph_domain", serde_json::json!("code")),
                        ("name", serde_json::json!("test_function")),
                    ],
                )
                .await?;

            println!("DEBUG: Reader query results: {:?}", reader_results);

            // Force output to ensure it's visible
            eprintln!("DEBUG: Raw Neo4j labels: {:?}", results);
            eprintln!("DEBUG: Reader query results: {:?}", reader_results);
        }

        Ok(())
    }
}
