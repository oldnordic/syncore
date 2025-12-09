//! Agent Planner and Reflection Stabilization Tests
//!
//! Comprehensive tests to ensure deterministic behavior and correct
//! implementation of core planner and reflection functionality.

use std::sync::Arc;
use syncore::agent::planner::{PlanNode, PlanNodeStatus, PlanTree};
use syncore::agent::reflection::FailureCategory;

#[cfg(test)]
mod planner_stabilization_tests {
    use super::*;

    #[test]
    fn test_plan_node_status_transitions() {
        let mut node = PlanNode::new("Test task".to_string(), 5);

        // Initial state should be Pending
        assert_eq!(node.status, PlanNodeStatus::Pending);
        assert!(!node.is_ready()); // Not Ready by default
        assert!(!node.is_completed());
        assert!(!node.is_failed());

        // Transition to Ready (dependencies satisfied)
        node.status = PlanNodeStatus::Ready;
        assert!(node.is_ready());
        assert!(!node.is_completed());
        assert!(!node.is_failed());

        // Transition to InProgress
        node.status = PlanNodeStatus::InProgress;
        assert!(!node.is_ready()); // InProgress is not Ready
        assert!(!node.is_completed());
        assert!(!node.is_failed());

        // Transition to Completed
        node.status = PlanNodeStatus::Completed;
        assert!(!node.is_ready());
        assert!(node.is_completed());
        assert!(!node.is_failed());

        // Transition to Failed
        node.status = PlanNodeStatus::Failed;
        assert!(!node.is_ready());
        assert!(!node.is_completed());
        assert!(node.is_failed());

        // Transition to Skipped
        node.status = PlanNodeStatus::Skipped;
        assert!(!node.is_ready());
        assert!(!node.is_completed());
        assert!(!node.is_failed());
    }

    #[test]
    fn test_simple_dependency_resolution() {
        let mut plan = PlanTree::new("Simple dependency test".to_string());

        // Create A -> B dependency chain
        let node_a = PlanNode::new("Task A".to_string(), 10);
        let node_b = PlanNode::new("Task B".to_string(), 5);

        let id_a = node_a.id.clone();
        let id_b = node_b.id.clone();

        plan.add_node(node_a);
        plan.add_node(node_b);

        // Add dependency B depends on A
        if let Some(node_b) = plan.nodes.iter_mut().find(|n| n.id == id_b) {
            node_b.dependencies.push(id_a.clone());
        }

        // Initially, A should be ready (no deps), B should not
        let ready_nodes = plan.get_ready_nodes_readonly();
        assert_eq!(ready_nodes.len(), 1);
        assert_eq!(ready_nodes[0].task, "Task A");

        // Get ready nodes (transitions A to Ready)
        let ready_nodes = plan.get_ready_nodes();
        assert_eq!(ready_nodes.len(), 1);
        assert_eq!(ready_nodes[0].task, "Task A");

        // Complete A
        if let Some(node_a) = plan.nodes.iter_mut().find(|n| n.id == id_a) {
            node_a.status = PlanNodeStatus::Completed;
        }

        // Now B should be ready (using readonly for checking)
        let ready_nodes = plan.get_ready_nodes_readonly();
        assert_eq!(ready_nodes.len(), 1);
        assert_eq!(ready_nodes[0].task, "Task B");
    }

    #[test]
    fn test_diamond_dependency_pattern() {
        let mut plan = PlanTree::new("Diamond pattern test".to_string());

        // Create diamond: A -> (B, C) -> D
        let node_a = PlanNode::new("Task A".to_string(), 10);
        let node_b = PlanNode::new("Task B".to_string(), 8);
        let node_c = PlanNode::new("Task C".to_string(), 6);
        let node_d = PlanNode::new("Task D".to_string(), 4);

        let id_a = node_a.id.clone();
        let id_b = node_b.id.clone();
        let id_c = node_c.id.clone();
        let id_d = node_d.id.clone();

        plan.add_node(node_a);
        plan.add_node(node_b);
        plan.add_node(node_c);
        plan.add_node(node_d);

        // Set dependencies: B depends on A, C depends on A, D depends on B and C
        for node in plan.nodes.iter_mut() {
            if node.id == id_b {
                node.dependencies.push(id_a.clone());
            } else if node.id == id_c {
                node.dependencies.push(id_a.clone());
            } else if node.id == id_d {
                node.dependencies.push(id_b.clone());
                node.dependencies.push(id_c.clone());
            }
        }

        // Initially only A should be ready
        let ready_nodes = plan.get_ready_nodes_readonly();
        assert_eq!(ready_nodes.len(), 1);
        assert_eq!(ready_nodes[0].task, "Task A");

        // Complete A
        if let Some(node_a) = plan.nodes.iter_mut().find(|n| n.id == id_a) {
            node_a.status = PlanNodeStatus::Completed;
        }

        // Now B and C should be ready (using readonly)
        let ready_nodes = plan.get_ready_nodes_readonly();
        assert_eq!(ready_nodes.len(), 2);

        // Sort by priority to check ordering
        let mut sorted_ready = ready_nodes.clone();
        sorted_ready.sort_by(|a, b| b.priority.cmp(&a.priority));
        assert_eq!(sorted_ready[0].task, "Task B"); // Higher priority
        assert_eq!(sorted_ready[1].task, "Task C");
    }

    #[test]
    fn test_priority_based_node_creation() {
        let mut plan = PlanTree::new("Priority test".to_string());

        // Create nodes with different priorities
        let node_low = PlanNode::new("Low priority".to_string(), 1);
        let node_high = PlanNode::new("High priority".to_string(), 10);
        let node_medium = PlanNode::new("Medium priority".to_string(), 5);

        let id_low = node_low.id.clone();
        let id_high = node_high.id.clone();
        let id_medium = node_medium.id.clone();

        // Add in random order
        plan.add_node(node_medium);
        plan.add_node(node_low);
        plan.add_node(node_high);

        // All should be ready (no dependencies)
        let ready_nodes = plan.get_ready_nodes_readonly();
        assert_eq!(ready_nodes.len(), 3);

        // Should be sortable by priority
        let mut sorted = ready_nodes.clone();
        sorted.sort_by(|a, b| b.priority.cmp(&a.priority));

        assert_eq!(sorted[0].priority, 10);
        assert_eq!(sorted[1].priority, 5);
        assert_eq!(sorted[2].priority, 1);
    }

    #[test]
    fn test_plan_completeness() {
        let mut plan = PlanTree::new("Completeness test".to_string());

        // Add some nodes
        let node1 = PlanNode::new("Task 1".to_string(), 10);
        let node2 = PlanNode::new("Task 2".to_string(), 5);

        plan.add_node(node1);
        plan.add_node(node2);

        // Initially not complete
        assert!(!plan.is_complete());

        // Complete all nodes
        for node in plan.nodes.iter_mut() {
            node.status = PlanNodeStatus::Completed;
        }

        // Now should be complete
        assert!(plan.is_complete());
    }

    #[test]
    fn test_mixed_statuses() {
        let mut plan = PlanTree::new("Mixed statuses test".to_string());

        // Add nodes with different statuses
        let mut node_completed = PlanNode::new("Completed task".to_string(), 10);
        let mut node_failed = PlanNode::new("Failed task".to_string(), 8);
        let mut node_skipped = PlanNode::new("Skipped task".to_string(), 6);

        node_completed.status = PlanNodeStatus::Completed;
        node_failed.status = PlanNodeStatus::Failed;
        node_skipped.status = PlanNodeStatus::Skipped;

        plan.add_node(node_completed);
        plan.add_node(node_failed);
        plan.add_node(node_skipped);

        // Plan should be complete (completed + skipped nodes count as complete)
        assert!(plan.is_complete());

        // Check status counts
        assert_eq!(plan.get_nodes_by_status(PlanNodeStatus::Completed).len(), 1);
        assert_eq!(plan.get_nodes_by_status(PlanNodeStatus::Failed).len(), 1);
        assert_eq!(plan.get_nodes_by_status(PlanNodeStatus::Skipped).len(), 1);
    }
}

#[cfg(test)]
mod reflection_stabilization_tests {
    use super::*;

    #[test]
    fn test_failure_category_display() {
        // Test that all FailureCategory variants can be created and displayed
        let categories = vec![
            FailureCategory::Network,
            FailureCategory::Database,
            FailureCategory::Authentication,
            FailureCategory::Resource,
            FailureCategory::Logic,
            FailureCategory::ExternalService,
            FailureCategory::Performance,
            FailureCategory::Unknown,
        ];

        // Ensure they can all be debug-formatted
        for category in categories {
            let _debug_str = format!("{:?}", category);
        }
    }

    #[test]
    fn test_reflection_report_creation() {
        use syncore::agent::reflection::ReflectionReport;

        let plan_id = "test_plan_123".to_string();
        let report = ReflectionReport::new(plan_id.clone());

        assert!(report.id.starts_with("reflection_"));
        assert_eq!(report.plan_id, plan_id);
        assert!(!report.failure_detected);
        assert!(report.insights.is_empty());
        assert!(report.created_at > 0);
    }

    #[test]
    fn test_reflection_equality() {
        // Test FailureCategory equality and ordering
        let cat1 = FailureCategory::Network;
        let cat2 = FailureCategory::Network;
        let cat3 = FailureCategory::Database;

        assert_eq!(cat1, cat2);
        assert_ne!(cat1, cat3);
    }

    #[test]
    fn test_reflection_copy() {
        // Test that FailureCategory can be copied (it's Clone)
        let cat1 = FailureCategory::Resource;
        let cat2 = cat1.clone();
        assert_eq!(cat1, cat2);
    }
}
