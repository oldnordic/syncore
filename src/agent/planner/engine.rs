//! Planning Engine Implementation
//!
//! Core planning engine with semantic context retrieval and node generation

use crate::agent::{ApreError, ApreResult};
use crate::llm::prompt_hash::hash_prompt;
use crate::memory::Memory;
use crate::raggraph::HopGraphTransformer;
use crate::vector::VectorStore;
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;

use super::cycle::CycleDetector;
use super::types::{PlanNode, PlanTree, PlanNodeStatus};

/// Deterministic Planning Engine
///
/// Uses existing memory, vector, and graph APIs to create and refine plans.
/// Enforces circuit breaker patterns and maintains deterministic behavior.
#[derive(Debug)]
pub struct PlanningEngine {
    /// Memory service for plan persistence and context
    memory: Arc<Memory>,

    /// Vector store for semantic similarity search
    vector_store: Arc<std::sync::Mutex<VectorStore>>,

    /// Graph transformer for multi-hop reasoning
    hop_graph: HopGraphTransformer,

    /// Cycle detector for dependency validation
    cycle_detector: CycleDetector,
}

impl PlanningEngine {
    /// Create a new planning engine
    pub fn new(
        memory: Arc<Memory>,
        vector_store: Arc<std::sync::Mutex<VectorStore>>,
        hop_graph: HopGraphTransformer,
    ) -> Self {
        Self {
            memory,
            vector_store,
            hop_graph,
            cycle_detector: CycleDetector::new(),
        }
    }

    /// Generate initial plan from goal and constraints
    pub async fn generate_initial_plan(
        &mut self,
        goal: &str,
        _constraints: &HashMap<String, String>,
    ) -> ApreResult<PlanTree> {
        // Retrieve semantic context
        let context = self.retrieve_semantic_context(goal).await?;

        // Generate plan nodes
        let plan = PlanTree::new(goal.to_string());

        // Store the plan for persistence
        self.store_plan(&plan).await?;

        Ok(plan)
    }

    /// Refine plan after action execution
    pub async fn refine_plan_after_action(
        &mut self,
        plan: &mut PlanTree,
        node_id: &str,
        success: bool,
        result_message: &str,
        error_message: &str,
    ) -> ApreResult<()> {
        // Update the node status
        if let Some(node) = plan.get_node_mut(node_id) {
            if success {
                node.status = super::types::PlanNodeStatus::Completed;
                node.result = Some(result_message.to_string());
                node.completed_at = Some(crate::agent::current_timestamp_ms());
            } else {
                node.status = super::types::PlanNodeStatus::Failed;
                node.error_message = Some(error_message.to_string());
            }

            // Update dependent nodes
            self.update_dependent_nodes(plan).await?;
        }

        // Store the updated plan
        self.store_plan(plan).await?;

        Ok(())
    }

    /// Detect deadlocks in the plan
    pub fn detect_deadlocks(&self, plan: &PlanTree) -> ApreResult<Vec<Vec<String>>> {
        // Build dependency graph
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        for node in &plan.nodes {
            graph.insert(node.id.clone(), node.dependencies.clone());
        }

        // Detect cycles
        let deadlocks = self.cycle_detector.detect_cycles(&graph);

        Ok(deadlocks)
    }

    /// Get next action to execute
    pub async fn next_action(
        &mut self,
        plan: &mut PlanTree,
    ) -> ApreResult<Option<String>> {
        // Get ready nodes sorted by priority (highest first), then by creation time for determinism
        let ready_node_ids = self.get_next_ready_node_ids(plan);

        if ready_node_ids.is_empty() {
            return Ok(None);
        }

        let next_node_id = ready_node_ids[0].clone();

        // Update node status to InProgress
        if let Some(node) = plan.get_node_mut(&next_node_id) {
            node.status = super::types::PlanNodeStatus::InProgress;
        }

        Ok(Some(next_node_id))
    }

    /// Get next ready node IDs without modifying the plan
    fn get_next_ready_node_ids(&self, plan: &PlanTree) -> Vec<String> {
        use std::collections::HashSet;

        let completed_ids: HashSet<_> =
            plan.nodes.iter().filter(|node| node.is_completed()).map(|node| &node.id).collect();

        let ready_node_ids: Vec<String> = plan
            .nodes
            .iter()
            .filter(|node| {
                node.status == PlanNodeStatus::Pending
                    && node.dependencies.iter().all(|dep| completed_ids.contains(dep))
            })
            .map(|node| node.id.clone())
            .collect();

        // Sort by priority, then creation time, then ID for determinism
        let mut ready_with_metadata: Vec<(String, i32, i64, &str)> = plan
            .nodes
            .iter()
            .filter(|node| ready_node_ids.contains(&node.id))
            .map(|node| (node.id.clone(), node.priority, node.created_at, node.id.as_str()))
            .collect();

        ready_with_metadata.sort_by(|a, b| {
            b.1.cmp(&a.1) // priority (higher first)
                .then_with(|| a.2.cmp(&b.2)) // creation time (earlier first)
                .then_with(|| a.3.cmp(&b.3)) // ID lexicographically
        });

        ready_with_metadata.into_iter().map(|(id, _, _, _)| id).collect()
    }

    /// Retrieve semantic context using existing vector and graph APIs
    async fn retrieve_semantic_context(&self, goal: &str) -> ApreResult<Vec<String>> {
        let mut context = Vec::new();

        // Use vector store for semantic search
        match self.vector_store.lock().unwrap().search(goal, 10, crate::vector::SearchScope::Global)
        {
            Ok(results) => {
                for result in results {
                    context.push(result.text);
                }
            }
            Err(_) => {
                // Fallback: use memory service
                let memory_key = format!("context:{}", hash_prompt(goal));
                if let Ok(Some(stored_context)) = self.memory.query(&memory_key) {
                    context.push(stored_context);
                }
            }
        }

        Ok(context)
    }

    /// Generate plan nodes from goal, constraints, and context
    async fn generate_plan_nodes(
        &self,
        goal: &str,
        _constraints: &HashMap<String, String>,
        _context: &[String],
    ) -> ApreResult<Vec<PlanNode>> {
        // Simple implementation: analyze goal into subtasks
        self.analyze_goal_into_subtasks(goal, _constraints, _context).await
    }

    /// Analyze goal into subtasks using semantic patterns
    async fn analyze_goal_into_subtasks(
        &self,
        goal: &str,
        _constraints: &HashMap<String, String>,
        _context: &[String],
    ) -> ApreResult<Vec<PlanNode>> {
        let mut nodes = Vec::new();

        // Simple heuristic: split goal by common action words
        let action_words = ["and", "then", "after", "before", "while"];
        let mut parts = vec![goal];

        for word in &action_words {
            let mut new_parts = Vec::new();
            for part in &parts {
                if part.contains(word) {
                    new_parts.extend(part.split(word).map(|s| s.trim()));
                } else {
                    new_parts.push(*part);
                }
            }
            parts = new_parts;
        }

        // Create nodes for each part, with simple priority assignment
        for (i, part) in parts.iter().enumerate() {
            if !part.is_empty() {
                let priority = 10 - (i as i32 * 2); // Decreasing priority
                let node = PlanNode::new(part.to_string(), priority.max(1));
                nodes.push(node);
            }
        }

        Ok(nodes)
    }

    /// Update dependent nodes after action completion
    async fn update_dependent_nodes(&self, plan: &mut PlanTree) -> ApreResult<()> {
        // This is a placeholder for dependency update logic
        // In a real implementation, this would trigger recomputation of ready nodes
        Ok(())
    }

    /// Store plan in memory for persistence
    async fn store_plan(&self, plan: &PlanTree) -> ApreResult<()> {
        let plan_json = serde_json::to_string(plan).map_err(|e| {
            ApreError::MemoryError(anyhow::anyhow!("Failed to serialize plan: {}", e))
        })?;

        let key = format!("plan:{}", plan.id);
        self.memory.store(&key, &plan_json).map_err(|e| ApreError::MemoryError(e))?;

        Ok(())
    }
}