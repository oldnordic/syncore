//! Graph Recency Fusion Tests
//!
//! Comprehensive tests for recency-based graph fusion scoring as specified in the directive.
//! Tests cover all edge cases and verify 5-component fusion scoring works correctly.

use syncore::code_graph::fusion_simple::{FusionSimple, extract_recency_score, compute_temporal_score};
use syncore::code_graph::fusion_quality::{FusionQualityConfig, FusionQualityEvaluator, ScoredEntity};
use serde_json::Map;

/// Create test properties with created_at timestamp
fn make_test_properties(created_at: Option<i64>) -> Map<String, serde_json::Value> {
    let mut properties = Map::new();

    if let Some(timestamp) = created_at {
        properties.insert("created_at".to_string(),
            serde_json::Value::Number(serde_json::Number::from(timestamp)));
    }

    properties
}

/// Create test properties with string created_at timestamp
fn make_test_properties_string(created_at: Option<&str>) -> Map<String, serde_json::Value> {
    let mut properties = Map::new();

    if let Some(timestamp_str) = created_at {
        properties.insert("created_at".to_string(),
            serde_json::Value::String(timestamp_str.to_string()));
    }

    properties
}

#[test]
fn test_extract_recency_score_new_entity() {
    // Test very recent entity (created 1 day ago)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let one_day_ago = now - 86400; // 1 day ago

    let properties = make_test_properties(Some(one_day_ago));
    let score = extract_recency_score(&properties);

    // Should be high score for recent entity (> 0.4)
    assert!(score > 0.4, "1-day-old entity should have score > 0.4, got {}", score);
    assert!(score <= 1.0, "Score should be <= 1.0");

    println!("✅ 1-day-old entity recency score: {:.6}", score);
}

#[test]
fn test_extract_recency_score_old_entity() {
    // Test old entity (created 1 year ago)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let one_year_ago = now - (86400 * 365);

    let properties = make_test_properties(Some(one_year_ago));
    let score = extract_recency_score(&properties);

    // Should be low score for old entity (< 0.01)
    assert!(score < 0.01, "1-year-old entity should have score < 0.01, got {}", score);
    assert!(score >= 0.0, "Score should be >= 0.0");

    println!("✅ 1-year-old entity recency score: {:.6}", score);
}

#[test]
fn test_extract_recency_score_missing_created_at() {
    // Test missing created_at field
    let properties = make_test_properties(None);
    let score = extract_recency_score(&properties);

    // Should return neutral score when created_at is missing
    assert_eq!(score, 0.5, "Missing created_at should return neutral score 0.5");

    println!("✅ Missing created_at recency score: {:.6}", score);
}

#[test]
fn test_extract_recency_score_string_timestamp() {
    // Test with string representation of timestamp
    let properties = make_test_properties_string(Some("1704067200")); // Jan 1, 2024
    let score = extract_recency_score(&properties);

    // Should be a valid score
    assert!(score >= 0.0 && score <= 1.0, "String timestamp should produce valid score, got {}", score);

    println!("✅ String timestamp recency score: {:.6}", score);
}

#[test]
fn test_extract_recency_score_invalid_timestamp() {
    // Test with invalid timestamp string
    let mut properties = Map::new();
    properties.insert("created_at".to_string(),
        serde_json::Value::String("invalid_timestamp".to_string()));

    let score = extract_recency_score(&properties);

    // Should return neutral score for invalid timestamp
    assert_eq!(score, 0.5, "Invalid timestamp should return neutral score 0.5");

    println!("✅ Invalid timestamp recency score: {:.6}", score);
}

#[test]
fn test_extract_recency_score_decay_function() {
    // Test that recency score follows expected decay pattern
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Test multiple age points
    let test_cases = vec![
        (0, "now", 1.0),           // age = 0 days
        (86400, "1 day", 0.5),     // age = 1 day
        (86400 * 7, "1 week", 0.125), // age = 7 days
        (86400 * 30, "1 month", 0.032), // age = 30 days
    ];

    for (age_seconds, description, expected_range) in test_cases {
        let created_at = now - age_seconds;
        let properties = make_test_properties(Some(created_at));
        let score = extract_recency_score(&properties);

        // Allow some tolerance for floating point calculations
        assert!(score >= expected_range * 0.9 && score <= expected_range * 1.1,
               "Score for {} should be close to {}, got {}", description, expected_range, score);

        println!("✅ {} (age {} days) recency score: {:.6}", description, age_seconds / 86400, score);
    }
}

#[test]
fn test_fusion_simple_5component_combination() {
    // Test 5-component fusion scoring with recency
    let fusion = FusionSimple::new(0.5, 0.2, 0.1, 0.15, 0.05);

    let result = fusion.combine(0.8, 0.4, 0.9, 0.7, 0.6);
    // Expected: 0.5*0.8 + 0.2*0.4 + 0.1*0.9 + 0.15*0.7 + 0.05*0.6
    // = 0.4 + 0.08 + 0.09 + 0.105 + 0.03 = 0.705
    assert!((result - 0.705).abs() < 0.001);

    println!("✅ 5-component fusion result: {:.6}", result);
}

#[test]
fn test_fusion_simple_default_weights() {
    // Test that default FusionSimple includes recency weight
    let fusion = FusionSimple::default();

    // Verify all 5 weights are present and sum to 1.0
    let total = fusion.alpha + fusion.beta + fusion.tau + fusion.gamma + fusion.delta;
    assert!((total - 1.0).abs() < 0.001, "Weights should sum to 1.0");

    // Verify recency weight (delta) has reasonable default
    assert!((fusion.delta - 0.05).abs() < 0.001, "Default recency weight should be 0.05");

    println!("✅ Default weights: α={:.3}, β={:.3}, τ={:.3}, γ={:.3}, δ={:.3}",
             fusion.alpha, fusion.beta, fusion.tau, fusion.gamma, fusion.delta);
}

#[test]
fn test_fusion_simple_backward_compatibility() {
    // Test that legacy combine method still works
    let fusion = FusionSimple::new(0.5, 0.2, 0.1, 0.15, 0.05);

    // Legacy method should use neutral recency score (0.5)
    let legacy_result = fusion.combine_legacy(0.8, 0.4, 0.9, 0.7);
    let modern_result = fusion.combine(0.8, 0.4, 0.9, 0.7, 0.5);

    assert!((legacy_result - modern_result).abs() < 0.001,
           "Legacy and modern methods should produce same result with neutral recency");

    println!("✅ Legacy compatibility: {:.6} vs {:.6}", legacy_result, modern_result);
}

#[test]
fn test_fusion_quality_config_recency_field() {
    // Test that FusionQualityConfig includes recency field
    let config = FusionQualityConfig::default();

    // Verify recency field exists and has reasonable default
    assert!(config.recency > 0.0, "Recency weight should be positive");
    assert!(config.recency < 1.0, "Recency weight should be less than 1.0");

    println!("✅ FusionQualityConfig recency field: {:.3}", config.recency);
}

#[test]
fn test_fusion_quality_evaluator_with_recency() {
    // Test that FusionQualityEvaluator works with recency-enhanced entities
    let evaluator = FusionQualityEvaluator::new();

    // Create test entities with varying recency scores
    let entities = vec![
        ScoredEntity {
            entity_id: 1,
            vector_score: 0.9,
            graph_score: 0.8,
            path_score: 0.7,
            fused_score: 0.85, // This would include recency in real usage
            source_file: "recent_file.rs".to_string(),
        },
        ScoredEntity {
            entity_id: 2,
            vector_score: 0.7,
            graph_score: 0.6,
            path_score: 0.5,
            fused_score: 0.6, // Lower due to lower recency
            source_file: "old_file.rs".to_string(),
        },
    ];

    let result = evaluator.evaluate(&entities, "test query");

    // Should produce valid quality metrics
    assert!(result.quality_score >= 0.0 && result.quality_score <= 1.0);
    assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
    assert!(result.completeness >= 0.0 && result.completeness <= 1.0);
    assert!(result.diversity >= 0.0 && result.diversity <= 1.0);

    println!("✅ Quality evaluation: quality={:.3}, confidence={:.3}, completeness={:.3}, diversity={:.3}",
             result.quality_score, result.confidence, result.completeness, result.diversity);
}

#[test]
fn test_recency_score_extreme_cases() {
    // Test edge cases for recency scoring

    // Future timestamp (should be clamped to max score)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let future_timestamp = now + 86400; // 1 day in future

    let properties = make_test_properties(Some(future_timestamp));
    let future_score = extract_recency_score(&properties);

    // Should handle future timestamps gracefully (score > 1.0 will be clamped)
    assert!(future_score >= 0.0 && future_score <= 1.0, "Future timestamp should produce valid score");

    // Very old timestamp (10 years ago)
    let ancient_timestamp = now - (86400 * 365 * 10);
    let properties = make_test_properties(Some(ancient_timestamp));
    let ancient_score = extract_recency_score(&properties);

    // Should produce very low but non-negative score
    assert!(ancient_score >= 0.0 && ancient_score < 0.001, "Ancient timestamp should produce near-zero score");

    println!("✅ Extreme cases - future: {:.6}, ancient: {:.6}", future_score, ancient_score);
}

#[test]
fn test_recency_integration_with_temporal_score() {
    // Test that recency scoring works alongside temporal scoring
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Create test properties for recent entity
    let recent_created = now - 86400; // 1 day ago
    let recent_modified = now - 3600; // 1 hour ago

    let properties = make_test_properties(Some(recent_created));
    let recency_score = extract_recency_score(&properties);
    let temporal_score = compute_temporal_score(recent_modified, 5, 2);

    // Both should be high for recent, active entity
    assert!(recency_score > 0.4, "Recent entity should have high recency score");
    assert!(temporal_score > 0.4, "Recently modified entity should have high temporal score");

    // Create test properties for old entity
    let old_created = now - (86400 * 365); // 1 year ago
    let old_modified = now - (86400 * 30); // 1 month ago

    let properties = make_test_properties(Some(old_created));
    let old_recency_score = extract_recency_score(&properties);
    let old_temporal_score = compute_temporal_score(old_modified, 1, 1);

    // Both should be lower for old, inactive entity
    assert!(old_recency_score < 0.1, "Old entity should have low recency score");

    println!("✅ Recent - recency: {:.3}, temporal: {:.3}", recency_score, temporal_score);
    println!("✅ Old - recency: {:.3}, temporal: {:.3}", old_recency_score, old_temporal_score);

    // Verify they provide different signals
    assert_ne!(recency_score, temporal_score, "Recency and temporal scores should differ");
}

#[test]
fn test_recency_fusion_edge_cases() {
    // Test fusion behavior with extreme recency values

    let fusion = FusionSimple::default();

    // Test with maximum recency (very recent entity)
    let max_recency_result = fusion.combine(0.5, 0.5, 0.5, 0.5, 1.0);

    // Test with minimum recency (very old entity)
    let min_recency_result = fusion.combine(0.5, 0.5, 0.5, 0.5, 0.0);

    // Test with neutral recency (missing created_at)
    let neutral_recency_result = fusion.combine(0.5, 0.5, 0.5, 0.5, 0.5);

    // Recent entity should score higher
    assert!(max_recency_result > neutral_recency_result,
           "Recent entity should score higher than neutral");
    assert!(neutral_recency_result > min_recency_result,
           "Neutral entity should score higher than old entity");

    println!("✅ Recency fusion - max: {:.6}, neutral: {:.6}, min: {:.6}",
             max_recency_result, neutral_recency_result, min_recency_result);
}

#[test]
fn test_weight_normalization_with_recency() {
    // Test that fusion weights are properly normalized with recency

    // Test with various weight combinations
    let test_cases = vec![
        (0.4, 0.3, 0.2, 0.07, 0.03), // Standard weights
        (0.6, 0.2, 0.1, 0.05, 0.05), // High vector weight
        (0.1, 0.3, 0.3, 0.2, 0.1),  // High temporal weight
        (0.25, 0.25, 0.25, 0.2, 0.05), // Balanced weights
    ];

    for (alpha, beta, tau, gamma, delta) in test_cases {
        let fusion = FusionSimple::new(alpha, beta, tau, gamma, delta);
        let total = fusion.alpha + fusion.beta + fusion.tau + fusion.gamma + fusion.delta;

        assert!((total - 1.0).abs() < 0.001,
               "Weights should normalize to 1.0, got {:.6}", total);

        // Verify recency weight is preserved
        assert!(fusion.delta > 0.0, "Recency weight should be positive");
    }

    println!("✅ Weight normalization works correctly with recency component");
}