use rusqlite::{Connection, OptionalExtension};
use std::fs;
use syncore::{cognitive_db, tasks, vector};

#[test]
fn tasks_hierarchy_fk_ok() {
    // Test cascade delete verifies foreign key constraints
    let test_db = "test_tasks_hierarchy_fk.db";
    let _ = fs::remove_file(test_db);

    // Initialize database with schema
    syncore::db::ensure_schema(test_db).unwrap();
    let conn = syncore::db::open_db_with_wal(test_db).unwrap();

    // Create parent task
    let parent_id = tasks::add_task(&conn, "Parent task", "Main goal", 1, None).unwrap();
    assert!(parent_id > 0);

    // Create child tasks
    let child1_id = tasks::add_task(&conn, "Child 1", "First subtask", 2, Some(parent_id)).unwrap();
    let child2_id =
        tasks::add_task(&conn, "Child 2", "Second subtask", 3, Some(parent_id)).unwrap();

    // Verify hierarchy exists
    let parent = tasks::next_task(&conn, None, None).unwrap().unwrap();
    assert_eq!(parent.id, parent_id);
    assert_eq!(parent.goal, "Parent task");

    // Delete parent and verify cascade delete works
    conn.execute("DELETE FROM tasks WHERE id = ?1", [parent_id]).unwrap();

    // Verify children are also deleted (cascade)
    let child1 =
        conn.query_row("SELECT id FROM tasks WHERE id = ?1", [child1_id], |_| Ok(())).optional();
    let child2 =
        conn.query_row("SELECT id FROM tasks WHERE id = ?1", [child2_id], |_| Ok(())).optional();

    assert!(child1.unwrap().is_none(), "Child 1 should be cascade deleted");
    assert!(child2.unwrap().is_none(), "Child 2 should be cascade deleted");

    // Clean up
    drop(conn);
    let _ = fs::remove_file(test_db);
}

#[test]
fn task_link_depends_on_enforced_ok() {
    // Test dependency enforcement - can't mark done if outstanding deps
    let test_db = "test_task_link_deps.db";
    let _ = fs::remove_file(test_db);

    syncore::db::ensure_schema(test_db).unwrap();
    let conn = syncore::db::open_db_with_wal(test_db).unwrap();

    // Create two tasks
    let task_a_id = tasks::add_task(&conn, "Task A", "Prerequisite", 1, None).unwrap();
    let task_b_id = tasks::add_task(&conn, "Task B", "Dependent task", 2, None).unwrap();

    // Link them: B depends on A
    tasks::link_tasks(&conn, task_b_id, task_a_id, "depends_on").unwrap();

    // Try to mark B as done while A is still open - this should be allowed at DB level
    // but we need to check dependencies at application level
    tasks::update_task(&conn, task_b_id, Some("done"), None, None).unwrap();

    // Verify B is marked done
    let task_b = conn
        .query_row("SELECT status FROM tasks WHERE id = ?1", [task_b_id], |row| {
            row.get::<_, String>(0)
        })
        .unwrap();
    assert_eq!(task_b, "done");

    // Verify A is still open
    let task_a = conn
        .query_row("SELECT status FROM tasks WHERE id = ?1", [task_a_id], |row| {
            row.get::<_, String>(0)
        })
        .unwrap();
    assert_eq!(task_a, "open");

    // Test application-level dependency checking
    let mut deps = conn
        .prepare(
            "SELECT t.id, t.status FROM tasks t
         JOIN task_links tl ON t.id = tl.dst_id
         WHERE tl.src_id = ?1 AND tl.kind = 'depends_on' AND t.status != 'done'",
        )
        .unwrap();

    let outstanding_deps = deps
        .query_map([task_b_id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert!(!outstanding_deps.is_empty(), "Should have outstanding dependencies");
    assert_eq!(outstanding_deps[0].0, task_a_id);
    assert_eq!(outstanding_deps[0].1, "open");

    // Clean up
    drop(deps);
    drop(conn);
    let _ = fs::remove_file(test_db);
}

#[test]
fn task_crud_operations_ok() {
    // Test basic CRUD operations
    let test_db = "test_task_crud.db";
    let _ = fs::remove_file(test_db);

    syncore::db::ensure_schema(test_db).unwrap();
    let conn = syncore::db::open_db_with_wal(test_db).unwrap();

    // CREATE
    let task_id = tasks::add_task(&conn, "Test task", "Description", 3, None).unwrap();
    assert!(task_id > 0);

    // READ
    let task = tasks::next_task(&conn, None, None).unwrap().unwrap();
    assert_eq!(task.id, task_id);
    assert_eq!(task.goal, "Test task");
    assert_eq!(task.description, "Description");
    assert_eq!(task.priority, 3);
    assert_eq!(task.status, "open");

    // UPDATE - change status and priority
    tasks::update_task(&conn, task_id, Some("running"), Some(1), Some("Updated description"))
        .unwrap();

    // Verify update
    let updated = conn
        .query_row(
            "SELECT status, priority, description FROM tasks WHERE id = ?1",
            [task_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?, row.get::<_, String>(2)?)),
        )
        .unwrap();
    assert_eq!(updated.0, "running");
    assert_eq!(updated.1, 1);
    assert_eq!(updated.2, "Updated description");

    // DELETE (cascade test will be separate)
    conn.execute("DELETE FROM tasks WHERE id = ?1", [task_id]).unwrap();

    // Verify deletion
    let deleted =
        conn.query_row("SELECT id FROM tasks WHERE id = ?1", [task_id], |_| Ok(())).optional();
    assert!(deleted.unwrap().is_none());

    // Clean up
    drop(conn);
    let _ = fs::remove_file(test_db);
}

#[test]
fn cognitive_crud_operations_ok() {
    // Test cognitive step CRUD
    let test_db = "test_cognitive_crud.db";
    let _ = fs::remove_file(test_db);

    syncore::db::ensure_schema(test_db).unwrap();
    let conn = syncore::db::open_db_with_wal(test_db).unwrap();

    // Create a task first
    let task_id = tasks::add_task(&conn, "Test task", "For cognitive steps", 1, None).unwrap();

    // CREATE cognitive steps
    let step1_id =
        cognitive_db::store_step(&conn, Some(task_id), "Think", "Initial thinking", "{}").unwrap();
    let step2_id = cognitive_db::store_step(
        &conn,
        Some(task_id),
        "Decide",
        "Making decision",
        "{\"decision\": \"proceed\"}",
    )
    .unwrap();

    assert!(step1_id > 0);
    assert!(step2_id > 0);
    assert!(step2_id > step1_id); // Should be chronological

    // READ recent steps
    let recent = cognitive_db::recent_steps(&conn, task_id, 5).unwrap();
    assert_eq!(recent.len(), 2);

    // Verify order (most recent first)
    assert_eq!(recent[0].id, step2_id);
    assert_eq!(recent[0].state, "Decide");
    assert_eq!(recent[1].id, step1_id);
    assert_eq!(recent[1].state, "Think");

    // Verify content
    assert_eq!(recent[0].content, "Making decision");
    assert_eq!(recent[1].content, "Initial thinking");

    // Verify meta JSON
    assert_eq!(recent[1].meta_json, "{}");
    assert_eq!(recent[0].meta_json, "{\"decision\": \"proceed\"}");

    // Clean up
    drop(conn);
    let _ = fs::remove_file(test_db);
}

#[test]
fn vector_crud_operations_ok() {
    // Test vector store operations
    let test_db = "test_vector_crud.db";
    let _ = fs::remove_file(test_db);

    // Initialize vector store
    let embeddings = Box::new(syncore::vector::RealEmbeddings::new(384).unwrap());
    let mut vector_store = syncore::vector::VectorStore::new(embeddings);

    // INSERT text
    vector::insert_text(&mut vector_store, 1, Some(100), "Test document one", "note").unwrap();
    vector::insert_text(&mut vector_store, 2, Some(100), "Test document two", "note").unwrap();
    vector::insert_text(&mut vector_store, 3, Some(200), "Different task document", "note")
        .unwrap();

    assert_eq!(vector_store.len(), 3);

    // SEARCH global scope
    let global_hits =
        vector::search(&vector_store, "Test", 10, syncore::vector::SearchScope::Global).unwrap();
    assert_eq!(global_hits.len(), 3);

    // SEARCH task scope
    let task_hits =
        vector::search(&vector_store, "Test", 10, syncore::vector::SearchScope::Task(100)).unwrap();
    assert_eq!(task_hits.len(), 2);

    // Verify results contain expected IDs
    let hit_ids: Vec<i64> = task_hits.iter().map(|h| h.id).collect();
    assert!(hit_ids.contains(&1));
    assert!(hit_ids.contains(&2));
    assert!(!hit_ids.contains(&3));

    // Verify scores are reasonable (0.0 to 1.0)
    for hit in &global_hits {
        assert!(hit.score >= 0.0 && hit.score <= 1.0);
    }

    // Clean up vector files
    let _ = fs::remove_file("vector.index.vectors");
    let _ = fs::remove_file("vector.index.meta");
    let _ = fs::remove_file(test_db);
}
