//! TDD tests for DbManager (SQLite connection pooling layer)
//! These tests MUST be written BEFORE implementing DbManager

use std::path::{Path, PathBuf};
use std::sync::Arc;

mod fakes;

/// Test helper: Create isolated test database paths using UUID
/// This prevents test database corruption from reusing old files
fn create_test_db_paths() -> (PathBuf, PathBuf) {
    let uuid = uuid::Uuid::new_v4().to_string();
    let main_db = PathBuf::from(format!("/tmp/syncore_test_main_{}.db", uuid));
    let code_graph_db = PathBuf::from(format!("/tmp/syncore_test_code_graph_{}.db", uuid));

    // Clean up any existing files with these UUIDs (extremely unlikely collision)
    let _ = std::fs::remove_file(&main_db);
    let _ = std::fs::remove_file(&code_graph_db);
    let _ = std::fs::remove_file(format!("{}-wal", main_db.display()));
    let _ = std::fs::remove_file(format!("{}-shm", main_db.display()));
    let _ = std::fs::remove_file(format!("{}-wal", code_graph_db.display()));
    let _ = std::fs::remove_file(format!("{}-shm", code_graph_db.display()));

    (main_db, code_graph_db)
}

/// Test helper: Clean up test database files
fn cleanup_test_dbs(main_db: &Path, code_graph_db: &Path) {
    let _ = std::fs::remove_file(main_db);
    let _ = std::fs::remove_file(code_graph_db);
    let _ = std::fs::remove_file(format!("{}-wal", main_db.display()));
    let _ = std::fs::remove_file(format!("{}-shm", main_db.display()));
    let _ = std::fs::remove_file(format!("{}-wal", code_graph_db.display()));
    let _ = std::fs::remove_file(format!("{}-shm", code_graph_db.display()));
}

/// Test 1: DbManager initializes both main and code_graph databases
#[test]
fn test_db_manager_initializes_main_and_code_graph_dbs() {
    // Arrange: use isolated test database paths
    let (test_main_db, test_code_graph_db) = create_test_db_paths();

    // Act: Initialize DbManager
    let db_manager = syncore::db::DbManager::new(
        test_main_db.to_str().unwrap(),
        test_code_graph_db.to_str().unwrap(),
    )
    .expect("Failed to initialize DbManager");

    // Assert: Database files exist
    assert!(test_main_db.exists(), "Main DB file should exist");
    assert!(
        test_code_graph_db.exists(),
        "Code graph DB file should exist"
    );

    // Assert: PRAGMA database_list shows correct paths
    {
        let main_conn = db_manager.main_conn();
        let main_lock = main_conn.lock().unwrap();
        let db_path: String = main_lock
            .query_row(
                "SELECT file FROM pragma_database_list() WHERE name='main'",
                [],
                |row| row.get(0),
            )
            .expect("Failed to query main DB path");
        assert!(
            db_path.contains("syncore_test_main_"),
            "Main DB path mismatch: {}",
            db_path
        );
    }

    {
        let code_graph_conn = db_manager.code_graph_conn();
        let code_graph_lock = code_graph_conn.lock().unwrap();
        let db_path: String = code_graph_lock
            .query_row(
                "SELECT file FROM pragma_database_list() WHERE name='main'",
                [],
                |row| row.get(0),
            )
            .expect("Failed to query code graph DB path");
        assert!(
            db_path.contains("syncore_test_code_graph_"),
            "Code graph DB path mismatch: {}",
            db_path
        );
    }

    // Assert: Journal mode is WAL for both databases
    {
        let main_conn = db_manager.main_conn();
        let main_lock = main_conn.lock().unwrap();
        let journal_mode: String = main_lock
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .expect("Failed to query journal_mode");
        assert_eq!(
            journal_mode.to_lowercase(),
            "wal",
            "Main DB should use WAL mode"
        );
    }

    {
        let code_graph_conn = db_manager.code_graph_conn();
        let code_graph_lock = code_graph_conn.lock().unwrap();
        let journal_mode: String = code_graph_lock
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .expect("Failed to query journal_mode");
        assert_eq!(
            journal_mode.to_lowercase(),
            "wal",
            "Code graph DB should use WAL mode"
        );
    }

    // Assert: Schema tables exist (via migrations)
    {
        let main_conn = db_manager.main_conn();
        let main_lock = main_conn.lock().unwrap();
        let has_memory: bool = main_lock
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='memory'")
            .and_then(|mut stmt| stmt.exists([]))
            .expect("Failed to check memory table");
        assert!(
            has_memory,
            "Main DB should have 'memory' table from migrations"
        );

        let has_tasks: bool = main_lock
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='tasks'")
            .and_then(|mut stmt| stmt.exists([]))
            .expect("Failed to check tasks table");
        assert!(
            has_tasks,
            "Main DB should have 'tasks' table from migrations"
        );
    }

    {
        let code_graph_conn = db_manager.code_graph_conn();
        let code_graph_lock = code_graph_conn.lock().unwrap();
        let has_code_entities: bool = code_graph_lock
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='code_entities'")
            .and_then(|mut stmt| stmt.exists([]))
            .expect("Failed to check code_entities table");
        assert!(
            has_code_entities,
            "Code graph DB should have 'code_entities' table from migrations"
        );
    }

    // Cleanup
    drop(db_manager);
    cleanup_test_dbs(&test_main_db, &test_code_graph_db);
}

/// Test 2: DbManager uses the same connection instance across multiple accesses
#[test]
fn test_db_manager_uses_single_connection_for_code_graph() {
    // Arrange
    let (test_main_db, test_code_graph_db) = create_test_db_paths();

    let db_manager = syncore::db::DbManager::new(
        test_main_db.to_str().unwrap(),
        test_code_graph_db.to_str().unwrap(),
    )
    .expect("Failed to initialize DbManager");

    // Act: Get code_graph connection multiple times
    let conn1 = db_manager.code_graph_conn();
    let conn2 = db_manager.code_graph_conn();

    // Assert: Both should be Arc pointers to the same underlying Mutex<Connection>
    // (Arc::ptr_eq checks if two Arcs point to the same allocation)
    assert!(
        Arc::ptr_eq(&conn1, &conn2),
        "DbManager should return the same Arc<Mutex<Connection>> for code_graph_conn()"
    );

    // Cleanup
    drop(db_manager);
    cleanup_test_dbs(&test_main_db, &test_code_graph_db);
}

/// Test 3: Code graph indexing persists entities across multiple operations
#[test]
fn test_code_graph_index_persists_entities_across_calls() {
    use std::sync::Mutex;
    use syncore::code_graph::CodeGraph;
    use syncore::vector::VectorStore;

    // Arrange
    let (test_main_db, test_code_graph_db) = create_test_db_paths();
    let uuid = uuid::Uuid::new_v4().to_string();
    let test_file = PathBuf::from(format!("/tmp/test_persist_{}.rs", uuid));

    let _ = std::fs::remove_file(&test_file);

    // Create test Rust file
    std::fs::write(
        &test_file,
        r#"
pub fn test_function_one() {
    println!("test one");
}

pub fn test_function_two() {
    println!("test two");
}

pub struct TestStruct {
    pub field: i32,
}
"#,
    )
    .expect("Failed to write test file");

    let db_manager = Arc::new(
        syncore::db::DbManager::new(
            test_main_db.to_str().unwrap(),
            test_code_graph_db.to_str().unwrap(),
        )
        .expect("Failed to initialize DbManager"),
    );

    // Use FakeEmbeddings for fast tests (no ML model loading)
    let embeddings =
        Box::new(fakes::FakeEmbeddings::new(8).expect("Failed to create fake embeddings"));
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    // Act: Index the file using CodeGraph with DbManager connection
    let code_graph_conn = db_manager.code_graph_conn();
    let mut code_graph = CodeGraph::with_connection(code_graph_conn, Arc::clone(&vector_store))
        .expect("Failed to create CodeGraph");

    let indexed_count = code_graph
        .index_file(&test_file)
        .expect("Failed to index file");

    // Drop CodeGraph to ensure connection is released (but not closed, as it's managed by DbManager)
    drop(code_graph);

    // Assert: Entities persisted to database
    let code_graph_conn = db_manager.code_graph_conn();
    let conn_lock = code_graph_conn.lock().unwrap();
    let entity_count: i64 = conn_lock
        .query_row(
            "SELECT COUNT(*) FROM code_entities WHERE file_path = ?",
            [test_file.to_str().unwrap()],
            |row| row.get(0),
        )
        .expect("Failed to query entity count");

    assert!(indexed_count > 0, "Should have indexed at least one entity");
    assert_eq!(
        entity_count, indexed_count as i64,
        "Entity count in DB ({}) should match indexed count ({})",
        entity_count, indexed_count
    );

    // Act: Index the same file again (should replace old entities)
    let mut code_graph =
        CodeGraph::with_connection(db_manager.code_graph_conn(), Arc::clone(&vector_store))
            .expect("Failed to create CodeGraph for second index");

    let indexed_count2 = code_graph
        .index_file(&test_file)
        .expect("Failed to index file second time");

    drop(code_graph);

    // Assert: Entity count remains the same (re-index replaces, not duplicates)
    let conn_lock = code_graph_conn.lock().unwrap();
    let entity_count2: i64 = conn_lock
        .query_row(
            "SELECT COUNT(*) FROM code_entities WHERE file_path = ?",
            [test_file.to_str().unwrap()],
            |row| row.get(0),
        )
        .expect("Failed to query entity count after re-index");

    assert_eq!(
        entity_count2, indexed_count2 as i64,
        "Re-indexing should replace entities, count should match"
    );

    // Cleanup
    drop(db_manager);
    cleanup_test_dbs(&test_main_db, &test_code_graph_db);
    let _ = std::fs::remove_file(&test_file);
}

/// Test 4: Embeddings persist in main database
#[test]
fn test_embeddings_persist_in_main_db() {
    // Arrange
    let (test_main_db, test_code_graph_db) = create_test_db_paths();

    let db_manager = Arc::new(
        syncore::db::DbManager::new(
            test_main_db.to_str().unwrap(),
            test_code_graph_db.to_str().unwrap(),
        )
        .expect("Failed to initialize DbManager"),
    );

    // Act: Insert embedding into main DB
    {
        let main_conn = db_manager.main_conn();
        let conn_lock = main_conn.lock().unwrap();

        conn_lock
            .execute(
                "INSERT INTO embeddings (entity_id, entity_type, embedding_blob, created_at)
             VALUES (?, ?, ?, ?)",
                rusqlite::params![42, "function", b"\x00\x01\x02\x03", 1234567890],
            )
            .expect("Failed to insert embedding");
    }

    // Assert: Embedding persists and is readable
    {
        let main_conn = db_manager.main_conn();
        let conn_lock = main_conn.lock().unwrap();

        let count: i64 = conn_lock
            .query_row(
                "SELECT COUNT(*) FROM embeddings WHERE entity_id = ?",
                [42],
                |row| row.get(0),
            )
            .expect("Failed to query embedding count");

        assert_eq!(count, 1, "Embedding should persist in main DB");
    }

    // Cleanup
    drop(db_manager);
    cleanup_test_dbs(&test_main_db, &test_code_graph_db);
}

/// Test 5: DbManager allows parallel indexing without panics
#[test]
fn test_db_manager_allows_parallel_indexing_without_panic() {
    use std::sync::Mutex;
    use std::thread;
    use syncore::code_graph::CodeGraph;
    use syncore::vector::VectorStore;

    // Arrange
    let (test_main_db, test_code_graph_db) = create_test_db_paths();

    let db_manager = Arc::new(
        syncore::db::DbManager::new(
            test_main_db.to_str().unwrap(),
            test_code_graph_db.to_str().unwrap(),
        )
        .expect("Failed to initialize DbManager"),
    );

    // Use FakeEmbeddings for fast tests (no ML model loading)
    let embeddings =
        Box::new(fakes::FakeEmbeddings::new(8).expect("Failed to create fake embeddings"));
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    // Create multiple test files with unique names
    let uuid = uuid::Uuid::new_v4().to_string();
    let test_files = vec![
        (
            PathBuf::from(format!("/tmp/test_parallel1_{}.rs", uuid)),
            "pub fn func1() {}",
        ),
        (
            PathBuf::from(format!("/tmp/test_parallel2_{}.rs", uuid)),
            "pub fn func2() {}",
        ),
        (
            PathBuf::from(format!("/tmp/test_parallel3_{}.rs", uuid)),
            "pub fn func3() {}",
        ),
    ];

    for (path, content) in &test_files {
        std::fs::write(path, content).expect("Failed to write test file");
    }

    // Act: Spawn threads that index files concurrently
    let mut handles = vec![];

    for (file_path, _) in &test_files {
        let db_manager_clone = Arc::clone(&db_manager);
        let vector_store_clone = Arc::clone(&vector_store);
        let file_path_clone = file_path.clone();

        let handle = thread::spawn(move || {
            let mut code_graph =
                CodeGraph::with_connection(db_manager_clone.code_graph_conn(), vector_store_clone)
                    .expect("Failed to create CodeGraph in thread");

            code_graph
                .index_file(&file_path_clone)
                .expect("Failed to index file in parallel");
        });

        handles.push(handle);
    }

    // Assert: All threads complete without panics
    for handle in handles {
        handle
            .join()
            .expect("Thread panicked during parallel indexing");
    }

    // Assert: All entities persisted
    let code_graph_conn = db_manager.code_graph_conn();
    let conn_lock = code_graph_conn.lock().unwrap();

    for (file_path, _) in &test_files {
        let count: i64 = conn_lock
            .query_row(
                "SELECT COUNT(*) FROM code_entities WHERE file_path = ?",
                [file_path.to_str().unwrap()],
                |row| row.get(0),
            )
            .expect("Failed to query entity count");

        assert!(
            count > 0,
            "File {} should have indexed entities",
            file_path.display()
        );
    }

    // Cleanup
    drop(db_manager);
    cleanup_test_dbs(&test_main_db, &test_code_graph_db);
    for (path, _) in &test_files {
        let _ = std::fs::remove_file(path);
    }
}

/// Test 6: MCP code_index tool persists to code_graph DB (E2E test)
#[test]
#[ignore] // Run with: cargo test -- --ignored
fn test_code_index_tool_persists_to_code_graph_db() {
    // This is an E2E test that would require full MCP tool executor setup
    // For now, it's marked as ignored and serves as a specification
    //
    // Expected behavior:
    // 1. Initialize SynCoreState with DbManager
    // 2. Call code_index MCP tool on a test file
    // 3. Verify entities persist in code_graph DB after tool returns success
    // 4. Verify no "split-brain" issues (single database file)

    todo!("Implement E2E MCP tool test after DbManager is wired into SynCoreState");
}
