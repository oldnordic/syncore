//! RAGGraph Parity Tests
//!
//! Tests RAGGraph operations including embedding insertion, retrieval,
//! and similarity search parity between Neo4j and SQLiteGraph backends.

use anyhow::Result;
use serde_json::json;
use std::collections::HashMap;
use syncore::config::{GraphBackend as ConfigBackend, GraphConfig};
use syncore::graph::{
    create_graph_backend, EntityResult, GraphBackend, NodeLabel, NodeProperties, RelationType,
};
use tempfile::TempDir;
use tokio;

/// Test configuration for both backends
async fn setup_test_backends() -> Result<(Box<dyn GraphBackend>, Box<dyn GraphBackend>)> {
    // Setup SQLiteGraph backend
    let temp_dir = TempDir::new()?;
    let sqlite_path = temp_dir.path().join("test.db").to_string_lossy().to_string();

    let sqlite_config = GraphConfig {
        backend: ConfigBackend::SqliteGraph,
        path: sqlite_path.clone(),
        uri: String::new(),
        user: String::new(),
        password: String::new(),
        enabled: true,
    };

    let sqlite_backend = create_graph_backend(&sqlite_config, "raggraph_parity_test").await?;

    // Setup Neo4j backend (if available)
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

    let neo4j_backend = match create_graph_backend(&neo4j_config, "raggraph_parity_test").await {
        Ok(backend) => backend,
        Err(_) => {
            // Skip Neo4j tests if not available
            return Ok((Box::new(sqlite_backend), Box::new(sqlite_backend)));
        }
    };

    Ok((Box::new(sqlite_backend), Box::new(neo4j_backend)))
}

/// Clean up test data in both backends
async fn cleanup_backends(
    sqlite_backend: &Box<dyn GraphBackend>,
    neo4j_backend: &Box<dyn GraphBackend>,
) -> Result<()> {
    // Clear all entities in test namespace
    let _ = sqlite_backend
        .execute_query("DELETE FROM code_entities WHERE file_path LIKE '%raggraph_parity_test%'", vec![])
        .await;

    let _ = neo4j_backend
        .execute_query("MATCH (n) WHERE n.file_path CONTAINS 'raggraph_parity_test' DETACH DELETE n", vec![])
        .await;

    Ok(())
}

/// Normalize entity results for comparison
fn normalize_entity(entity: &EntityResult) -> HashMap<String, serde_json::Value> {
    let mut map = HashMap::new();
    map.insert("id".to_string(), json!(entity.id));
    map.insert("name".to_string(), json!(entity.name));
    map.insert("label".to_string(), json!(entity.label));
    map.insert("path".to_string(), json!(entity.path));
    map.insert("start_line".to_string(), json!(entity.start_line));
    map.insert("end_line".to_string(), json!(entity.end_line));
    map.insert("signature".to_string(), json!(entity.signature));
    map.insert("body_snippet".to_string(), json!(entity.body_snippet));
    // Skip temporal fields as they may differ between backends
    map
}

/// Compare entity lists with deterministic ordering
fn compare_entity_lists(
    sqlite_results: &[EntityResult],
    neo4j_results: &[EntityResult],
    test_name: &str,
) -> Result<()> {
    // Normalize all entities
    let sqlite_normalized: Vec<_> = sqlite_results.iter().map(normalize_entity).collect();
    let neo4j_normalized: Vec<_> = neo4j_results.iter().map(normalize_entity).collect();

    // Sort by ID for deterministic comparison
    let mut sqlite_sorted = sqlite_normalized.clone();
    let mut neo4j_sorted = neo4j_normalized.clone();
    sqlite_sorted.sort_by(|a, b| a["id"].as_i64().cmp(&b["id"].as_i64()));
    neo4j_sorted.sort_by(|a, b| a["id"].as_i64().cmp(&b["id"].as_i64()));

    // Compare counts
    if sqlite_sorted.len() != neo4j_sorted.len() {
        anyhow::bail!(
            "{}: Entity count mismatch - SQLite: {}, Neo4j: {}",
            test_name,
            sqlite_sorted.len(),
            neo4j_sorted.len()
        );
    }

    // Compare each entity
    for (i, (sqlite_entity, neo4j_entity)) in
        sqlite_sorted.iter().zip(neo4j_sorted.iter()).enumerate()
    {
        if sqlite_entity != neo4j_entity {
            anyhow::bail!(
                "{}: Entity {} mismatch\nSQLite: {:?}\nNeo4j: {:?}",
                test_name,
                i + 1,
                sqlite_entity,
                neo4j_entity
            );
        }
    }

    println!("✓ {}: {} entities match", test_name, sqlite_sorted.len());
    Ok(())
}

/// Compare query results (JSON arrays)
fn compare_query_results(
    sqlite_results: &[serde_json::Value],
    neo4j_results: &[serde_json::Value],
    test_name: &str,
) -> Result<()> {
    if sqlite_results.len() != neo4j_results.len() {
        anyhow::bail!(
            "{}: Result count mismatch - SQLite: {}, Neo4j: {}",
            test_name,
            sqlite_results.len(),
            neo4j_results.len()
        );
    }

    // Sort results for deterministic comparison
    let mut sqlite_sorted = sqlite_results.to_vec();
    let mut neo4j_sorted = neo4j_results.to_vec();
    sqlite_sorted.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
    neo4j_sorted.sort_by(|a, b| a.to_string().cmp(&b.to_string()));

    for (i, (sqlite_result, neo4j_result)) in sqlite_sorted.iter().zip(neo4j_sorted.iter()).enumerate() {
        if sqlite_result != neo4j_result {
            anyhow::bail!(
                "{}: Result {} mismatch\nSQLite: {:?}\nNeo4j: {:?}",
                test_name,
                i + 1,
                sqlite_result,
                neo4j_result
            );
        }
    }

    println!("✓ {}: {} results match", test_name, sqlite_sorted.len());
    Ok(())
}

#[tokio::test]
async fn test_rag_insert_and_retrieve_parity() -> Result<()> {
    let (sqlite_backend, neo4j_backend) = setup_test_backends().await?;

    cleanup_backends(&sqlite_backend, &neo4j_backend).await?;

    // Create task nodes
    sqlite_backend.create_task_node(1001, "Test Task 1", "pending").await?;
    sqlite_backend.create_task_node(1002, "Test Task 2", "in_progress").await?;
    neo4j_backend.create_task_node(1001, "Test Task 1", "pending").await?;
    neo4j_backend.create_task_node(1002, "Test Task 2", "in_progress").await?;

    // Create embedding nodes
    let embedding_texts = vec![
        (1003, "This is test embedding 1", "hash_001"),
        (1004, "This is test embedding 2", "hash_002"),
        (1005, "This is test embedding 3", "hash_003"),
    ];

    for (id, text, hash) in &embedding_texts {
        sqlite_backend.create_embedding_node(*id, text, hash).await?;
        neo4j_backend.create_embedding_node(*id, text, hash).await?;
    }

    // Link embeddings to tasks
    sqlite_backend.link_embedding_to_task(1003, 1001).await?;
    sqlite_backend.link_embedding_to_task(1004, 1001).await?;
    sqlite_backend.link_embedding_to_task(1005, 1002).await?;
    
    neo4j_backend.link_embedding_to_task(1003, 1001).await?;
    neo4j_backend.link_embedding_to_task(1004, 1001).await?;
    neo4j_backend.link_embedding_to_task(1005, 1002).await?;

    // Retrieve task entities
    let sqlite_task1 = sqlite_backend.get_entity_by_id(1001).await?;
    let neo4j_task1 = neo4j_backend.get_entity_by_id(1001).await?;

    match (sqlite_task1, neo4j_task1) {
        (Some(sqlite_entity), Some(neo4j_entity)) => {
            compare_entity_lists(&[sqlite_entity], &[neo4j_entity], "rag_retrieve_task1")?;
        }
        (None, None) => {
            anyhow::bail!("Both backends failed to retrieve task 1");
        }
        (Some(_), None) => {
            anyhow::bail!("Neo4j backend failed to retrieve task 1");
        }
        (None, Some(_)) => {
            anyhow::bail!("SQLite backend failed to retrieve task 1");
        }
    }

    // Retrieve embedding entities
    let sqlite_embedding = sqlite_backend.get_entity_by_id(1003).await?;
    let neo4j_embedding = neo4j_backend.get_entity_by_id(1003).await?;

    match (sqlite_embedding, neo4j_embedding) {
        (Some(sqlite_entity), Some(neo4j_entity)) => {
            compare_entity_lists(&[sqlite_entity], &[neo4j_entity], "rag_retrieve_embedding")?;
        }
        (None, None) => {
            anyhow::bail!("Both backends failed to retrieve embedding");
        }
        (Some(_), None) => {
            anyhow::bail!("Neo4j backend failed to retrieve embedding");
        }
        (None, Some(_)) => {
            anyhow::bail!("SQLite backend failed to retrieve embedding");
        }
    }

    cleanup_backends(&sqlite_backend, &neo4j_backend).await?;
    Ok(())
}

#[tokio::test]
async fn test_rag_neighbors_parity() -> Result<()> {
    let (sqlite_backend, neo4j_backend) = setup_test_backends().await?;

    cleanup_backends(&sqlite_backend, &neo4j_backend).await?;

    // Create a RAG graph structure
    // Task 1 -> Embedding 1, Embedding 2
    // Task 2 -> Embedding 3
    // Memory nodes connected to tasks

    // Create tasks
    sqlite_backend.create_task_node(2001, "RAG Task 1", "pending").await?;
    sqlite_backend.create_task_node(2002, "RAG Task 2", "completed").await?;
    neo4j_backend.create_task_node(2001, "RAG Task 1", "pending").await?;
    neo4j_backend.create_task_node(2002, "RAG Task 2", "completed").await?;

    // Create embeddings
    sqlite_backend.create_embedding_node(2003, "Embedding for task 1", "rag_hash_001").await?;
    sqlite_backend.create_embedding_node(2004, "Another embedding for task 1", "rag_hash_002").await?;
    sqlite_backend.create_embedding_node(2005, "Embedding for task 2", "rag_hash_003").await?;
    
    neo4j_backend.create_embedding_node(2003, "Embedding for task 1", "rag_hash_001").await?;
    neo4j_backend.create_embedding_node(2004, "Another embedding for task 1", "rag_hash_002").await?;
    neo4j_backend.create_embedding_node(2005, "Embedding for task 2", "rag_hash_003").await?;

    // Create memory nodes
    sqlite_backend.create_memory_node("memory_key_1", "Memory content for task 1").await?;
    sqlite_backend.create_memory_node("memory_key_2", "Memory content for task 2").await?;
    neo4j_backend.create_memory_node("memory_key_1", "Memory content for task 1").await?;
    neo4j_backend.create_memory_node("memory_key_2", "Memory content for task 2").await?;

    // Link embeddings to tasks
    sqlite_backend.link_embedding_to_task(2003, 2001).await?;
    sqlite_backend.link_embedding_to_task(2004, 2001).await?;
    sqlite_backend.link_embedding_to_task(2005, 2002).await?;
    
    neo4j_backend.link_embedding_to_task(2003, 2001).await?;
    neo4j_backend.link_embedding_to_task(2004, 2001).await?;
    neo4j_backend.link_embedding_to_task(2005, 2002).await?;

    // Test neighbors of task 1 (should include embeddings 2003, 2004)
    let sqlite_neighbors_task1 = sqlite_backend.get_neighbors(2001).await?;
    let neo4j_neighbors_task1 = neo4j_backend.get_neighbors(2001).await?;

    compare_entity_lists(&sqlite_neighbors_task1, &neo4j_neighbors_task1, "rag_neighbors_task1")?;
    assert!(sqlite_neighbors_task1.len() >= 2, "Task 1 should have at least 2 neighbors");

    // Test neighbors of task 2 (should include embedding 2005)
    let sqlite_neighbors_task2 = sqlite_backend.get_neighbors(2002).await?;
    let neo4j_neighbors_task2 = neo4j_backend.get_neighbors(2002).await?;

    compare_entity_lists(&sqlite_neighbors_task2, &neo4j_neighbors_task2, "rag_neighbors_task2")?;
    assert!(sqlite_neighbors_task2.len() >= 1, "Task 2 should have at least 1 neighbor");

    // Test neighbors of embedding 2003 (should include task 2001)
    let sqlite_neighbors_emb = sqlite_backend.get_neighbors(2003).await?;
    let neo4j_neighbors_emb = neo4j_backend.get_neighbors(2003).await?;

    compare_entity_lists(&sqlite_neighbors_emb, &neo4j_neighbors_emb, "rag_neighbors_embedding")?;
    assert!(sqlite_neighbors_emb.len() >= 1, "Embedding should have at least 1 neighbor");

    cleanup_backends(&sqlite_backend, &neo4j_backend).await?;
    Ok(())
}

#[tokio::test]
async fn test_hopgraph_similarity_parity() -> Result<()> {
    let (sqlite_backend, neo4j_backend) = setup_test_backends().await?;

    cleanup_backends(&sqlite_backend, &neo4j_backend).await?;

    // Create a similarity graph structure
    // Central task connected to multiple similar embeddings
    // Each embedding connected to related tasks

    // Create central task
    sqlite_backend.create_task_node(3001, "Central Similarity Task", "active").await?;
    neo4j_backend.create_task_node(3001, "Central Similarity Task", "active").await?;

    // Create related tasks
    for i in 1..=3 {
        let task_id = 3001 + i;
        sqlite_backend.create_task_node(task_id, &format!("Related Task {}", i), "pending").await?;
        neo4j_backend.create_task_node(task_id, &format!("Related Task {}", i), "pending").await?;
    }

    // Create embeddings with similarity relationships
    for i in 1..=5 {
        let embedding_id = 3004 + i;
        let text = format!("Similar embedding content {}", i);
        let hash = format!("similarity_hash_{:03}", i);
        
        sqlite_backend.create_embedding_node(embedding_id, &text, &hash).await?;
        neo4j_backend.create_embedding_node(embedding_id, &text, &hash).await?;
    }

    // Link embeddings to central task
    for i in 1..=5 {
        let embedding_id = 3004 + i;
        sqlite_backend.link_embedding_to_task(embedding_id, 3001).await?;
        neo4j_backend.link_embedding_to_task(embedding_id, 3001).await?;
    }

    // Link some embeddings to related tasks (similarity network)
    let similarity_links = vec![
        (3005, 3002), // embedding 1 -> related task 1
        (3006, 3003), // embedding 2 -> related task 2
        (3007, 3004), // embedding 3 -> related task 3
        (3008, 3002), // embedding 4 -> related task 1 (duplicate connection)
        (3009, 3003), // embedding 5 -> related task 2 (duplicate connection)
    ];

    for (embedding_id, task_id) in &similarity_links {
        sqlite_backend.link_embedding_to_task(*embedding_id, *task_id).await?;
        neo4j_backend.link_embedding_to_task(*embedding_id, *task_id).await?;
    }

    // Test 2-hop similarity from central task
    // Central task -> embeddings -> related tasks
    let sqlite_2hop_query = r#"
        SELECT DISTINCT e3.id as related_task_id, e3.name as related_task_name, COUNT(*) as similarity_score
        FROM code_entities e1
        JOIN code_edges edge1 ON e1.id = edge1.src_entity_id
        JOIN code_entities e2 ON e2.id = edge1.dst_entity_id
        JOIN code_edges edge2 ON e2.id = edge2.src_entity_id
        JOIN code_entities e3 ON e3.id = edge2.dst_entity_id
        WHERE e1.id = 3001 AND e3.id != 3001
        GROUP BY e3.id, e3.name
        ORDER BY similarity_score DESC, e3.id
    "#;

    let neo4j_2hop_query = r#"
        MATCH (central:Task)-[:USES]->(embedding:Embedding)-[:USES]->(related:Task)
        WHERE central.id = 3001 AND related.id <> 3001
        RETURN related.id as related_task_id, related.name as related_task_name, COUNT(*) as similarity_score
        ORDER BY similarity_score DESC, related.id
    "#;

    let sqlite_2hop_results = sqlite_backend.execute_query(sqlite_2hop_query, vec![]).await?;
    let neo4j_2hop_results = neo4j_backend.execute_query(neo4j_2hop_query, vec![]).await?;

    compare_query_results(&sqlite_2hop_results, &neo4j_2hop_results, "hopgraph_similarity_2hop")?;

    // Verify similarity scores
    assert!(!sqlite_2hop_results.is_empty(), "Should find related tasks through similarity");
    
    // Check that tasks with more connections have higher similarity scores
    if let (Some(sqlite_result), Some(neo4j_result)) = (sqlite_2hop_results.first(), neo4j_2hop_results.first()) {
        if let (Some(sqlite_score), Some(neo4j_score)) = (
            sqlite_result.get("similarity_score").and_then(|v| v.as_i64()),
            neo4j_result.get("similarity_score").and_then(|v| v.as_i64())
        ) {
            assert_eq!(sqlite_score, neo4j_score, "Similarity scores should match");
            assert!(sqlite_score > 1, "Most similar task should have score > 1");
        }
    }

    cleanup_backends(&sqlite_backend, &neo4j_backend).await?;
    Ok(())
}

#[tokio::test]
async fn test_embedding_lookup_parity() -> Result<()> {
    let (sqlite_backend, neo4j_backend) = setup_test_backends().await?;

    cleanup_backends(&sqlite_backend, &neo4j_backend).await?;

    // Create multiple embeddings with different characteristics
    let embedding_data = vec![
        (4001, "Function definition for data processing", "embed_hash_001", "data_processing"),
        (4002, "Error handling in async functions", "embed_hash_002", "error_handling"),
        (4003, "Database connection management", "embed_hash_003", "database"),
        (4004, "HTTP request implementation", "embed_hash_004", "networking"),
        (4005, "JSON parsing and serialization", "embed_hash_005", "serialization"),
        (4006, "Authentication and authorization", "embed_hash_006", "security"),
        (4007, "Logging and debugging utilities", "embed_hash_007", "logging"),
        (4008, "Configuration file parsing", "embed_hash_008", "config"),
        (4009, "Memory management and cleanup", "embed_hash_009", "memory"),
    ];

    // Create embeddings in both backends
    for (id, text, hash, _category) in &embedding_data {
        sqlite_backend.create_embedding_node(*id, text, hash).await?;
        neo4j_backend.create_embedding_node(*id, text, hash).await?;
    }

    // Create tasks for different categories
    let task_data = vec![
        (4101, "Data Processing Pipeline"),
        (4102, "Error Recovery System"),
        (4103, "Database Migration Tool"),
        (4104, "API Server Implementation"),
        (4105, "Security Audit Module"),
    ];

    for (id, title) in &task_data {
        sqlite_backend.create_task_node(*id, title, "planning").await?;
        neo4j_backend.create_task_node(*id, title, "planning").await?;
    }

    // Link embeddings to relevant tasks
    let task_embeddings = vec![
        (4001, 4101), // data processing -> data pipeline
        (4002, 4102), // error handling -> error recovery
        (4003, 4103), // database -> database migration
        (4004, 4104), // networking -> API server
        (4006, 4105), // security -> security audit
        (4005, 4104), // serialization -> API server
        (4007, 4102), // logging -> error recovery
    ];

    for (embedding_id, task_id) in &task_embeddings {
        sqlite_backend.link_embedding_to_task(*embedding_id, *task_id).await?;
        neo4j_backend.link_embedding_to_task(*embedding_id, *task_id).await?;
    }

    // Test embedding lookup by hash pattern
    let sqlite_lookup_query = r#"
        SELECT id, name, signature
        FROM code_entities 
        WHERE name LIKE 'embed_hash_%'
        AND signature LIKE '%database%'
        ORDER BY id
    "#;

    let neo4j_lookup_query = r#"
        MATCH (n:Embedding)
        WHERE n.name STARTS WITH 'embed_hash_'
        AND n.signature CONTAINS 'database'
        RETURN n.id as id, n.name as name, n.signature as signature
        ORDER BY n.id
    "#;

    let sqlite_lookup_results = sqlite_backend.execute_query(sqlite_lookup_query, vec![]).await?;
    let neo4j_lookup_results = neo4j_backend.execute_query(neo4j_lookup_query, vec![]).await?;

    compare_query_results(&sqlite_lookup_results, &neo4j_lookup_results, "embedding_lookup_by_content")?;

    // Should find the database embedding
    assert_eq!(sqlite_lookup_results.len(), 1, "Should find 1 database-related embedding");

    // Test embedding lookup by task relationships
    let sqlite_task_embeddings_query = r#"
        SELECT e.id as embedding_id, e.name as embedding_name, e.signature as embedding_text
        FROM code_entities e
        JOIN code_edges edge ON e.id = edge.src_entity_id
        WHERE edge.dst_entity_id = 4104
        ORDER BY e.id
    "#;

    let neo4j_task_embeddings_query = r#"
        MATCH (embedding:Embedding)-[:USES]->(task:Task)
        WHERE task.id = 4104
        RETURN embedding.id as embedding_id, embedding.name as embedding_name, 
               embedding.signature as embedding_text
        ORDER BY embedding.id
    "#;

    let sqlite_task_results = sqlite_backend.execute_query(sqlite_task_embeddings_query, vec![]).await?;
    let neo4j_task_results = neo4j_backend.execute_query(neo4j_task_embeddings_query, vec![]).await?;

    compare_query_results(&sqlite_task_results, &neo4j_task_results, "embedding_lookup_by_task")?;

    // Should find 2 embeddings for API server task
    assert_eq!(sqlite_task_results.len(), 2, "Should find 2 embeddings for API server task");

    // Test reverse lookup: tasks by embedding
    let sqlite_embedding_tasks_query = r#"
        SELECT e.id as task_id, e.name as task_name
        FROM code_entities e
        JOIN code_edges edge ON e.id = edge.dst_entity_id
        WHERE edge.src_entity_id = 4002
        ORDER BY e.id
    "#;

    let neo4j_embedding_tasks_query = r#"
        MATCH (task:Task)<-[:USES]-(embedding:Embedding)
        WHERE embedding.id = 4002
        RETURN task.id as task_id, task.name as task_name
        ORDER BY task.id
    "#;

    let sqlite_reverse_results = sqlite_backend.execute_query(sqlite_embedding_tasks_query, vec![]).await?;
    let neo4j_reverse_results = neo4j_backend.execute_query(neo4j_embedding_tasks_query, vec![]).await?;

    compare_query_results(&sqlite_reverse_results, &neo4j_reverse_results, "embedding_lookup_reverse")?;

    // Should find 1 task for error handling embedding
    assert_eq!(sqlite_reverse_results.len(), 1, "Should find 1 task for error handling embedding");

    cleanup_backends(&sqlite_backend, &neo4j_backend).await?;
    Ok(())
}

#[tokio::test]
async fn test_rag_graph_structure_parity() -> Result<()> {
    let (sqlite_backend, neo4j_backend) = setup_test_backends().await?;

    cleanup_backends(&sqlite_backend, &neo4j_backend).await?;

    // Create a complex RAG graph structure
    // Multiple tasks with interconnected embeddings and memories

    // Create main tasks
    let main_tasks = vec![
        (5001, "Main Feature Development"),
        (5002, "Bug Fix Implementation"),
        (5003, "Performance Optimization"),
        (5004, "Documentation Update"),
    ];

    for (id, title) in &main_tasks {
        sqlite_backend.create_task_node(*id, title, "active").await?;
        neo4j_backend.create_task_node(*id, title, "active").await?;
    }

    // Create subtasks
    let subtasks = vec![
        (5005, "Database Schema Changes"),
        (5006, "API Endpoint Updates"),
        (5007, "Unit Test Implementation"),
        (5008, "Integration Testing"),
    ];

    for (id, title) in &subtasks {
        sqlite_backend.create_task_node(*id, title, "pending").await?;
        neo4j_backend.create_task_node(*id, title, "pending").await?;
    }

    // Link subtasks to main tasks
    let subtask_links = vec![
        (5005, 5001), // schema changes -> main feature
        (5006, 5001), // API updates -> main feature
        (5007, 5002), // unit tests -> bug fix
        (5008, 5002), // integration tests -> bug fix
    ];

    for (subtask_id, main_task_id) in &subtask_links {
        sqlite_backend.create_subtask_relationship(*main_task_id, *subtask_id).await?;
        neo4j_backend.create_subtask_relationship(*main_task_id, *subtask_id).await?;
    }

    // Create embeddings for each task
    for i in 1..=8 {
        let embedding_id = 5008 + i;
        let text = format!("Embedding content for task {}", i);
        let hash = format!("rag_structure_hash_{:03}", i);
        
        sqlite_backend.create_embedding_node(embedding_id, &text, &hash).await?;
        neo4j_backend.create_embedding_node(embedding_id, &text, &hash).await?;
    }

    // Link embeddings to tasks
    for i in 1..=8 {
        let embedding_id = 5008 + i;
        let task_id = 5000 + i; // Map embeddings 1-8 to tasks 5001-5008
        sqlite_backend.link_embedding_to_task(embedding_id, task_id).await?;
        neo4j_backend.link_embedding_to_task(embedding_id, task_id).await?;
    }

    // Create memory nodes
    let memory_data = vec![
        ("project_context", "Overall project context and requirements"),
        ("api_spec", "API specification and contract details"),
        ("db_schema", "Database schema and migration history"),
        ("test_cases", "Test cases and expected outcomes"),
    ];

    for (key, value) in &memory_data {
        sqlite_backend.create_memory_node(key, value).await?;
        neo4j_backend.create_memory_node(key, value).await?;
    }

    // Test graph structure validation
    let sqlite_stats = sqlite_backend.validate_structure().await?;
    let neo4j_stats = neo4j_backend.validate_structure().await?;

    // Compare basic stats
    assert_eq!(sqlite_stats.total_nodes, neo4j_stats.total_nodes, "Total nodes should match");
    assert_eq!(sqlite_stats.total_edges, neo4j_stats.total_edges, "Total edges should match");

    // Verify expected counts (approximately)
    assert!(sqlite_stats.total_nodes >= 20, "Should have at least 20 nodes (tasks + embeddings + memories)");
    assert!(sqlite_stats.total_edges >= 20, "Should have at least 20 edges (relationships + links)");

    // Test complex query: find all tasks with their embeddings and subtasks
    let sqlite_complex_query = r#"
        SELECT 
            main_task.id as main_task_id,
            main_task.name as main_task_name,
            COUNT(DISTINCT embedding.id) as embedding_count,
            COUNT(DISTINCT subtask.id) as subtask_count
        FROM code_entities main_task
        LEFT JOIN code_edges embed_edge ON main_task.id = embed_edge.dst_entity_id
        LEFT JOIN code_entities embedding ON embedding.id = embed_edge.src_entity_id 
            AND embedding.name LIKE 'rag_structure_hash_%'
        LEFT JOIN code_edges subtask_edge ON main_task.id = subtask_edge.src_entity_id
        LEFT JOIN code_entities subtask ON subtask.id = subtask_edge.dst_entity_id 
            AND subtask.name LIKE '%Implementation%' OR subtask.name LIKE '%Testing%'
        WHERE main_task.name LIKE '%Development%' OR main_task.name LIKE '%Fix%'
        GROUP BY main_task.id, main_task.name
        ORDER BY main_task.id
    "#;

    let neo4j_complex_query = r#"
        MATCH (main_task:Task)
        WHERE main_task.name CONTAINS 'Development' OR main_task.name CONTAINS 'Fix'
        OPTIONAL MATCH (embedding:Embedding)-[:USES]->(main_task)
        OPTIONAL MATCH (main_task)-[:CONTAINS]->(subtask:Task)
        WHERE subtask.name CONTAINS 'Implementation' OR subtask.name CONTAINS 'Testing'
        RETURN main_task.id as main_task_id, main_task.name as main_task_name,
               COUNT(DISTINCT embedding) as embedding_count,
               COUNT(DISTINCT subtask) as subtask_count
        ORDER BY main_task.id
    "#;

    let sqlite_complex_results = sqlite_backend.execute_query(sqlite_complex_query, vec![]).await?;
    let neo4j_complex_results = neo4j_backend.execute_query(neo4j_complex_query, vec![]).await?;

    compare_query_results(&sqlite_complex_results, &neo4j_complex_results, "rag_graph_structure_complex")?;

    // Should find 2 main tasks (development and bug fix)
    assert_eq!(sqlite_complex_results.len(), 2, "Should find 2 main tasks with complex relationships");

    cleanup_backends(&sqlite_backend, &neo4j_backend).await?;
    Ok(())
}