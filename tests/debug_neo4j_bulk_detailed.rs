//! Debug Neo4j Bulk Operations in Detail
//!
//! This test helps identify why Neo4j only inserts 100/1000 entities

use anyhow::Result;
use std::sync::Arc;
use syncore::config::{GraphBackend as ConfigBackend, GraphConfig};
use syncore::graph::{create_graph_backend, NodeLabel, NodeProperties};
use tempfile::TempDir;

#[tokio::test]
async fn debug_neo4j_bulk_detailed() -> Result<()> {
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

    let neo4j_backend = match create_graph_backend(&neo4j_config, "debug_detailed").await {
        Ok(backend) => backend,
        Err(e) => {
            println!("❌ Neo4j not available: {}", e);
            return Ok(());
        }
    };

    println!("🔍 Debugging Neo4j bulk operations in detail...");

    // Test 1: Small batch (10 entities)
    println!("\n--- Test 1: Small batch (10 entities) ---");
    let small_entities: Vec<NodeProperties> = (1..=10)
        .map(|i| NodeProperties {
            id: i,
            name: format!("debug_small_{}", i),
            path: Some(format!("/tmp/debug_small_{}.rs", i)),
            start_line: Some(i),
            end_line: Some(i + 5),
            signature: Some(format!("fn debug_small_{}()", i)),
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

    let small_result =
        neo4j_backend.batch_upsert_entities(NodeLabel::Function, small_entities, 5).await?;
    println!("Small batch result: {} entities processed", small_result);

    let small_count = neo4j_backend.get_entities_by_type(NodeLabel::Function).await?.len();
    println!("Small batch actual count: {} entities in database", small_count);

    // Clear for next test
    for i in 1..=10 {
        neo4j_backend.delete_entity(i).await?;
    }

    // Test 2: Medium batch (100 entities)
    println!("\n--- Test 2: Medium batch (100 entities) ---");
    let medium_entities: Vec<NodeProperties> = (1..=100)
        .map(|i| NodeProperties {
            id: i,
            name: format!("debug_medium_{}", i),
            path: Some(format!("/tmp/debug_medium_{}.rs", i)),
            start_line: Some(i),
            end_line: Some(i + 5),
            signature: Some(format!("fn debug_medium_{}()", i)),
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

    let medium_result =
        neo4j_backend.batch_upsert_entities(NodeLabel::Function, medium_entities, 10).await?;
    println!("Medium batch result: {} entities processed", medium_result);

    let medium_count = neo4j_backend.get_entities_by_type(NodeLabel::Function).await?.len();
    println!("Medium batch actual count: {} entities in database", medium_count);

    // Clear for next test
    for i in 1..=100 {
        neo4j_backend.delete_entity(i).await?;
    }

    // Test 3: Large batch (1000 entities) - step by step
    println!("\n--- Test 3: Large batch (1000 entities) - step by step ---");
    let large_entities: Vec<NodeProperties> = (1..=1000)
        .map(|i| NodeProperties {
            id: i,
            name: format!("debug_large_{}", i),
            path: Some(format!("/tmp/debug_large_{}.rs", i)),
            start_line: Some(i),
            end_line: Some(i + 5),
            signature: Some(format!("fn debug_large_{}()", i)),
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

    println!("Total entities to process: {}", large_entities.len());

    // Test with different batch sizes
    let batch_sizes = vec![10, 50, 100, 200];

    for batch_size in batch_sizes {
        println!("\n--- Testing with batch size: {} ---", batch_size);

        // Clear database first
        for i in 1..=1000 {
            neo4j_backend.delete_entity(i).await.ok();
        }

        let start_time = std::time::Instant::now();
        let result = neo4j_backend
            .batch_upsert_entities(NodeLabel::Function, large_entities.clone(), batch_size)
            .await?;
        let duration = start_time.elapsed();

        let actual_count = neo4j_backend.get_entities_by_type(NodeLabel::Function).await?.len();

        println!(
            "Batch size {}: processed={}, actual={}, duration={:?}",
            batch_size, result, actual_count, duration
        );

        if result != actual_count {
            println!("❌ MISMATCH: processed {} but actual count is {}", result, actual_count);
        } else {
            println!("✅ MATCH: processed {} equals actual count {}", result, actual_count);
        }

        // Break early if we see the issue
        if actual_count < 1000 {
            println!("🔍 Found the issue! Breaking out of batch size testing");
            break;
        }
    }

    // Test 4: Individual entity insertion to see if there's a pattern
    println!("\n--- Test 4: Individual entity insertion (first 150) ---");

    // Clear database
    for i in 1..=1000 {
        neo4j_backend.delete_entity(i).await.ok();
    }

    let mut success_count = 0;
    let mut error_count = 0;

    for i in 1..=150 {
        let props = NodeProperties {
            id: i,
            name: format!("debug_individual_{}", i),
            path: Some(format!("/tmp/debug_individual_{}.rs", i)),
            start_line: Some(i),
            end_line: Some(i + 5),
            signature: Some(format!("fn debug_individual_{}()", i)),
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

        match neo4j_backend.upsert_entity(NodeLabel::Function, props).await {
            Ok(_) => success_count += 1,
            Err(e) => {
                error_count += 1;
                println!("❌ Error inserting entity {}: {}", i, e);
                if error_count > 5 {
                    println!("Too many errors, stopping individual test");
                    break;
                }
            }
        }

        // Check count every 10 insertions
        if i % 10 == 0 {
            let current_count =
                neo4j_backend.get_entities_by_type(NodeLabel::Function).await?.len();
            println!(
                "After {} insertions: success={}, errors={}, actual_count={}",
                i, success_count, error_count, current_count
            );
        }
    }

    let final_count = neo4j_backend.get_entities_by_type(NodeLabel::Function).await?.len();
    println!(
        "Final individual test: success={}, errors={}, actual_count={}",
        success_count, error_count, final_count
    );

    println!("\n🔍 Debug complete!");
    Ok(())
}
