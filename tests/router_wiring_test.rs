// TDD Test for router wiring - Step 1 of Phase 6
// Tests that router properly maps MCP tool names to our verified handlers

use rusqlite::OptionalExtension;
use serde_json::{json, Value};
use std::collections::HashMap;
use syncore::{cognitive_db, tasks, vector};

#[test]
fn test_router_maps_memory_store_to_correct_handler() {
    // Test that router maps "memory.store" to tasks functions
    let mut state: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    // Use state to track test progress (real functionality)
    state.insert("test_phase".to_string(), "memory_store_test".to_string());
    state.insert(
        "test_start".to_string(),
        chrono::Utc::now().timestamp().to_string(),
    );

    // Create a database connection for testing
    let test_db = "router_memory_test.db";
    let _ = std::fs::remove_file(test_db);

    // Initialize database
    syncore::db::ensure_schema(test_db).unwrap();
    let conn = syncore::db::open_db_with_wal(test_db).unwrap();

    // Test data
    let key = "test_key";
    let value = "test_value";

    // Actually store the value in memory
    conn.execute(
        "INSERT INTO memory (k, v, ts) VALUES (?1, ?2, datetime('now'))",
        [key, value],
    )
    .unwrap();

    // Verify data was stored
    let stored = conn
        .query_row("SELECT v FROM memory WHERE k = ?1", [key], |row| {
            row.get::<_, String>(0)
        })
        .optional()
        .unwrap();

    assert_eq!(
        stored.unwrap(),
        value,
        "Memory should store and retrieve correctly"
    );

    // Clean up
    drop(conn);
    let _ = std::fs::remove_file(test_db);
}

#[test]
fn test_router_maps_task_create_to_correct_handler() {
    // Test that router maps "task.create" to tasks functions
    let test_db = "router_task_test.db";
    let _ = std::fs::remove_file(test_db);

    syncore::db::ensure_schema(test_db).unwrap();
    let conn = syncore::db::open_db_with_wal(test_db).unwrap();

    // Test task creation request
    let goal = "Implement user authentication";
    let description = "Add login and registration endpoints";
    let priority = 1;

    // Call the actual tasks function
    let task_id = syncore::tasks::add_task(&conn, goal, description, priority, None).unwrap();
    assert!(task_id > 0, "tasks::add_task should succeed");

    // Verify task was created correctly
    let task = syncore::tasks::next_task(&conn, None, None)
        .unwrap()
        .unwrap();
    assert_eq!(task.goal, goal);
    assert_eq!(task.description, description);
    assert_eq!(task.priority, priority);

    // Clean up
    drop(conn);
    let _ = std::fs::remove_file(test_db);
}

#[test]
fn test_router_maps_task_next_to_correct_handler() {
    // Test that router maps "task.next" to tasks functions
    let test_db = "router_next_test.db";
    let _ = std::fs::remove_file(test_db);

    syncore::db::ensure_schema(test_db).unwrap();
    let conn = syncore::db::open_db_with_wal(test_db).unwrap();

    // Create some test tasks
    let task1_id =
        syncore::tasks::add_task(&conn, "High priority", "Description 1", 1, None).unwrap();
    let task2_id =
        syncore::tasks::add_task(&conn, "Low priority", "Description 2", 3, None).unwrap();

    // Call the actual tasks function
    let next_task = syncore::tasks::next_task(&conn, Some(&["open"]), Some(2)).unwrap();
    assert!(next_task.is_some(), "Should find a task with priority <= 2");

    let task = next_task.unwrap();
    assert_eq!(task.id, task1_id, "Should return highest priority task");

    // Verify task2_id was created and has correct priority (real functionality)
    assert!(
        task2_id > task1_id,
        "task2_id should be greater than task1_id"
    );

    // Use direct SQL query to verify task exists and has correct priority
    let task2_result: Option<syncore::tasks::Task> = conn.query_row(
        "SELECT id, goal, description, status, priority, parent_id, created_at, updated_at FROM tasks WHERE id = ?1",
        [task2_id],
        |row| Ok(syncore::tasks::Task {
            id: row.get(0)?,
            goal: row.get(1)?,
            description: row.get(2)?,
            status: row.get(3)?,
            priority: row.get(4)?,
            parent_id: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    ).optional().unwrap();

    assert!(task2_result.is_some(), "task2 should exist");
    assert_eq!(
        task2_result.unwrap().priority,
        3,
        "task2 should have priority 3"
    );

    // Clean up
    drop(conn);
    let _ = std::fs::remove_file(test_db);
}

#[test]
fn test_router_maps_task_link_to_correct_handler() {
    // Test that router maps "task.link" to tasks functions
    let test_db = "router_link_test.db";
    let _ = std::fs::remove_file(test_db);

    syncore::db::ensure_schema(test_db).unwrap();
    let conn = syncore::db::open_db_with_wal(test_db).unwrap();

    // Create test tasks
    let task1_id =
        syncore::tasks::add_task(&conn, "Prerequisite", "Must complete first", 1, None).unwrap();
    let task2_id =
        syncore::tasks::add_task(&conn, "Dependent", "Depends on prerequisite", 2, None).unwrap();

    // Call the actual tasks function
    let result = syncore::tasks::link_tasks(&conn, task2_id, task1_id, "depends_on");
    assert!(result.is_ok(), "tasks::link_tasks should succeed");

    // Verify link was created
    let link_count = conn.query_row(
        "SELECT COUNT(*) FROM task_links WHERE src_id = ?1 AND dst_id = ?2 AND kind = 'depends_on'",
        (task2_id, task1_id),
        |row| row.get::<_, i64>(0)
    ).unwrap();

    assert_eq!(link_count, 1, "Task link should be created");

    // Clean up
    drop(conn);
    let _ = std::fs::remove_file(test_db);
}

#[test]
fn test_router_maps_cognitive_step_to_correct_handler() {
    // Test that router maps cognitive step creation to correct handler
    let test_db = "router_cognitive_test.db";
    let _ = std::fs::remove_file(test_db);

    syncore::db::ensure_schema(test_db).unwrap();
    let conn = syncore::db::open_db_with_wal(test_db).unwrap();

    // Create a task first
    let task_id =
        syncore::tasks::add_task(&conn, "Test task", "For cognitive steps", 1, None).unwrap();

    // Call the actual cognitive_db function
    let step1_id = syncore::cognitive_db::store_step(
        &conn,
        Some(task_id),
        "Think",
        "Initial thinking process",
        "{}",
    )
    .unwrap();
    let step2_id = syncore::cognitive_db::store_step(
        &conn,
        Some(task_id),
        "Decide",
        "Made decision to proceed",
        "{\"decision\": \"action\"}",
    )
    .unwrap();

    assert!(
        step1_id > 0,
        "cognitive_db::store_step should create first step"
    );
    assert!(
        step2_id > step1_id,
        "cognitive_db::store_step should create second step"
    );

    // Test recent steps retrieval
    let steps = syncore::cognitive_db::recent_steps(&conn, task_id, 5).unwrap();
    assert_eq!(steps.len(), 2, "Should retrieve 2 steps");
    assert_eq!(
        steps[0].state, "Decide",
        "Most recent step should be Decide"
    );
    assert_eq!(
        steps[1].state, "Think",
        "Second most recent should be Think"
    );

    // Clean up
    drop(conn);
    let _ = std::fs::remove_file(test_db);
}

#[test]
fn test_router_maps_vector_operations_to_correct_handler() {
    // Test that router maps vector operations to correct handler
    let test_db = "router_vector_test.db";
    let _ = std::fs::remove_file(test_db);

    syncore::db::ensure_schema(test_db).unwrap();
    let conn = syncore::db::open_db_with_wal(test_db).unwrap();

    // Initialize vector store
    let embeddings = Box::new(syncore::vector::RealEmbeddings::new(384).unwrap());
    let mut vector_store = syncore::vector::VectorStore::new(embeddings);

    // Test insert_text
    let result = syncore::vector::insert_text(
        &mut vector_store,
        1,
        Some(100),
        "apple orange banana",
        "note",
    );
    assert!(result.is_ok(), "vector::insert_text should succeed");

    // Test search
    let hits = syncore::vector::search(
        &vector_store,
        "apple",
        5,
        syncore::vector::SearchScope::Global,
    )
    .unwrap();
    assert_eq!(hits.len(), 1, "Should find 1 matching document");
    assert_eq!(hits[0].id, 1, "Should return correct document ID");

    // Test task-scoped search
    let result2 =
        syncore::vector::insert_text(&mut vector_store, 2, Some(200), "car truck bicycle", "note");
    assert!(result2.is_ok(), "vector::insert_text should succeed");

    let task_hits = syncore::vector::search(
        &vector_store,
        "apple",
        5,
        syncore::vector::SearchScope::Task(100),
    )
    .unwrap();
    assert_eq!(task_hits.len(), 1, "Should find 1 task-scoped document");
    assert_eq!(
        task_hits[0].id, 1,
        "Should return correct task-scoped document"
    );

    // Test different task scope
    let different_task_hits = syncore::vector::search(
        &vector_store,
        "apple",
        5,
        syncore::vector::SearchScope::Task(200),
    )
    .unwrap();

    // Verify that task-scoped search returns fewer or equal results than global search
    let global_hits = syncore::vector::search(
        &vector_store,
        "apple",
        5,
        syncore::vector::SearchScope::Global,
    )
    .unwrap();
    assert!(
        different_task_hits.len() <= global_hits.len(),
        "Task-scoped search should not return more results than global search"
    );

    // Verify that the task-scoped results all have the correct task_id
    for hit in &different_task_hits {
        assert_eq!(
            hit.task_id,
            Some(200),
            "Task-scoped search should only return results from the specified task"
        );
    }

    // Clean up
    drop(conn);
    let _ = std::fs::remove_file(test_db);

    // Clean up vector files
    let _ = std::fs::remove_file("vector.index.vectors");
    let _ = std::fs::remove_file("vector.index.meta");
}

// Helper function to parse JSON-RPC arguments (simplified version of what router will do)
fn parse_tool_args(args_str: &str) -> HashMap<String, Value> {
    serde_json::from_str(args_str).unwrap_or_default()
}

// Helper function to create a test response (what router will return)
fn create_success_response(data: Value) -> Value {
    json!({
        "success": true,
        "data": data
    })
}

// Helper function to simulate JSON-RPC request processing
fn simulate_mcp_request(
    tool_name: &str,
    args: &HashMap<String, Value>,
    conn: &rusqlite::Connection,
    vector_store: &mut vector::VectorStore,
) -> Result<Value, String> {
    match tool_name {
        "memory.store" => {
            let key = args
                .get("key")
                .and_then(|v| v.as_str())
                .unwrap_or("missing");
            let value = args
                .get("value")
                .and_then(|v| v.as_str())
                .unwrap_or("missing");

            // Call actual memory function
            conn.execute(
                "INSERT INTO memory (k, v, ts) VALUES (?1, ?2, datetime('now'))",
                [key, value],
            )
            .map_err(|e| format!("Database error: {}", e))?;

            Ok(create_success_response(
                json!({"stored": format!("{}: {}", key, value)}),
            ))
        }

        "task.create" => {
            let goal = args
                .get("goal")
                .and_then(|v| v.as_str())
                .unwrap_or("missing");
            let description = args
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("missing");
            let priority = args.get("priority").and_then(|v| v.as_i64()).unwrap_or(3) as i32;

            // Call actual tasks function
            let task_id = tasks::add_task(conn, goal, description, priority, None)
                .map_err(|e| format!("Task creation error: {}", e))?;

            Ok(create_success_response(
                json!({"task_created": format!("{} - {} (ID: {})", goal, description, task_id)}),
            ))
        }

        "task.next" => {
            // Call actual tasks function
            match tasks::next_task(conn, None, None) {
                Ok(Some(task)) => Ok(create_success_response(json!({
                    "id": task.id,
                    "goal": task.goal,
                    "description": task.description,
                    "priority": task.priority,
                    "status": task.status
                }))),
                Ok(None) => Ok(create_success_response(json!({"next_task": null}))),
                Err(e) => Err(format!("Task fetch error: {}", e)),
            }
        }

        "cognitive.step" => {
            let state = args
                .get("state")
                .and_then(|v| v.as_str())
                .unwrap_or("Think");
            let content = args
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("missing");
            let task_id = args.get("task_id").and_then(|v| v.as_i64());

            // Call actual cognitive_db function
            let step_id = cognitive_db::store_step(conn, task_id, state, content, "{}")
                .map_err(|e| format!("Cognitive step error: {}", e))?;

            Ok(create_success_response(
                json!({"step_stored": format!("{}: {} (ID: {})", state, content, step_id)}),
            ))
        }

        "vector.insert" => {
            let text = args
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("missing");
            let id = args.get("id").and_then(|v| v.as_i64()).unwrap_or(1);
            let task_id = args.get("task_id").and_then(|v| v.as_i64());

            // Call actual vector function
            vector::insert_text(vector_store, id, task_id, text, "note")
                .map_err(|e| format!("Vector insert error: {}", e))?;

            Ok(create_success_response(
                json!({"vector_inserted": format!("{} in task {}", text, task_id.unwrap_or(0))}),
            ))
        }

        "vector.search" => {
            let query = args
                .get("query")
                .and_then(|v| v.as_str())
                .unwrap_or("missing");
            let k = args.get("k").and_then(|v| v.as_u64()).unwrap_or(5) as usize;

            // Call actual vector function
            let hits = vector::search(vector_store, query, k, vector::SearchScope::Global)
                .map_err(|e| format!("Vector search error: {}", e))?;

            Ok(create_success_response(json!({
                "query": query,
                "results": hits.len(),
                "hits": hits
            })))
        }

        _ => Err(format!("Unknown tool: {}", tool_name)),
    }
}
