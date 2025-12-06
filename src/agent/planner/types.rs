//! Planner Core Types
//!
//! Defines the fundamental data structures for planning functionality

use crate::agent::current_timestamp_ms;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Plan node status representing execution state
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PlanNodeStatus {
    /// Node is ready to be executed (dependencies satisfied)
    Ready,
    /// Node is currently being executed
    InProgress,
    /// Node completed successfully
    Completed,
    /// Node failed and needs attention
    Failed,
    /// Node is blocked by dependencies
    Pending,
    /// Node was skipped (not needed)
    Skipped,
}

/// Plan node representing a single action in the plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanNode {
    /// Unique identifier for this plan node
    pub id: String,

    /// Task description
    pub task: String,

    /// Node priority (higher = more important)
    pub priority: i32,

    /// Current execution status
    pub status: PlanNodeStatus,

    /// Dependencies (IDs of nodes that must complete first)
    pub dependencies: Vec<String>,

    /// Estimated complexity (1-10)
    pub complexity: i32,

    /// Creation timestamp
    pub created_at: i64,

    /// Completion timestamp (if applicable)
    pub completed_at: Option<i64>,

    /// Error message (if failed)
    pub error_message: Option<String>,

    /// Result/output from execution (if any)
    pub result: Option<String>,

    /// Metadata for graph retrieval
    pub metadata: HashMap<String, String>,
}

impl PlanNode {
    /// Create a new plan node
    pub fn new(task: String, priority: i32) -> Self {
        let id = format!("plan_node_{}", Uuid::new_v4());
        Self {
            id,
            task,
            priority,
            status: PlanNodeStatus::Pending,
            dependencies: Vec::new(),
            complexity: 5, // Default medium complexity
            created_at: current_timestamp_ms(),
            completed_at: None,
            error_message: None,
            result: None,
            metadata: HashMap::new(),
        }
    }

    /// Check if node is ready for execution
    pub fn is_ready(&self) -> bool {
        self.status == PlanNodeStatus::Ready
    }

    /// Check if node is completed successfully
    pub fn is_completed(&self) -> bool {
        self.status == PlanNodeStatus::Completed
    }

    /// Check if node has failed
    pub fn is_failed(&self) -> bool {
        self.status == PlanNodeStatus::Failed
    }
}

/// Plan tree containing all nodes and their relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanTree {
    /// Unique identifier for this plan tree
    pub id: String,

    /// Plan goal description
    pub goal: String,

    /// All nodes in the plan
    pub nodes: Vec<PlanNode>,

    /// Creation timestamp
    pub created_at: i64,

    /// Last updated timestamp
    pub updated_at: i64,
}

impl PlanTree {
    /// Create a new plan tree
    pub fn new(goal: String) -> Self {
        let id = format!("plan_{}", Uuid::new_v4());
        let now = current_timestamp_ms();
        Self {
            id,
            goal,
            nodes: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Add a node to the plan
    pub fn add_node(&mut self, node: PlanNode) {
        self.updated_at = current_timestamp_ms();
        self.nodes.push(node);
    }

    /// Get node by ID
    pub fn get_node(&self, node_id: &str) -> Option<&PlanNode> {
        self.nodes.iter().find(|node| node.id == node_id)
    }

    /// Get mutable node by ID
    pub fn get_node_mut(&mut self, node_id: &str) -> Option<&mut PlanNode> {
        self.nodes.iter_mut().find(|node| node.id == node_id)
    }

    /// Get nodes by status
    pub fn get_nodes_by_status(&self, status: PlanNodeStatus) -> Vec<&PlanNode> {
        self.nodes.iter().filter(|node| node.status == status).collect()
    }

    /// Get ready nodes (those with satisfied dependencies)
    /// This method transitions Pending nodes with satisfied dependencies to Ready
    pub fn get_ready_nodes(&mut self) -> Vec<&mut PlanNode> {
        use std::collections::HashSet;

        let completed_ids: HashSet<_> =
            self.nodes.iter().filter(|node| node.is_completed()).map(|node| &node.id).collect();

        // First, identify ready nodes by their IDs
        let ready_node_ids: Vec<String> = self.nodes
            .iter()
            .filter(|node| {
                node.status == PlanNodeStatus::Pending
                    && node.dependencies.iter().all(|dep| completed_ids.contains(dep))
            })
            .map(|node| node.id.clone())
            .collect();

        // Then transition them to Ready (separate phase)
        for node in &mut self.nodes {
            if ready_node_ids.contains(&node.id) {
                node.status = PlanNodeStatus::Ready;
            }
        }

        // Finally, return mutable references to the ready nodes
        self.nodes.iter_mut()
            .filter(|node| node.status == PlanNodeStatus::Ready)
            .collect()
    }

    /// Get ready nodes without modifying their status (read-only check)
    pub fn get_ready_nodes_readonly(&self) -> Vec<&PlanNode> {
        use std::collections::HashSet;

        let completed_ids: HashSet<_> =
            self.nodes.iter().filter(|node| node.is_completed()).map(|node| &node.id).collect();

        self.nodes
            .iter()
            .filter(|node| {
                node.status == PlanNodeStatus::Ready ||
                    (node.status == PlanNodeStatus::Pending
                        && node.dependencies.iter().all(|dep| completed_ids.contains(dep)))
            })
            .collect()
    }

    /// Check if plan is complete (all nodes completed or skipped)
    pub fn is_complete(&self) -> bool {
        self.nodes.iter().all(|node| node.is_completed() || node.status == PlanNodeStatus::Skipped)
    }
}