use rusqlite::Connection;

use syncore::{cognitive_db, tasks};

#[test]
fn mcp_create_task_tool_ok() {
    // Test MCP tool for creating tasks
    let test_db = "test_mcp_create.db";
    let _ = fs::remove_file(test_db);

    syncore::db::ensure_schema(test_db).unwrap();
    let conn = syncore::db::open_db_with_wal(test_db).unwrap();

    // Simulate MCP tool call to create task
    let goal = "Implement vector search";
    let description = "Add HNSW indexing for efficient similarity search";
    let priority = 2;
    let parent = None;

    let task_id = tasks::add_task(&conn, goal, description, priority, parent).unwrap();
    assert!(task_id > 0);

    // Verify task was created correctly
    let task = tasks::next_task(&conn, None, None).unwrap().unwrap();
    assert_eq!(task.id, task_id);
    assert_eq!(task.goal, goal);
    assert_eq!(task.description, description);
    assert_eq!(task.priority, priority);
    assert_eq!(task.status, "open");
    assert!(task.parent_id.is_none());

    // Clean up
    drop(conn);
    let _ = fs::remove_file(test_db);
}

#[test]
fn mcp_update_task_tool_ok() {
    // Test MCP tool for updating tasks
    let test_db = "test_mcp_update.db";
    let _ = fs::remove_file(test_db);

    syncore::db::ensure_schema(test_db).unwrap();
    let conn = syncore::db::open_db_with_wal(test_db).unwrap();

    // Create a task first
    let task_id = tasks::add_task(&conn, "Original task", "Original description", 3, None).unwrap();

    // Simulate MCP tool call to update task status
    tasks::update_task(&conn, task_id, Some("running"), None, None).unwrap();

    // Verify status update
    let status = conn
        .query_row("SELECT status FROM tasks WHERE id = ?1", [task_id], |row| {
            row.get::<_, String>(0)
        })
        .unwrap();
    assert_eq!(status, "running");

    // Update description and priority
    tasks::update_task(&conn, task_id, None, Some(1), Some("Updated description")).unwrap();

    // Verify updates
    let updated = conn
        .query_row("SELECT priority, description FROM tasks WHERE id = ?1", [task_id], |row| {
            Ok((row.get::<_, i32>(0)?, row.get::<_, String>(1)?))
        })
        .unwrap();
    assert_eq!(updated.0, 1);
    assert_eq!(updated.1, "Updated description");

    // Clean up
    drop(conn);
    let _ = fs::remove_file(test_db);
}

#[test]
fn mcp_next_task_tool_ok() {
    // Test MCP tool for getting next task
    let test_db = "test_mcp_next.db";
    let _ = fs::remove_file(test_db);

    syncore::db::ensure_schema(test_db).unwrap();
    let conn = syncore::db::open_db_with_wal(test_db).unwrap();

    // Create multiple tasks with different priorities
    let _low_id = tasks::add_task(&conn, "Low priority", "Description", 5, None).unwrap();
    let high_id = tasks::add_task(&conn, "High priority", "Description", 1, None).unwrap();
    let medium_id = tasks::add_task(&conn, "Medium priority", "Description", 3, None).unwrap();

    // Test basic next_task (should return highest priority)
    let next = tasks::next_task(&conn, None, None).unwrap();
    assert!(next.is_some());
    assert_eq!(next.unwrap().id, high_id);

    // Test with status filter
    let open_tasks = tasks::next_task(&conn, Some(&["open"]), None).unwrap();
    assert!(open_tasks.is_some());

    // Mark high priority as done
    tasks::update_task(&conn, high_id, Some("done"), None, None).unwrap();

    // Should now return medium priority
    let next = tasks::next_task(&conn, None, None).unwrap();
    assert!(next.is_some());
    assert_eq!(next.unwrap().id, medium_id);

    // Test with min_prio filter - should return no results since medium and low both > 2
    let high_prio_only = tasks::next_task(&conn, None, Some(2)).unwrap();
    assert!(high_prio_only.is_none());

    // Test with priority too high
    let no_results = tasks::next_task(&conn, None, Some(0)).unwrap();
    assert!(no_results.is_none());

    // Clean up
    drop(conn);
    let _ = fs::remove_file(test_db);
}

#[test]
fn mcp_link_tasks_tool_ok() {
    // Test MCP tool for linking tasks
    let test_db = "test_mcp_link.db";
    let _ = fs::remove_file(test_db);

    syncore::db::ensure_schema(test_db).unwrap();
    let conn = syncore::db::open_db_with_wal(test_db).unwrap();

    // Create two tasks
    let task_a_id = tasks::add_task(&conn, "Task A", "Prerequisite", 1, None).unwrap();
    let task_b_id = tasks::add_task(&conn, "Task B", "Dependent", 2, None).unwrap();

    // Test depends_on link
    tasks::link_tasks(&conn, task_b_id, task_a_id, "depends_on").unwrap();

    // Verify link exists
    let link_exists = conn.query_row(
        "SELECT COUNT(*) FROM task_links WHERE src_id = ?1 AND dst_id = ?2 AND kind = 'depends_on'",
        (task_b_id, task_a_id),
        |row| row.get::<_, i64>(0)
    ).unwrap();
    assert_eq!(link_exists, 1);

    // Test relates_to link
    tasks::link_tasks(&conn, task_a_id, task_b_id, "relates_to").unwrap();

    // Verify second link
    let relates_link = conn.query_row(
        "SELECT COUNT(*) FROM task_links WHERE src_id = ?1 AND dst_id = ?2 AND kind = 'relates_to'",
        (task_a_id, task_b_id),
        |row| row.get::<_, i64>(0)
    ).unwrap();
    assert_eq!(relates_link, 1);

    // Test link replacement (should replace existing depends_on)
    tasks::link_tasks(&conn, task_b_id, task_a_id, "depends_on").unwrap();

    // Should still only have one depends_on link
    let depends_count = conn.query_row(
        "SELECT COUNT(*) FROM task_links WHERE src_id = ?1 AND dst_id = ?2 AND kind = 'depends_on'",
        (task_b_id, task_a_id),
        |row| row.get::<_, i64>(0)
    ).unwrap();
    assert_eq!(depends_count, 1);

    // Clean up
    drop(conn);
    let _ = fs::remove_file(test_db);
}

#[test]
fn mcp_cognitive_step_tool_ok() {
    // Test MCP tool for cognitive steps
    let test_db = "test_mcp_cognitive.db";
    let _ = fs::remove_file(test_db);

    syncore::db::ensure_schema(test_db).unwrap();
    let conn = syncore::db::open_db_with_wal(test_db).unwrap();

    // Create a task
    let task_id = tasks::add_task(&conn, "Test task", "For cognitive steps", 1, None).unwrap();

    // Test storing different step types
    let think_id = cognitive_db::store_step(
        &conn,
        Some(task_id),
        "Think",
        "Analyzing the problem",
        "{\"context\": \"initial\"}",
    )
    .unwrap();
    let decide_id = cognitive_db::store_step(
        &conn,
        Some(task_id),
        "Decide",
        "Choosing approach",
        "{\"decision\": \"use_algorithm_x\"}",
    )
    .unwrap();
    let act_id = cognitive_db::store_step(
        &conn,
        Some(task_id),
        "Act",
        "Implementing solution",
        "{\"action\": \"code_written\"}",
    )
    .unwrap();

    // Verify all steps were created
    assert!(think_id > 0);
    assert!(decide_id > think_id);
    assert!(act_id > decide_id);

    // Test retrieving recent steps
    let recent = cognitive_db::recent_steps(&conn, task_id, 3).unwrap();
    assert_eq!(recent.len(), 3);

    // Should be in reverse chronological order
    assert_eq!(recent[0].state, "Act");
    assert_eq!(recent[1].state, "Decide");
    assert_eq!(recent[2].state, "Think");

    // Verify metadata is preserved
    let think_meta: serde_json::Value = serde_json::from_str(&recent[2].meta_json).unwrap();
    assert_eq!(think_meta["context"], "initial");

    let decide_meta: serde_json::Value = serde_json::from_str(&recent[1].meta_json).unwrap();
    assert_eq!(decide_meta["decision"], "use_algorithm_x");

    let act_meta: serde_json::Value = serde_json::from_str(&recent[0].meta_json).unwrap();
    assert_eq!(act_meta["action"], "code_written");

    // Test steps without task association
    let global_step_id =
        cognitive_db::store_step(&conn, None, "Think", "Global thinking session", "{}").unwrap();
    assert!(global_step_id > act_id); // Should be later

    // Clean up
    drop(conn);
    let _ = fs::remove_file(test_db);
}

#[test]
fn mcp_tool_call_audit_ok() {
    // Test that tool calls are properly audited
    let test_db = "test_mcp_audit.db";
    let _ = fs::remove_file(test_db);

    syncore::db::ensure_schema(test_db).unwrap();
    let conn = syncore::db::open_db_with_wal(test_db).unwrap();

    // Simulate MCP tool call logging
    let tool_name = "task.create";
    let args_json = r#"{"goal": "Test task", "priority": 1}"#;
    let result_json = r#"{"task_id": 1}"#;

    conn.execute(
        "INSERT INTO tool_calls (tool_name, args_json, result_json, status, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        (tool_name, args_json, result_json, "ok", 1234567890),
    )
    .unwrap();

    // Verify audit entry
    let audit = conn.query_row(
        "SELECT tool_name, args_json, result_json, status FROM tool_calls ORDER BY id DESC LIMIT 1",
        [],
        |row| Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?
        ))
    ).unwrap();

    assert_eq!(audit.0, tool_name);
    assert_eq!(audit.1, args_json);
    assert_eq!(audit.2, result_json);
    assert_eq!(audit.3, "ok");

    // Clean up
    drop(conn);
    let _ = fs::remove_file(test_db);
}
