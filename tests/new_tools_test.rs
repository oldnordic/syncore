// Test new tools implementation
use serde_json::json;
use syncore::{memory, router, tasks, vector};

#[test]
fn test_vector_insert_tool() {
    let test_db = "test_vector_insert_tool.db";
    let _ = std::fs::remove_file(test_db);

    let memory = memory::Memory::new(test_db).unwrap();
    let tasks = tasks::Tasks::new(&format!("{}_tasks", test_db)).unwrap();
    let embeddings = Box::new(vector::RealEmbeddings::new(384).unwrap());
    let vector_store =
        std::sync::Arc::new(std::sync::Mutex::new(vector::VectorStore::new(embeddings)));

    let state = router::SynCoreState::new(memory, tasks, vector_store);

    // Test vector.insert tool
    let args = rmp_serde::to_vec(&(
        1i64,
        Some(2i64),
        "test document".to_string(),
        "note".to_string(),
    ))
    .unwrap();
    let result = router::route_tool("vector.insert", &args, &state);
    if let Err(e) = &result {
        println!("Error: {:?}", e);
    }
    assert!(result.is_ok(), "vector.insert should succeed: {:?}", result);

    let response_bytes = result.unwrap();
    let response: serde_json::Value = rmp_serde::from_slice(&response_bytes).unwrap();
    assert_eq!(response["success"], true);
    assert_eq!(response["id"], 1);
    assert_eq!(response["task_id"], 2);

    let _ = std::fs::remove_file(test_db);
}

#[test]
fn test_graph_link_tool() {
    let test_db = "test_graph_link_tool.db";
    let _ = std::fs::remove_file(test_db);

    let memory = memory::Memory::new(test_db).unwrap();
    let tasks = tasks::Tasks::new(&format!("{}_tasks", test_db)).unwrap();
    let embeddings = Box::new(vector::RealEmbeddings::new(384).unwrap());
    let vector_store =
        std::sync::Arc::new(std::sync::Mutex::new(vector::VectorStore::new(embeddings)));

    let state = router::SynCoreState::new(memory, tasks, vector_store);

    // Create some tasks first
    let task1_id = state
        .tasks
        .with_db(|db| tasks::add_task(db, "Task 1", "Description 1", 1, None))
        .unwrap();

    let task2_id = state
        .tasks
        .with_db(|db| tasks::add_task(db, "Task 2", "Description 2", 2, None))
        .unwrap();

    // Test graph.link tool
    let args = rmp_serde::to_vec(&(task1_id, task2_id, "depends_on".to_string())).unwrap();
    let result = router::route_tool("graph.link", &args, &state);
    assert!(result.is_ok(), "graph.link should succeed");

    let response_bytes = result.unwrap();
    let response: serde_json::Value = rmp_serde::from_slice(&response_bytes).unwrap();
    assert_eq!(response["success"], true);
    assert_eq!(response["src_id"], task1_id);
    assert_eq!(response["dst_id"], task2_id);
    assert_eq!(response["kind"], "depends_on");

    let _ = std::fs::remove_file(test_db);
}

#[test]
fn test_graph_query_tool() {
    let test_db = "test_graph_query_tool.db";
    let _ = std::fs::remove_file(test_db);

    let memory = memory::Memory::new(test_db).unwrap();
    let tasks = tasks::Tasks::new(&format!("{}_tasks", test_db)).unwrap();
    let embeddings = Box::new(vector::RealEmbeddings::new(384).unwrap());
    let vector_store =
        std::sync::Arc::new(std::sync::Mutex::new(vector::VectorStore::new(embeddings)));

    let state = router::SynCoreState::new(memory, tasks, vector_store);

    // Create some tasks and links first
    let task1_id = state
        .tasks
        .with_db(|db| tasks::add_task(db, "Task 1", "Description 1", 1, None))
        .unwrap();

    let task2_id = state
        .tasks
        .with_db(|db| tasks::add_task(db, "Task 2", "Description 2", 2, None))
        .unwrap();

    let task3_id = state
        .tasks
        .with_db(|db| tasks::add_task(db, "Task 3", "Description 3", 3, None))
        .unwrap();

    // Create links: task1 -> task2, task1 -> task3
    state
        .tasks
        .with_db(|db| {
            tasks::link_tasks(db, task1_id, task2_id, "depends_on").unwrap();
            tasks::link_tasks(db, task1_id, task3_id, "blocks").unwrap();
            Ok(())
        })
        .unwrap();

    // Test graph.query tool - outgoing
    let args = rmp_serde::to_vec(&(task1_id, "outgoing".to_string())).unwrap();
    let result = router::route_tool("graph.query", &args, &state);
    assert!(result.is_ok(), "graph.query should succeed");

    let response_bytes = result.unwrap();
    let response: serde_json::Value = rmp_serde::from_slice(&response_bytes).unwrap();
    assert_eq!(response["task_id"], task1_id);
    assert_eq!(response["direction"], "outgoing");
    assert_eq!(response["links"].as_array().unwrap().len(), 2);

    let _ = std::fs::remove_file(test_db);
}
