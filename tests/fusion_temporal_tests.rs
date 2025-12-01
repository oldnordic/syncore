//! PHASE 5 Integration Tests: Fusion Scoring with Temporal Component
//!
//! Tests verify 3-component fusion formula: S = α*S_v + β*S_g + τ*S_t
//! Default weights: α=0.65, β=0.25, τ=0.10

use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH};
use syncore::code_graph::fusion_simple::{compute_temporal_score, FusionSimple};

#[test]
fn test_fusion_only_vector() -> Result<()> {
    // When vector is 1.0, graph and temporal are 0.0
    // Expected: 0.65*1.0 + 0.25*0.0 + 0.10*0.0 = 0.65
    let fusion = FusionSimple::default();
    let result = fusion.combine(1.0, 0.0, 0.0, 0.0);

    assert!((result - 0.65).abs() < 0.001, "Pure vector score should be 0.65");
    Ok(())
}

#[test]
fn test_fusion_only_graph() -> Result<()> {
    // When graph is 1.0, vector and temporal are 0.0
    // Expected: 0.65*0.0 + 0.25*1.0 + 0.10*0.0 = 0.25
    let fusion = FusionSimple::default();
    let result = fusion.combine(0.0, 1.0, 0.0, 0.0);

    assert!((result - 0.25).abs() < 0.001, "Pure graph score should be 0.25");
    Ok(())
}

#[test]
fn test_fusion_temporal_recency_increases_score() -> Result<()> {
    // Test that recent files get higher scores than old files
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;

    // Recent file (modified today)
    let recent_score = compute_temporal_score(now, 10, 1);

    // Old file (modified 2 years ago)
    let two_years_ago = now - (2 * 365 * 24 * 3600);
    let old_score = compute_temporal_score(two_years_ago, 10, 1);

    assert!(
        recent_score > old_score,
        "Recent file should have higher temporal score than old file"
    );
    assert!(recent_score > 0.5, "Recent file should have temporal score > 0.5");
    assert!(old_score < 0.3, "2-year-old file should have temporal score < 0.3");

    Ok(())
}

#[test]
fn test_fusion_temporal_churn_increases_score() -> Result<()> {
    // Test that high-churn files get higher scores than low-churn files
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;

    // High churn: 50 commits
    let high_churn = compute_temporal_score(now, 50, 1);

    // Low churn: 2 commits
    let low_churn = compute_temporal_score(now, 2, 1);

    assert!(
        high_churn > low_churn,
        "High-churn file should have higher temporal score than low-churn file"
    );

    Ok(())
}

#[test]
fn test_fusion_end_to_end_rag_query() -> Result<()> {
    // Integration test: simulate full fusion pipeline
    let fusion = FusionSimple::default();

    // Scenario: High vector match (0.9), medium graph (0.5), recent+churned (0.7)
    let vector = 0.9;
    let graph = 0.5;
    let temporal = 0.7;

    let combined = fusion.combine(vector, graph, temporal, 0.0);

    // Expected: 0.65*0.9 + 0.25*0.5 + 0.10*0.7 = 0.585 + 0.125 + 0.07 = 0.78
    let expected = 0.78;
    assert!(
        (combined - expected).abs() < 0.01,
        "Combined score should match expected value. Got {}, expected {}",
        combined,
        expected
    );

    // Verify all components contribute
    assert!(combined > vector * 0.65, "Temporal and graph should boost score");

    Ok(())
}

#[test]
fn test_fusion_clamps_score() -> Result<()> {
    // Test that fusion scores are clamped to [0.0, 1.0]
    let fusion = FusionSimple::default();

    // Test upper clamp (scores > 1.0 should be clamped to 1.0)
    let result_high = fusion.combine(1.2, 1.5, 1.8, 2.0);
    assert!(result_high <= 1.0, "Score should be clamped to 1.0");
    assert!(result_high >= 0.95, "Clamping should preserve near-1.0 values");

    // Test lower clamp (negative scores should be clamped to 0.0)
    let result_low = fusion.combine(0.0, 0.0, 0.0, 0.0);
    assert!(result_low >= 0.0, "Score should never be negative");
    assert!(result_low < 0.01, "All-zero inputs should give near-zero score");

    Ok(())
}

#[test]
fn test_temporal_score_boundary_conditions() -> Result<()> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;

    // Test 1: Brand new file (created now, 1 commit)
    let new_file = compute_temporal_score(now, 1, 1);
    assert!(new_file > 0.5, "New file should have high temporal score");

    // Test 2: Ancient file (5 years old, 1 commit)
    let ancient = now - (5 * 365 * 24 * 3600);
    let ancient_file = compute_temporal_score(ancient, 1, 1);
    assert!(ancient_file < 0.3, "Ancient file should have low temporal score");

    // Test 3: Active old file (2 years old, 100 commits)
    let two_years = now - (2 * 365 * 24 * 3600);
    let active_old = compute_temporal_score(two_years, 100, 5);
    assert!(active_old > 0.3, "Active old file should maintain decent score due to churn");

    Ok(())
}
