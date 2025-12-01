use rusqlite::Connection;
use std::fs;
use syncore::{cognitive_db, tasks, vector};

#[test]
fn test_basic_task_crud() {
    let test_db = "simple_test.db";
    let _ = fs::remove_file(test_db);

    // Initialize database
    syncore::db::ensure_schema(test_db).unwrap();
    let conn = syncore::db::open_db_with_wal(test_db).unwrap();

    // Test add_task
    let task_id = tasks::add_task(&conn, "Test goal", "Test description", 3, None).unwrap();
    assert!(task_id > 0);

    // Test next_task
    let task = tasks::next_task(&conn, None, None).unwrap();
    assert!(task.is_some());
    let task = task.unwrap();
    assert_eq!(task.id, task_id);
    assert_eq!(task.goal, "Test goal");

    // Test update_task
    tasks::update_task(&conn, task_id, Some("done"), None, None).unwrap();

    // Verify update
    let status = conn
        .query_row("SELECT status FROM tasks WHERE id = ?1", [task_id], |row| {
            row.get::<_, String>(0)
        })
        .unwrap();
    assert_eq!(status, "done");

    // Test link_tasks
    let task2_id = tasks::add_task(&conn, "Task 2", "Description 2", 2, None).unwrap();
    tasks::link_tasks(&conn, task2_id, task_id, "depends_on").unwrap();

    // Verify link
    let link_count = conn.query_row(
        "SELECT COUNT(*) FROM task_links WHERE src_id = ?1 AND dst_id = ?2 AND kind = 'depends_on'",
        (task2_id, task_id),
        |row| row.get::<_, i64>(0)
    ).unwrap();
    assert_eq!(link_count, 1);

    // Clean up
    drop(conn);
    let _ = fs::remove_file(test_db);
}

#[test]
fn test_cognitive_step_crud() {
    let test_db = "simple_cognitive_test.db";
    let _ = fs::remove_file(test_db);

    syncore::db::ensure_schema(test_db).unwrap();
    let conn = syncore::db::open_db_with_wal(test_db).unwrap();

    // Create a task
    let task_id = tasks::add_task(&conn, "Test task", "For cognitive steps", 1, None).unwrap();

    // Test store_step
    let step1_id =
        cognitive_db::store_step(&conn, Some(task_id), "Think", "Initial thought", "{}").unwrap();
    let step2_id = cognitive_db::store_step(
        &conn,
        Some(task_id),
        "Decide",
        "Made decision",
        "{\"decision\": \"proceed\"}",
    )
    .unwrap();

    assert!(step1_id > 0);
    assert!(step2_id > step1_id);

    // Test recent_steps
    let recent = cognitive_db::recent_steps(&conn, task_id, 5).unwrap();
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0].state, "Decide");
    assert_eq!(recent[1].state, "Think");

    // Clean up
    drop(conn);
    let _ = fs::remove_file(test_db);
}

#[test]
fn test_vector_operations() {
    // Setup vector store
    let embeddings = Box::new(syncore::vector::RealEmbeddings::new(384).unwrap());
    let mut vector_store = syncore::vector::VectorStore::new(embeddings);

    // Test insert_text
    vector::insert_text(&mut vector_store, 1, Some(100), "Test document", "note").unwrap();
    vector::insert_text(&mut vector_store, 2, Some(100), "Another document", "note").unwrap();
    vector::insert_text(&mut vector_store, 3, Some(200), "Different task", "note").unwrap();

    assert_eq!(vector_store.len(), 3);

    // Test search global scope
    let global_hits =
        vector::search(&vector_store, "Test", 10, syncore::vector::SearchScope::Global).unwrap();
    assert_eq!(global_hits.len(), 3);

    // Test search task scope
    let task_hits =
        vector::search(&vector_store, "Test", 10, syncore::vector::SearchScope::Task(100)).unwrap();
    assert_eq!(task_hits.len(), 2);

    // Clean up vector files
    let _ = fs::remove_file("vector.index.vectors");
    let _ = fs::remove_file("vector.index.meta");
}
