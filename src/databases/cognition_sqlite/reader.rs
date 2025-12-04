//! Cognition SQLite Reader - Read Operations for Reasoning Episodes
//!
//! Provides all read operations for cognition entities using SQLite backend.

use anyhow::Result;
use rusqlite::OptionalExtension;
use serde_json::Value;

use super::schema::*;
use crate::graph::SQLiteGraphBackend;

/// Reader for cognition entities using SQLite backend
#[derive(Debug, Clone)]
pub struct CognitionSqliteReader {
    backend: SQLiteGraphBackend,
}

impl CognitionSqliteReader {
    /// Create a new reader with the given SQLite backend
    pub fn new(backend: SQLiteGraphBackend) -> Self {
        Self {
            backend,
        }
    }

    /// Get a reasoning session by ID
    pub async fn get_session(&self, session_id: &str) -> Result<Option<SessionResult>> {
        let conn = self.backend.code_graph().db_conn();
        let db = conn.lock().map_err(|e| anyhow::anyhow!("Failed to lock database: {}", e))?;

        let mut stmt = db.prepare(
            r#"
                SELECT id, title, description, created_at, namespace, graph_domain
                FROM reasoning_sessions
                WHERE id = ?
            "#,
        )?;

        let session = stmt
            .query_row([session_id], |row| {
                Ok(SessionResult {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                    namespace: row.get(4)?,
                    graph_domain: row.get(5)?,
                })
            })
            .optional()?;

        Ok(session)
    }

    /// Get all nodes for a reasoning session
    pub async fn get_nodes_for_session(&self, session_id: &str) -> Result<Vec<ThoughtNodeResult>> {
        let conn = self.backend.code_graph().db_conn();
        let db = conn.lock().map_err(|e| anyhow::anyhow!("Failed to lock database: {}", e))?;

        let mut stmt = db.prepare(
            r#"
                SELECT id, session_id, parent_id, content, thought_type, depth, breadth,
                       confidence, metadata, created_at, namespace, graph_domain
                FROM reasoning_nodes
                WHERE session_id = ?
                ORDER BY depth, breadth, id
            "#,
        )?;

        let nodes = stmt.query_map([session_id], |row| {
            let metadata_str: Option<String> = row.get(8)?;
            let metadata = if let Some(s) = metadata_str {
                serde_json::from_str(&s).unwrap_or_default()
            } else {
                serde_json::Value::Object(Default::default())
            };

            Ok(ThoughtNodeResult {
                id: row.get(0)?,
                session_id: row.get(1)?,
                parent_id: row.get(2)?,
                content: row.get(3)?,
                thought_type: row.get(4)?,
                depth: row.get(5)?,
                breadth: row.get(6)?,
                confidence: row.get(7)?,
                metadata,
                created_at: row.get(9)?,
                namespace: row.get(10)?,
                graph_domain: row.get(11)?,
            })
        })?;

        let mut results = Vec::new();
        for node in nodes {
            results.push(node?);
        }

        Ok(results)
    }

    /// Get children of a specific node
    pub async fn get_children(&self, parent_id: i64) -> Result<Vec<ThoughtNodeResult>> {
        let conn = self.backend.code_graph().db_conn();
        let db = conn.lock().map_err(|e| anyhow::anyhow!("Failed to lock database: {}", e))?;

        let mut stmt = db.prepare(
            r#"
                SELECT n.id, n.session_id, n.parent_id, n.content, n.thought_type, n.depth, n.breadth,
                       n.confidence, n.metadata, n.created_at, n.namespace, n.graph_domain
                FROM reasoning_nodes n
                JOIN reasoning_edges e ON n.id = e.child_id
                WHERE e.parent_id = ?
                ORDER BY n.breadth, n.id
            "#,
        )?;

        let children = stmt.query_map([parent_id], |row| {
            let metadata_str: Option<String> = row.get(8)?;
            let metadata = if let Some(s) = metadata_str {
                serde_json::from_str(&s).unwrap_or_default()
            } else {
                serde_json::Value::Object(Default::default())
            };

            Ok(ThoughtNodeResult {
                id: row.get(0)?,
                session_id: row.get(1)?,
                parent_id: row.get(2)?,
                content: row.get(3)?,
                thought_type: row.get(4)?,
                depth: row.get(5)?,
                breadth: row.get(6)?,
                confidence: row.get(7)?,
                metadata,
                created_at: row.get(9)?,
                namespace: row.get(10)?,
                graph_domain: row.get(11)?,
            })
        })?;

        let mut results = Vec::new();
        for child in children {
            results.push(child?);
        }

        Ok(results)
    }

    /// Get session metrics
    pub async fn get_session_metrics(&self, session_id: &str) -> Result<SessionMetrics> {
        let conn = self.backend.code_graph().db_conn();
        let db = conn.lock().map_err(|e| anyhow::anyhow!("Failed to lock database: {}", e))?;

        // Get total nodes and max depth
        let mut stmt = db.prepare(
            r#"
                SELECT COUNT(*) as total_nodes, MAX(depth) as max_depth, AVG(confidence) as avg_confidence
                FROM reasoning_nodes
                WHERE session_id = ?
            "#,
        )?;

        let (total_nodes, max_depth, avg_confidence) = stmt.query_row([session_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<i64>>(1)?.unwrap_or(0),
                row.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
            ))
        })?;

        // Get node type counts
        let mut stmt = db.prepare(
            r#"
                SELECT thought_type, COUNT(*) as count
                FROM reasoning_nodes
                WHERE session_id = ?
                GROUP BY thought_type
            "#,
        )?;

        let node_type_rows = stmt
            .query_map([session_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))?;

        let mut node_types = std::collections::HashMap::new();
        for row in node_type_rows {
            let (thought_type, count) = row?;
            node_types.insert(thought_type, count);
        }

        Ok(SessionMetrics {
            total_nodes,
            max_depth,
            avg_confidence,
            node_types,
        })
    }

    /// Count reasoning episodes (legacy compatibility)
    pub async fn count_reasoning_episodes(&self) -> Result<i64> {
        // This is for legacy ReasoningEpisode nodes
        // In SQLite implementation, we count sessions instead
        let conn = self.backend.code_graph().db_conn();
        let db = conn.lock().map_err(|e| anyhow::anyhow!("Failed to lock database: {}", e))?;

        let count: i64 =
            db.query_row("SELECT COUNT(*) FROM reasoning_sessions", [], |row| row.get(0))?;

        Ok(count)
    }

    /// Get reasoning episode by ID (legacy compatibility)
    pub async fn get_reasoning_episode_by_id(
        &self,
        _episode_id: i64,
    ) -> Result<Option<ReasoningEpisodeResult>> {
        // This is for legacy ReasoningEpisode nodes
        // In SQLite implementation, we don't have separate episodes
        Ok(None)
    }

    /// Fetch episodes related to code (legacy compatibility)
    pub async fn fetch_related_episodes(
        &self,
        _code_id: i64,
    ) -> Result<Vec<ReasoningEpisodeResult>> {
        // This is for legacy ReasoningEpisode nodes
        // In SQLite implementation, we don't have separate episodes
        Ok(vec![])
    }
}

/// Result type for reasoning episodes (legacy compatibility)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReasoningEpisodeResult {
    pub id: i64,
    pub timestamp: i64,
    pub user_query: String,
    pub selected_mode: String,
    pub outcome: String,
    pub notes: Option<String>,
    pub namespace: String,
    pub graph_domain: String,
}
