//! Tree-of-Thought Engine - ST-3 Core Orchestrator
//!
//! Main orchestrator for ToT reasoning operations.
//! Handles session management, node expansion, and tree traversal.

use crate::databases::cognition_graph::ThoughtNodeProperties;
use crate::graph::Neo4jClient;
use crate::llm::factory::LlmFactory;
use crate::llm::LanguageModel;
use crate::reasoning::llm_adapter::ThoughtNodeDraft;
use crate::reasoning::{
    branch_manager::BranchManager,
    llm_adapter::{LLMOutputParser, PromptBuilder},
    metrics::{ReasoningMetrics, ThreadSafeReasoningMetrics},
    tree_logger::TreeLogger,
    ReasoningError, ReasoningNodeContext, ReasoningResult, ReasoningSessionManager,
};
use std::sync::Arc;
use std::time::Instant;

/// Tree-of-Thought Reasoning Engine
///
/// Core orchestrator that manages reasoning sessions and coordinates
/// node expansion, evaluation, and selection.
pub struct ToTEngine {
    /// Neo4j client for graph operations
    client: Arc<Neo4jClient>,

    /// Session manager for session lifecycle
    session_manager: ReasoningSessionManager,

    /// Language model for LLM-based expansion
    language_model: Option<Box<dyn LanguageModel>>,

    /// Branch manager for safety enforcement and circuit breaking
    branch_manager: BranchManager,

    /// Tree logger for deterministic logging
    tree_logger: Option<Arc<TreeLogger>>,

    /// Reasoning metrics for performance and health monitoring
    metrics: ThreadSafeReasoningMetrics,
}

impl ToTEngine {
    /// Create a new ToT engine with Neo4j client
    pub fn new(client: Arc<Neo4jClient>) -> Self {
        let session_manager = ReasoningSessionManager::new(client.clone());
        let branch_manager = BranchManager::default();
        let metrics = crate::reasoning::metrics::new_reasoning_metrics();

        Self {
            client,
            session_manager,
            language_model: None,
            branch_manager,
            tree_logger: None,
            metrics,
        }
    }

    /// Create a new ToT engine with language model
    pub fn with_language_model(
        client: Arc<Neo4jClient>,
        language_model: Box<dyn LanguageModel>,
    ) -> Self {
        let session_manager = ReasoningSessionManager::new(client.clone());
        let branch_manager = BranchManager::default();
        let metrics = crate::reasoning::metrics::new_reasoning_metrics();

        Self {
            client,
            session_manager,
            language_model: Some(language_model),
            branch_manager,
            tree_logger: None,
            metrics,
        }
    }

    /// Create a new ToT engine with LLM from environment
    pub async fn with_llm_from_env(client: Arc<Neo4jClient>) -> ReasoningResult<Self> {
        let language_model =
            LlmFactory::from_env().await.map_err(|e| ReasoningError::Neo4j(e.to_string()))?;

        let session_manager = ReasoningSessionManager::new(client.clone());
        let branch_manager = BranchManager::default();
        let metrics = crate::reasoning::metrics::new_reasoning_metrics();

        Ok(Self {
            client,
            session_manager,
            language_model: Some(language_model),
            branch_manager,
            tree_logger: None,
            metrics,
        })
    }

    /// Create a new ToT engine with tree logger
    pub fn with_tree_logger(mut self, tree_logger: Arc<TreeLogger>) -> Self {
        self.tree_logger = Some(tree_logger);
        self
    }

    /// Start a new reasoning session
    ///
    /// Creates a new session with root node and returns the session ID.
    /// Optional task_id and metadata can be provided for context.
    pub async fn start_session(
        &self,
        task_id: Option<String>,
        metadata: Option<String>,
    ) -> ReasoningResult<String> {
        let session_id =
            self.session_manager.start_session(task_id.clone(), metadata.clone()).await?;

        // Log session start if tree logger is available
        if let Some(ref tree_logger) = self.tree_logger {
            if let Err(e) =
                tree_logger.log_session_start(&session_id, task_id.as_deref(), metadata.as_deref())
            {
                eprintln!("Warning: Failed to log session start: {}", e);
            }
        }

        Ok(session_id)
    }

    /// Get the active node for a session
    ///
    /// For ST-3, returns the most recent leaf node.
    /// In future phases, will use sophisticated selection algorithms.
    pub async fn get_active_node(
        &self,
        session_id: &str,
    ) -> ReasoningResult<Option<ThoughtNodeProperties>> {
        self.session_manager.get_active_node(session_id).await
    }

    /// Expand a node once, creating child branches
    ///
    /// Takes a node and expands it into multiple thought branches.
    /// Production reasoning engine with LLM-based expansion.
    pub async fn expand_once(
        &mut self,
        session_id: &str,
        node_id: &str,
    ) -> ReasoningResult<Vec<ThoughtNodeProperties>> {
        let start_time = Instant::now();

        // Load node context
        let node_context =
            ReasoningNodeContext::load(self.client.clone(), session_id, node_id).await?;

        // Convert node to JSON for BranchManager
        let node_json = serde_json::to_value(&node_context.node)
            .map_err(|e| ReasoningError::Neo4j(format!("Failed to serialize node: {}", e)))?;

        // Check safety before expansion
        let safety_check_result = self.branch_manager.check_before_expand(session_id, &node_json);

        // Record safety violations if any
        if let Err(ref safety_error) = safety_check_result {
            let mut metrics = self.metrics.lock().unwrap();
            match safety_error {
                ReasoningError::RepetitiveThoughtPattern(_) => {
                    metrics.record_safety_violation(true);
                }
                _ => {
                    metrics.record_safety_violation(false);
                }
            }
        }

        safety_check_result?;

        // Perform expansion with real LLM - no stub fallback allowed
        let language_model = self.language_model.as_ref()
            .ok_or_else(|| ReasoningError::Neo4j("Language model not available for node expansion".to_string()))?;

        let expansion_result = self.llm_expand(&node_context, language_model.as_ref()).await;

        let branches = match expansion_result {
            Ok(branches) => {
                // Record successful expansion
                self.branch_manager.record_success(session_id, &node_json)?;
                branches
            }
            Err(e) => {
                // Record failed expansion
                self.branch_manager.record_failure(session_id, &node_json, &e.to_string())?;

                // Record metrics for failed expansion (no branches created)
                let duration_ms = start_time.elapsed().as_millis() as f64;
                let mut metrics = self.metrics.lock().unwrap();
                metrics.record_expand(node_context.depth() as u32, 0, false, duration_ms);
                metrics.record_llm_failure();

                return Err(e);
            }
        };

        // Store the new nodes
        let branch_contents: Vec<String> = branches.iter().map(|b| b.text.clone()).collect();
        let created_node_ids =
            self.session_manager.store_nodes(session_id, node_id, branch_contents.clone()).await?;

        // Record metrics for successful expansion
        let duration_ms = start_time.elapsed().as_millis() as f64;
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.record_expand(
                node_context.depth() as u32,
                branches.len() as u32,
                true,
                duration_ms,
            );

            // Export safety counters from branch manager and integrate them
            let (identical_count, safety_count) = self.branch_manager.export_safety_counters();
            for _ in 0..identical_count {
                metrics.record_safety_violation(true);
            }
            for _ in 0..safety_count {
                metrics.record_safety_violation(false);
            }
        }

        // Log expansion if tree logger is available
        if let Some(ref tree_logger) = self.tree_logger {
            if let Err(e) = tree_logger.log_expansion(
                session_id,
                node_id,
                node_context.depth() as i64,
                branches.len(),
                &branch_contents,
            ) {
                eprintln!("Warning: Failed to log expansion: {}", e);
            }
        }

        // Combine created node IDs with branch properties
        let mut created_nodes = Vec::new();
        for (i, (branch, node_id)) in branches.into_iter().zip(created_node_ids).enumerate() {
            let created_node = ThoughtNodeProperties {
                id: node_id.clone(),
                session_id: session_id.to_string(),
                parent_id: Some(node_id),
                step_index: node_context.node.step_index + 1 + i as i64,
                content: branch.text,
                score: Some(branch.confidence),
            };

            // Log score if tree logger is available
            if let Some(ref tree_logger) = self.tree_logger {
                if let Err(e) = tree_logger.log_score(
                    session_id,
                    &created_node.id,
                    (node_context.depth() + 1) as i64,
                    branch.confidence,
                    &format!("Branch {} confidence score", i + 1),
                ) {
                    eprintln!("Warning: Failed to log score: {}", e);
                }
            }

            created_nodes.push(created_node);
        }

        Ok(created_nodes)
    }

    /// Perform a single reasoning step
    ///
    /// Gets the active node and expands it once.
    /// Returns the newly created child nodes.
    pub async fn reasoning_step(
        &mut self,
        session_id: &str,
    ) -> ReasoningResult<Vec<ThoughtNodeProperties>> {
        // Get active node
        let active_node = self
            .get_active_node(session_id)
            .await?
            .ok_or_else(|| ReasoningError::SessionNotFound(session_id.to_string()))?;

        // Expand the active node
        self.expand_once(session_id, &active_node.id).await
    }

    /// Get all nodes for a session (for testing and debugging)
    pub async fn get_session_nodes(
        &self,
        session_id: &str,
    ) -> ReasoningResult<Vec<ThoughtNodeProperties>> {
        self.session_manager.get_all_nodes(session_id).await
    }

    /// Validate session invariants
    pub async fn validate_session(&self, session_id: &str) -> ReasoningResult<bool> {
        self.session_manager.validate_session(session_id).await
    }

    /// Get branch manager diagnostics for a session
    pub fn get_branch_diagnostics(
        &self,
        session_id: &str,
    ) -> crate::reasoning::branch_manager::BranchDiagnostics {
        self.branch_manager.get_diagnostics(session_id)
    }

    /// Get formatted logs for a session
    pub fn get_session_logs(&self, session_id: &str) -> ReasoningResult<String> {
        if let Some(ref tree_logger) = self.tree_logger {
            tree_logger
                .get_formatted_logs(session_id)
                .map_err(|e| ReasoningError::Neo4j(e.to_string()))
        } else {
            Ok("Tree logger not available".to_string())
        }
    }

    /// Get reasoning metrics snapshot
    ///
    /// Returns an immutable snapshot of current reasoning metrics
    /// for MCP integration and health monitoring.
    pub fn get_metrics_snapshot(&self) -> crate::reasoning::metrics::ReasoningMetricsSnapshot {
        let metrics = self.metrics.lock().unwrap();
        metrics.snapshot()
    }

    /// Get recent logs for a session
    pub fn get_recent_logs(
        &self,
        session_id: &str,
        limit: i64,
    ) -> ReasoningResult<Vec<crate::databases::logs::ReasoningLogEntry>> {
        if let Some(ref tree_logger) = self.tree_logger {
            tree_logger
                .get_recent_logs(session_id, limit)
                .map_err(|e| ReasoningError::Neo4j(e.to_string()))
        } else {
            Ok(Vec::new())
        }
    }

    /// LLM-based node expansion
    ///
    /// Uses LanguageModel to generate thought branches from context.
    async fn llm_expand(
        &self,
        node_context: &ReasoningNodeContext,
        language_model: &dyn LanguageModel,
    ) -> ReasoningResult<Vec<ThoughtNodeDraft>> {
        // Build prompt from context
        let prompt = PromptBuilder::create_prompt(node_context);

        // Call LLM directly (most implementations should be fast enough)
        let completion =
            language_model.complete(&prompt).map_err(|e| ReasoningError::Neo4j(e.to_string()))?;

        // Parse response into thought drafts
        let drafts = LLMOutputParser::parse_thoughts(&completion)
            .map_err(|e| ReasoningError::Neo4j(e.to_string()))?;

        // Validate each draft
        for draft in &drafts {
            LLMOutputParser::validate_draft(draft)
                .map_err(|e| ReasoningError::Neo4j(e.to_string()))?;
        }

        Ok(drafts)
    }

    
    /// Get session statistics
    pub async fn get_session_stats(&self, session_id: &str) -> ReasoningResult<SessionStats> {
        let nodes = self.get_session_nodes(session_id).await?;

        let total_nodes = nodes.len();
        let max_depth = nodes
            .iter()
            .map(|node| {
                // Calculate depth by counting parents
                let mut depth = 0;
                let mut current_parent = node.parent_id.clone();

                while let Some(parent_id) = current_parent {
                    depth += 1;
                    if let Some(parent) = nodes.iter().find(|n| n.id == parent_id) {
                        current_parent = parent.parent_id.clone();
                    } else {
                        break;
                    }
                }

                depth
            })
            .max()
            .unwrap_or(0);

        let leaf_nodes = nodes
            .iter()
            .filter(|node| !nodes.iter().any(|other| other.parent_id.as_ref() == Some(&node.id)))
            .count();

        let avg_score = nodes.iter().filter_map(|node| node.score).sum::<f64>()
            / nodes.iter().filter(|node| node.score.is_some()).count().max(1) as f64;

        Ok(SessionStats {
            total_nodes,
            max_depth,
            leaf_nodes,
            avg_score,
        })
    }
}

/// Session statistics for monitoring and debugging
#[derive(Debug, Clone)]
pub struct SessionStats {
    pub total_nodes: usize,
    pub max_depth: usize,
    pub leaf_nodes: usize,
    pub avg_score: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::databases::cognition_graph::ThoughtNodeProperties;

    // Mock Neo4j client for testing
    struct MockNeo4jClient;

    impl MockNeo4jClient {
        fn new() -> Arc<Self> {
            Arc::new(Self)
        }
    }

    #[test]
    fn test_engine_creation() {
        // This test would require a mock Neo4j client
        // For now, we test the type structure by checking it exists
        let _engine_type_check: std::marker::PhantomData<ToTEngine> = std::marker::PhantomData;
    }

    #[test]
    fn test_engine_with_tree_logger() {
        // Test that we can create the type structure with tree logger
        let _engine_type_check: std::marker::PhantomData<ToTEngine> = std::marker::PhantomData;
    }

    #[test]
    fn test_thought_node_draft_structure() {
        let branch = ThoughtNodeDraft {
            text: "Test branch content".to_string(),
            confidence: 0.75,
        };

        assert_eq!(branch.text, "Test branch content");
        assert_eq!(branch.confidence, 0.75);
    }

    #[test]
    fn test_session_stats_structure() {
        let stats = SessionStats {
            total_nodes: 10,
            max_depth: 5,
            leaf_nodes: 3,
            avg_score: 0.65,
        };

        assert_eq!(stats.total_nodes, 10);
        assert_eq!(stats.max_depth, 5);
        assert_eq!(stats.leaf_nodes, 3);
        assert_eq!(stats.avg_score, 0.65);
    }

    
    #[tokio::test]
    async fn test_session_stats_calculation() {
        // Create mock nodes for testing
        let nodes = vec![
            ThoughtNodeProperties {
                id: "root".to_string(),
                session_id: "test".to_string(),
                parent_id: None,
                step_index: 0,
                content: "Root".to_string(),
                score: Some(1.0),
            },
            ThoughtNodeProperties {
                id: "child1".to_string(),
                session_id: "test".to_string(),
                parent_id: Some("root".to_string()),
                step_index: 1,
                content: "Child 1".to_string(),
                score: Some(0.8),
            },
            ThoughtNodeProperties {
                id: "child2".to_string(),
                session_id: "test".to_string(),
                parent_id: Some("root".to_string()),
                step_index: 2,
                content: "Child 2".to_string(),
                score: Some(0.6),
            },
        ];

        // Calculate expected stats
        let total_nodes = nodes.len();
        let leaf_nodes = nodes
            .iter()
            .filter(|node| !nodes.iter().any(|other| other.parent_id.as_ref() == Some(&node.id)))
            .count();
        let avg_score = nodes.iter().filter_map(|node| node.score).sum::<f64>()
            / nodes.iter().filter(|node| node.score.is_some()).count() as f64;

        assert_eq!(total_nodes, 3);
        assert_eq!(leaf_nodes, 2); // child1 and child2 are leaves
        assert_eq!(avg_score, (1.0 + 0.8 + 0.6) / 3.0);
    }
}
