// Simple standalone test that avoids problematic modules
use rusqlite::Connection;
use std::fs;

#[test]
fn test_database_basic_operations() {
    let test_db = "standalone_test.db";
    let _ = fs::remove_file(test_db);

    // Initialize database with schema
    let conn = Connection::open(test_db).unwrap();

    // Enable foreign keys and WAL
    conn.pragma_update(None, "foreign_keys", &"ON").unwrap();
    conn.pragma_update(None, "journal_mode", &"WAL").unwrap();

    // Create tables manually
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            goal TEXT NOT NULL,
            description TEXT DEFAULT '',
            status TEXT NOT NULL DEFAULT 'open',
            priority INTEGER NOT NULL DEFAULT 3,
            parent_id INTEGER REFERENCES tasks(id) ON DELETE CASCADE,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS task_links (
            src_id INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            dst_id INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            kind TEXT NOT NULL,
            PRIMARY KEY (src_id, dst_id, kind)
        );

        CREATE TABLE IF NOT EXISTS steps (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id INTEGER REFERENCES tasks(id) ON DELETE SET NULL,
            state TEXT NOT NULL,
            content TEXT NOT NULL,
            meta_json TEXT NOT NULL DEFAULT '{}',
            created_at INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_tasks_status_prio ON tasks(status, priority, id);
        CREATE INDEX IF NOT EXISTS idx_tasks_parent ON tasks(parent_id);
        CREATE INDEX IF NOT EXISTS idx_steps_task_state_time ON steps(task_id, state, created_at DESC);
    ").unwrap();

    // Test basic task creation
    let task_id = conn.execute("
        INSERT INTO tasks (goal, description, priority, parent_id, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?5)
    ", ("Test goal", "Test description", 3, None, 1234567890, 1234567890)).unwrap();

    assert!(task_id > 0);

    // Test task retrieval
    let task = conn.query_row("
        SELECT id, goal, description, status, priority, parent_id, created_at, updated_at
        FROM tasks WHERE id = ?1
    ", [task_id], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<i64>>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, i64>(6)?,
        ))
    }).unwrap();

    assert_eq!(task.0, task_id);
    assert_eq!(task.1, "Test goal");
    assert_eq!(task.2, "Test description");
    assert_eq!(task.3, 3);
    assert_eq!(task.4, None);

    // Test task update
    conn.execute("
        UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id = ?3
    ", ("done", 1234567891, task_id)).unwrap();

    // Verify update
    let status = conn.query_row("
        SELECT status FROM tasks WHERE id = ?1
    ", [task_id], |row| row.get::<_, String>(0)).unwrap();
    assert_eq!(status, "done");

    // Test task linking
    let task2_id = conn.execute("
        INSERT INTO tasks (goal, description, priority, parent_id, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?5)
    ", ("Task 2", "Description 2", 2, None, 1234567892, 1234567892)).unwrap();

    conn.execute("
        INSERT OR REPLACE INTO task_links (src_id, dst_id, kind)
        VALUES (?1, ?2, ?3)
    ", (task2_id, task_id, "depends_on")).unwrap();

    // Verify link
    let link_exists = conn.query_row("
        SELECT COUNT(*) FROM task_links WHERE src_id = ?1 AND dst_id = ?2 AND kind = 'depends_on'
    ", (task2_id, task_id), |row| row.get::<_, i64>(0)).unwrap();
    assert_eq!(link_exists, 1);

    // Test cascade delete
    conn.execute("DELETE FROM tasks WHERE id = ?1", [task_id]).unwrap();

    let child_remaining = conn.query_row("
        SELECT COUNT(*) FROM tasks WHERE id = ?1
    ", [task2_id], |row| row.get::<_, i64>(0)).unwrap_optional().unwrap_or(0);
    assert_eq!(child_remaining, 0);

    // Clean up
    drop(conn);
    let _ = fs::remove_file(test_db);
}

#[test]
fn test_cognitive_step_operations() {
    let test_db = "standalone_cognitive_test.db";
    let _ = fs::remove_file(test_db);

    let conn = Connection::open(test_db).unwrap();
    conn.pragma_update(None, "foreign_keys", &"ON").unwrap();
    conn.pragma_update(None, "journal_mode", &"WAL").unwrap();

    // Create tables
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            goal TEXT NOT NULL,
            description TEXT DEFAULT '',
            status TEXT NOT NULL DEFAULT 'open',
            priority INTEGER NOT NULL DEFAULT 3,
            parent_id INTEGER REFERENCES tasks(id) ON DELETE CASCADE,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS steps (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id INTEGER REFERENCES tasks(id) ON DELETE SET NULL,
            state TEXT NOT NULL,
            content TEXT NOT NULL,
            meta_json TEXT NOT NULL DEFAULT '{}',
            created_at INTEGER NOT NULL
        );
    ").unwrap();

    // Create a task first
    let task_id = conn.execute("
        INSERT INTO tasks (goal, description, priority, parent_id, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?5)
    ", ("Test task", "For cognitive steps", 1, None, 1234567890, 1234567890)).unwrap();

    // Test step creation
    let step1_id = conn.execute("
        INSERT INTO steps (task_id, state, content, meta_json, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5)
    ", (Some(task_id), "Think", "Initial thinking", "{}", 1234567891)).unwrap();

    let step2_id = conn.execute("
        INSERT INTO steps (task_id, state, content, meta_json, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5)
    ", (Some(task_id), "Decide", "Made decision", "{\"decision\": \"proceed\"}", 1234567892)).unwrap();

    assert!(step1_id > 0);
    assert!(step2_id > step1_id);

    // Test recent steps retrieval
    let steps = conn.prepare("
        SELECT id, task_id, state, content, meta_json, created_at
        FROM steps
        WHERE task_id = ?1
        ORDER BY created_at DESC
        LIMIT ?2
    ").unwrap().query_map([task_id, 5], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, Option<i64>>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, i64>(4)?,
        ))
    }).unwrap().collect::<Result<Vec<_>, _>>().unwrap();

    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].2, "Decide");
    assert_eq!(steps[1].2, "Think");

    // Clean up
    drop(conn);
    let _ = fs::remove_file(test_db);
}
