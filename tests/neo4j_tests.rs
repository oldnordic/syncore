//! TDD Tests for Neo4j Graph Integration
//!
//! IMPORTANT: These tests require a running Neo4j instance at localhost:7687
//! Default credentials: neo4j/testpassword123
//!
//! NO MOCKS, NO STUBS - REAL DATABASE OPERATIONS ONLY

use anyhow::Result;
use std::sync::Arc;

// Test 1: Connects to Neo4j
#[tokio::test]
async fn test_connects_to_neo4j() -> Result<()> {
    use syncore::graph::neo4j_client::Neo4jClient;

    let client = Neo4jClient::connect(
        "bolt://localhost:7687",
        "neo4j",
        "testpassword123",
    )
    .await?;

    // Verify connection by running a simple query
    let result = client.execute_query("RETURN 1 as n", vec![]).await?;
    assert!(!result.is_empty());
    assert_eq!(result[0]["n"], serde_json::json!(1));

    Ok(())
}

// Test 2: Creates task node
#[tokio::test]
async fn test_creates_task_node() -> Result<()> {
    use syncore::graph::neo4j_client::Neo4jClient;

    let client = Neo4jClient::connect(
        "bolt://localhost:7687",
        "neo4j",
        "testpassword123",
    )
    .await?;

    // Create a task node
    let task_id = 42i64;
    let title = "Test Task";
    let status = "pending";

    client
        .create_task_node(task_id, title, status)
        .await?;

    // Verify the node exists
    let query = "MATCH (t:Task {id: $id}) RETURN t.id as id, t.title as title, t.status as status";
    let params = vec![("id", serde_json::json!(task_id))];
    let result = client.execute_query(query, params).await?;

    assert_eq!(result.len(), 1);
    assert_eq!(result[0]["id"], serde_json::json!(task_id));
    assert_eq!(result[0]["title"], serde_json::json!(title));
    assert_eq!(result[0]["status"], serde_json::json!(status));

    // Cleanup
    client
        .execute_query("MATCH (t:Task {id: $id}) DELETE t", vec![("id", serde_json::json!(task_id))])
        .await?;

    Ok(())
}

// Test 3: Creates subtask relationship
#[tokio::test]
async fn test_creates_subtask_relationship() -> Result<()> {
    use syncore::graph::neo4j_client::Neo4jClient;

    let client = Neo4jClient::connect(
        "bolt://localhost:7687",
        "neo4j",
        "testpassword123",
    )
    .await?;

    // Create parent and child tasks
    let parent_id = 100i64;
    let child_id = 101i64;

    client.create_task_node(parent_id, "Parent Task", "pending").await?;
    client.create_task_node(child_id, "Child Task", "pending").await?;

    // Create relationship
    client.create_subtask_relationship(parent_id, child_id).await?;

    // Verify relationship exists
    let query = r#"
        MATCH (p:Task {id: $parent_id})-[:HAS_SUBTASK]->(c:Task {id: $child_id})
        RETURN p.id as parent, c.id as child
    "#;
    let params = vec![
        ("parent_id", serde_json::json!(parent_id)),
        ("child_id", serde_json::json!(child_id)),
    ];
    let result = client.execute_query(query, params).await?;

    assert_eq!(result.len(), 1);
    assert_eq!(result[0]["parent"], serde_json::json!(parent_id));
    assert_eq!(result[0]["child"], serde_json::json!(child_id));

    // Cleanup
    client
        .execute_query(
            "MATCH (t:Task) WHERE t.id IN [$p, $c] DETACH DELETE t",
            vec![
                ("p", serde_json::json!(parent_id)),
                ("c", serde_json::json!(child_id)),
            ],
        )
        .await?;

    Ok(())
}

// Test 4: Creates memory node
#[tokio::test]
async fn test_memory_node_creation() -> Result<()> {
    use syncore::graph::neo4j_client::Neo4jClient;

    let client = Neo4jClient::connect(
        "bolt://localhost:7687",
        "neo4j",
        "testpassword123",
    )
    .await?;

    let key = "test_memory_key";
    let value = "test_memory_value";

    client.create_memory_node(key, value).await?;

    // Verify the node exists
    let query = "MATCH (m:Memory {key: $key}) RETURN m.key as key, m.value as value";
    let params = vec![("key", serde_json::json!(key))];
    let result = client.execute_query(query, params).await?;

    assert_eq!(result.len(), 1);
    assert_eq!(result[0]["key"], serde_json::json!(key));
    assert_eq!(result[0]["value"], serde_json::json!(value));

    // Cleanup
    client
        .execute_query("MATCH (m:Memory {key: $key}) DELETE m", vec![("key", serde_json::json!(key))])
        .await?;

    Ok(())
}

// Test 5: Creates embedding node
#[tokio::test]
async fn test_embedding_node_creation() -> Result<()> {
    use syncore::graph::neo4j_client::Neo4jClient;

    let client = Neo4jClient::connect(
        "bolt://localhost:7687",
        "neo4j",
        "testpassword123",
    )
    .await?;

    let id = 999i64;
    let text = "This is an embedding text";
    let hash = "abc123def456";

    client.create_embedding_node(id, text, hash).await?;

    // Verify the node exists
    let query = "MATCH (e:Embedding {id: $id}) RETURN e.id as id, e.text as text, e.hash as hash";
    let params = vec![("id", serde_json::json!(id))];
    let result = client.execute_query(query, params).await?;

    assert_eq!(result.len(), 1);
    assert_eq!(result[0]["id"], serde_json::json!(id));
    assert_eq!(result[0]["text"], serde_json::json!(text));
    assert_eq!(result[0]["hash"], serde_json::json!(hash));

    // Cleanup
    client
        .execute_query("MATCH (e:Embedding {id: $id}) DELETE e", vec![("id", serde_json::json!(id))])
        .await?;

    Ok(())
}

// Test 6: Reads graph nodes
#[tokio::test]
async fn test_reading_graph_nodes() -> Result<()> {
    use syncore::graph::neo4j_client::Neo4jClient;

    let client = Neo4jClient::connect(
        "bolt://localhost:7687",
        "neo4j",
        "testpassword123",
    )
    .await?;

    // Create multiple tasks
    for i in 200..203 {
        client.create_task_node(i, &format!("Task {}", i), "pending").await?;
    }

    // Read all tasks in range
    let query = "MATCH (t:Task) WHERE t.id >= 200 AND t.id < 203 RETURN t.id as id ORDER BY t.id";
    let result = client.execute_query(query, vec![]).await?;

    assert_eq!(result.len(), 3);
    assert_eq!(result[0]["id"], serde_json::json!(200));
    assert_eq!(result[1]["id"], serde_json::json!(201));
    assert_eq!(result[2]["id"], serde_json::json!(202));

    // Cleanup
    client
        .execute_query("MATCH (t:Task) WHERE t.id >= 200 AND t.id < 203 DELETE t", vec![])
        .await?;

    Ok(())
}

// Test 7: Reads graph relationships
#[tokio::test]
async fn test_reading_graph_relationships() -> Result<()> {
    use syncore::graph::neo4j_client::Neo4jClient;

    let client = Neo4jClient::connect(
        "bolt://localhost:7687",
        "neo4j",
        "testpassword123",
    )
    .await?;

    // Create a chain: Task 300 -> Task 301 -> Task 302
    client.create_task_node(300, "Root Task", "pending").await?;
    client.create_task_node(301, "Middle Task", "pending").await?;
    client.create_task_node(302, "Leaf Task", "pending").await?;
    client.create_subtask_relationship(300, 301).await?;
    client.create_subtask_relationship(301, 302).await?;

    // Query the chain
    let query = r#"
        MATCH path = (root:Task {id: 300})-[:HAS_SUBTASK*]->(leaf)
        RETURN [node IN nodes(path) | node.id] as chain
    "#;
    let result = client.execute_query(query, vec![]).await?;

    assert!(result.len() >= 2); // At least two paths: 300->301, 300->301->302

    // Cleanup
    client
        .execute_query("MATCH (t:Task) WHERE t.id IN [300, 301, 302] DETACH DELETE t", vec![])
        .await?;

    Ok(())
}

// Test 8: SQLite and Neo4j dual-write consistency
#[tokio::test]
async fn test_sqlite_and_neo4j_dual_write_consistency() -> Result<()> {
    use syncore::graph::neo4j_client::Neo4jClient;
    use syncore::router::SynCoreState;
    use syncore::memory::Memory;
    use syncore::tasks::Tasks;
    use syncore::vector::{VectorStore, RealEmbeddings};
    use std::sync::Mutex;

    // Setup state with Neo4j
    let id = std::process::id();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mem_path = format!("/tmp/syncore_neo4j_test_mem_{}_{}.db", id, ts);
    let task_path = format!("/tmp/syncore_neo4j_test_task_{}_{}.db", id, ts);

    let memory = Memory::new(&mem_path)?;
    let tasks = Tasks::new(&task_path)?;
    let embeddings = Box::new(RealEmbeddings::new(384)?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    let neo4j_client = Neo4jClient::connect(
        "bolt://localhost:7687",
        "neo4j",
        "testpassword123",
    )
    .await?;

    let state = SynCoreState::new(memory, tasks, vector_store)
        .with_neo4j(Arc::new(neo4j_client));

    // Create task in SQLite
    let task_id = state.tasks.add_task("Dual Write Test", "Testing consistency", 1, None)?;

    // Sync to Neo4j
    if let Some(neo4j) = &state.neo4j {
        neo4j.create_task_node(task_id, "Dual Write Test", "pending").await?;
    }

    // Verify both stores have the data
    // SQLite check
    let sqlite_task = state.tasks.get_task(task_id)?.expect("Task should exist");
    assert_eq!(sqlite_task.goal, "Dual Write Test");

    // Neo4j check
    if let Some(neo4j) = &state.neo4j {
        let query = "MATCH (t:Task {id: $id}) RETURN t.title as title";
        let result = neo4j.execute_query(query, vec![("id", serde_json::json!(task_id))]).await?;
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["title"], serde_json::json!("Dual Write Test"));

        // Cleanup Neo4j
        neo4j.execute_query("MATCH (t:Task {id: $id}) DELETE t", vec![("id", serde_json::json!(task_id))]).await?;
    }

    Ok(())
}

// Test 9: Neo4j connection pooling
#[tokio::test]
async fn test_neo4j_connection_pooling() -> Result<()> {
    use syncore::graph::neo4j_client::Neo4jClient;
    use tokio::task::JoinSet;

    let client = Arc::new(
        Neo4jClient::connect("bolt://localhost:7687", "neo4j", "testpassword123").await?,
    );

    // Spawn multiple concurrent queries
    let mut set = JoinSet::new();
    for i in 0..10 {
        let client_clone = client.clone();
        set.spawn(async move {
            let query = "RETURN $i as i";
            let result = client_clone
                .execute_query(query, vec![("i", serde_json::json!(i))])
                .await
                .unwrap();
            assert_eq!(result[0]["i"], serde_json::json!(i));
        });
    }

    // Wait for all queries to complete
    while let Some(result) = set.join_next().await {
        result?;
    }

    Ok(())
}

// Test 10: Zero-copy query execution (borrowed strings)
#[tokio::test]
async fn test_zero_copy_query_execution() -> Result<()> {
    use syncore::graph::neo4j_client::Neo4jClient;

    let client = Neo4jClient::connect(
        "bolt://localhost:7687",
        "neo4j",
        "testpassword123",
    )
    .await?;

    // Create test data with borrowed string slice (no allocation)
    let query_str: &str = "RETURN 'borrowed' as text, 42 as num";
    let result = client.execute_query(query_str, vec![]).await?;

    assert_eq!(result[0]["text"], serde_json::json!("borrowed"));
    assert_eq!(result[0]["num"], serde_json::json!(42));

    Ok(())
}
