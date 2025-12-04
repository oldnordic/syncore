//! Unified Reasoning Session - Backend Agnostic (ST-12)
//!
//! Provides a unified interface for reasoning sessions that can work
//! with either Neo4j or SQLiteGraph backends.

use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;

use crate::config::SyncoreConfig;
use crate::databases::cognition_graph::ThoughtNodeProperties as Neo4jThoughtNodeProperties;
use crate::databases::cognition_sqlite::{
    SessionResult, ThoughtNodeProperties as SqliteThoughtNodeProperties,
};
use crate::graph::{Neo4jClient, SQLiteGraphBackend};
use crate::reasoning::{
    ReasoningError, ReasoningResult, ReasoningSessionManager, ReasoningSessionManagerSqlite,
};

/// Backend-agnostic reasoning session
#[derive(Debug, Clone)]
pub enum ReasoningSessionBackend {
    Neo4j(Arc<ReasoningSessionManager>),
    Sqlite(Arc<ReasoningSessionManagerSqlite>),
}

/// Unified reasoning session that works with both backends
#[derive(Debug, Clone)]
pub struct ReasoningSession {
    backend: ReasoningSessionBackend,
    session_id: String,
}

impl ReasoningSession {
    /// Create a new reasoning session with the configured backend
    pub async fn new(title: &str, description: &str, config: &SyncoreConfig) -> Result<Self> {
        let (backend, session_id) = match config.reasoning.backend.as_str() {
            "neo4j" => {
                // Create Neo4j backend
                let neo4j_client = Arc::new(
                    Neo4jClient::connect(
                        &config.neo4j.uri,
                        &config.neo4j.user,
                        &config.neo4j.password,
                    )
                    .await?,
                );

                let session_manager = Arc::new(ReasoningSessionManager::new(neo4j_client));

                // Start session (Neo4j uses task_id and metadata)
                let session_id = session_manager
                    .start_session(Some(title.to_string()), Some(description.to_string()))
                    .await?;

                (ReasoningSessionBackend::Neo4j(session_manager), session_id)
            }
            "sqlite" | _ => {
                // Create SQLite backend (default)
                let sqlite_backend =
                    SQLiteGraphBackend::new(&config.paths.db_path, &config.reasoning.namespace)
                        .await?;

                let session_manager =
                    Arc::new(ReasoningSessionManagerSqlite::new(sqlite_backend).await?);

                // Start session (SQLite uses title and description)
                let session_id = session_manager.start_session(title, description).await?;

                (ReasoningSessionBackend::Sqlite(session_manager), session_id)
            }
        };

        Ok(Self {
            backend,
            session_id,
        })
    }

    /// Get the session ID
    pub fn id(&self) -> &str {
        &self.session_id
    }

    /// Add a thought to the session
    pub async fn add_thought(
        &self,
        parent_id: Option<i64>,
        content: &str,
        thought_type: &str,
        confidence: f64,
        metadata: Value,
    ) -> ReasoningResult<i64> {
        match &self.backend {
            ReasoningSessionBackend::Neo4j(manager) => {
                // Convert to Neo4j format
                let node_props = Neo4jThoughtNodeProperties {
                    id: format!("{}_{}", self.session_id, uuid::Uuid::new_v4()),
                    session_id: self.session_id.clone(),
                    parent_id: parent_id.map(|id| format!("{}_{}", self.session_id, id)),
                    step_index: 0, // TODO: Calculate proper step index
                    content: content.to_string(),
                    score: Some(confidence),
                };

                // Add node using Neo4j backend
                crate::databases::cognition_graph::add_thought_node(
                    // Get Neo4j client from manager
                    &manager.client,
                    node_props,
                )
                .await
                .map_err(|e| ReasoningError::Neo4j(e.to_string()))?;

                // Return generated ID (simplified)
                Ok(parent_id.unwrap_or(0) + 1)
            }
            ReasoningSessionBackend::Sqlite(manager) => {
                // Use SQLite backend directly
                manager
                    .add_thought(
                        &self.session_id,
                        parent_id,
                        content,
                        thought_type,
                        confidence,
                        metadata,
                    )
                    .await
            }
        }
    }

    /// Get the reasoning tree
    pub async fn get_tree(&self) -> ReasoningResult<Vec<SqliteThoughtNodeProperties>> {
        match &self.backend {
            ReasoningSessionBackend::Neo4j(manager) => {
                // Get nodes from Neo4j and convert
                let nodes = crate::databases::cognition_graph::get_nodes_for_session(
                    &manager.client,
                    &self.session_id,
                )
                .await
                .map_err(|e| ReasoningError::Neo4j(e.to_string()))?;

                // Convert Neo4j nodes to SQLite format
                let converted_nodes = nodes
                    .into_iter()
                    .map(|n| {
                        SqliteThoughtNodeProperties {
                            id: n.id.parse().unwrap_or(0), // Extract numeric ID from string
                            session_id: n.session_id,
                            parent_id: n.parent_id.and_then(|p| p.parse().ok()),
                            content: n.content,
                            thought_type: "thought".to_string(), // Default type
                            depth: 0,                            // TODO: Calculate from Neo4j data
                            breadth: 0,                          // TODO: Calculate from Neo4j data
                            confidence: n.score.unwrap_or(1.0),
                            metadata: serde_json::json!({}),
                            created_at: chrono::Utc::now().to_rfc3339(),
                            namespace: "syncore_default".to_string(),
                            graph_domain: "reasoning".to_string(),
                        }
                    })
                    .collect();

                Ok(converted_nodes)
            }
            ReasoningSessionBackend::Sqlite(manager) => {
                manager.get_session_nodes(&self.session_id).await
            }
        }
    }

    /// Prune a subtree
    pub async fn prune_subtree(&self, node_id: i64) -> ReasoningResult<()> {
        match &self.backend {
            ReasoningSessionBackend::Neo4j(_manager) => {
                // TODO: Implement Neo4j subtree pruning
                Err(ReasoningError::Neo4j(
                    "Subtree pruning not implemented for Neo4j backend".to_string(),
                ))
            }
            ReasoningSessionBackend::Sqlite(manager) => manager.prune_subtree(node_id).await,
        }
    }

    /// Get session context
    pub async fn get_context(&self) -> ReasoningResult<SessionResult> {
        match &self.backend {
            ReasoningSessionBackend::Neo4j(manager) => {
                let session = crate::databases::cognition_graph::get_session(
                    &manager.client,
                    &self.session_id,
                )
                .await
                .map_err(|e| ReasoningError::Neo4j(e.to_string()))?;

                session.ok_or_else(|| ReasoningError::SessionNotFound(self.session_id.clone())).map(
                    |s| SessionResult {
                        id: s.id,
                        title: s.task_id.unwrap_or_default(),
                        description: s.metadata.unwrap_or_default(),
                        created_at: s.created_at.to_string(),
                        namespace: "syncore_default".to_string(),
                        graph_domain: "reasoning".to_string(),
                    },
                )
            }
            ReasoningSessionBackend::Sqlite(manager) => {
                manager.get_session_context(&self.session_id).await
            }
        }
    }

    /// Get session metrics
    pub async fn get_metrics(
        &self,
    ) -> ReasoningResult<crate::databases::cognition_sqlite::SessionMetrics> {
        match &self.backend {
            ReasoningSessionBackend::Neo4j(_manager) => {
                // TODO: Implement Neo4j metrics
                Err(ReasoningError::Neo4j("Metrics not implemented for Neo4j backend".to_string()))
            }
            ReasoningSessionBackend::Sqlite(manager) => {
                manager.get_session_metrics(&self.session_id).await
            }
        }
    }
}
