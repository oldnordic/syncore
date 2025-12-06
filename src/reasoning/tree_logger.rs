//! Tree-of-Thought Logger
//!
//! Provides deterministic, branch-aware logging for reasoning sessions.
//! Stores all logs in SQLite for retrieval and visualization.

use crate::databases::logs::{ReasoningLogEntry, ReasoningLogWriter};
use anyhow::Result;
use std::sync::Arc;

/// Tree logger for reasoning sessions
#[derive(Debug)]
pub struct TreeLogger {
    log_writer: Arc<ReasoningLogWriter>,
}

impl TreeLogger {
    /// Create new tree logger with SQLite backend
    pub fn new(log_writer: Arc<ReasoningLogWriter>) -> Self {
        Self {
            log_writer,
        }
    }

    /// Log a generic tree event
    pub fn log_tree_event(
        &self,
        session_id: &str,
        node_id: &str,
        event_type: &str,
        message: &str,
        depth: i64,
    ) -> Result<()> {
        let timestamp = current_timestamp();
        let log_id = ReasoningLogWriter::generate_log_id(session_id, node_id, event_type);

        self.log_writer
            .insert_log(&log_id, session_id, node_id, event_type, message, depth, timestamp)?;

        Ok(())
    }

    /// Log node expansion event
    pub fn log_expansion(
        &self,
        session_id: &str,
        node_id: &str,
        depth: i64,
        child_count: usize,
        branch_contents: &[String],
    ) -> Result<()> {
        let message = format!(
            "Expanded into {} branches: {}",
            child_count,
            branch_contents
                .iter()
                .enumerate()
                .map(|(i, content)| format!("{}: {}", i + 1, truncate_content(content, 50)))
                .collect::<Vec<_>>()
                .join(", ")
        );

        self.log_tree_event(session_id, node_id, "expansion", &message, depth)
    }

    /// Log node prune event
    pub fn log_prune(
        &self,
        session_id: &str,
        node_id: &str,
        depth: i64,
        reason: &str,
        pruned_branches: &[String],
    ) -> Result<()> {
        let message = if pruned_branches.is_empty() {
            format!("Pruned node: {}", reason)
        } else {
            format!(
                "Pruned {} branches: {} - {}",
                pruned_branches.len(),
                pruned_branches
                    .iter()
                    .map(|b| truncate_content(b, 30))
                    .collect::<Vec<_>>()
                    .join(", "),
                reason
            )
        };

        self.log_tree_event(session_id, node_id, "prune", &message, depth)
    }

    /// Log node score event
    pub fn log_score(
        &self,
        session_id: &str,
        node_id: &str,
        depth: i64,
        score: f64,
        evaluation: &str,
    ) -> Result<()> {
        let message = format!("Score: {:.3} - {}", score, evaluation);
        self.log_tree_event(session_id, node_id, "score", &message, depth)
    }

    /// Log session start event
    pub fn log_session_start(
        &self,
        session_id: &str,
        task_id: Option<&str>,
        metadata: Option<&str>,
    ) -> Result<()> {
        let message = if let (Some(task), Some(meta)) = (task_id, metadata) {
            format!("Session started for task {} with metadata: {}", task, meta)
        } else if let Some(task) = task_id {
            format!("Session started for task {}", task)
        } else {
            "Session started".to_string()
        };

        self.log_tree_event(
            session_id,
            &format!("{}_root", session_id),
            "session_start",
            &message,
            0,
        )
    }

    /// Log reasoning step event
    pub fn log_reasoning_step(
        &self,
        session_id: &str,
        node_id: &str,
        depth: i64,
        step_type: &str,
        content: &str,
    ) -> Result<()> {
        let message = format!("{}: {}", step_type, truncate_content(content, 100));
        self.log_tree_event(session_id, node_id, "reasoning_step", &message, depth)
    }

    /// Get formatted logs for a session with tree structure
    pub fn get_formatted_logs(&self, session_id: &str) -> Result<String> {
        let logs = self.log_writer.fetch_logs_by_session(session_id)?;
        let mut formatted = String::new();

        formatted.push_str(&format!("=== Reasoning Session: {} ===\n\n", session_id));

        for log in logs {
            let indent = "  ".repeat(log.depth as usize);
            let prefix = match log.event_type.as_str() {
                "session_start" => "🚀",
                "expansion" => "🌳",
                "prune" => "✂️",
                "score" => "📊",
                "reasoning_step" => "💭",
                _ => "📝",
            };

            formatted.push_str(&format!(
                "{}{} [{}] {}\n",
                indent,
                prefix,
                format_timestamp(log.timestamp),
                log.message
            ));
        }

        Ok(formatted)
    }

    /// Get logs for a specific node
    pub fn get_node_logs(&self, node_id: &str) -> Result<Vec<ReasoningLogEntry>> {
        self.log_writer.fetch_logs_by_node(node_id)
    }

    /// Get recent logs for a session
    pub fn get_recent_logs(&self, session_id: &str, limit: i64) -> Result<Vec<ReasoningLogEntry>> {
        self.log_writer.fetch_recent_logs(session_id, limit)
    }

    /// Clear all logs for a session
    pub fn clear_session_logs(&self, session_id: &str) -> Result<usize> {
        self.log_writer.delete_logs_for_session(session_id)
    }
}

/// Helper functions

/// Get current timestamp as i64 (milliseconds for better precision)
fn current_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// Format timestamp for display
fn format_timestamp(timestamp: i64) -> String {
    // Convert milliseconds to seconds for chrono
    let seconds = timestamp / 1000;
    chrono::DateTime::from_timestamp(seconds, 0).unwrap_or_default().format("%H:%M:%S").to_string()
}

/// Truncate content to specified length with ellipsis
fn truncate_content(content: &str, max_len: usize) -> String {
    if content.len() <= max_len {
        content.to_string()
    } else {
        format!("{}...", &content[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::databases::logs::ReasoningLogWriter;
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    fn create_test_logger() -> (TreeLogger, Arc<Mutex<Connection>>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_logs.db");
        let conn = Arc::new(Mutex::new(Connection::open(db_path).unwrap()));
        let log_writer = Arc::new(ReasoningLogWriter::new(conn.clone()).unwrap());
        let logger = TreeLogger::new(log_writer);
        (logger, conn, temp_dir)
    }

    #[test]
    fn test_log_expansion() {
        let (logger, _conn, _temp) = create_test_logger();

        let session_id = "test_session";
        let node_id = "test_node";
        let depth = 2;
        let branches = vec![
            "Branch A: Continue approach".to_string(),
            "Branch B: Alternative strategy".to_string(),
            "Branch C: Explore edge case".to_string(),
        ];

        logger.log_expansion(session_id, node_id, depth, branches.len(), &branches).unwrap();

        let logs = logger.get_node_logs(node_id).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].event_type, "expansion");
        assert_eq!(logs[0].depth, depth);
        assert!(logs[0].message.contains("Expanded into 3 branches"));
    }

    #[test]
    fn test_log_prune() {
        let (logger, _conn, _temp) = create_test_logger();

        let session_id = "test_session";
        let node_id = "test_node";
        let depth = 3;
        let reason = "Low confidence score";
        let pruned_branches = vec!["Weak branch A".to_string(), "Irrelevant branch B".to_string()];

        logger.log_prune(session_id, node_id, depth, reason, &pruned_branches).unwrap();

        let logs = logger.get_node_logs(node_id).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].event_type, "prune");
        assert!(logs[0].message.contains("Pruned 2 branches"));
        assert!(logs[0].message.contains(reason));
    }

    #[test]
    fn test_log_score() {
        let (logger, _conn, _temp) = create_test_logger();

        let session_id = "test_session";
        let node_id = "test_node";
        let depth = 1;
        let score = 0.85;
        let evaluation = "High quality reasoning";

        logger.log_score(session_id, node_id, depth, score, evaluation).unwrap();

        let logs = logger.get_node_logs(node_id).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].event_type, "score");
        assert!(logs[0].message.contains("Score: 0.850"));
        assert!(logs[0].message.contains(evaluation));
    }

    #[test]
    fn test_session_start_logging() {
        let (logger, _conn, _temp) = create_test_logger();

        let session_id = "test_session";
        let task_id = Some("task_123");
        let metadata = Some("Test reasoning session");

        logger.log_session_start(session_id, task_id, metadata).unwrap();

        let logs = logger.get_formatted_logs(session_id).unwrap();
        assert!(logs.contains("🚀"));
        assert!(logs.contains("Session started for task task_123"));
        assert!(logs.contains("Test reasoning session"));
    }

    #[test]
    fn test_formatted_logs_structure() {
        let (logger, _conn, _temp) = create_test_logger();
        let session_id = "test_session";

        // Log session start
        logger.log_session_start(session_id, None, None).unwrap();

        // Log expansion
        logger
            .log_expansion(
                session_id,
                "node1",
                1,
                2,
                &["Branch A".to_string(), "Branch B".to_string()],
            )
            .unwrap();

        // Log score
        logger.log_score(session_id, "node1", 1, 0.75, "Good reasoning").unwrap();

        let formatted = logger.get_formatted_logs(session_id).unwrap();

        // Should contain session header
        assert!(formatted.contains("=== Reasoning Session: test_session ==="));

        // Should contain proper icons and structure
        assert!(formatted.contains("🚀"));
        assert!(formatted.contains("🌳"));
        assert!(formatted.contains("📊"));

        // Should have proper indentation
        assert!(formatted.contains("  🌳"));
        assert!(formatted.contains("  📊"));
    }

    #[test]
    fn test_truncate_content() {
        assert_eq!(truncate_content("short", 10), "short");
        assert_eq!(truncate_content("exactly ten", 11), "exactly ten");
        assert_eq!(truncate_content("this is way too long content", 10), "this is...");
        assert_eq!(truncate_content("", 5), "");
    }

    // Helper functions for testing
    fn create_indentation(depth: usize) -> String {
        match depth {
            0 => "".to_string(),
            1 => "├── ".to_string(),
            2 => "│  ├── ".to_string(),
            3 => "│  │  ├── ".to_string(),
            _ => "  ".repeat(depth) + "├── ",
        }
    }

    fn format_tree_log(log: &ReasoningLogEntry, _include_metadata: bool) -> String {
        let indent = create_indentation(log.depth as usize);
        format!("🌳 {}[{}] {} (depth {})", indent, log.node_id, log.message, log.depth)
    }

    #[test]
    fn test_create_indentation() {
        assert_eq!(create_indentation(0), "");
        assert_eq!(create_indentation(1), "├── ");
        assert_eq!(create_indentation(2), "│  ├── ");
        assert_eq!(create_indentation(3), "│  │  ├── ");
    }

    #[test]
    fn test_format_tree_log() {
        let log = ReasoningLogEntry {
            id: "test_log".to_string(),
            session_id: "test_session".to_string(),
            node_id: "test_node".to_string(),
            event_type: "expansion".to_string(),
            message: "Test expansion message".to_string(),
            depth: 2,
            timestamp: 1234567890,
        };

        let formatted = format_tree_log(&log, false);
        assert!(formatted.contains("🌳"));
        assert!(formatted.contains("Test expansion message"));
        assert!(formatted.contains("│  ├── ")); // depth 2 indentation
    }

    #[test]
    fn test_clear_session_logs() {
        let (logger, _conn, _temp) = create_test_logger();
        let session_id = "test_session";

        // Add some logs
        logger.log_session_start(session_id, None, None).unwrap();
        logger
            .log_expansion(session_id, "node1", 1, 2, &["A".to_string(), "B".to_string()])
            .unwrap();

        // Verify logs exist
        let logs_before = logger.get_formatted_logs(session_id).unwrap();
        assert!(logs_before.contains("🚀"));
        assert!(logs_before.contains("🌳"));

        // Clear logs
        let deleted_count = logger.clear_session_logs(session_id).unwrap();
        assert!(deleted_count > 0);

        // Verify logs are gone
        let logs_after = logger.get_formatted_logs(session_id).unwrap();
        assert!(!logs_after.contains("🚀"));
        assert!(!logs_after.contains("🌳"));
    }
}
