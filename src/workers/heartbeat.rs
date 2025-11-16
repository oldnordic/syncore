use anyhow::Result;
use rusqlite::Connection;
use tokio::time::{interval, Duration};
use crate::taskmaster;

pub async fn run(db_path: &str) -> Result<()> {
    let mut heartbeat_interval = interval(Duration::from_secs(900)); // 15 minutes
    
    loop {
        heartbeat_interval.tick().await;
        
        if let Err(e) = check_stuck_tasks(db_path) {
            eprintln!("Heartbeat check failed: {e}");
        }
    }
}

fn check_stuck_tasks(db_path: &str) -> Result<()> {
    let conn = Connection::open(db_path)?;
    
    // Find tasks stuck in progress for more than 15 minutes
    let fifteen_min_ago = chrono::Utc::now().timestamp() - (15 * 60);
    
    let mut stmt = conn.prepare(
        "SELECT id FROM tasks 
         WHERE status = 'in_progress' 
         AND updated_at < ?1"
    )?;
    
    let stuck_tasks = stmt.query_map([fifteen_min_ago], |row| {
        row.get::<_, i64>(0)
    })?;
    
    for task_id in stuck_tasks {
        let task_id = task_id?;
        println!("Requeuing stuck task: {task_id}");
        
        // Reset to open status
        taskmaster::update_task(&conn, task_id, Some("open"), None, None)?;
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use crate::db::ensure_schema;

    #[test]
    fn test_check_stuck_tasks() -> Result<()> {
        let temp_file = NamedTempFile::new()?;
        let db_path = temp_file.path().to_str().unwrap();
        
        ensure_schema(db_path)?;
        
        // Create a task that appears stuck
        let old_time = chrono::Utc::now().timestamp() - (20 * 60); // 20 minutes ago
        let conn = Connection::open(db_path)?;
        
        conn.execute(
            "INSERT INTO tasks (goal, description, status, priority, created_at, updated_at)
             VALUES ('test', 'test task', 'in_progress', 1, ?1, ?1)",
            [old_time],
        )?;
        
        // Check stuck tasks should reset it
        check_stuck_tasks(db_path)?;
        
        // Verify it's now open
        let status: String = conn.query_row(
            "SELECT status FROM tasks WHERE goal = 'test'",
            [],
            |row| row.get(0),
        )?;
        
        assert_eq!(status, "open");
        Ok(())
    }
}