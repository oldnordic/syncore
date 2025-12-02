//! Debug Neo4j Transaction Issues
//!
//! This test investigates if there are transaction or connection issues causing the 100-entity limit

use anyhow::Result;
use syncore::config::{GraphBackend as ConfigBackend, GraphConfig};
use syncore::graph::{create_graph_backend, NodeLabel, NodeProperties};

#[tokio::test]
async fn debug_neo4j_transaction() -> Result<()> {
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

    let neo4j_backend = match create_graph_backend(&neo4j_config, "debug_transaction").await {
        Ok(backend) => backend,
        Err(e) => {
            println!("❌ Neo4j not available: {}", e);
            return Ok(());
        }
    };

    println!("🔍 Debugging Neo4j transaction issues...");

    // Test 1: Direct query execution to see if there's a limit
    println!("\n--- Test 1: Direct query execution ---");

    // Clear any existing entities
    for i in 1..=200 {
        neo4j_backend.delete_entity(i).await.ok();
    }

    // Insert entities one by one with direct queries
    for i in 1..=150 {
        let props = NodeProperties {
            id: i,
            name: format!("direct_test_{}", i),
            path: Some(format!("/tmp/direct_test_{}.rs", i)),
            start_line: Some(i),
            end_line: Some(i + 5),
            signature: Some(format!("fn direct_test_{}()", i)),
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
            Ok(_) => {
                if i % 10 == 0 {
                    let count =
                        neo4j_backend.get_entities_by_type(NodeLabel::Function).await?.len();
                    println!("After {} direct insertions: {} entities", i, count);
                }
            }
            Err(e) => {
                println!("❌ Error at entity {}: {}", i, e);
                break;
            }
        }
    }

    let final_direct_count = neo4j_backend.get_entities_by_type(NodeLabel::Function).await?.len();
    println!("Final direct insertion count: {}", final_direct_count);

    // Test 2: Check if entities are being overwritten by examining names
    println!("\n--- Test 2: Check for overwriting ---");

    let all_entities = neo4j_backend.get_entities_by_type(NodeLabel::Function).await?;
    println!("Total entities retrieved: {}", all_entities.len());

    // Group by ID to see if there are duplicates
    let mut id_groups = std::collections::HashMap::new();
    for entity in &all_entities {
        id_groups.entry(entity.id).or_insert_with(Vec::new).push(entity);
    }

    let mut duplicate_ids = Vec::new();
    for (id, entities) in &id_groups {
        if entities.len() > 1 {
            duplicate_ids.push(*id);
            println!("❌ Duplicate ID {}: {} entities", id, entities.len());
            for entity in entities {
                println!("  - Name: {}, Path: {:?}", entity.name, entity.path);
            }
        }
    }

    if duplicate_ids.is_empty() {
        println!("✅ No duplicate IDs found");
    }

    // Check for missing IDs
    let mut missing_ids = Vec::new();
    for i in 1..=150 {
        if !id_groups.contains_key(&i) {
            missing_ids.push(i);
        }
    }

    if missing_ids.len() <= 20 {
        println!("Missing IDs: {:?}", missing_ids);
    } else {
        println!("Missing IDs: {} total (first 20: {:?})", missing_ids.len(), &missing_ids[..20]);
    }

    // Test 3: Check if it's a namespace issue
    println!("\n--- Test 3: Namespace investigation ---");

    // Use a different namespace
    let neo4j_backend2 = match create_graph_backend(&neo4j_config, "debug_transaction_alt").await {
        Ok(backend) => backend,
        Err(e) => {
            println!("❌ Failed to create second backend: {}", e);
            return Ok(());
        }
    };

    // Insert 50 entities into the new namespace
    for i in 1..=50 {
        let props = NodeProperties {
            id: i,
            name: format!("namespace_test_{}", i),
            path: Some(format!("/tmp/namespace_test_{}.rs", i)),
            start_line: Some(i),
            end_line: Some(i + 5),
            signature: Some(format!("fn namespace_test_{}()", i)),
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

        neo4j_backend2.upsert_entity(NodeLabel::Function, props).await?;
    }

    let namespace2_count = neo4j_backend2.get_entities_by_type(NodeLabel::Function).await?.len();
    println!("Second namespace entity count: {}", namespace2_count);

    // Test 4: Check raw Neo4j queries
    println!("\n--- Test 4: Raw Neo4j query test ---");

    // Use execute_query directly to count entities
    let count_results = neo4j_backend
        .execute_query(
            "MATCH (e:Function:SynCore {namespace: $ns}) RETURN count(e) as count",
            vec![("ns", serde_json::json!("code_debug_transaction"))],
        )
        .await?;

    println!("Count query results:");
    for result in count_results.iter() {
        if let Some(count) = result.get("count") {
            println!("  Entity count: {}", count);
        }
    }

    println!("\n🔍 Transaction debug complete!");
    Ok(())
}
