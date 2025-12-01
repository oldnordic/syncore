use crate::intellitask::{Complexity, ParentTask, Subtask, TaskBreakdown};
use anyhow::{anyhow, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// Status for IntelliTask tasks matching TaskMaster.ai
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Review,
    Done,
    Deferred,
    Cancelled,
    Blocked,
}

impl TaskStatus {
    pub fn as_str(&self) -> &str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::InProgress => "in-progress",
            TaskStatus::Review => "review",
            TaskStatus::Done => "done",
            TaskStatus::Deferred => "deferred",
            TaskStatus::Cancelled => "cancelled",
            TaskStatus::Blocked => "blocked",
        }
    }

    pub fn try_parse(s: &str) -> Result<Self> {
        match s {
            "pending" => Ok(TaskStatus::Pending),
            "in-progress" => Ok(TaskStatus::InProgress),
            "review" => Ok(TaskStatus::Review),
            "done" => Ok(TaskStatus::Done),
            "deferred" => Ok(TaskStatus::Deferred),
            "cancelled" => Ok(TaskStatus::Cancelled),
            "blocked" => Ok(TaskStatus::Blocked),
            _ => Err(anyhow!("Unknown status: {}", s)),
        }
    }
}

/// Extended task structure with IntelliTask fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelliTask {
    pub id: i64,
    pub task_id: String, // e.g., "1.0" or "1.1"
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub complexity: Complexity,
    pub estimated_hours: f32,
    pub parent_id: Option<i64>,
    pub prd_title: Option<String>,
    pub dependencies: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub files_to_modify: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Filter for querying tasks
#[derive(Debug, Clone, Default)]
pub struct TaskFilter {
    pub status: Option<TaskStatus>,
    pub prd_title: Option<String>,
    pub parent_id: Option<i64>,
}

/// Updates for a task
#[derive(Debug, Clone, Default)]
pub struct TaskUpdates {
    pub description: Option<String>,
    pub status: Option<TaskStatus>,
    pub complexity: Option<Complexity>,
    pub estimated_hours: Option<f32>,
    pub acceptance_criteria: Option<Vec<String>>,
    pub files_to_modify: Option<Vec<String>>,
}

/// Statistics for subtasks of a parent task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtaskStatistics {
    pub total: i32,
    pub completed: i32,
    pub pending: i32,
    pub in_progress: i32,
    pub blocked: i32,
    pub progress_percent: i32,
}

/// Statistics for all tasks (overall or per-PRD)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStatistics {
    pub total: i32,
    pub completed: i32,
    pub pending: i32,
    pub in_progress: i32,
    pub blocked: i32,
    pub review: i32,
    pub deferred: i32,
    pub cancelled: i32,
    pub progress_percent: i32,
}

/// IntelliTask persistence layer
pub struct IntelliTaskPersistence {
    db: Arc<Mutex<Connection>>,
}

impl IntelliTaskPersistence {
    /// Create IntelliTaskPersistence using an existing database connection from DbManager.
    ///
    /// This is the preferred constructor when using DbManager. It reuses long-lived
    /// connections instead of creating new ones per-call.
    ///
    /// # Arguments
    ///
    /// * `db` - Arc<Mutex<Connection>> from DbManager.main_conn()
    ///
    /// # Example
    ///
    /// ```rust
    /// let db_manager = DbManager::new("syncore.db", "syncore_code_graph.db")?;
    /// let intellitask = IntelliTaskPersistence::with_connection(db_manager.main_conn())?;
    /// ```
    pub fn with_connection(db: Arc<Mutex<Connection>>) -> Result<Self> {
        // Ensure schema extensions exist
        {
            let conn = db.lock().unwrap();
            Self::ensure_schema(&conn)?;
        }

        Ok(Self {
            db,
        })
    }

    /// Legacy constructor - opens its own connection (deprecated, use with_connection instead).
    ///
    /// This method is kept for backward compatibility with existing code that hasn't
    /// been refactored to use DbManager yet.
    pub fn new(db_path: &str) -> Result<Self> {
        // Ensure base schema first
        crate::db::ensure_schema(db_path)?;

        let conn = crate::db::open_db_with_wal(db_path)?;

        // Ensure schema extensions exist
        Self::ensure_schema(&conn)?;

        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
        })
    }

    /// Ensure schema extensions for IntelliTask
    fn ensure_schema(db: &Connection) -> Result<()> {
        // Helper to check if column exists
        fn column_exists(db: &Connection, column: &str) -> bool {
            db.prepare("SELECT COUNT(*) FROM pragma_table_info('tasks') WHERE name=?1")
                .and_then(|mut stmt| stmt.query_row([column], |row| row.get::<_, i32>(0)))
                .map(|count| count > 0)
                .unwrap_or(false)
        }

        // Add each column individually only if it doesn't exist (idempotent)
        let columns_to_add = vec![
            ("task_type", "TEXT DEFAULT 'task'"),
            ("task_id", "TEXT"),
            ("prd_title", "TEXT"),
            ("complexity", "TEXT"),
            ("estimated_hours", "REAL"),
            ("acceptance_criteria", "TEXT"),
            ("files_to_modify", "TEXT"),
        ];

        for (col_name, col_type) in columns_to_add {
            if !column_exists(db, col_name) {
                let sql = format!("ALTER TABLE tasks ADD COLUMN {} {}", col_name, col_type);
                db.execute(&sql, [])?;
            }
        }

        // Create indexes (IF NOT EXISTS is idempotent)
        db.execute_batch(
            r#"
            CREATE INDEX IF NOT EXISTS idx_tasks_task_id ON tasks(task_id);
            CREATE INDEX IF NOT EXISTS idx_tasks_prd ON tasks(prd_title);
        "#,
        )?;

        Ok(())
    }

    /// Save TaskBreakdown to database
    pub fn save_task_breakdown(&self, breakdown: &TaskBreakdown) -> Result<()> {
        let db = self.db.lock().unwrap();

        // First pass: Save all parent tasks and subtasks
        let mut parent_db_ids = Vec::new();
        for parent_task in &breakdown.parent_tasks {
            let parent_db_id = self.save_parent_task(&db, parent_task, &breakdown.prd_title)?;
            parent_db_ids.push(parent_db_id);

            // Save subtasks
            for subtask in &parent_task.subtasks {
                self.save_subtask(&db, subtask, parent_db_id, &breakdown.prd_title)?;
            }
        }

        // Second pass: Save dependencies (now all tasks exist in DB)
        for (i, parent_task) in breakdown.parent_tasks.iter().enumerate() {
            let parent_db_id = parent_db_ids[i];
            self.save_dependencies(&db, parent_db_id, &parent_task.dependencies)?;
        }

        Ok(())
    }

    fn save_parent_task(&self, db: &Connection, task: &ParentTask, prd_title: &str) -> Result<i64> {
        let now =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
                as i64;

        db.execute(
            "INSERT INTO tasks (
                goal, description, status, priority, task_type, task_id,
                prd_title, complexity, estimated_hours, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
            (
                &task.title,
                &task.description,
                TaskStatus::Pending.as_str(),
                1, // Default priority
                "parent",
                &task.id,
                prd_title,
                format!("{:?}", task.complexity),
                task.estimated_hours,
                now,
            ),
        )?;

        Ok(db.last_insert_rowid())
    }

    fn save_subtask(
        &self,
        db: &Connection,
        subtask: &Subtask,
        parent_id: i64,
        prd_title: &str,
    ) -> Result<i64> {
        let now =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
                as i64;

        let acceptance_json = serde_json::to_string(&subtask.acceptance_criteria)?;
        let files_json = serde_json::to_string(&subtask.files_to_modify)?;

        db.execute(
            "INSERT INTO tasks (
                goal, description, status, priority, parent_id, task_type, task_id,
                prd_title, complexity, estimated_hours, acceptance_criteria,
                files_to_modify, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)",
            (
                &subtask.description,
                &subtask.description,
                TaskStatus::Pending.as_str(),
                2, // Subtasks have lower priority
                parent_id,
                "subtask",
                &subtask.id,
                prd_title,
                format!("{:?}", subtask.complexity),
                subtask.estimated_hours,
                acceptance_json,
                files_json,
                now,
            ),
        )?;

        Ok(db.last_insert_rowid())
    }

    fn save_dependencies(
        &self,
        db: &Connection,
        task_db_id: i64,
        dep_task_ids: &[String],
    ) -> Result<()> {
        // For each dependency task_id string (e.g., "1.0"), find the DB id and create link
        for dep_task_id_str in dep_task_ids {
            // Find the task with this task_id
            let mut stmt = db.prepare("SELECT id FROM tasks WHERE task_id = ?1")?;
            if let Ok(dep_db_id) = stmt.query_row([dep_task_id_str], |row| row.get::<_, i64>(0)) {
                // Create dependency link: task_db_id depends_on dep_db_id
                db.execute(
                    "INSERT OR REPLACE INTO task_links (src_id, dst_id, kind) VALUES (?1, ?2, 'depends_on')",
                    (task_db_id, dep_db_id),
                )?;
            }
        }
        Ok(())
    }

    /// Get task dependencies (tasks that this task depends on)
    pub fn get_task_dependencies(&self, task_id: i64) -> Result<Vec<i64>> {
        let db = self.db.lock().unwrap();
        let mut stmt =
            db.prepare("SELECT dst_id FROM task_links WHERE src_id = ?1 AND kind = 'depends_on'")?;

        let deps = stmt.query_map([task_id], |row| row.get(0))?;
        deps.collect::<Result<Vec<_>, _>>()
            .map_err(|e| anyhow!("Failed to get dependencies: {}", e))
    }

    /// Get tasks that depend on this task
    pub fn get_dependent_tasks(&self, task_id: i64) -> Result<Vec<i64>> {
        let db = self.db.lock().unwrap();
        let mut stmt =
            db.prepare("SELECT src_id FROM task_links WHERE dst_id = ?1 AND kind = 'depends_on'")?;

        let deps = stmt.query_map([task_id], |row| row.get(0))?;
        deps.collect::<Result<Vec<_>, _>>()
            .map_err(|e| anyhow!("Failed to get dependent tasks: {}", e))
    }

    /// Check if all dependencies are satisfied (done)
    pub fn are_dependencies_satisfied(&self, task_id: i64) -> Result<bool> {
        let db = self.db.lock().unwrap();
        let count: i64 = db.query_row(
            "SELECT COUNT(*)
             FROM task_links tl
             JOIN tasks t ON t.id = tl.dst_id
             WHERE tl.src_id = ?1 AND tl.kind = 'depends_on'
               AND t.status != 'done'",
            [task_id],
            |row| row.get(0),
        )?;

        Ok(count == 0)
    }

    /// Find the next task to work on
    /// Returns a Pending task with all dependencies satisfied, ordered by priority
    pub fn next_task(&self) -> Result<Option<IntelliTask>> {
        let pending_tasks = self.get_tasks(Some(TaskFilter {
            status: Some(TaskStatus::Pending),
            ..Default::default()
        }))?;

        // Find first task with satisfied dependencies
        for task in pending_tasks {
            if self.are_dependencies_satisfied(task.id)? {
                return Ok(Some(task));
            }
        }

        Ok(None)
    }

    /// Get all subtasks for a parent task
    pub fn get_subtasks(&self, parent_id: i64) -> Result<Vec<IntelliTask>> {
        self.get_tasks(Some(TaskFilter {
            parent_id: Some(parent_id),
            ..Default::default()
        }))
    }

    /// Get statistics for subtasks of a parent task
    pub fn get_subtask_statistics(&self, parent_id: i64) -> Result<SubtaskStatistics> {
        let db = self.db.lock().unwrap();

        let total: i32 =
            db.query_row("SELECT COUNT(*) FROM tasks WHERE parent_id = ?1", [parent_id], |row| {
                row.get(0)
            })?;

        let completed: i32 = db.query_row(
            "SELECT COUNT(*) FROM tasks WHERE parent_id = ?1 AND status = 'done'",
            [parent_id],
            |row| row.get(0),
        )?;

        let pending: i32 = db.query_row(
            "SELECT COUNT(*) FROM tasks WHERE parent_id = ?1 AND status = 'pending'",
            [parent_id],
            |row| row.get(0),
        )?;

        let in_progress: i32 = db.query_row(
            "SELECT COUNT(*) FROM tasks WHERE parent_id = ?1 AND status = 'in-progress'",
            [parent_id],
            |row| row.get(0),
        )?;

        let blocked: i32 = db.query_row(
            "SELECT COUNT(*) FROM tasks WHERE parent_id = ?1 AND status = 'blocked'",
            [parent_id],
            |row| row.get(0),
        )?;

        let progress_percent = if total > 0 {
            ((completed as f32 / total as f32) * 100.0) as i32
        } else {
            0
        };

        Ok(SubtaskStatistics {
            total,
            completed,
            pending,
            in_progress,
            blocked,
            progress_percent,
        })
    }

    /// Get overall statistics for all tasks
    pub fn get_task_statistics(&self) -> Result<TaskStatistics> {
        let db = self.db.lock().unwrap();

        let total: i32 = db.query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))?;

        let completed: i32 =
            db.query_row("SELECT COUNT(*) FROM tasks WHERE status = 'done'", [], |row| row.get(0))?;

        let pending: i32 =
            db.query_row("SELECT COUNT(*) FROM tasks WHERE status = 'pending'", [], |row| {
                row.get(0)
            })?;

        let in_progress: i32 =
            db.query_row("SELECT COUNT(*) FROM tasks WHERE status = 'in-progress'", [], |row| {
                row.get(0)
            })?;

        let blocked: i32 =
            db.query_row("SELECT COUNT(*) FROM tasks WHERE status = 'blocked'", [], |row| {
                row.get(0)
            })?;

        let review: i32 =
            db.query_row("SELECT COUNT(*) FROM tasks WHERE status = 'review'", [], |row| {
                row.get(0)
            })?;

        let deferred: i32 =
            db.query_row("SELECT COUNT(*) FROM tasks WHERE status = 'deferred'", [], |row| {
                row.get(0)
            })?;

        let cancelled: i32 =
            db.query_row("SELECT COUNT(*) FROM tasks WHERE status = 'cancelled'", [], |row| {
                row.get(0)
            })?;

        let progress_percent = if total > 0 {
            ((completed as f32 / total as f32) * 100.0) as i32
        } else {
            0
        };

        Ok(TaskStatistics {
            total,
            completed,
            pending,
            in_progress,
            blocked,
            review,
            deferred,
            cancelled,
            progress_percent,
        })
    }

    /// Get statistics for tasks belonging to a specific PRD
    pub fn get_prd_statistics(&self, prd_title: &str) -> Result<TaskStatistics> {
        let db = self.db.lock().unwrap();

        let total: i32 =
            db.query_row("SELECT COUNT(*) FROM tasks WHERE prd_title = ?1", [prd_title], |row| {
                row.get(0)
            })?;

        let completed: i32 = db.query_row(
            "SELECT COUNT(*) FROM tasks WHERE prd_title = ?1 AND status = 'done'",
            [prd_title],
            |row| row.get(0),
        )?;

        let pending: i32 = db.query_row(
            "SELECT COUNT(*) FROM tasks WHERE prd_title = ?1 AND status = 'pending'",
            [prd_title],
            |row| row.get(0),
        )?;

        let in_progress: i32 = db.query_row(
            "SELECT COUNT(*) FROM tasks WHERE prd_title = ?1 AND status = 'in-progress'",
            [prd_title],
            |row| row.get(0),
        )?;

        let blocked: i32 = db.query_row(
            "SELECT COUNT(*) FROM tasks WHERE prd_title = ?1 AND status = 'blocked'",
            [prd_title],
            |row| row.get(0),
        )?;

        let review: i32 = db.query_row(
            "SELECT COUNT(*) FROM tasks WHERE prd_title = ?1 AND status = 'review'",
            [prd_title],
            |row| row.get(0),
        )?;

        let deferred: i32 = db.query_row(
            "SELECT COUNT(*) FROM tasks WHERE prd_title = ?1 AND status = 'deferred'",
            [prd_title],
            |row| row.get(0),
        )?;

        let cancelled: i32 = db.query_row(
            "SELECT COUNT(*) FROM tasks WHERE prd_title = ?1 AND status = 'cancelled'",
            [prd_title],
            |row| row.get(0),
        )?;

        let progress_percent = if total > 0 {
            ((completed as f32 / total as f32) * 100.0) as i32
        } else {
            0
        };

        Ok(TaskStatistics {
            total,
            completed,
            pending,
            in_progress,
            blocked,
            review,
            deferred,
            cancelled,
            progress_percent,
        })
    }

    /// Get all tasks with optional filtering
    pub fn get_tasks(&self, filter: Option<TaskFilter>) -> Result<Vec<IntelliTask>> {
        let db = self.db.lock().unwrap();
        let filter = filter.unwrap_or_default();

        let mut query = "SELECT id, task_id, goal, description, status, complexity, estimated_hours,
                         parent_id, prd_title, acceptance_criteria, files_to_modify, created_at, updated_at
                         FROM tasks WHERE 1=1".to_string();

        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![];

        if let Some(status) = filter.status {
            query.push_str(&format!(" AND status = ?{}", params.len() + 1));
            params.push(Box::new(status.as_str().to_string()));
        }

        if let Some(prd_title) = filter.prd_title {
            query.push_str(&format!(" AND prd_title = ?{}", params.len() + 1));
            params.push(Box::new(prd_title));
        }

        if let Some(parent_id) = filter.parent_id {
            query.push_str(&format!(" AND parent_id = ?{}", params.len() + 1));
            params.push(Box::new(parent_id));
        }

        query.push_str(" ORDER BY created_at ASC");

        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let mut stmt = db.prepare(&query)?;

        let mut tasks: Vec<IntelliTask> = stmt
            .query_map(rusqlite::params_from_iter(param_refs), |row| {
                Ok(IntelliTask {
                    id: row.get(0)?,
                    task_id: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    title: row.get(2)?,
                    description: row.get(3)?,
                    status: TaskStatus::try_parse(&row.get::<_, String>(4)?)
                        .unwrap_or(TaskStatus::Pending),
                    complexity: serde_json::from_str(
                        &row.get::<_, Option<String>>(5)?
                            .unwrap_or_else(|| "\"Simple\"".to_string()),
                    )
                    .unwrap_or(Complexity::Simple),
                    estimated_hours: row.get::<_, Option<f32>>(6)?.unwrap_or(1.0),
                    parent_id: row.get(7)?,
                    prd_title: row.get(8)?,
                    dependencies: vec![], // Will be populated below
                    acceptance_criteria: serde_json::from_str(
                        &row.get::<_, Option<String>>(9)?.unwrap_or_else(|| "[]".to_string()),
                    )
                    .unwrap_or_default(),
                    files_to_modify: serde_json::from_str(
                        &row.get::<_, Option<String>>(10)?.unwrap_or_else(|| "[]".to_string()),
                    )
                    .unwrap_or_default(),
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| anyhow!("Failed to collect tasks: {}", e))?;

        // Load dependencies for each task
        for task in &mut tasks {
            // Get task IDs this task depends on
            let mut stmt = db.prepare(
                "SELECT t.task_id FROM task_links tl
                 JOIN tasks t ON t.id = tl.dst_id
                 WHERE tl.src_id = ?1 AND tl.kind = 'depends_on'",
            )?;

            let deps: Vec<String> = stmt
                .query_map([task.id], |row| row.get(0))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| anyhow!("Failed to load dependencies: {}", e))?;

            task.dependencies = deps;
        }

        Ok(tasks)
    }

    /// Get task by ID
    pub fn get_task(&self, id: i64) -> Result<Option<IntelliTask>> {
        let tasks = self.get_tasks(Some(TaskFilter::default()))?;
        Ok(tasks.into_iter().find(|t| t.id == id))
    }

    /// Update task status
    pub fn update_task_status(&self, id: i64, status: TaskStatus) -> Result<()> {
        let db = self.db.lock().unwrap();
        let now =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
                as i64;

        db.execute(
            "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id = ?3",
            (status.as_str(), now, id),
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intellitask::FileAction;
    use tempfile::NamedTempFile;

    fn create_test_breakdown() -> TaskBreakdown {
        TaskBreakdown {
            prd_title: "Test Project".to_string(),
            parent_tasks: vec![ParentTask {
                id: "1.0".to_string(),
                title: "Implement Feature X".to_string(),
                description: "Build the feature".to_string(),
                subtasks: vec![
                    Subtask {
                        id: "1.1".to_string(),
                        description: "Write tests".to_string(),
                        acceptance_criteria: vec!["Tests pass".to_string()],
                        dependencies: vec![],
                        files_to_modify: vec!["tests/test.rs".to_string()],
                        complexity: Complexity::Simple,
                        estimated_hours: 2.0,
                    },
                    Subtask {
                        id: "1.2".to_string(),
                        description: "Implement logic".to_string(),
                        acceptance_criteria: vec!["Code works".to_string()],
                        dependencies: vec!["1.1".to_string()],
                        files_to_modify: vec!["src/lib.rs".to_string()],
                        complexity: Complexity::Moderate,
                        estimated_hours: 4.0,
                    },
                ],
                dependencies: vec![],
                complexity: Complexity::Moderate,
                estimated_hours: 6.0,
            }],
            relevant_files: vec![crate::intellitask::FileReference {
                path: "src/lib.rs".to_string(),
                purpose: "Main implementation".to_string(),
                action: FileAction::Modify,
            }],
            estimated_complexity: Complexity::Moderate,
        }
    }

    #[test]
    fn test_save_task_breakdown_to_db() -> Result<()> {
        let temp_db = NamedTempFile::new()?;
        let db_path = temp_db.path().to_str().unwrap();

        let persistence = IntelliTaskPersistence::new(db_path)?;
        let breakdown = create_test_breakdown();

        persistence.save_task_breakdown(&breakdown)?;

        // Verify tasks exist
        let tasks = persistence.get_tasks(None)?;
        assert_eq!(tasks.len(), 3); // 1 parent + 2 subtasks

        // Verify parent task
        let parent = tasks.iter().find(|t| t.task_id == "1.0").unwrap();
        assert_eq!(parent.title, "Implement Feature X");
        assert_eq!(parent.status, TaskStatus::Pending);

        // Verify subtasks
        let subtask1 = tasks.iter().find(|t| t.task_id == "1.1").unwrap();
        assert_eq!(subtask1.description, "Write tests");
        assert_eq!(subtask1.estimated_hours, 2.0);

        Ok(())
    }

    #[test]
    fn test_get_tasks_by_status() -> Result<()> {
        let temp_db = NamedTempFile::new()?;
        let db_path = temp_db.path().to_str().unwrap();

        let persistence = IntelliTaskPersistence::new(db_path)?;
        let breakdown = create_test_breakdown();
        persistence.save_task_breakdown(&breakdown)?;

        // All should be pending initially
        let pending_tasks = persistence.get_tasks(Some(TaskFilter {
            status: Some(TaskStatus::Pending),
            ..Default::default()
        }))?;
        assert_eq!(pending_tasks.len(), 3);

        // Update one task
        let task_id = pending_tasks[0].id;
        persistence.update_task_status(task_id, TaskStatus::InProgress)?;

        // Now should have 2 pending, 1 in-progress
        let pending = persistence.get_tasks(Some(TaskFilter {
            status: Some(TaskStatus::Pending),
            ..Default::default()
        }))?;
        assert_eq!(pending.len(), 2);

        let in_progress = persistence.get_tasks(Some(TaskFilter {
            status: Some(TaskStatus::InProgress),
            ..Default::default()
        }))?;
        assert_eq!(in_progress.len(), 1);

        Ok(())
    }

    #[test]
    fn test_update_task_status() -> Result<()> {
        let temp_db = NamedTempFile::new()?;
        let db_path = temp_db.path().to_str().unwrap();

        let persistence = IntelliTaskPersistence::new(db_path)?;
        let breakdown = create_test_breakdown();
        persistence.save_task_breakdown(&breakdown)?;

        let tasks = persistence.get_tasks(None)?;
        let task_id = tasks[0].id;

        // Update status
        persistence.update_task_status(task_id, TaskStatus::Done)?;

        // Verify status changed
        let updated_task = persistence.get_task(task_id)?.unwrap();
        assert_eq!(updated_task.status, TaskStatus::Done);

        Ok(())
    }

    #[test]
    fn test_get_task_by_id() -> Result<()> {
        let temp_db = NamedTempFile::new()?;
        let db_path = temp_db.path().to_str().unwrap();

        let persistence = IntelliTaskPersistence::new(db_path)?;
        let breakdown = create_test_breakdown();
        persistence.save_task_breakdown(&breakdown)?;

        let tasks = persistence.get_tasks(None)?;
        let task_id = tasks[0].id;

        // Get specific task
        let task = persistence.get_task(task_id)?;
        assert!(task.is_some());
        assert_eq!(task.unwrap().id, task_id);

        // Try non-existent task
        let missing = persistence.get_task(9999)?;
        assert!(missing.is_none());

        Ok(())
    }

    #[test]
    fn test_get_all_tasks() -> Result<()> {
        let temp_db = NamedTempFile::new()?;
        let db_path = temp_db.path().to_str().unwrap();

        let persistence = IntelliTaskPersistence::new(db_path)?;
        let breakdown = create_test_breakdown();
        persistence.save_task_breakdown(&breakdown)?;

        let all_tasks = persistence.get_tasks(None)?;
        assert_eq!(all_tasks.len(), 3);

        Ok(())
    }

    #[test]
    fn test_task_status_serialization() {
        assert_eq!(TaskStatus::Pending.as_str(), "pending");
        assert_eq!(TaskStatus::InProgress.as_str(), "in-progress");
        assert_eq!(TaskStatus::Done.as_str(), "done");

        assert_eq!(TaskStatus::try_parse("pending").unwrap(), TaskStatus::Pending);
        assert_eq!(TaskStatus::try_parse("in-progress").unwrap(), TaskStatus::InProgress);
        assert!(TaskStatus::try_parse("invalid").is_err());
    }

    #[test]
    fn test_save_task_dependencies() -> Result<()> {
        let temp_db = NamedTempFile::new()?;
        let db_path = temp_db.path().to_str().unwrap();

        let persistence = IntelliTaskPersistence::new(db_path)?;

        // Create task with dependencies
        let mut breakdown = create_test_breakdown();
        breakdown.parent_tasks[0].dependencies = vec!["2.0".to_string()];

        // Add second parent task that first depends on
        breakdown.parent_tasks.push(ParentTask {
            id: "2.0".to_string(),
            title: "Setup task".to_string(),
            description: "Must be done first".to_string(),
            subtasks: vec![],
            dependencies: vec![],
            complexity: Complexity::Simple,
            estimated_hours: 1.0,
        });

        persistence.save_task_breakdown(&breakdown)?;

        // Get first task
        let tasks = persistence.get_tasks(None)?;
        let task1 = tasks.iter().find(|t| t.task_id == "1.0").unwrap();

        // Verify dependencies saved
        let deps = persistence.get_task_dependencies(task1.id)?;
        assert_eq!(deps.len(), 1);

        Ok(())
    }

    #[test]
    fn test_get_dependent_tasks() -> Result<()> {
        let temp_db = NamedTempFile::new()?;
        let db_path = temp_db.path().to_str().unwrap();

        let persistence = IntelliTaskPersistence::new(db_path)?;

        let mut breakdown = create_test_breakdown();
        breakdown.parent_tasks[0].dependencies = vec!["2.0".to_string()];
        breakdown.parent_tasks.push(ParentTask {
            id: "2.0".to_string(),
            title: "Setup task".to_string(),
            description: "Must be done first".to_string(),
            subtasks: vec![],
            dependencies: vec![],
            complexity: Complexity::Simple,
            estimated_hours: 1.0,
        });

        persistence.save_task_breakdown(&breakdown)?;

        let tasks = persistence.get_tasks(None)?;
        let task2 = tasks.iter().find(|t| t.task_id == "2.0").unwrap();

        // Task 1 depends on Task 2, so Task 2 should have Task 1 as dependent
        let dependents = persistence.get_dependent_tasks(task2.id)?;
        assert_eq!(dependents.len(), 1);

        Ok(())
    }

    #[test]
    fn test_are_dependencies_satisfied() -> Result<()> {
        let temp_db = NamedTempFile::new()?;
        let db_path = temp_db.path().to_str().unwrap();

        let persistence = IntelliTaskPersistence::new(db_path)?;

        let mut breakdown = create_test_breakdown();
        breakdown.parent_tasks[0].dependencies = vec!["2.0".to_string()];
        breakdown.parent_tasks.push(ParentTask {
            id: "2.0".to_string(),
            title: "Setup task".to_string(),
            description: "Must be done first".to_string(),
            subtasks: vec![],
            dependencies: vec![],
            complexity: Complexity::Simple,
            estimated_hours: 1.0,
        });

        persistence.save_task_breakdown(&breakdown)?;

        let tasks = persistence.get_tasks(None)?;
        let task1 = tasks.iter().find(|t| t.task_id == "1.0").unwrap();
        let task2 = tasks.iter().find(|t| t.task_id == "2.0").unwrap();

        // Task 2 is pending, so Task 1's dependencies NOT satisfied
        assert!(!persistence.are_dependencies_satisfied(task1.id)?);

        // Complete Task 2
        persistence.update_task_status(task2.id, TaskStatus::Done)?;

        // Now Task 1's dependencies should be satisfied
        assert!(persistence.are_dependencies_satisfied(task1.id)?);

        Ok(())
    }

    #[test]
    fn test_next_task_simple() -> Result<()> {
        let temp_db = NamedTempFile::new()?;
        let db_path = temp_db.path().to_str().unwrap();

        let persistence = IntelliTaskPersistence::new(db_path)?;
        let breakdown = create_test_breakdown();
        persistence.save_task_breakdown(&breakdown)?;

        // Get next task - should be the only pending task
        let next = persistence.next_task()?;
        assert!(next.is_some());
        let task = next.unwrap();
        assert_eq!(task.task_id, "1.0");
        assert_eq!(task.status, TaskStatus::Pending);

        Ok(())
    }

    #[test]
    fn test_next_task_respects_dependencies() -> Result<()> {
        let temp_db = NamedTempFile::new()?;
        let db_path = temp_db.path().to_str().unwrap();

        let persistence = IntelliTaskPersistence::new(db_path)?;
        let mut breakdown = create_test_breakdown();

        // Task 1 depends on Task 2
        breakdown.parent_tasks[0].dependencies = vec!["2.0".to_string()];
        breakdown.parent_tasks.push(ParentTask {
            id: "2.0".to_string(),
            title: "Setup task".to_string(),
            description: "Must be done first".to_string(),
            subtasks: vec![],
            dependencies: vec![],
            complexity: Complexity::Simple,
            estimated_hours: 1.0,
        });

        persistence.save_task_breakdown(&breakdown)?;

        // Next task should be Task 2 (no dependencies)
        let next = persistence.next_task()?;
        assert!(next.is_some());
        assert_eq!(next.unwrap().task_id, "2.0");

        Ok(())
    }

    #[test]
    fn test_next_task_skips_in_progress() -> Result<()> {
        let temp_db = NamedTempFile::new()?;
        let db_path = temp_db.path().to_str().unwrap();

        let persistence = IntelliTaskPersistence::new(db_path)?;
        let mut breakdown = create_test_breakdown();
        breakdown.parent_tasks.push(ParentTask {
            id: "2.0".to_string(),
            title: "Another task".to_string(),
            description: "Second task".to_string(),
            subtasks: vec![],
            dependencies: vec![],
            complexity: Complexity::Simple,
            estimated_hours: 1.0,
        });

        persistence.save_task_breakdown(&breakdown)?;

        // Mark first task as in-progress
        let tasks = persistence.get_tasks(None)?;
        let task1 = tasks.iter().find(|t| t.task_id == "1.0").unwrap();
        persistence.update_task_status(task1.id, TaskStatus::InProgress)?;

        // Next task should be Task 2 (Task 1 is in-progress)
        let next = persistence.next_task()?;
        assert!(next.is_some());
        assert_eq!(next.unwrap().task_id, "2.0");

        Ok(())
    }

    #[test]
    fn test_next_task_none_available() -> Result<()> {
        let temp_db = NamedTempFile::new()?;
        let db_path = temp_db.path().to_str().unwrap();

        let persistence = IntelliTaskPersistence::new(db_path)?;
        let breakdown = create_test_breakdown();
        persistence.save_task_breakdown(&breakdown)?;

        // Mark all tasks as done
        let tasks = persistence.get_tasks(None)?;
        for task in tasks {
            persistence.update_task_status(task.id, TaskStatus::Done)?;
        }

        // No next task available
        let next = persistence.next_task()?;
        assert!(next.is_none());

        Ok(())
    }

    #[test]
    fn test_get_subtasks() -> Result<()> {
        let temp_db = NamedTempFile::new()?;
        let db_path = temp_db.path().to_str().unwrap();

        let persistence = IntelliTaskPersistence::new(db_path)?;

        // Create breakdown with subtasks
        let mut breakdown = create_test_breakdown();
        breakdown.parent_tasks[0].subtasks = vec![
            Subtask {
                id: "1.1".to_string(),
                description: "First subtask".to_string(),
                acceptance_criteria: vec!["Test passes".to_string()],
                dependencies: vec![],
                files_to_modify: vec!["file1.rs".to_string()],
                complexity: Complexity::Simple,
                estimated_hours: 1.0,
            },
            Subtask {
                id: "1.2".to_string(),
                description: "Second subtask".to_string(),
                acceptance_criteria: vec!["Code works".to_string()],
                dependencies: vec![],
                files_to_modify: vec!["file2.rs".to_string()],
                complexity: Complexity::Moderate,
                estimated_hours: 2.0,
            },
        ];

        persistence.save_task_breakdown(&breakdown)?;

        // Get parent task
        let tasks = persistence.get_tasks(None)?;
        let parent = tasks.iter().find(|t| t.task_id == "1.0").unwrap();

        // Get subtasks
        let subtasks = persistence.get_subtasks(parent.id)?;
        assert_eq!(subtasks.len(), 2);
        assert_eq!(subtasks[0].task_id, "1.1");
        assert_eq!(subtasks[1].task_id, "1.2");

        Ok(())
    }

    #[test]
    fn test_subtask_statistics() -> Result<()> {
        let temp_db = NamedTempFile::new()?;
        let db_path = temp_db.path().to_str().unwrap();

        let persistence = IntelliTaskPersistence::new(db_path)?;

        // Create breakdown with subtasks
        let mut breakdown = create_test_breakdown();
        breakdown.parent_tasks[0].subtasks = vec![
            Subtask {
                id: "1.1".to_string(),
                description: "First subtask".to_string(),
                acceptance_criteria: vec![],
                dependencies: vec![],
                files_to_modify: vec![],
                complexity: Complexity::Simple,
                estimated_hours: 1.0,
            },
            Subtask {
                id: "1.2".to_string(),
                description: "Second subtask".to_string(),
                acceptance_criteria: vec![],
                dependencies: vec![],
                files_to_modify: vec![],
                complexity: Complexity::Simple,
                estimated_hours: 1.0,
            },
        ];

        persistence.save_task_breakdown(&breakdown)?;

        // Get parent task
        let tasks = persistence.get_tasks(None)?;
        let parent = tasks.iter().find(|t| t.task_id == "1.0").unwrap();

        // Mark one subtask as done
        let subtasks = persistence.get_subtasks(parent.id)?;
        persistence.update_task_status(subtasks[0].id, TaskStatus::Done)?;

        // Get statistics
        let stats = persistence.get_subtask_statistics(parent.id)?;
        assert_eq!(stats.total, 2);
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.pending, 1);
        assert_eq!(stats.progress_percent, 50);

        Ok(())
    }

    #[test]
    fn test_get_overall_task_statistics() -> Result<()> {
        let temp_db = NamedTempFile::new()?;
        let db_path = temp_db.path().to_str().unwrap();

        let persistence = IntelliTaskPersistence::new(db_path)?;

        // Create tasks with different statuses
        let mut breakdown = create_test_breakdown();
        breakdown.parent_tasks.push(ParentTask {
            id: "2.0".to_string(),
            title: "Second task".to_string(),
            description: "Another task".to_string(),
            subtasks: vec![],
            dependencies: vec![],
            complexity: Complexity::Simple,
            estimated_hours: 2.0,
        });
        breakdown.parent_tasks.push(ParentTask {
            id: "3.0".to_string(),
            title: "Third task".to_string(),
            description: "Yet another".to_string(),
            subtasks: vec![],
            dependencies: vec![],
            complexity: Complexity::Simple,
            estimated_hours: 3.0,
        });

        persistence.save_task_breakdown(&breakdown)?;

        let tasks = persistence.get_tasks(None)?;

        // Mark first as done
        persistence.update_task_status(tasks[0].id, TaskStatus::Done)?;

        // Mark second as in-progress
        persistence.update_task_status(tasks[1].id, TaskStatus::InProgress)?;

        // Get overall statistics
        let stats = persistence.get_task_statistics()?;

        assert_eq!(stats.total, 5); // 1 parent + 2 subtasks + 2 more parents = 5 tasks
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.in_progress, 1);
        assert_eq!(stats.pending, 3); // 3 tasks still pending
        assert_eq!(stats.progress_percent, 20); // 1/5 = 20%

        Ok(())
    }

    #[test]
    fn test_get_prd_statistics() -> Result<()> {
        let temp_db = NamedTempFile::new()?;
        let db_path = temp_db.path().to_str().unwrap();

        let persistence = IntelliTaskPersistence::new(db_path)?;

        // Create two different PRDs
        let mut breakdown1 = create_test_breakdown();
        breakdown1.prd_title = "PRD One".to_string();
        breakdown1.parent_tasks[0].subtasks = vec![];

        let breakdown2 = TaskBreakdown {
            prd_title: "PRD Two".to_string(),
            parent_tasks: vec![
                ParentTask {
                    id: "1.0".to_string(),
                    title: "Task A".to_string(),
                    description: "First".to_string(),
                    subtasks: vec![],
                    dependencies: vec![],
                    complexity: Complexity::Simple,
                    estimated_hours: 1.0,
                },
                ParentTask {
                    id: "2.0".to_string(),
                    title: "Task B".to_string(),
                    description: "Second".to_string(),
                    subtasks: vec![],
                    dependencies: vec![],
                    complexity: Complexity::Simple,
                    estimated_hours: 2.0,
                },
            ],
            relevant_files: vec![],
            estimated_complexity: Complexity::Simple,
        };

        persistence.save_task_breakdown(&breakdown1)?;
        persistence.save_task_breakdown(&breakdown2)?;

        let tasks = persistence.get_tasks(None)?;

        // Mark one task from PRD Two as done
        let prd2_task = tasks.iter().find(|t| t.prd_title.as_deref() == Some("PRD Two")).unwrap();
        persistence.update_task_status(prd2_task.id, TaskStatus::Done)?;

        // Get stats for PRD Two only
        let stats = persistence.get_prd_statistics("PRD Two")?;

        assert_eq!(stats.total, 2);
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.pending, 1);
        assert_eq!(stats.progress_percent, 50);

        Ok(())
    }
}
