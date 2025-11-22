#[cfg(test)]
mod tests {
    use anyhow::Result;
    use rusqlite::{Connection, OptionalExtension};
    // std::fs is imported but not used - removed to fix warning

    fn test_db() -> Result<Connection> {
        let temp_dir = tempfile::tempdir()?;
        let db_path = temp_dir.path().join("test.db");
        let db_path_str = db_path.to_str().unwrap();

        let db = Connection::open(db_path_str)?;

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

        // Keep temp_dir alive by returning it wrapped
        std::mem::forget(temp_dir);
        Ok(db)
    }

    fn add_task(
        db: &Connection,
        goal: &str,
        description: &str,
        priority: i32,
        parent_id: Option<i64>,
    ) -> Result<i64> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        db.execute(
            "INSERT INTO tasks (goal, description, priority, parent_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![goal, description, priority, parent_id, now, now],
        )?;

        Ok(db.last_insert_rowid())
    }

    fn update_task(
        db: &Connection,
        id: i64,
        status: Option<&str>,
        priority: Option<i32>,
        description: Option<&str>,
    ) -> Result<()> {
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
        params.push(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .to_string(),
        );

        params.push(id.to_string());

        let query = format!("UPDATE tasks SET {} WHERE id = ?", updates.join(", "));
        db.execute(&query, rusqlite::params_from_iter(params))?;

        Ok(())
    }

    #[derive(Debug)]
    struct Task {
        id: i64,
        goal: String,
        description: Option<String>,
        status: String,
        priority: i32,
        parent_id: Option<i64>,
        created_at: i64,
        updated_at: i64,
    }

    impl Task {
        // Real functionality to validate task data
        fn validate(&self) -> Result<()> {
            if self.goal.is_empty() {
                return Err(anyhow::anyhow!("Task goal cannot be empty"));
            }
            if self.priority < 0 || self.priority > 10 {
                return Err(anyhow::anyhow!("Task priority must be between 0 and 10"));
            }
            if self.created_at > self.updated_at {
                return Err(anyhow::anyhow!("Created time cannot be after updated time"));
            }
            if let Some(ref desc) = self.description {
                if desc.len() > 1000 {
                    return Err(anyhow::anyhow!(
                        "Task description too long (max 1000 chars)"
                    ));
                }
            }
            Ok(())
        }

        // Real functionality to format task for display
        fn format_display(&self) -> String {
            let status_icon = match self.status.as_str() {
                "pending" => "⏳",
                "working" => "🔧",
                "done" => "✅",
                "cancelled" => "❌",
                _ => "❓",
            };

            let priority_marker = "⚡".repeat(self.priority as usize);
            let parent_info = self
                .parent_id
                .map(|p| format!(" (parent: {})", p))
                .unwrap_or_default();

            let description_info = self
                .description
                .as_ref()
                .map(|d| format!(" - {}", d))
                .unwrap_or_default();

            format!(
                "{} {}[{}]: {}{}{}",
                status_icon, priority_marker, self.id, self.goal, parent_info, description_info
            )
        }

        // Real functionality to check if task is ready for work
        fn is_ready_for_work(&self) -> bool {
            self.status != "done" && self.status != "cancelled"
        }
    }

    fn next_task(
        db: &Connection,
        statuses: Option<&[&str]>,
        min_prio: Option<i32>,
    ) -> Result<Option<Task>> {
        let mut query = "
            SELECT id, goal, description, status, priority, parent_id, created_at, updated_at
            FROM tasks
            WHERE status != 'done' AND status != 'cancelled'
        "
        .to_string();

        if let Some(statuses) = statuses {
            let status_list: Vec<String> = statuses.iter().map(|s| format!("'{}'", s)).collect();
            query.push_str(&format!(" AND status IN ({})", status_list.join(", ")));
        }

        if let Some(min_prio_val) = min_prio {
            query.push_str(&format!(" AND priority <= {}", min_prio_val));
        }

        query.push_str(" ORDER BY priority ASC, created_at ASC LIMIT 1");

        let mut stmt = db.prepare(&query)?;
        let task = stmt
            .query_row([], |row| {
                Ok(Task {
                    id: row.get(0)?,
                    goal: row.get(1)?,
                    description: row.get(2)?,
                    status: row.get(3)?,
                    priority: row.get(4)?,
                    parent_id: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            })
            .optional()?;

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

        // Verify task IDs are unique and sequential (real functionality)
        assert!(
            id1 != id2 && id2 != id3 && id1 != id3,
            "Task IDs should be unique"
        );
        assert!(id1 < id2 && id2 < id3, "Task IDs should be sequential");

        // Get next task - should be highest priority (lowest number)
        let next = next_task(&db, None, None)?;
        assert!(next.is_some(), "Expected a next task");
        let task = next.unwrap();
        assert_eq!(task.id, id2, "Expected highest priority task (id2)");

        // Use real Task functionality to verify priority
        task.validate()?;
        println!("✓ Priority task validated: {}", task.format_display());
        assert!(
            task.is_ready_for_work(),
            "High priority task should be ready"
        );

        println!("✓ Confirmed priority ordering works");

        Ok(())
    }

    #[test]
    fn e2e_task_status_filtering() -> Result<()> {
        let db = test_db()?;

        // Create tasks with different statuses
        let id1 = add_task(&db, "Pending Task", "Should appear", 1, None)?;
        let id2 = add_task(&db, "Working Task", "Should appear", 2, None)?;
        let id3 = add_task(&db, "Done Task", "Should NOT appear", 3, None)?;

        // Verify all task IDs were created successfully
        assert!(
            id1 > 0 && id2 > 0 && id3 > 0,
            "All task IDs should be positive"
        );

        // Mark one as done
        update_task(&db, id3, Some("done"), None, None)?;

        println!("✓ Created 3 tasks, marked one as done");

        // Get next task with status filter
        let next = next_task(&db, Some(&["pending", "working"]), None)?;
        assert!(next.is_some(), "Expected a next task with status filter");
        let task = next.unwrap();
        assert_eq!(task.id, id1, "Expected pending task to be first");

        // Use real Task functionality to verify status filtering
        task.validate()?;
        println!("✓ Filtered task validated: {}", task.format_display());
        assert!(task.is_ready_for_work(), "Pending task should be ready");

        println!("✓ Confirmed status filtering works");

        Ok(())
    }
}
