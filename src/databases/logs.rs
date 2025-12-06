//! SQLite-backed Reasoning Logs Database
//!
//! Provides persistent storage for Tree-of-Thought reasoning logs.
//! Replaces filesystem-based Markdown logging with structured SQLite storage.

use anyhow::Result;
use rusqlite::{params, Connection, Row};
use std::sync::{Arc, Mutex};

/// Reasoning log entry
#[derive(Debug, Clone)]
pub struct ReasoningLogEntry {
    pub id: String,
    pub session_id: String,
    pub node_id: String,
    pub event_type: String,
    pub message: String,
    pub depth: i64,
    pub timestamp: i64,
}

/// SQLite database operations for reasoning logs
#[derive(Debug)]
pub struct ReasoningLogWriter {
    conn: Arc<Mutex<Connection>>,
}

impl ReasoningLogWriter {
    /// Create new log writer with SQLite connection
    pub fn new(conn: Arc<Mutex<Connection>>) -> Result<Self> {
        let writer = Self {
            conn,
        };
        writer.init_table()?;
        Ok(writer)
    }

    /// Initialize the reasoning_logs table
    fn init_table(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS reasoning_logs (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                node_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                message TEXT NOT NULL,
                depth INTEGER NOT NULL,
                timestamp INTEGER NOT NULL
            )",
            [],
        )?;

        // Create indexes for performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_reasoning_logs_session 
             ON reasoning_logs(session_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_reasoning_logs_node 
             ON reasoning_logs(node_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_reasoning_logs_timestamp 
             ON reasoning_logs(timestamp)",
            [],
        )?;

        Ok(())
    }

    /// Insert a new log entry
    pub fn insert_log(
        &self,
        id: &str,
        session_id: &str,
        node_id: &str,
        event_type: &str,
        message: &str,
        depth: i64,
        timestamp: i64,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO reasoning_logs (id, session_id, node_id, event_type, message, depth, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, session_id, node_id, event_type, message, depth, timestamp],
        )?;
        Ok(())
    }

    /// Fetch all logs for a session, ordered by timestamp
    pub fn fetch_logs_by_session(&self, session_id: &str) -> Result<Vec<ReasoningLogEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, node_id, event_type, message, depth, timestamp
             FROM reasoning_logs 
             WHERE session_id = ?1 
             ORDER BY timestamp ASC",
        )?;

        let rows = stmt.query_map([session_id], Self::row_to_log_entry)?;
        let mut logs = Vec::new();

        for row in rows {
            logs.push(row?);
        }

        Ok(logs)
    }

    /// Fetch all logs for a specific node, ordered by timestamp
    pub fn fetch_logs_by_node(&self, node_id: &str) -> Result<Vec<ReasoningLogEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, node_id, event_type, message, depth, timestamp
             FROM reasoning_logs 
             WHERE node_id = ?1 
             ORDER BY timestamp ASC",
        )?;

        let rows = stmt.query_map([node_id], Self::row_to_log_entry)?;
        let mut logs = Vec::new();

        for row in rows {
            logs.push(row?);
        }

        Ok(logs)
    }

    /// Delete all logs for a session
    pub fn delete_logs_for_session(&self, session_id: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let rows_affected =
            conn.execute("DELETE FROM reasoning_logs WHERE session_id = ?1", [session_id])?;
        Ok(rows_affected)
    }

    /// Get recent logs for a session (limit by count)
    pub fn fetch_recent_logs(
        &self,
        session_id: &str,
        limit: i64,
    ) -> Result<Vec<ReasoningLogEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, node_id, event_type, message, depth, timestamp
             FROM reasoning_logs 
             WHERE session_id = ?1 
             ORDER BY timestamp DESC
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![session_id, limit], Self::row_to_log_entry)?;
        let mut logs = Vec::new();

        for row in rows {
            logs.push(row?);
        }

        // Reverse to get chronological order
        logs.reverse();
        Ok(logs)
    }

    /// Convert a database row to ReasoningLogEntry
    fn row_to_log_entry(row: &Row) -> Result<ReasoningLogEntry, rusqlite::Error> {
        Ok(ReasoningLogEntry {
            id: row.get(0)?,
            session_id: row.get(1)?,
            node_id: row.get(2)?,
            event_type: row.get(3)?,
            message: row.get(4)?,
            depth: row.get(5)?,
            timestamp: row.get(6)?,
        })
    }

    /// Generate unique log ID
    pub fn generate_log_id(session_id: &str, node_id: &str, event_type: &str) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp =
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();

        format!("{}_{}_{}_{}", session_id, node_id, event_type, timestamp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn create_test_db() -> (Arc<Mutex<Connection>>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_logs.db");
        let conn = Connection::open(db_path).unwrap();
        (Arc::new(Mutex::new(conn)), temp_dir)
    }

    #[test]
    fn test_table_creation() {
        let (conn, _temp) = create_test_db();
        let writer = ReasoningLogWriter::new(conn).unwrap();

        // Table should exist without error
        let conn = writer.conn.lock().unwrap();
        assert!(conn.prepare("SELECT COUNT(*) FROM reasoning_logs").is_ok());
    }

    #[test]
    fn test_insert_and_fetch_log() {
        let (conn, _temp) = create_test_db();
        let writer = ReasoningLogWriter::new(conn).unwrap();

        let session_id = "test_session";
        let node_id = "test_node";
        let event_type = "expansion";
        let message = "Test log message";
        let depth = 2;
        let timestamp = 1234567890;

        let log_id = ReasoningLogWriter::generate_log_id(session_id, node_id, event_type);

        writer
            .insert_log(&log_id, session_id, node_id, event_type, message, depth, timestamp)
            .unwrap();

        let logs = writer.fetch_logs_by_session(session_id).unwrap();
        assert_eq!(logs.len(), 1);

        let log = &logs[0];
        assert_eq!(log.id, log_id);
        assert_eq!(log.session_id, session_id);
        assert_eq!(log.node_id, node_id);
        assert_eq!(log.event_type, event_type);
        assert_eq!(log.message, message);
        assert_eq!(log.depth, depth);
        assert_eq!(log.timestamp, timestamp);
    }

    #[test]
    fn test_fetch_logs_by_node() {
        let (conn, _temp) = create_test_db();
        let writer = ReasoningLogWriter::new(conn).unwrap();

        let session_id = "test_session";
        let node_id = "test_node";

        // Insert multiple logs for the same node
        for i in 0..3 {
            let log_id =
                ReasoningLogWriter::generate_log_id(session_id, node_id, &format!("event_{}", i));
            writer
                .insert_log(
                    &log_id,
                    session_id,
                    node_id,
                    &format!("event_{}", i),
                    &format!("Message {}", i),
                    i,
                    1234567890 + i,
                )
                .unwrap();
        }

        let logs = writer.fetch_logs_by_node(node_id).unwrap();
        assert_eq!(logs.len(), 3);

        // Should be ordered by timestamp
        for (i, log) in logs.iter().enumerate() {
            assert_eq!(log.event_type, format!("event_{}", i));
            assert_eq!(log.message, format!("Message {}", i));
        }
    }

    #[test]
    fn test_delete_logs_for_session() {
        let (conn, _temp) = create_test_db();
        let writer = ReasoningLogWriter::new(conn).unwrap();

        let session_id = "test_session";
        let node_id = "test_node";

        // Insert logs
        let log_id = ReasoningLogWriter::generate_log_id(session_id, node_id, "test");
        writer.insert_log(&log_id, session_id, node_id, "test", "message", 0, 1234567890).unwrap();

        // Verify log exists
        let logs_before = writer.fetch_logs_by_session(session_id).unwrap();
        assert_eq!(logs_before.len(), 1);

        // Delete logs
        let deleted_count = writer.delete_logs_for_session(session_id).unwrap();
        assert_eq!(deleted_count, 1);

        // Verify log is gone
        let logs_after = writer.fetch_logs_by_session(session_id).unwrap();
        assert_eq!(logs_after.len(), 0);
    }

    #[test]
    fn test_fetch_recent_logs() {
        let (conn, _temp) = create_test_db();
        let writer = ReasoningLogWriter::new(conn).unwrap();

        let session_id = "test_session";
        let base_timestamp = 1234567890;

        // Insert 5 logs with different timestamps
        for i in 0..5 {
            let node_id = format!("node_{}", i);
            let log_id = ReasoningLogWriter::generate_log_id(session_id, &node_id, "test");
            writer
                .insert_log(
                    &log_id,
                    session_id,
                    &node_id,
                    "test",
                    &format!("Message {}", i),
                    0,
                    base_timestamp + i,
                )
                .unwrap();
        }

        // Fetch recent 3 logs
        let recent_logs = writer.fetch_recent_logs(session_id, 3).unwrap();
        assert_eq!(recent_logs.len(), 3);

        // Should be the 3 most recent logs in chronological order
        assert_eq!(recent_logs[0].message, "Message 2");
        assert_eq!(recent_logs[1].message, "Message 3");
        assert_eq!(recent_logs[2].message, "Message 4");
    }

    #[test]
    fn test_generate_log_id() {
        let session_id = "session123";
        let node_id = "node456";
        let event_type = "expansion";

        let log_id1 = ReasoningLogWriter::generate_log_id(session_id, node_id, event_type);
        let log_id2 = ReasoningLogWriter::generate_log_id(session_id, node_id, event_type);

        // IDs should be unique (due to timestamp)
        assert_ne!(log_id1, log_id2);

        // IDs should contain all components
        assert!(log_id1.contains(session_id));
        assert!(log_id1.contains(node_id));
        assert!(log_id1.contains(event_type));
    }
}
