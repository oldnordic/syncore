use crate::taskmaster::TaskMaster;
use crate::logger::MarkdownLogger;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::{interval, sleep};
use anyhow::Result;
use serde_json::json;

pub struct HeartbeatMonitor {
    taskmaster: Arc<Mutex<TaskMaster>>,
    logger: Arc<MarkdownLogger>,
}

impl HeartbeatMonitor {
    pub fn new(taskmaster: Arc<Mutex<TaskMaster>>, logger: Arc<MarkdownLogger>) -> Self {
        Self {
            taskmaster,
            logger,
        }
    }
    
    // Start heartbeat monitoring task
    pub async fn start_monitoring(&self) {
        let taskmaster = Arc::clone(&self.taskmaster);
        let logger = Arc::clone(&self.logger);
        
        tokio::spawn(async move {
            let mut heartbeat_interval = interval(Duration::from_secs(60)); // Every 60 seconds
            
            loop {
                heartbeat_interval.tick().await;
                
                if let Err(e) = Self::check_and_resume_stuck_tasks(&taskmaster, &logger).await {
                    eprintln!("Heartbeat check failed: {}", e);
                }
            }
        });
    }
    
    // Check for stuck tasks and resume them
    async fn check_and_resume_stuck_tasks(
        taskmaster: &Arc<Mutex<TaskMaster>>, 
        logger: &Arc<MarkdownLogger>
    ) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        let fifteen_minutes_ago = now - (15 * 60);
        
        // Get stuck tasks (mock implementation)
        let stuck_tasks = Self::get_stuck_tasks(taskmaster, fifteen_minutes_ago).await?;
        
        for task_id in stuck_tasks {
            // Log heartbeat resumption
            let log_entry = json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "level": "warn",
                "message": format!("⚠ heartbeat resumed task #{}", task_id),
                "event_type": "heartbeat_resume",
                "task_id": task_id
            });
            
            // In real implementation, this would write to markdown log
            eprintln!("HEARTBEAT: ⚠ resumed task #{}", task_id);
            
            // Enqueue resume subtask (mock)
            Self::enqueue_resume_subtask(taskmaster, task_id).await?;
        }
        
        Ok(())
    }
    
    // Mock implementation to get stuck tasks
    async fn get_stuck_tasks(
        _taskmaster: &Arc<Mutex<TaskMaster>>, 
        _cutoff_time: i64
    ) -> Result<Vec<u64>> {
        // In real implementation, this would query the database
        // For now, return empty (no stuck tasks)
        Ok(vec![])
    }
    
    // Mock implementation to enqueue resume subtask
    async fn enqueue_resume_subtask(
        _taskmaster: &Arc<Mutex<TaskMaster>>, 
        task_id: u64
    ) -> Result<()> {
        // In real implementation, this would create a "resume" subtask
        eprintln!("HEARTBEAT: Enqueued resume subtask for task #{}", task_id);
        Ok(())
    }
}

// Autonomy manager that combines all features
pub struct AutonomyManager {
    heartbeat: HeartbeatMonitor,
}

impl AutonomyManager {
    pub fn new(
        taskmaster: Arc<Mutex<TaskMaster>>, 
        logger: Arc<MarkdownLogger>
    ) -> Self {
        let heartbeat = HeartbeatMonitor::new(taskmaster, logger);
        
        Self {
            heartbeat,
        }
    }
    
    // Start all autonomy features
    pub async fn start(&self) {
        self.heartbeat.start_monitoring().await;
    }
}