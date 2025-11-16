use anyhow::Result;
use rusqlite::Connection;
use std::fs;
use tempfile::NamedTempFile;

fn test_db() -> Result<Connection> {
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
        );
        
        CREATE TABLE IF NOT EXISTS memory (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        
        CREATE TABLE IF NOT EXISTS vector_embeddings (
            id INTEGER PRIMARY KEY,
            task_id INTEGER,
            text TEXT NOT NULL,
            kind TEXT NOT NULL,
            embedding BLOB,
            created_at INTEGER NOT NULL,
            FOREIGN KEY (task_id) REFERENCES tasks (id)
        );",
    )?;
    
    Ok(db)
}

fn add_task(db: &Connection, goal: &str, description: &str, priority: i32, parent_id: Option<i64>) -> Result<i64> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    
    db.execute(
        "INSERT INTO tasks (goal, description, priority, parent_id, created_at, updated_at) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        [goal, description, &priority.to_string(), &parent_id.map(|p| p.to_string()).unwrap_or_else(|| "null".to_string()), &now.to_string(), &now.to_string()],
    )?;
    
    Ok(db.last_insert_rowid())
}

fn update_task(db: &Connection, id: i64, status: Option<&str>, priority: Option<i32>, description: Option<&str>) -> Result<()> {
    let mut updates = Vec::new();
    let mut params = Vec::new();
    
    if let Some(s) = status {
        updates.push("status = ?".to_string());
        params.push(s.to_string());
    }
    
    if let Some(p) = priority {
        updates.push("priority = ?".to_string());
        params.push(p.to_string());
    }
    
    if let Some(d) = description {
        updates.push("description = ?".to_string());
        params.push(d.to_string());
    }
    
    if updates.is_empty() {
        return Ok(());
    }
    
    updates.push("updated_at = ?".to_string());
    params.push(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs().to_string());
    
    params.push(id.to_string());
    
    let query = format!("UPDATE tasks SET {} WHERE id = ?", updates.join(", "));
    db.execute(&query, rusqlite::params_from_iter(params))?;
    
    Ok(())
}

fn next_task(db: &Connection, statuses: Option<&[&str]>, min_prio: Option<i32>) -> Result<Option<crate::tasks::Task>> {
    let mut query = "
        SELECT id, goal, description, status, priority, parent_id, created_at, updated_at
        FROM tasks
        WHERE status != 'done' AND status != 'cancelled'
    ".to_string();

    if let Some(statuses) = statuses {
        let status_list: Vec<String> = statuses.iter().map(|s| format!("'{}'", s)).collect();
        query.push_str(&format!(" AND status IN ({})", status_list.join(", ")));
    }

    if let Some(min_prio_val) = min_prio {
        query.push_str(&format!(" AND priority <= {}", min_prio_val));
    }

    query.push_str(" ORDER BY priority ASC, created_at ASC LIMIT 1");

    let mut stmt = db.prepare(&query)?;
    let task = stmt.query_row([], |row| {
        Ok(crate::tasks::Task {
            id: row.get(0)?,
            goal: row.get(1)?,
            description: row.get(2)?,
            status: row.get(3)?,
            priority: row.get(4)?,
            parent_id: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    }).optional()?;

    Ok(task)
}

#[test]
fn e2e_task_lifecycle() -> Result<()> {
    let db = test_db()?;
    
    // Create a task
    let id = add_task(&db, "Demo", "Validate end-to-end", 1, None)?;
    println!("✓ Created task with ID: {}", id);
    
    // Update task to done
    update_task(&db, id, Some("done"), None, None)?;
    println!("✓ Updated task to 'done'");
    
    // Try to get next task - should be None since we marked it as done
    let next = next_task(&db, None, None)?;
    assert!(next.is_none(), "Expected no next task when all are done");
    println!("✓ Confirmed no pending tasks");
    
    Ok(())
}

#[test]
fn e2e_task_priority_ordering() -> Result<()> {
    let db = test_db()?;
    
    // Create tasks with different priorities
    let id1 = add_task(&db, "Low Priority", "Should be last", 3, None)?;
    let id2 = add_task(&db, "High Priority", "Should be first", 1, None)?;
    let id3 = add_task(&db, "Medium Priority", "Should be middle", 2, None)?;
    
    println!("✓ Created 3 tasks with priorities 3, 1, 2");
    
    // Get next task - should be the highest priority (lowest number)
    let next = next_task(&db, None, None)?;
    assert!(next.is_some(), "Expected a next task");
    assert_eq!(next.unwrap().id, id2, "Expected highest priority task (id2)");
    println!("✓ Confirmed priority ordering works");
    
    Ok(())
}

#[test]
fn e2e_task_status_filtering() -> Result<()> {
    let db = test_db()?;
    
    // Create tasks with different statuses
    let id1 = add_task(&db, "Pending Task", "Should appear", 1, None)?;
    let id2 = add_task(&db, "Working Task", "Should appear", 2, None)?;
    let _id3 = add_task(&db, "Done Task", "Should NOT appear", 3, None)?;
    
    // Mark one as done
    update_task(&db, _id3, Some("done"), None, None)?;
    
    println!("✓ Created 3 tasks, marked one as done");
    
    // Get next task with status filter
    let next = next_task(&db, Some(&["pending", "working"]), None)?;
    assert!(next.is_some(), "Expected a next task with status filter");
    let task = next.unwrap();
    assert_eq!(task.id, id1, "Expected pending task to be first");
    println!("✓ Confirmed status filtering works");
    
    Ok(())
}