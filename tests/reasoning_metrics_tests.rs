//! Tests for Reasoning Metrics - PHASE ST-9
//!
//! TDD-first test suite for Tree-of-Thought level metrics and health reporting.
//! Tests metrics collection, aggregation, and MCP integration.

use anyhow::Result;
use std::sync::{Arc, Mutex};
use syncore::reasoning::branch_manager::{BranchLimits, BranchManager};
use syncore::reasoning::metrics::{ReasoningMetrics, ReasoningMetricsSnapshot};

#[test]
fn test_metrics_increment_on_expand() -> Result<()> {
    // Create metrics instance
    let mut metrics = ReasoningMetrics::new();

    // Simulate 3 expansions: 2 success, 1 failure
    metrics.record_expand(1, 2, true, 50.0); // depth=1, breadth=2, success, 50ms
    metrics.record_expand(2, 3, true, 75.0); // depth=2, breadth=3, success, 75ms
    metrics.record_expand(3, 1, false, 25.0); // depth=3, breadth=1, failure, 25ms

    // Assert expansion counters
    assert_eq!(metrics.nodes_expanded_total, 3);
    assert_eq!(metrics.nodes_expanded_success, 2);
    assert_eq!(metrics.nodes_expanded_failed, 1);

    // Assert reasoning steps incremented (each expand is a step)
    assert_eq!(metrics.reasoning_steps, 3);

    // Assert depth/breadth tracking
    assert_eq!(metrics.max_tree_depth, 3);
    assert_eq!(metrics.max_tree_breadth, 3);
    assert_eq!(metrics.current_depth, 3);
    assert_eq!(metrics.current_breadth, 1);

    // Assert timing
    assert_eq!(metrics.total_expansion_time_ms, 150.0);
    assert_eq!(metrics.average_expansion_time_ms(), 50.0);

    Ok(())
}

#[test]
fn test_metrics_track_depth_and_breadth() -> Result<()> {
    // Create metrics instance
    let mut metrics = ReasoningMetrics::new();

    // Simulate tree: root -> 2 children -> 1 child has 2 children
    // Root expansion
    metrics.record_expand(0, 2, true, 10.0);
    assert_eq!(metrics.max_tree_depth, 0);
    assert_eq!(metrics.max_tree_breadth, 2);

    // First child expansion
    metrics.record_expand(1, 1, true, 15.0);
    assert_eq!(metrics.max_tree_depth, 1);
    assert_eq!(metrics.max_tree_breadth, 2); // unchanged

    // Second child expansion
    metrics.record_expand(1, 2, true, 20.0);
    assert_eq!(metrics.max_tree_depth, 1);
    assert_eq!(metrics.max_tree_breadth, 2); // unchanged

    // Grandchild expansion (from second child)
    metrics.record_expand(2, 2, true, 25.0);
    assert_eq!(metrics.max_tree_depth, 2);
    assert_eq!(metrics.max_tree_breadth, 2); // unchanged

    Ok(())
}

#[test]
fn test_metrics_pruning_updates() -> Result<()> {
    let mut metrics = ReasoningMetrics::new();

    // Record pruning operations
    metrics.record_prune_low_score(3);
    metrics.record_prune_safety(2);

    // Assert pruning counters
    assert_eq!(metrics.nodes_pruned_total, 5);
    assert_eq!(metrics.pruned_low_score, 3);
    assert_eq!(metrics.pruned_safety, 2);

    Ok(())
}

#[test]
fn test_metrics_safety_events() -> Result<()> {
    let mut metrics = ReasoningMetrics::new();

    // Record safety violations
    metrics.record_safety_violation(true); // identical expansion
    metrics.record_safety_violation(false); // other safety violation
    metrics.record_safety_violation(true); // another identical expansion

    // Assert safety counters
    assert_eq!(metrics.safety_violations_total, 3);
    assert_eq!(metrics.identical_expansions_detected, 2);

    Ok(())
}

#[test]
fn test_metrics_merge() -> Result<()> {
    let mut metrics1 = ReasoningMetrics::new();
    let mut metrics2 = ReasoningMetrics::new();

    // Set up first metrics
    metrics1.nodes_expanded_total = 5;
    metrics1.nodes_expanded_success = 4;
    metrics1.nodes_expanded_failed = 1;
    metrics1.total_expansion_time_ms = 100.0;

    // Set up second metrics
    metrics2.nodes_expanded_total = 3;
    metrics2.nodes_expanded_success = 2;
    metrics2.nodes_expanded_failed = 1;
    metrics2.total_expansion_time_ms = 75.0;

    // Merge metrics2 into metrics1
    metrics1.merge(&metrics2);

    // Assert merged values
    assert_eq!(metrics1.nodes_expanded_total, 8); // 5 + 3
    assert_eq!(metrics1.nodes_expanded_success, 6); // 4 + 2
    assert_eq!(metrics1.nodes_expanded_failed, 2); // 1 + 1
    assert_eq!(metrics1.total_expansion_time_ms, 175.0); // 100 + 75

    // Average should be recalculated
    assert_eq!(metrics1.average_expansion_time_ms(), 175.0 / 8.0);

    Ok(())
}

#[test]
fn test_metrics_snapshot_is_deterministic() -> Result<()> {
    let mut metrics = ReasoningMetrics::new();

    // Add some data
    metrics.record_expand(1, 2, true, 50.0);
    metrics.record_prune_low_score(1);
    metrics.record_safety_violation(true);

    // Create snapshot
    let snapshot1 = metrics.snapshot();

    // Create another snapshot without changes
    let snapshot2 = metrics.snapshot();

    // Snapshots should be identical
    assert_eq!(snapshot1.nodes_expanded_total, snapshot2.nodes_expanded_total);
    assert_eq!(snapshot1.nodes_pruned_total, snapshot2.nodes_pruned_total);
    assert_eq!(snapshot1.safety_violations_total, snapshot2.safety_violations_total);
    assert_eq!(snapshot1.max_tree_depth, snapshot2.max_tree_depth);
    assert_eq!(snapshot1.average_expansion_time_ms, snapshot2.average_expansion_time_ms);

    Ok(())
}

#[test]
fn test_branch_manager_export_safety_counters() -> Result<()> {
    let limits = BranchLimits::default();
    let mut manager = BranchManager::new(limits);

    // Simulate some safety events by accessing internal state
    // Note: This would need to be done through public methods in real implementation
    let (identical_count, safety_count) = manager.export_safety_counters();

    // Initially should be zero
    assert_eq!(identical_count, 0);
    assert_eq!(safety_count, 0);

    Ok(())
}

#[test]
fn test_reasoning_metrics_snapshot_structure() -> Result<()> {
    let mut metrics = ReasoningMetrics::new();

    // Add some test data
    metrics.record_expand(2, 3, true, 100.0);
    metrics.record_prune_safety(1);
    metrics.record_safety_violation(false);
    metrics.record_llm_failure();

    // Create snapshot
    let snapshot = metrics.snapshot();

    // Verify all fields are present and have expected values
    assert_eq!(snapshot.nodes_expanded_total, 1);
    assert_eq!(snapshot.nodes_expanded_success, 1);
    assert_eq!(snapshot.nodes_expanded_failed, 0);
    assert_eq!(snapshot.nodes_pruned_total, 1);
    assert_eq!(snapshot.pruned_safety, 1);
    assert_eq!(snapshot.safety_violations_total, 1);
    assert_eq!(snapshot.reasoning_steps, 1);
    assert_eq!(snapshot.llm_failures, 1);
    assert_eq!(snapshot.max_tree_depth, 2);
    assert_eq!(snapshot.max_tree_breadth, 3);
    assert_eq!(snapshot.total_expansion_time_ms, 100.0);

    Ok(())
}

#[test]
fn test_metrics_thread_safety() -> Result<()> {
    use std::sync::Arc;
    use std::thread;

    let metrics = Arc::new(Mutex::new(ReasoningMetrics::new()));
    let mut handles = vec![];

    // Spawn multiple threads to update metrics
    for i in 0..3 {
        let metrics_clone = metrics.clone();
        let handle = thread::spawn(move || {
            let mut m = metrics_clone.lock().unwrap();
            m.record_expand(i as u32, 2, true, 10.0 * i as f64);
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all updates were applied
    let final_metrics = metrics.lock().unwrap();
    assert_eq!(final_metrics.nodes_expanded_total, 3);
    assert_eq!(final_metrics.nodes_expanded_success, 3);
    assert_eq!(final_metrics.total_expansion_time_ms, 30.0); // 0 + 10 + 20

    Ok(())
}
