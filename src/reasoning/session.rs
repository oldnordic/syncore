//! Reasoning Session Management - ST-3
//!
//! Provides session lifecycle management for Tree-of-Thought reasoning.
//! Handles session creation, context retrieval, and node management.

use crate::databases::cognition_graph::{
    create_session, get_session, SessionResult, ThoughtNodeProperties,
};
use crate::graph::Neo4jClient;
use crate::reasoning::{current_timestamp, generate_session_id, ReasoningError, ReasoningResult};

use std::sync::Arc;

/// Manages reasoning sessions and their context
#[derive(Debug, Clone)]
pub struct ReasoningSessionManager {
    pub client: Arc<Neo4jClient>,
}

impl ReasoningSessionManager {
    /// Create a new session manager with Neo4j client
    pub fn new(client: Arc<Neo4jClient>) -> Self {
        Self {
            client,
        }
    }

    /// Start a new reasoning session with root node
    ///
    /// Creates a new ReasoningSession and its root ThoughtNode.
    /// Returns the session ID for subsequent operations.
    pub async fn start_session(
        &self,
        task_id: Option<String>,
        metadata: Option<String>,
    ) -> ReasoningResult<String> {
        let session_id = generate_session_id();
        let created_at = current_timestamp();

        // Create session in Neo4j
        let session_props = crate::databases::cognition_graph::ReasoningSessionProperties {
            id: session_id.clone(),
            task_id: task_id.clone(),
            metadata: metadata.clone(),
            created_at,
            // Initialize PHASE ST-6 circuit breaker counters
            total_nodes: 0,
            depth: 0,
            breadth: 0,
            identical_expansions: 0,
            consecutive_errors: 0,
        };

        create_session(&self.client, session_props)
            .await
            .map_err(|e| ReasoningError::Neo4j(e.to_string()))?;

        // Create root thought node
        let root_node_props = ThoughtNodeProperties {
            id: format!("{}_root", session_id),
            session_id: session_id.clone(),
            parent_id: None, // Root node has no parent
            step_index: 0,
            content: "Root node - reasoning session started".to_string(),
            score: Some(1.0), // Root nodes get maximum score
        };

        crate::databases::cognition_graph::add_thought_node(&self.client, root_node_props)
            .await
            .map_err(|e| ReasoningError::Neo4j(e.to_string()))?;

        Ok(session_id)
    }

    /// Retrieve session context by ID
    ///
    /// Returns session metadata and basic statistics.
    pub async fn get_session_context(&self, session_id: &str) -> ReasoningResult<SessionResult> {
        let session =
            get_session(&self.client, session_id).await.map_err(ReasoningError::Database)?;

        session.ok_or_else(|| ReasoningError::SessionNotFound(session_id.to_string()))
    }

    /// Get the active node for a session
    ///
    /// For ST-3, the active node is the most recent leaf node.
    /// In future phases, this will use more sophisticated selection.
    pub async fn get_active_node(
        &self,
        session_id: &str,
    ) -> ReasoningResult<Option<ThoughtNodeProperties>> {
        use crate::databases::cognition_graph::get_nodes_for_session;

        let nodes = get_nodes_for_session(&self.client, session_id)
            .await
            .map_err(ReasoningError::Database)?;

        if nodes.is_empty() {
            return Ok(None);
        }

        // Find leaf nodes (nodes with no children)
        // For ST-3, we'll use the node with highest step_index as active
        let active_node = nodes.into_iter().max_by_key(|node| node.step_index).map(|result| {
            ThoughtNodeProperties {
                id: result.id,
                session_id: result.session_id,
                parent_id: result.parent_id,
                step_index: result.step_index,
                content: result.content,
                score: result.score,
            }
        });

        Ok(active_node)
    }

    /// Store new thought nodes in the session
    ///
    /// Creates multiple child nodes for a given parent.
    /// Maintains tree structure and ordering invariants.
    pub async fn store_nodes(
        &self,
        session_id: &str,
        parent_id: &str,
        branches: Vec<String>,
    ) -> ReasoningResult<Vec<String>> {
        use crate::databases::cognition_graph::get_nodes_for_session;

        // Verify parent exists in session
        let existing_nodes = get_nodes_for_session(&self.client, session_id)
            .await
            .map_err(ReasoningError::Database)?;

        let parent_exists =
            existing_nodes.iter().any(|node| node.id == parent_id && node.session_id == session_id);

        if !parent_exists {
            return Err(ReasoningError::InvalidParent(parent_id.to_string()));
        }

        // Determine next step index
        let next_step_index =
            existing_nodes.iter().map(|node| node.step_index).max().unwrap_or(0) + 1;

        let mut created_node_ids = Vec::new();

        for (i, branch_content) in branches.into_iter().enumerate() {
            let node_id = format!("{}_step{}_branch{}", session_id, next_step_index, i);

            let node_props = ThoughtNodeProperties {
                id: node_id.clone(),
                session_id: session_id.to_string(),
                parent_id: Some(parent_id.to_string()),
                step_index: next_step_index + i as i64,
                content: branch_content,
                score: None, // Scores will be assigned in future phases
            };

            crate::databases::cognition_graph::add_thought_node(&self.client, node_props)
                .await
                .map_err(|e| ReasoningError::Neo4j(e.to_string()))?;

            created_node_ids.push(node_id);
        }

        Ok(created_node_ids)
    }

    /// Get all nodes for a session (for testing and debugging)
    pub async fn get_all_nodes(
        &self,
        session_id: &str,
    ) -> ReasoningResult<Vec<ThoughtNodeProperties>> {
        use crate::databases::cognition_graph::get_nodes_for_session;

        let nodes = get_nodes_for_session(&self.client, session_id)
            .await
            .map_err(ReasoningError::Database)?;

        let node_props: Vec<ThoughtNodeProperties> = nodes
            .into_iter()
            .map(|result| ThoughtNodeProperties {
                id: result.id,
                session_id: result.session_id,
                parent_id: result.parent_id,
                step_index: result.step_index,
                content: result.content,
                score: result.score,
            })
            .collect();

        Ok(node_props)
    }

    /// Validate session invariants
    ///
    /// Ensures:
    /// - Session has exactly one root node
    /// - All nodes belong to the session
    /// - Parent/child relationships are valid
    pub async fn validate_session(&self, session_id: &str) -> ReasoningResult<bool> {
        let nodes = self.get_all_nodes(session_id).await?;

        if nodes.is_empty() {
            return Ok(false); // Invalid: no nodes
        }

        // Count root nodes (nodes with no parent)
        let root_count = nodes.iter().filter(|node| node.parent_id.is_none()).count();

        if root_count != 1 {
            return Ok(false); // Invalid: must have exactly one root
        }

        // Verify all nodes belong to the session
        let all_belong_to_session = nodes.iter().all(|node| node.session_id == session_id);

        if !all_belong_to_session {
            return Ok(false); // Invalid: node belongs to wrong session
        }

        // Verify parent references are valid
        for node in &nodes {
            if let Some(ref parent_id) = node.parent_id {
                let parent_exists =
                    nodes.iter().any(|potential_parent| potential_parent.id == *parent_id);

                if !parent_exists {
                    return Ok(false); // Invalid: parent doesn't exist
                }
            }
        }

        Ok(true) // All invariants satisfied
    }

    // ==================== PHASE ST-8: TASK INTEGRATION ====================

    /// Get or create session for a task
    ///
    /// Helper method for IntelliTask integration.
    /// Checks if session exists for task_id, creates new one if not.
    pub async fn get_or_create_session_for_task(
        &self,
        task_id: &str,
        task_description: Option<&str>,
    ) -> ReasoningResult<String> {
        // For now, always create a new session
        // In future implementation, we could check for existing sessions
        let session_id = self
            .start_session(Some(task_id.to_string()), task_description.map(|s| s.to_string()))
            .await?;

        Ok(session_id)
    }

    /// Find sessions associated with a task
    ///
    /// Returns all session IDs that have the given task_id in their metadata.
    /// Note: This is a simplified implementation - in practice would query Neo4j directly.
    pub async fn find_sessions_for_task(&self, _task_id: &str) -> ReasoningResult<Vec<String>> {
        // For now, return empty vector - would need to implement get_all_sessions
        // or query Neo4j directly for sessions with matching task_id
        Ok(Vec::new())
    }

    /// Check if session is associated with a task
    pub async fn is_session_for_task(
        &self,
        session_id: &str,
        task_id: &str,
    ) -> ReasoningResult<bool> {
        let session_context = self.get_session_context(session_id).await?;
        Ok(session_context.task_id.as_ref().map_or(false, |tid| tid == task_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::databases::cognition_graph::ThoughtNodeProperties;
    use std::sync::Arc;

    // Mock Neo4j client for testing
    struct MockNeo4jClient;

    impl MockNeo4jClient {
        fn new() -> Arc<Self> {
            Arc::new(Self)
        }
    }

    #[test]
    fn test_session_manager_creation() {
        // Test that we can create the type structure
        let _manager_type_check: std::marker::PhantomData<ReasoningSessionManager> =
            std::marker::PhantomData;
    }

    #[test]
    fn test_thought_node_properties_structure() {
        let node = ThoughtNodeProperties {
            id: "test_node".to_string(),
            session_id: "test_session".to_string(),
            parent_id: Some("parent_node".to_string()),
            step_index: 5,
            content: "Test content".to_string(),
            score: Some(0.85),
        };

        assert_eq!(node.id, "test_node");
        assert_eq!(node.session_id, "test_session");
        assert_eq!(node.parent_id, Some("parent_node".to_string()));
        assert_eq!(node.step_index, 5);
        assert_eq!(node.content, "Test content");
        assert_eq!(node.score, Some(0.85));
    }
}
