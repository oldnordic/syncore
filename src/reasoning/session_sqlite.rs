//! Reasoning Session Management - SQLite Backend (ST-12)
//!
//! Provides session lifecycle management for Tree-of-Thought reasoning
//! using SQLite backend instead of Neo4j.

use crate::databases::cognition_sqlite::{
    CognitionSqliteReader, CognitionSqliteWriter, SessionResult, ThoughtNodeProperties,
};
use crate::graph::SQLiteGraphBackend;
use crate::reasoning::{current_timestamp, generate_session_id, ReasoningError, ReasoningResult};

use std::sync::Arc;

/// Manages reasoning sessions and their context using SQLite backend
#[derive(Debug, Clone)]
pub struct ReasoningSessionManagerSqlite {
    backend: Arc<SQLiteGraphBackend>,
    writer: Arc<CognitionSqliteWriter>,
    reader: Arc<CognitionSqliteReader>,
}

impl ReasoningSessionManagerSqlite {
    /// Create a new session manager with SQLite backend
    pub async fn new(backend: SQLiteGraphBackend) -> Result<Self, ReasoningError> {
        let backend = Arc::new(backend);
        let backend_clone = (*backend).clone();
        let writer = Arc::new(CognitionSqliteWriter::new(backend_clone));
        let backend_clone2 = (*backend).clone();
        let reader = Arc::new(CognitionSqliteReader::new(backend_clone2));

        // Initialize schema
        writer.initialize_schema().await.map_err(ReasoningError::Database)?;

        Ok(Self {
            backend,
            writer,
            reader,
        })
    }

    /// Start a new reasoning session with root node
    ///
    /// Creates a new ReasoningSession and its root ThoughtNode.
    /// Returns session ID for subsequent operations.
    pub async fn start_session(&self, title: &str, description: &str) -> ReasoningResult<String> {
        let session_id = generate_session_id();
        let created_at = chrono::Utc::now().to_rfc3339();
        let namespace = "syncore_default"; // TODO: Get from config

        // Create session in SQLite
        let session_props = crate::databases::cognition_sqlite::ReasoningSessionProperties {
            id: session_id.clone(),
            title: title.to_string(),
            description: description.to_string(),
            created_at: created_at.clone(),
            namespace: namespace.to_string(),
            graph_domain: "reasoning".to_string(),
        };

        self.writer.create_session(session_props).await.map_err(|e| {
            ReasoningError::Database(anyhow::anyhow!("Failed to create session: {}", e))
        })?;

        // Create root thought node
        let root_node_props = ThoughtNodeProperties {
            id: 1, // Start with ID 1 for SQLite autoincrement
            session_id: session_id.clone(),
            parent_id: None, // Root node has no parent
            content: "Root node - reasoning session started".to_string(),
            thought_type: "root".to_string(),
            depth: 0,
            breadth: 0,
            confidence: 1.0,
            metadata: serde_json::json!({}),
            created_at,
            namespace: namespace.to_string(),
            graph_domain: "reasoning".to_string(),
        };

        self.writer.add_thought_node(root_node_props).await.map_err(|e| {
            ReasoningError::Database(anyhow::anyhow!("Failed to add thought node: {}", e))
        })?;

        Ok(session_id)
    }

    /// Retrieve session context by ID
    ///
    /// Returns session metadata and basic statistics.
    pub async fn get_session_context(&self, session_id: &str) -> ReasoningResult<SessionResult> {
        let session = self.reader.get_session(session_id).await.map_err(|e| {
            ReasoningError::Database(anyhow::anyhow!("Failed to get session: {}", e))
        })?;

        session.ok_or_else(|| ReasoningError::SessionNotFound(session_id.to_string()))
    }

    /// Get active node for a session
    ///
    /// For ST-12, active node is most recent leaf node.
    /// In future phases, this will use more sophisticated selection.
    pub async fn get_active_node(
        &self,
        session_id: &str,
    ) -> ReasoningResult<Option<ThoughtNodeProperties>> {
        let nodes = self.reader.get_nodes_for_session(session_id).await.map_err(|e| {
            ReasoningError::Database(anyhow::anyhow!("Failed to get nodes for session: {}", e))
        })?;

        if nodes.is_empty() {
            return Ok(None);
        }

        // Find leaf nodes (nodes with no children)
        let mut leaf_nodes = Vec::new();
        for node in &nodes {
            let children = self.reader.get_children(node.id).await.map_err(|e| {
                ReasoningError::Database(anyhow::anyhow!("Failed to get children: {}", e))
            })?;
            if children.is_empty() {
                leaf_nodes.push(node.clone());
            }
        }

        // Return the most recent leaf node (highest ID)
        let active_node = leaf_nodes.into_iter().max_by_key(|n| n.id);

        Ok(active_node.map(|n| ThoughtNodeProperties {
            id: n.id,
            session_id: n.session_id,
            parent_id: n.parent_id,
            content: n.content,
            thought_type: n.thought_type,
            depth: n.depth,
            breadth: n.breadth,
            confidence: n.confidence,
            metadata: n.metadata,
            created_at: n.created_at,
            namespace: n.namespace,
            graph_domain: n.graph_domain,
        }))
    }

    /// Add a thought node to a session
    pub async fn add_thought(
        &self,
        session_id: &str,
        parent_id: Option<i64>,
        content: &str,
        thought_type: &str,
        confidence: f64,
        metadata: serde_json::Value,
    ) -> ReasoningResult<i64> {
        // Get next ID (SQLite autoincrement)
        let next_id = self.get_next_node_id().await?;

        let node_props = ThoughtNodeProperties {
            id: next_id,
            session_id: session_id.to_string(),
            parent_id,
            content: content.to_string(),
            thought_type: thought_type.to_string(),
            depth: if parent_id.is_some() {
                1
            } else {
                0
            }, // TODO: Calculate proper depth
            breadth: 0, // TODO: Calculate proper breadth
            confidence,
            metadata,
            created_at: chrono::Utc::now().to_rfc3339(),
            namespace: "syncore_default".to_string(),
            graph_domain: "reasoning".to_string(),
        };

        self.writer.add_thought_node(node_props).await.map_err(|e| {
            ReasoningError::Database(anyhow::anyhow!("Failed to add thought node: {}", e))
        })?;

        Ok(next_id)
    }

    /// Get all nodes for a session
    pub async fn get_session_nodes(
        &self,
        session_id: &str,
    ) -> ReasoningResult<Vec<ThoughtNodeProperties>> {
        let nodes = self
            .reader
            .get_nodes_for_session(session_id)
            .await
            .map_err(ReasoningError::Database)?;

        Ok(nodes
            .into_iter()
            .map(|n| ThoughtNodeProperties {
                id: n.id,
                session_id: n.session_id,
                parent_id: n.parent_id,
                content: n.content,
                thought_type: n.thought_type,
                depth: n.depth,
                breadth: n.breadth,
                confidence: n.confidence,
                metadata: n.metadata,
                created_at: n.created_at,
                namespace: n.namespace,
                graph_domain: n.graph_domain,
            })
            .collect())
    }

    /// Prune a subtree starting from a given node
    pub async fn prune_subtree(&self, node_id: i64) -> ReasoningResult<()> {
        self.writer.delete_subtree(node_id).await.map_err(|e| {
            ReasoningError::Database(anyhow::anyhow!("Failed to delete subtree: {}", e))
        })?;

        Ok(())
    }

    /// Get session metrics
    pub async fn get_session_metrics(
        &self,
        session_id: &str,
    ) -> ReasoningResult<crate::databases::cognition_sqlite::SessionMetrics> {
        let metrics = self.reader.get_session_metrics(session_id).await.map_err(|e| {
            ReasoningError::Database(anyhow::anyhow!("Failed to get session metrics: {}", e))
        })?;

        Ok(metrics)
    }

    /// Get the next available node ID
    async fn get_next_node_id(&self) -> ReasoningResult<i64> {
        let conn = self.backend.code_graph().db_conn();
        let db = conn.lock().map_err(|e| {
            ReasoningError::Database(anyhow::anyhow!("Failed to lock database: {}", e))
        })?;

        let next_id: i64 = db
            .query_row("SELECT COALESCE(MAX(id), 0) + 1 FROM reasoning_nodes", [], |row| row.get(0))
            .map_err(|e| ReasoningError::Database(anyhow::anyhow!("Database error: {}", e)))?;

        Ok(next_id)
    }
}
