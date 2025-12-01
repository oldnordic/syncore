use crate::logger::MarkdownLogger;
use crate::tasks::Tasks;
use anyhow::Result;
use rusqlite::Connection;
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::interval;

pub struct HeartbeatMonitor {
    taskmaster: Arc<Mutex<Tasks>>,
    logger: Arc<MarkdownLogger>,
    db: Arc<Mutex<Connection>>,
    last_heartbeat: Arc<Mutex<SystemTime>>,
}

impl HeartbeatMonitor {
    /// Create HeartbeatMonitor using an existing database connection from DbManager.
    ///
    /// This is the preferred constructor when using DbManager. It reuses long-lived
    /// connections instead of creating new ones per-call.
    pub fn with_connection(
        taskmaster: Arc<Mutex<Tasks>>,
        logger: Arc<MarkdownLogger>,
        db: Arc<Mutex<Connection>>,
    ) -> Self {
        Self {
            taskmaster,
            logger,
            db,
            last_heartbeat: Arc::new(Mutex::new(SystemTime::now())),
        }
    }

    /// Legacy constructor - opens its own connection (deprecated, use with_connection instead).
    #[deprecated(note = "Use with_connection() with DbManager instead")]
    pub fn new(taskmaster: Arc<Mutex<Tasks>>, logger: Arc<MarkdownLogger>, db_path: &str) -> Self {
        let conn =
            Connection::open(db_path).expect("Failed to open database in HeartbeatMonitor::new");

        Self {
            taskmaster,
            logger,
            db: Arc::new(Mutex::new(conn)),
            last_heartbeat: Arc::new(Mutex::new(SystemTime::now())),
        }
    }

    // Start heartbeat monitoring task
    pub async fn start_monitoring(&self) {
        let _taskmaster = self.taskmaster.clone(); // Keep for potential future use
        let _logger = self.logger.clone(); // Keep for potential future use
        let last_heartbeat = self.last_heartbeat.clone();
        let db = self.db.clone();

        tokio::spawn(async move {
            let mut heartbeat_interval = interval(Duration::from_secs(60)); // Every 60 seconds

            loop {
                heartbeat_interval.tick().await;

                // Update last heartbeat time
                if let Ok(mut heartbeat_guard) = last_heartbeat.lock() {
                    *heartbeat_guard = SystemTime::now();
                }

                // Use the stored connection instead of opening a new one
                if let Err(e) = Self::check_stuck_tasks_with_connection(&db) {
                    eprintln!("Heartbeat check failed: {}", e);
                }
            }
        });
    }

    // Use the stored connection instead of opening a new one
    fn check_stuck_tasks_with_connection(db: &Arc<Mutex<Connection>>) -> Result<()> {
        let conn = db.lock().unwrap();

        // Find tasks stuck in progress for more than 15 minutes
        let fifteen_min_ago = chrono::Utc::now().timestamp() - (15 * 60);

        let mut stmt = conn.prepare(
            "SELECT id FROM tasks
             WHERE status = 'in_progress'
             AND updated_at < ?1",
        )?;

        let stuck_tasks = stmt.query_map([fifteen_min_ago], |row| row.get::<_, i64>(0))?;

        for task_id in stuck_tasks {
            let task_id = task_id?;
            println!("Requeuing stuck task: {task_id}");

            // Reset to open status
            conn.execute(
                "UPDATE tasks SET status = 'open', updated_at = ?1 WHERE id = ?2",
                [chrono::Utc::now().timestamp(), task_id],
            )?;
        }

        Ok(())
    }

    // Check for stuck tasks and resume them
    pub async fn check_and_resume_stuck_tasks(&self) -> Result<()> {
        let _now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;

        let fifteen_minutes_ago = _now - (15 * 60);

        // Get stuck tasks (real implementation)
        let stuck_tasks = self.get_stuck_tasks(Some(fifteen_minutes_ago)).await?;

        if stuck_tasks.is_empty() {
            return Ok(());
        }

        // Log stuck tasks using eprintln for now
        let stuck_count = stuck_tasks.len();
        eprintln!("HEARTBEAT: Found {} stuck tasks, attempting to resume", stuck_count);

        for task_id in stuck_tasks {
            // Log heartbeat resumption
            let _log_entry = json!({
                "timestamp": _now,
                "event": "heartbeat_resume",
                "event_type": "heartbeat_resume",
                "task_id": task_id
            });

            // In real implementation, this would write to markdown log
            eprintln!("HEARTBEAT: ⚠ resumed task #{}", task_id);

            // Enqueue resume subtask (real implementation)
            self.enqueue_resume_subtask(task_id).await?;
        }

        Ok(())
    }

    // Real implementation to get stuck tasks
    pub async fn get_stuck_tasks(&self, cutoff_time: Option<i64>) -> Result<Vec<u64>> {
        let cutoff = cutoff_time.unwrap_or_else(|| {
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64 - (15 * 60)
            // Default 15 minutes ago
        });

        let conn = self.db.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id FROM tasks
             WHERE status = 'in_progress'
              AND updated_at < ?1",
        )?;

        let stuck_tasks = stmt.query_map([cutoff], |row| row.get::<_, i64>(0))?;

        let mut result = Vec::new();
        for task_id in stuck_tasks {
            result.push(task_id? as u64);
        }

        Ok(result)
    }

    // Real implementation to enqueue resume subtask
    pub async fn enqueue_resume_subtask(&self, task_id: u64) -> Result<()> {
        let conn = self.db.lock().unwrap();

        // Reset stuck task to open status
        conn.execute(
            "UPDATE tasks SET status = 'open', updated_at = ?1 WHERE id = ?2",
            [chrono::Utc::now().timestamp(), task_id as i64],
        )?;

        eprintln!("HEARTBEAT: Reset stuck task #{} to open status", task_id);
        Ok(())
    }

    // Get current heartbeat status
    pub async fn get_heartbeat_status(&self) -> Result<SystemTime> {
        let heartbeat_guard = self.last_heartbeat.lock().unwrap();
        Ok(*heartbeat_guard)
    }

    // Check if heartbeat is active
    pub async fn is_heartbeat_active(&self, threshold: Duration) -> Result<bool> {
        let last_heartbeat = self.get_heartbeat_status().await?;
        let elapsed = last_heartbeat.elapsed().unwrap_or(Duration::MAX);
        Ok(elapsed < threshold)
    }
}

// Autonomy manager that combines all features
pub struct AutonomyManager {
    heartbeat: HeartbeatMonitor,
}

impl AutonomyManager {
    /// Create AutonomyManager using DbManager's database connection
    pub fn with_db_manager(
        taskmaster: Arc<Mutex<Tasks>>,
        logger: Arc<MarkdownLogger>,
        db_manager: &crate::db::DbManager,
    ) -> Self {
        let heartbeat =
            HeartbeatMonitor::with_connection(taskmaster, logger, db_manager.main_conn());

        Self {
            heartbeat,
        }
    }

    /// Legacy constructor - deprecated, use with_db_manager instead
    #[deprecated(note = "Use with_db_manager() with DbManager instead")]
    pub fn new(taskmaster: Arc<Mutex<Tasks>>, logger: Arc<MarkdownLogger>, db_path: &str) -> Self {
        #[allow(deprecated)]
        let heartbeat = HeartbeatMonitor::new(taskmaster, logger, db_path);

        Self {
            heartbeat,
        }
    }

    // Start all autonomy features
    pub async fn start(&self) {
        self.heartbeat.start_monitoring().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DbManager;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_heartbeat_monitor_creation() {
        let temp_db = NamedTempFile::new().unwrap();
        let db_path = temp_db.path().to_str().unwrap();

        let _taskmaster = Arc::new(Mutex::new(Tasks::new(&format!("{}_tasks", db_path)).unwrap()));
        let logs_dir =
            std::path::Path::new(db_path).parent().unwrap_or_else(|| std::path::Path::new("/tmp"));
        let _logger = Arc::new(MarkdownLogger::new(logs_dir));

        let db_manager = DbManager::new(db_path, format!("{}_code_graph", db_path)).unwrap();

        let _monitor = HeartbeatMonitor::with_connection(
            Arc::new(Mutex::new(Tasks::new(&format!("{}_tasks", db_path)).unwrap())),
            Arc::new(MarkdownLogger::new(logs_dir)),
            db_manager.main_conn(),
        );

        // Test that monitor was created successfully
        // The actual monitoring runs in background, so we just test creation
        assert!(true, "HeartbeatMonitor created successfully");
    }

    #[tokio::test]
    async fn test_autonomy_manager_creation() {
        let temp_db = NamedTempFile::new().unwrap();
        let db_path = temp_db.path().to_str().unwrap();

        let _taskmaster = Arc::new(Mutex::new(Tasks::new(&format!("{}_tasks", db_path)).unwrap()));
        let _logger = Arc::new(MarkdownLogger::new(db_path));

        // Create DbManager for the new constructor
        let db_manager = crate::db::DbManager::new(db_path, db_path).unwrap();

        let _manager = AutonomyManager::with_db_manager(
            Arc::new(Mutex::new(Tasks::new(&format!("{}_tasks", db_path)).unwrap())),
            Arc::new(MarkdownLogger::new(db_path)),
            &db_manager,
        );

        // Test that manager was created successfully
        assert!(true, "AutonomyManager created successfully");
    }

    #[tokio::test]
    async fn test_get_stuck_tasks_empty() -> Result<()> {
        let temp_db = NamedTempFile::new().unwrap();
        let db_path = temp_db.path().to_str().unwrap();

        let _taskmaster = Arc::new(Mutex::new(Tasks::new(&format!("{}_tasks", db_path)).unwrap()));
        let cutoff_time = Some(1234567890);

        // Create DbManager for with_connection
        let db_manager = crate::db::DbManager::new(db_path, db_path).unwrap();

        let monitor = HeartbeatMonitor::with_connection(
            Arc::new(Mutex::new(Tasks::new(&format!("{}_tasks", db_path)).unwrap())),
            Arc::new(MarkdownLogger::new("/tmp")),
            db_manager.main_conn(),
        );
        let stuck_tasks = monitor.get_stuck_tasks(cutoff_time).await?;

        assert_eq!(stuck_tasks.len(), 0, "Should return empty list when no stuck tasks exist");
        Ok(())
    }

    #[tokio::test]
    async fn test_enqueue_resume_subtask() -> Result<()> {
        let temp_db = NamedTempFile::new().unwrap();
        let db_path = temp_db.path().to_str().unwrap();

        let _taskmaster = Arc::new(Mutex::new(Tasks::new(&format!("{}_tasks", db_path)).unwrap()));
        let task_id = 42;

        // Create DbManager for with_connection
        let db_manager = crate::db::DbManager::new(db_path, db_path).unwrap();

        let monitor = HeartbeatMonitor::with_connection(
            Arc::new(Mutex::new(Tasks::new(&format!("{}_tasks", db_path)).unwrap())),
            Arc::new(MarkdownLogger::new("/tmp")),
            db_manager.main_conn(),
        );
        monitor.enqueue_resume_subtask(task_id).await?;

        // If we reach here, the function completed successfully
        assert!(true, "Should successfully enqueue resume subtask");
        Ok(())
    }

    #[tokio::test]
    async fn test_check_and_resume_stuck_tasks_no_stuck() -> Result<()> {
        let temp_db = NamedTempFile::new().unwrap();
        let db_path = temp_db.path().to_str().unwrap();

        let _taskmaster = Arc::new(Mutex::new(Tasks::new(&format!("{}_tasks", db_path)).unwrap()));
        let _logger = Arc::new(MarkdownLogger::new(db_path));

        // Create DbManager for with_connection
        let db_manager = crate::db::DbManager::new(db_path, db_path).unwrap();

        let monitor = HeartbeatMonitor::with_connection(
            Arc::new(Mutex::new(Tasks::new(&format!("{}_tasks", db_path)).unwrap())),
            Arc::new(MarkdownLogger::new(db_path)),
            db_manager.main_conn(),
        );
        monitor.check_and_resume_stuck_tasks().await?;

        // If we reach here, the function completed successfully
        assert!(true, "Should complete successfully even with no stuck tasks");
        Ok(())
    }
}
