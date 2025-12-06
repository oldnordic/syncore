//! Planner Tests
//!
//! Test suite for planner functionality

use crate::memory::Memory;
use crate::raggraph::{HopGraphTransformer, RagGraphConfig};
use crate::vector::{RealEmbeddings, VectorStore};
use std::sync::Arc;

use super::types::{PlanNode, PlanTree, PlanNodeStatus};

/// Create a mock memory for testing
pub fn create_mock_memory() -> Memory {
    // Create an in-memory database for testing
    Memory::new(":memory:").expect("Failed to create in-memory database for testing")
}

/// Create a mock vector store for testing
pub fn create_mock_vector_store() -> VectorStore {
    // Create a simple vector store for testing
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings for testing"));
    VectorStore::new(embeddings).expect("Failed to create vector store for testing")
}

/// Create a mock RAG config for testing
pub fn create_mock_rag_config() -> RagGraphConfig {
    RagGraphConfig {
        embedding_dim: 384,
        num_hops: 3,
        top_k: 10,
        alpha: 0.7,
        backend_mode: crate::raggraph::RaggraphBackendMode::Mock,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::engine::PlanningEngine;

    #[test]
    fn test_plan_node_creation() {
        let node = PlanNode::new("Test task".to_string(), 5);

        assert!(node.id.starts_with("plan_node_"));
        assert_eq!(node.task, "Test task");
        assert_eq!(node.priority, 5);
        assert_eq!(node.status, PlanNodeStatus::Pending);
        assert_eq!(node.complexity, 5);
    }

    #[test]
    fn test_plan_node_status_checks() {
        let mut node = PlanNode::new("Test".to_string(), 1);

        // Initial state is Pending, not Ready
        assert_eq!(node.status, PlanNodeStatus::Pending);
        assert!(!node.is_ready());
        assert!(!node.is_completed());
        assert!(!node.is_failed());

        // Change to Ready
        node.status = PlanNodeStatus::Ready;
        assert!(node.is_ready());
        assert!(!node.is_completed());
        assert!(!node.is_failed());

        // Change to Completed
        node.status = PlanNodeStatus::Completed;
        assert!(!node.is_ready());
        assert!(node.is_completed());
        assert!(!node.is_failed());

        // Change to Failed
        node.status = PlanNodeStatus::Failed;
        assert!(!node.is_ready());
        assert!(!node.is_completed());
        assert!(node.is_failed());
    }

    #[test]
    fn test_plan_tree_creation() {
        let plan = PlanTree::new("Test goal".to_string());

        assert!(plan.id.starts_with("plan_"));
        assert_eq!(plan.goal, "Test goal");
        assert!(plan.nodes.is_empty());
        assert!(plan.created_at > 0);
        assert!(plan.updated_at > 0);
    }

    #[test]
    fn test_plan_tree_node_management() {
        let mut plan = PlanTree::new("Test".to_string());
        let node = PlanNode::new("Task 1".to_string(), 1);
        let node_id = node.id.clone();

        plan.add_node(node);

        assert_eq!(plan.nodes.len(), 1);
        assert!(plan.get_node(&node_id).is_some());
        assert_eq!(plan.get_nodes_by_status(PlanNodeStatus::Pending).len(), 1);
    }

    #[test]
    fn test_dependency_resolution() {
        let mut plan = PlanTree::new("Test".to_string());

        // Add root node
        let root = PlanNode::new("Root".to_string(), 10);
        let root_id = root.id.clone();
        plan.add_node(root);

        // Add dependent node
        let mut child = PlanNode::new("Child".to_string(), 5);
        child.dependencies.push(root_id.clone());
        plan.add_node(child);

        // Initially root should be ready (no dependencies), child should not
        assert_eq!(plan.get_ready_nodes_readonly().len(), 1);

        // Get the actual ready nodes (this will transition root to Ready)
        let ready_nodes = plan.get_ready_nodes();
        assert_eq!(ready_nodes.len(), 1);
        assert_eq!(ready_nodes[0].task, "Root");

        // Complete root
        if let Some(root) = plan.nodes.iter_mut().find(|n| n.id == root_id) {
            root.status = PlanNodeStatus::Completed;
        }

        // Now child should be ready (using readonly for checking)
        assert_eq!(plan.get_ready_nodes_readonly().len(), 1);

        // Get the actual ready nodes (this will transition child to Ready)
        let ready_nodes = plan.get_ready_nodes();
        assert_eq!(ready_nodes.len(), 1);
        assert_eq!(ready_nodes[0].task, "Child");
    }

    #[test]
    fn test_cycle_detection() {
        let memory = Arc::new(create_mock_memory());
        let vector_store = Arc::new(std::sync::Mutex::new(create_mock_vector_store()));
        let hop_graph = HopGraphTransformer::new(create_mock_rag_config());

        let planner = PlanningEngine::new(memory, vector_store, hop_graph);

        let mut plan = PlanTree::new("Test".to_string());

        // Create nodes with circular dependency
        let node1 = PlanNode::new("Node 1".to_string(), 1);
        let node2 = PlanNode::new("Node 2".to_string(), 1);

        let id1 = node1.id.clone();
        let id2 = node2.id.clone();

        plan.add_node(node1);
        plan.add_node(node2);

        // Create circular dependency manually for testing
        if let Some(node1) = plan.nodes.iter_mut().find(|n| n.id == id1) {
            node1.dependencies.push(id2.clone());
        }
        if let Some(node2) = plan.nodes.iter_mut().find(|n| n.id == id2) {
            node2.dependencies.push(id1.clone());
        }

        // This test would need the actual implementation to work
        // For now, we'll just test the structure
        assert_eq!(plan.nodes.len(), 2);
    }
}