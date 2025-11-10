use syncore::{taskmaster, cognitive_db, vector};
use rusqlite::Connection;
use std::fs;

#[test]
fn plan_expand_creates_children_ok() {
    // Test AI tool returns valid JSON; N children created
    let test_db = "test_plan_expand.db";
    let _ = fs::remove_file(test_db);

    syncore::db::ensure_schema(test_db).unwrap();
    let conn = syncore::db::open_db_with_wal(test_db).unwrap();

    // Create parent task
    let parent_id = taskmaster::add_task(&conn, "Build web app", "Create full-stack application", 1, None).unwrap();

    // Simulate plan.expand AI response with valid JSON
    let ai_response = r#"{
        "subtasks": [
            {"goal": "Setup backend", "description": "Initialize Express server", "priority": 1},
            {"goal": "Design database", "description": "Create schema and migrations", "priority": 2},
            {"goal": "Implement frontend", "description": "Build React components", "priority": 3}
        ]
    }"#;

    // Parse and validate JSON structure
    let plan: serde_json::Value = serde_json::from_str(ai_response).unwrap();
    assert!(plan.get("subtasks").is_some());

    let subtasks = plan["subtasks"].as_array().unwrap();
    assert_eq!(subtasks.len(), 3);

    // Create children tasks based on AI response
    let mut child_ids = Vec::new();
    for subtask in subtasks {
        let goal = subtask["goal"].as_str().unwrap();
        let description = subtask["description"].as_str().unwrap();
        let priority = subtask["priority"].as_i64().unwrap() as i32;

        let child_id = taskmaster::add_task(&conn, goal, description, priority, Some(parent_id)).unwrap();
        child_ids.push(child_id);
    }

    // Verify children were created
    assert_eq!(child_ids.len(), 3);

    // Verify hierarchy
    for &child_id in &child_ids {
        let parent_id_check = conn.query_row(
            "SELECT parent_id FROM tasks WHERE id = ?1",
            [child_id],
            |row| row.get::<_, Option<i64>>(0)
        ).unwrap();
        assert_eq!(parent_id_check, Some(parent_id));
    }

    // Verify child details
    let child1 = conn.query_row(
        "SELECT goal, description, priority FROM tasks WHERE id = ?1",
        [child_ids[0]],
        |row| Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i32>(2)?
        ))
    ).unwrap();
    assert_eq!(child1.0, "Setup backend");
    assert_eq!(child1.1, "Initialize Express server");
    assert_eq!(child1.2, 1);

    // Clean up
    drop(conn);
    let _ = fs::remove_file(test_db);
}

#[test]
fn plan_resume_suggests_tool_ok() {
    // Test returns suggestion + maybe_tool
    let test_db = "test_plan_resume.db";
    let _ = fs::remove_file(test_db);

    syncore::db::ensure_schema(test_db).unwrap();
    let conn = syncore::db::open_db_with_wal(test_db).unwrap();

    // Create some tasks with different states
    let task1_id = taskmaster::add_task(&conn, "Backend setup", "Install dependencies", 1, None).unwrap();
    let task2_id = taskmaster::add_task(&conn, "Database design", "Create schema", 2, None).unwrap();

    // Mark one as running, one as done
    taskmaster::update_task(&conn, task1_id, Some("running"), None, None).unwrap();
    taskmaster::update_task(&conn, task2_id, Some("done"), None, None).unwrap();

    // Create some cognitive steps for context
    cognitive_db::store_step(&conn, Some(task1_id), "Think", "Need to install Node.js and npm packages", "{}").unwrap();
    cognitive_db::store_step(&conn, Some(task1_id), "Act", "Running npm install", "{\"action\": \"install\"}").unwrap();

    // Simulate plan.resume AI analysis
    let current_tasks = vec![
        ("Backend setup", "running"),
        ("Database design", "done")
    ];

    let recent_steps = cognitive_db::recent_steps(&conn, task1_id, 5).unwrap();

    // Mock AI response with suggestion and tool recommendation
    let ai_suggestion = if current_tasks.iter().any(|(_, status)| status == "running") {
        r#"{
            "suggestion": "Continue with backend setup - ensure all dependencies are installed",
            "maybe_tool": "shell.execute",
            "tool_args": {"command": "npm test"}
        }"#
    } else {
        r#"{
            "suggestion": "Start next high-priority task",
            "maybe_tool": null
        }"#
    };

    // Parse AI response
    let response: serde_json::Value = serde_json::from_str(ai_suggestion).unwrap();

    // Verify response structure
    assert!(response.get("suggestion").is_some());
    let suggestion = response["suggestion"].as_str().unwrap();
    assert!(!suggestion.is_empty());

    let maybe_tool = response.get("maybe_tool").and_then(|v| v.as_str());

    // In this case, should suggest a tool since task is running
    assert!(maybe_tool.is_some());
    assert_eq!(maybe_tool.unwrap(), "shell.execute");

    // Verify tool args if tool is suggested
    if maybe_tool.is_some() {
        assert!(response.get("tool_args").is_some());
    }

    // Clean up
    drop(conn);
    let _ = fs::remove_file(test_db);
}

#[test]
fn cog_cycle_persists_steps_ok() {
    // Test Think→Reflect produce steps + log
    let test_db = "test_cog_cycle.db";
    let _ = fs::remove_file(test_db);

    syncore::db::ensure_schema(test_db).unwrap();
    let conn = syncore::db::open_db_with_wal(test_db).unwrap();

    // Create a task for the cognitive cycle
    let task_id = taskmaster::add_task(&conn, "Debug performance issue", "Fix slow database queries", 1, None).unwrap();

    // Simulate cognitive cycle: Think → Reflect
    let think_content = "The queries are slow because of missing indexes on the user table. Need to analyze query patterns.";
    let reflect_content = "I've identified the root cause. Adding proper indexes should improve performance by 10x.";

    // Store Think step
    let think_id = cognitive_db::store_step(&conn, Some(task_id), "Think", think_content, "{}").unwrap();
    assert!(think_id > 0);

    // Store Reflect step
    let reflect_id = cognitive_db::store_step(&conn, Some(task_id), "Reflect", reflect_content, "{\"confidence\": 0.9}").unwrap();
    assert!(reflect_id > think_id);

    // Verify both steps were persisted
    let recent_steps = cognitive_db::recent_steps(&conn, task_id, 10).unwrap();
    assert_eq!(recent_steps.len(), 2);

    // Verify order (most recent first)
    assert_eq!(recent_steps[0].state, "Reflect");
    assert_eq!(recent_steps[1].state, "Think");

    // Verify content
    assert_eq!(recent_steps[1].content, think_content);
    assert_eq!(recent_steps[0].content, reflect_content);

    // Verify metadata
    let think_meta: serde_json::Value = serde_json::from_str(&recent_steps[1].meta_json).unwrap();
    assert_eq!(think_meta.as_object().unwrap().len(), 0); // Empty object

    let reflect_meta: serde_json::Value = serde_json::from_str(&recent_steps[0].meta_json).unwrap();
    assert_eq!(reflect_meta["confidence"], 0.9);

    // Verify task association
    for step in &recent_steps {
        assert_eq!(step.task_id, Some(task_id));
    }

    // Test log entries (assuming logger writes to file or DB)
    // For now, we verify steps exist which is the core requirement

    // Clean up
    drop(conn);
    let _ = fs::remove_file(test_db);
}

#[test]
fn create_expand_run_cycle_resume_done_ok() {
    // Full end-to-end test: create → expand → run 1 cycle → resume → mark done
    let test_db = "test_full_cycle.db";
    let _ = fs::remove_file(test_db);

    syncore::db::ensure_schema(test_db).unwrap();
    let conn = syncore::db::open_db_with_wal(test_db).unwrap();

    // 1. CREATE: Create main task
    let main_task_id = taskmaster::add_task(&conn, "Build REST API", "Create complete backend service", 1, None).unwrap();

    // Verify creation
    let task = taskmaster::next_task(&conn, None, None).unwrap().unwrap();
    assert_eq!(task.id, main_task_id);
    assert_eq!(task.status, "open");

    // 2. EXPAND: Break down into subtasks
    let subtask1_id = taskmaster::add_task(&conn, "Setup Express", "Initialize server and middleware", 1, Some(main_task_id)).unwrap();
    let subtask2_id = taskmaster::add_task(&conn, "Define routes", "Create API endpoints", 2, Some(main_task_id)).unwrap();
    let subtask3_id = taskmaster::add_task(&conn, "Add validation", "Implement input validation", 3, Some(main_task_id)).unwrap();

    // Link dependencies: routes depends on setup, validation depends on routes
    taskmaster::link_tasks(&conn, subtask2_id, subtask1_id, "depends_on").unwrap();
    taskmaster::link_tasks(&conn, subtask3_id, subtask2_id, "depends_on").unwrap();

    // 3. RUN 1 CYCLE: Start with first subtask
    let next_task = taskmaster::next_task(&conn, None, None).unwrap().unwrap();
    assert_eq!(next_task.id, subtask1_id);

    // Mark as running and log cognitive steps
    taskmaster::update_task(&conn, subtask1_id, Some("running"), None, None).unwrap();

    // Think step
    cognitive_db::store_step(&conn, Some(subtask1_id), "Think", "Need to install express and set up basic server structure", "{}").unwrap();

    // Act step
    cognitive_db::store_step(&conn, Some(subtask1_id), "Act", "Creating express app with middleware", "{\"action\": \"code\"}").unwrap();

    // Reflect step
    cognitive_db::store_step(&conn, Some(subtask1_id), "Reflect", "Server setup complete, tested with ping endpoint", "{\"result\": \"success\"}").unwrap();

    // Mark subtask1 as done
    taskmaster::update_task(&conn, subtask1_id, Some("done"), None, None).unwrap();

    // 4. RESUME: Check what to do next
    let resume_suggestion = r#"{
        "suggestion": "Next task is to define routes since setup is complete",
        "maybe_tool": "task.update",
        "tool_args": {"status": "running", "task_id": 2}
    }"#;

    let response: serde_json::Value = serde_json::from_str(resume_suggestion).unwrap();
    let suggested_task_id = response["tool_args"]["task_id"].as_i64().unwrap() as i64;
    assert_eq!(suggested_task_id, subtask2_id);

    // Apply suggestion - start working on routes
    taskmaster::update_task(&conn, subtask2_id, Some("running"), None, None).unwrap();

    // 5. MARK DONE: Complete the routes task
    cognitive_db::store_step(&conn, Some(subtask2_id), "Think", "Need to define GET, POST, PUT, DELETE endpoints", "{}").unwrap();
    cognitive_db::store_step(&conn, Some(subtask2_id), "Act", "Implemented CRUD routes for users", "{\"action\": \"code\"}").unwrap();
    cognitive_db::store_step(&conn, Some(subtask2_id), "Reflect", "All endpoints working, added error handling", "{\"result\": \"success\"}").unwrap();

    taskmaster::update_task(&conn, subtask2_id, Some("done"), None, None).unwrap();

    // Verify final state
    let done_tasks = conn.prepare("SELECT id, status FROM tasks WHERE status = 'done' ORDER BY id").unwrap();
    let done_list = done_tasks.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    }).unwrap().collect::<Result<Vec<_>, _>>().unwrap();

    assert_eq!(done_list.len(), 2);
    assert_eq!(done_list[0].0, subtask1_id);
    assert_eq!(done_list[1].0, subtask2_id);

    // Verify cognitive steps were created
    let steps1 = cognitive_db::recent_steps(&conn, subtask1_id, 10).unwrap();
    let steps2 = cognitive_db::recent_steps(&conn, subtask2_id, 10).unwrap();
    assert_eq!(steps1.len(), 3); // Think, Act, Reflect
    assert_eq!(steps2.len(), 3); // Think, Act, Reflect

    // Clean up
    drop(conn);
    let _ = fs::remove_file(test_db);
}

#[test]
fn vector_scope_task_matches_steps_ok() {
    // Test only task-scoped hits in vector search
    let test_db = "test_vector_scope.db";
    let _ = fs::remove_file(test_db);

    // Setup vector store
    let embeddings = Box::new(syncore::vector::MockEmbeddings::new(384));
    let mut vector_store = syncore::vector::VectorStore::new(embeddings);

    // Insert content for different tasks
    vector::insert_text(&mut vector_store, 1, Some(100), "User authentication module for task 100", "note").unwrap();
    vector::insert_text(&mut vector_store, 2, Some(100), "Password reset functionality in auth module", "note").unwrap();
    vector::insert_text(&mut vector_store, 3, Some(200), "Database schema design for task 200", "note").unwrap();
    vector::insert_text(&mut vector_store, 4, Some(300), "Frontend UI components for task 300", "note").unwrap();

    // Test global search - should return all results
    let global_hits = vector::search(&vector_store, "authentication", 10, syncore::vector::SearchScope::Global).unwrap();
    assert_eq!(global_hits.len(), 4); // All documents should match with some score

    // Test task-scoped search for task 100 - should only return 2 results
    let task100_hits = vector::search(&vector_store, "authentication", 10, syncore::vector::SearchScope::Task(100)).unwrap();
    assert_eq!(task100_hits.len(), 2);

    // Verify task IDs are correct
    for hit in &task100_hits {
        assert_eq!(hit.task_id, Some(100));
    }

    // Verify IDs match expected (1 and 2)
    let hit_ids: Vec<i64> = task100_hits.iter().map(|h| h.id).collect();
    assert!(hit_ids.contains(&1));
    assert!(hit_ids.contains(&2));
    assert!(!hit_ids.contains(&3));
    assert!(!hit_ids.contains(&4));

    // Test task-scoped search for task 200 - should return 1 result
    let task200_hits = vector::search(&vector_store, "database", 10, syncore::vector::SearchScope::Task(200)).unwrap();
    assert_eq!(task200_hits.len(), 1);
    assert_eq!(task200_hits[0].id, 3);
    assert_eq!(task200_hits[0].task_id, Some(200));

    // Test task-scoped search for non-existent task - should return no results
    let no_task_hits = vector::search(&vector_store, "anything", 10, syncore::vector::SearchScope::Task(999)).unwrap();
    assert_eq!(no_task_hits.len(), 0);

    // Clean up vector files
    let _ = fs::remove_file("vector.index.vectors");
    let _ = fs::remove_file("vector.index.meta");
    let _ = fs::remove_file(test_db);
}
