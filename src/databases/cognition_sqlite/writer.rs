//! Cognition SQLite Writer - Write Operations for Reasoning Episodes
//!
//! Provides all write operations for cognition entities using SQLite backend.

use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::Value;

use super::schema::*;
use crate::graph::SQLiteGraphBackend;

/// Writer for cognition entities using SQLite backend
#[derive(Debug, Clone)]
pub struct CognitionSqliteWriter {
    backend: SQLiteGraphBackend,
}

impl CognitionSqliteWriter {
    /// Create a new writer with the given SQLite backend
    pub fn new(backend: SQLiteGraphBackend) -> Self {
        Self {
            backend,
        }
    }

    /// Initialize the cognition schema in SQLite
    pub async fn initialize_schema(&self) -> Result<()> {
        let conn = self.backend.code_graph().db_conn();
        let db = conn.lock().map_err(|e| anyhow::anyhow!("Failed to lock database: {}", e))?;

        // Create reasoning_sessions table
        db.execute(
            r#"
                CREATE TABLE IF NOT EXISTS reasoning_sessions (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    description TEXT,
                    created_at TEXT NOT NULL,
                    namespace TEXT NOT NULL,
                    graph_domain TEXT NOT NULL DEFAULT 'cognition'
                )
            "#,
            [],
        )?;

        // Create reasoning_nodes table
        db.execute(
            r#"
                CREATE TABLE IF NOT EXISTS reasoning_nodes (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    session_id TEXT NOT NULL,
                    parent_id INTEGER,
                    content TEXT NOT NULL,
                    thought_type TEXT NOT NULL,
                    depth INTEGER NOT NULL DEFAULT 0,
                    breadth INTEGER NOT NULL DEFAULT 0,
                    confidence REAL NOT NULL DEFAULT 1.0,
                    metadata TEXT,
                    created_at TEXT NOT NULL,
                    namespace TEXT NOT NULL,
                    graph_domain TEXT NOT NULL DEFAULT 'cognition',
                    FOREIGN KEY (session_id) REFERENCES reasoning_sessions(id) ON DELETE CASCADE
                )
            "#,
            [],
        )?;

        // Create reasoning_edges table for parent-child relationships
        db.execute(
            r#"
                CREATE TABLE IF NOT EXISTS reasoning_edges (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    parent_id INTEGER NOT NULL,
                    child_id INTEGER NOT NULL,
                    session_id TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    FOREIGN KEY (parent_id) REFERENCES reasoning_nodes(id) ON DELETE CASCADE,
                    FOREIGN KEY (child_id) REFERENCES reasoning_nodes(id) ON DELETE CASCADE,
                    FOREIGN KEY (session_id) REFERENCES reasoning_sessions(id) ON DELETE CASCADE,
                    UNIQUE(parent_id, child_id)
                )
            "#,
            [],
        )?;

        // Create indexes for performance
        db.execute(
            "CREATE INDEX IF NOT EXISTS idx_reasoning_nodes_session_id ON reasoning_nodes(session_id)",
            [],
        )?;
        db.execute(
            "CREATE INDEX IF NOT EXISTS idx_reasoning_nodes_parent_id ON reasoning_nodes(parent_id)",
            [],
        )?;
        db.execute(
            "CREATE INDEX IF NOT EXISTS idx_reasoning_nodes_namespace ON reasoning_nodes(namespace)",
            [],
        )?;
        db.execute(
            "CREATE INDEX IF NOT EXISTS idx_reasoning_sessions_namespace ON reasoning_sessions(namespace)",
            [],
        )?;

        Ok(())
    }

    /// Create a new reasoning session
    pub async fn create_session(&self, props: ReasoningSessionProperties) -> Result<()> {
        let conn = self.backend.code_graph().db_conn();
        let db = conn.lock().map_err(|e| anyhow::anyhow!("Failed to lock database: {}", e))?;

        db.execute(
            r#"
                INSERT OR REPLACE INTO reasoning_sessions 
                (id, title, description, created_at, namespace, graph_domain)
                VALUES (?, ?, ?, ?, ?, ?)
            "#,
            rusqlite::params![
                props.id,
                props.title,
                props.description,
                props.created_at,
                props.namespace,
                props.graph_domain,
            ],
        )?;

        Ok(())
    }

    /// Add a thought node to a reasoning session
    pub async fn add_thought_node(&self, props: ThoughtNodeProperties) -> Result<()> {
        let conn = self.backend.code_graph().db_conn();
        let db = conn.lock().map_err(|e| anyhow::anyhow!("Failed to lock database: {}", e))?;

        // Insert the node
        db.execute(
            r#"
                INSERT OR REPLACE INTO reasoning_nodes 
                (id, session_id, parent_id, content, thought_type, depth, breadth, 
                 confidence, metadata, created_at, namespace, graph_domain)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            rusqlite::params![
                props.id,
                props.session_id,
                props.parent_id,
                props.content,
                props.thought_type,
                props.depth,
                props.breadth,
                props.confidence,
                serde_json::to_string(&props.metadata)?,
                props.created_at,
                props.namespace,
                props.graph_domain,
            ],
        )?;

        // Create parent-child edge if parent exists
        if let Some(parent_id) = props.parent_id {
            db.execute(
                r#"
                    INSERT OR REPLACE INTO reasoning_edges 
                    (parent_id, child_id, session_id, created_at)
                    VALUES (?, ?, ?, ?)
                "#,
                rusqlite::params![parent_id, props.id, props.session_id, Utc::now().to_rfc3339(),],
            )?;
        }

        Ok(())
    }

    /// Delete a reasoning episode (legacy compatibility)
    pub async fn delete_reasoning_episode(&self, episode_id: i64) -> Result<()> {
        // This is for legacy ReasoningEpisode nodes
        // In SQLite implementation, we don't have separate episodes
        // This is a no-op for now
        Ok(())
    }

    /// Upsert a reasoning episode (legacy compatibility)
    pub async fn upsert_reasoning_episode(&self, _props: ReasoningEpisodeProperties) -> Result<()> {
        // This is for legacy ReasoningEpisode nodes
        // In SQLite implementation, we don't have separate episodes
        // This is a no-op for now
        Ok(())
    }

    /// Create a USES relationship between reasoning episode and code reference
    pub async fn create_uses_relationship(&self, _episode_id: i64, _code_id: i64) -> Result<()> {
        // This is for legacy ReasoningEpisode nodes
        // In SQLite implementation, we don't have separate episodes
        // This is a no-op for now
        Ok(())
    }

    /// Delete a subtree starting from a given node
    pub async fn delete_subtree(&self, node_id: i64) -> Result<()> {
        let conn = self.backend.code_graph().db_conn();
        let db = conn.lock().map_err(|e| anyhow::anyhow!("Failed to lock database: {}", e))?;

        // First, find all descendants of the node
        let mut descendants = vec![node_id];
        let mut current_level = vec![node_id];

        while !current_level.is_empty() {
            let mut next_level = Vec::new();
            let placeholders = current_level.iter().map(|_| "?").collect::<Vec<_>>().join(",");

            let query = format!(
                "SELECT DISTINCT child_id FROM reasoning_edges WHERE parent_id IN ({})",
                placeholders
            );

            let params: Vec<&dyn rusqlite::ToSql> =
                current_level.iter().map(|id| id as &dyn rusqlite::ToSql).collect();

            let mut stmt = db.prepare(&query)?;
            let rows = stmt.query_map(&params[..], |row| Ok(row.get::<_, i64>(0)?))?;

            for row in rows {
                let child_id = row?;
                if !descendants.contains(&child_id) {
                    descendants.push(child_id);
                    next_level.push(child_id);
                }
            }

            current_level = next_level;
        }

        // Delete all descendants and their edges
        if !descendants.is_empty() {
            let placeholders = descendants.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let params: Vec<&dyn rusqlite::ToSql> =
                descendants.iter().map(|id| id as &dyn rusqlite::ToSql).collect();

            // Delete edges (need to double the params for both IN clauses)
            let delete_edges_query = format!(
                "DELETE FROM reasoning_edges WHERE parent_id IN ({}) OR child_id IN ({})",
                placeholders, placeholders
            );
            let mut edge_params = params.clone();
            edge_params.extend_from_slice(&params);
            db.execute(&delete_edges_query, &edge_params[..])?;

            // Delete nodes
            let delete_nodes_query =
                format!("DELETE FROM reasoning_nodes WHERE id IN ({})", placeholders);
            db.execute(&delete_nodes_query, &params[..])?;
        }

        Ok(())
    }
}
