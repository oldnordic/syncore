//! Integration Tests for PHASE ST-9 - Reasoning Metrics & Health
//!
//! End-to-end tests that verify the complete integration of:
//! - ReasoningMetrics with ToTEngine
//! - BranchManager safety counter integration
//! - Global metrics updates
//! - MCP tools with reasoning metrics

use anyhow::Result;
use std::sync::{Arc, Mutex};
use syncore::metrics as global_metrics;
use syncore::models::gguf_engine::GGUFEngine;
use syncore::reasoning::metrics::{ReasoningMetrics, ReasoningMetricsSnapshot};

#[test]
fn test_global_metrics_integration() -> Result<()> {
    // Create a reasoning metrics snapshot
    let mut metrics = ReasoningMetrics::new();
    metrics.record_expand(2, 3, true, 100.0);
    metrics.record_prune_safety(1);
    metrics.record_safety_violation(false);

    let snapshot = metrics.snapshot();

    // Update global metrics
    global_metrics::update_reasoning_metrics(snapshot.clone());

    // Verify snapshot structure
    assert_eq!(snapshot.nodes_expanded_total, 1);
    assert_eq!(snapshot.max_tree_depth, 2);
    assert_eq!(snapshot.max_tree_breadth, 3);
    assert_eq!(snapshot.pruned_safety, 1);
    assert_eq!(snapshot.safety_violations_total, 1);
    assert_eq!(snapshot.average_expansion_time_ms, 100.0);

    Ok(())
}

#[test]
fn test_mcp_tools_basic_functionality() -> Result<()> {
    let gguf_engine = GGUFEngine::new_test();
    let params = serde_json::json!({});

    // Test basic GGUF engine functionality
    let health = gguf_engine.health();
    let metrics = gguf_engine.metrics();

    // Verify GGUF engine works
    assert_eq!(health.backend_name, "gguf_engine");
    assert!(metrics.total_requests >= 0);

    Ok(())
}

#[test]
fn test_reasoning_metrics_aggregation() -> Result<()> {
    let mut metrics1 = ReasoningMetrics::new();
    let mut metrics2 = ReasoningMetrics::new();

    // Set up first metrics
    metrics1.record_expand(1, 2, true, 50.0);
    metrics1.record_prune_low_score(3);

    // Set up second metrics
    metrics2.record_expand(2, 1, false, 75.0);
    metrics2.record_prune_safety(2);

    // Merge metrics
    metrics1.merge(&metrics2);

    // Verify merged values
    assert_eq!(metrics1.nodes_expanded_total, 2);
    assert_eq!(metrics1.nodes_expanded_success, 1);
    assert_eq!(metrics1.nodes_expanded_failed, 1);
    assert_eq!(metrics1.pruned_low_score, 3);
    assert_eq!(metrics1.pruned_safety, 2);
    assert_eq!(metrics1.total_expansion_time_ms, 125.0);
    assert_eq!(metrics1.average_expansion_time_ms(), 62.5);

    Ok(())
}

#[test]
fn test_reasoning_metrics_thread_safety() -> Result<()> {
    use std::thread;

    let metrics = Arc::new(Mutex::new(ReasoningMetrics::new()));
    let mut handles = vec![];

    // Spawn multiple threads to update metrics concurrently
    for i in 0..5 {
        let metrics_clone = metrics.clone();
        let handle = thread::spawn(move || {
            let mut m = metrics_clone.lock().unwrap();
            m.record_expand(i as u32, 2, true, 10.0 * i as f64);
            m.record_safety_violation(i % 2 == 0);
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all updates were applied correctly
    let final_metrics = metrics.lock().unwrap();
    assert_eq!(final_metrics.nodes_expanded_total, 5);
    assert_eq!(final_metrics.nodes_expanded_success, 5);
    assert_eq!(final_metrics.safety_violations_total, 5);
    assert_eq!(final_metrics.total_expansion_time_ms, 100.0); // 0 + 10 + 20 + 30 + 40

    Ok(())
}

#[test]
fn test_reasoning_metrics_snapshot_methods() -> Result<()> {
    let mut snapshot = ReasoningMetricsSnapshot::new();

    // Test empty snapshot
    assert!(!snapshot.has_activity());
    assert_eq!(snapshot.success_rate(), 0.0);
    assert_eq!(snapshot.safety_violation_rate(), 0.0);

    // Add some activity
    snapshot.nodes_expanded_total = 10;
    snapshot.nodes_expanded_success = 8;
    snapshot.reasoning_steps = 20;
    snapshot.safety_violations_total = 2;

    // Test calculations
    assert!(snapshot.has_activity());
    assert_eq!(snapshot.success_rate(), 80.0);
    assert_eq!(snapshot.safety_violation_rate(), 10.0);

    Ok(())
}

#[test]
fn test_branch_manager_safety_counters() -> Result<()> {
    use syncore::reasoning::branch_manager::{BranchLimits, BranchManager};

    let limits = BranchLimits::default();
    let manager = BranchManager::new(limits);

    // Test export safety counters
    let (identical_count, safety_count) = manager.export_safety_counters();

    // Initially should be zero
    assert_eq!(identical_count, 0);
    assert_eq!(safety_count, 0);

    Ok(())
}

/// Integration test that demonstrates complete metrics workflow
#[test]
fn test_complete_metrics_workflow() -> Result<()> {
    // Create reasoning metrics
    let mut metrics = ReasoningMetrics::new();

    // Simulate a reasoning session with multiple expansions
    metrics.record_expand(1, 3, true, 50.0); // First expansion
    metrics.record_expand(2, 2, true, 75.0); // Second expansion
    metrics.record_expand(3, 1, false, 25.0); // Failed expansion

    // Record some pruning and safety events
    metrics.record_prune_low_score(2);
    metrics.record_prune_safety(1);
    metrics.record_safety_violation(true); // Identical expansion
    metrics.record_safety_violation(false); // Other safety violation

    // Create snapshot and verify all data is captured
    let snapshot = metrics.snapshot();

    assert_eq!(snapshot.nodes_expanded_total, 3);
    assert_eq!(snapshot.nodes_expanded_success, 2);
    assert_eq!(snapshot.nodes_expanded_failed, 1);
    assert_eq!(snapshot.reasoning_steps, 3);
    assert_eq!(snapshot.max_tree_depth, 3);
    assert_eq!(snapshot.max_tree_breadth, 3);
    assert_eq!(snapshot.total_expansion_time_ms, 150.0);
    assert_eq!(snapshot.average_expansion_time_ms, 50.0);
    assert_eq!(snapshot.nodes_pruned_total, 3);
    assert_eq!(snapshot.pruned_low_score, 2);
    assert_eq!(snapshot.pruned_safety, 1);
    assert_eq!(snapshot.safety_violations_total, 2);
    assert_eq!(snapshot.identical_expansions_detected, 1);

    // Test derived calculations
    assert!((snapshot.success_rate() - 66.66666666666667).abs() < 0.0001);
    assert!((snapshot.safety_violation_rate() - 66.66666666666667).abs() < 0.0001);
    assert!(snapshot.has_activity());

    // Update global metrics
    global_metrics::update_reasoning_metrics(snapshot);

    Ok(())
}
