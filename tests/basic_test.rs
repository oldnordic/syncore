use anyhow::Result;
use rusqlite::{Connection, OptionalExtension};
use tempfile::NamedTempFile;

#[test]
fn test_basic_task_operations() -> Result<()> {
    // Create a test database
    let temp_file = NamedTempFile::new()?;
    let db_path = temp_file.path().to_str().unwrap();
    let db = Connection::open(db_path)?;
    
    // Initialize database schema
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS tasks (
            id INTEGER PRIMARY KEY,
            goal TEXT NOT NULL,
            description TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            priority INTEGER NOT NULL DEFAULT 0,
            parent_id INTEGER,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );",
    )?;
    
    // Test adding a task
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    
    let task_id = db.execute(
        "INSERT INTO tasks (goal, description, priority, created_at, updated_at) 
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params!["Test Task", "Test Description", 1, now, now],
    )? as i64;
    
    assert!(task_id > 0);
    println!("✓ Created task with ID: {}", task_id);
    
    // Test reading the task back
    let task: Option<(i64, String, String, i32)> = db.query_row(
        "SELECT id, goal, description, priority FROM tasks WHERE id = ?1",
        [task_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    ).optional()?;
    
    assert!(task.is_some());
    let task = task.unwrap();
    assert_eq!(task.0, task_id);
    assert_eq!(task.1, "Test Task");
    assert_eq!(task.2, "Test Description");
    assert_eq!(task.3, 1);
    println!("✓ Read task back successfully");
    
    // Test updating the task
    db.execute(
        "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params!["done", now, task_id],
    )?;
    
    let status: Option<String> = db.query_row(
        "SELECT status FROM tasks WHERE id = ?1",
        [task_id],
        |row| row.get(0)
    ).optional()?;
    
    assert_eq!(status, Some("done".to_string()));
    println!("✓ Updated task status successfully");
    
    Ok(())
}