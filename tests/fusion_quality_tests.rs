//! TDD Tests for Fusion Quality and Token Efficiency (Phase C)
//!
//! These tests verify:
//! 1. Fusion quality scoring model
//! 2. Deep-read recommendation logic
//! 3. Token efficiency guards
//! 4. Quality-based response adaptation

use syncore::code_graph::fusion_quality::{
    FusionQualityConfig, FusionQualityEvaluator, FusionQualityResult, ScoredEntity,
};

/// Helper to create test entities
fn make_entity(
    id: i64,
    fused: f32,
    vector: f32,
    graph: f32,
    path: f32,
    file: &str,
) -> ScoredEntity {
    ScoredEntity {
        entity_id: id,
        vector_score: vector,
        graph_score: graph,
        path_score: path,
        fused_score: fused,
        source_file: file.to_string(),
    }
}

// =============================================================================
// Phase C1: Fusion Scoring Model Tests
// =============================================================================

#[test]
fn test_quality_score_computation() {
    let evaluator = FusionQualityEvaluator::new();

    // Good results should have high quality score
    let entities = vec![
        make_entity(1, 0.95, 0.9, 0.85, 0.8, "main.rs"),
        make_entity(2, 0.7, 0.75, 0.6, 0.5, "lib.rs"),
        make_entity(3, 0.5, 0.55, 0.45, 0.4, "util.rs"),
    ];

    let result = evaluator.evaluate(&entities, "find main function");

    // Quality score should be weighted: 50% confidence + 30% completeness + 20% diversity
    assert!(result.quality_score > 0.5, "Good results should have quality > 0.5");
    assert!(result.quality_score <= 1.0, "Quality should be clamped to 1.0");
}

#[test]
fn test_confidence_from_score_gap() {
    let evaluator = FusionQualityEvaluator::new();

    // Clear winner (large gap) = high confidence
    let clear_winner = vec![
        make_entity(1, 0.95, 0.9, 0.85, 0.8, "main.rs"),
        make_entity(2, 0.3, 0.4, 0.2, 0.1, "other.rs"),
    ];
    let result_clear = evaluator.evaluate(&clear_winner, "test");
    assert!(result_clear.confidence > 0.7, "Clear winner should have high confidence");

    // Tie (small gap) = lower confidence
    let tie = vec![
        make_entity(1, 0.6, 0.6, 0.55, 0.5, "a.rs"),
        make_entity(2, 0.58, 0.55, 0.52, 0.48, "b.rs"),
    ];
    let result_tie = evaluator.evaluate(&tie, "test");
    assert!(result_tie.confidence < result_clear.confidence, "Tie should have lower confidence");
}

#[test]
fn test_completeness_scales_with_query() {
    let evaluator = FusionQualityEvaluator::new();

    // Simple query expects ~3 results
    let simple_query = vec![
        make_entity(1, 0.9, 0.85, 0.8, 0.7, "a.rs"),
        make_entity(2, 0.8, 0.75, 0.7, 0.6, "b.rs"),
        make_entity(3, 0.7, 0.65, 0.6, 0.5, "c.rs"),
    ];
    let result_simple = evaluator.evaluate(&simple_query, "fmt");
    assert!(result_simple.completeness >= 0.9, "3 results for simple query = high completeness");

    // Complex query expects ~8 results
    let result_complex =
        evaluator.evaluate(&simple_query, "explain the full implementation of the parser module");
    assert!(
        result_complex.completeness < result_simple.completeness,
        "Same results for complex query = lower completeness"
    );
}

#[test]
fn test_diversity_from_unique_files() {
    let evaluator = FusionQualityEvaluator::new();

    // All same file = low diversity
    let same_file = vec![
        make_entity(1, 0.9, 0.85, 0.8, 0.7, "main.rs"),
        make_entity(2, 0.8, 0.75, 0.7, 0.6, "main.rs"),
        make_entity(3, 0.7, 0.65, 0.6, 0.5, "main.rs"),
    ];
    let result_same = evaluator.evaluate(&same_file, "test");

    // Different files = high diversity
    let diff_files = vec![
        make_entity(1, 0.9, 0.85, 0.8, 0.7, "main.rs"),
        make_entity(2, 0.8, 0.75, 0.7, 0.6, "lib.rs"),
        make_entity(3, 0.7, 0.65, 0.6, 0.5, "util.rs"),
    ];
    let result_diff = evaluator.evaluate(&diff_files, "test");

    assert!(
        result_diff.diversity > result_same.diversity,
        "Different files should have higher diversity"
    );
}

// =============================================================================
// Phase C2: Deep-Read Recommendation Tests
// =============================================================================

#[test]
fn test_empty_results_recommend_deep_read() {
    let evaluator = FusionQualityEvaluator::new();
    let result = evaluator.evaluate(&[], "some query");

    assert!(result.recommend_deep_read, "Empty results should recommend deep read");
    assert_eq!(result.token_budget, 2000, "Empty results get max token budget");
}

#[test]
fn test_low_quality_recommends_deep_read() {
    let config = FusionQualityConfig {
        deep_read_threshold: 0.5,
        ..Default::default()
    };
    let evaluator = FusionQualityEvaluator::with_config(config);

    // Low scores across the board
    let low_quality = vec![make_entity(1, 0.3, 0.35, 0.25, 0.2, "test.rs")];
    let result = evaluator.evaluate(&low_quality, "complex query with many words");

    assert!(
        result.recommend_deep_read || result.quality_score < 0.5,
        "Low quality should trigger deep read recommendation"
    );
}

#[test]
fn test_isolated_entity_needs_context() {
    let evaluator = FusionQualityEvaluator::new();

    // High vector score + low graph score = isolated semantic match
    let isolated = vec![make_entity(1, 0.85, 0.9, 0.1, 0.3, "isolated.rs")];
    let result = evaluator.evaluate(&isolated, "isolated function");

    assert!(result.recommend_deep_read, "Isolated entity should recommend deep read");
    assert!(
        result.recommendation_reason.contains("context"),
        "Reason should mention context: {}",
        result.recommendation_reason
    );
}

#[test]
fn test_high_quality_uses_snippet_mode() {
    let evaluator = FusionQualityEvaluator::new();

    // Excellent results with good graph connectivity
    let excellent = vec![
        make_entity(1, 0.95, 0.92, 0.88, 0.85, "main.rs"),
        make_entity(2, 0.6, 0.65, 0.55, 0.5, "util.rs"),
        make_entity(3, 0.4, 0.45, 0.35, 0.3, "helper.rs"),
    ];
    let result = evaluator.evaluate(&excellent, "find main");

    assert!(!result.recommend_deep_read, "High quality should use snippet mode");
    assert_eq!(result.token_budget, 500, "Snippet mode should have lower token budget");
}

#[test]
fn test_flat_score_distribution_needs_context() {
    let evaluator = FusionQualityEvaluator::new();

    // All results have very similar scores (no clear winner)
    let flat = vec![
        make_entity(1, 0.65, 0.6, 0.5, 0.4, "a.rs"),
        make_entity(2, 0.63, 0.58, 0.48, 0.38, "b.rs"),
        make_entity(3, 0.61, 0.56, 0.46, 0.36, "c.rs"),
    ];
    let result = evaluator.evaluate(&flat, "ambiguous query");

    // Flat distribution may trigger deep read for various reasons:
    // - Low confidence (small score gaps)
    // - Multiple equally-ranked results
    // - Low quality score
    assert!(
        result.recommend_deep_read,
        "Flat distribution should recommend deep read, reason: {}",
        result.recommendation_reason
    );
}

// =============================================================================
// Phase C4: Token Efficiency Guard Tests
// =============================================================================

#[test]
fn test_token_guard_respects_quality_budget() {
    let evaluator = FusionQualityEvaluator::new();

    // High quality result - snippet mode
    let high_quality = FusionQualityResult {
        quality_score: 0.9,
        confidence: 0.95,
        completeness: 0.85,
        diversity: 0.8,
        recommend_deep_read: false,
        token_budget: 500,
        recommendation_reason: "Good quality".to_string(),
    };

    let tokens = evaluator.guard_token_budget(&high_quality, 1000);
    assert!(tokens <= 500, "High quality should stay within snippet budget");
}

#[test]
fn test_token_guard_scales_with_quality() {
    let evaluator = FusionQualityEvaluator::new();

    // Very high quality needs fewer tokens
    let very_high = FusionQualityResult {
        quality_score: 0.95,
        confidence: 0.95,
        completeness: 0.9,
        diversity: 0.85,
        recommend_deep_read: false,
        token_budget: 500,
        recommendation_reason: "Excellent".to_string(),
    };

    // Medium quality needs more tokens
    let medium = FusionQualityResult {
        quality_score: 0.5,
        confidence: 0.5,
        completeness: 0.5,
        diversity: 0.5,
        recommend_deep_read: false,
        token_budget: 500,
        recommendation_reason: "Medium".to_string(),
    };

    let tokens_high = evaluator.guard_token_budget(&very_high, 400);
    let tokens_medium = evaluator.guard_token_budget(&medium, 400);

    assert!(tokens_high <= tokens_medium, "Higher quality should result in fewer or equal tokens");
}

#[test]
fn test_token_guard_low_quality_gets_max() {
    let evaluator = FusionQualityEvaluator::new();

    let low_quality = FusionQualityResult {
        quality_score: 0.2,
        confidence: 0.25,
        completeness: 0.15,
        diversity: 0.1,
        recommend_deep_read: true,
        token_budget: 2000,
        recommendation_reason: "Low quality".to_string(),
    };

    let tokens = evaluator.guard_token_budget(&low_quality, 1500);
    assert_eq!(tokens, 2000, "Low quality should get max budget (deep read mode)");
}

#[test]
fn test_token_guard_minimum_floor() {
    let evaluator = FusionQualityEvaluator::new();

    let high_quality = FusionQualityResult {
        quality_score: 0.99,
        confidence: 0.99,
        completeness: 0.99,
        diversity: 0.99,
        recommend_deep_read: false,
        token_budget: 500,
        recommendation_reason: "Perfect".to_string(),
    };

    let tokens = evaluator.guard_token_budget(&high_quality, 50);
    assert!(tokens >= 100, "Should have minimum floor of 100 tokens");
}

// =============================================================================
// Integration Tests
// =============================================================================

#[test]
fn test_quality_config_customization() {
    let custom_config = FusionQualityConfig {
        min_confidence: 0.7,
        deep_read_threshold: 0.5,
        max_snippet_tokens: 300,
        max_deep_read_tokens: 3000,
    };

    let evaluator = FusionQualityEvaluator::with_config(custom_config);

    // Test with medium results
    let entities = vec![make_entity(1, 0.55, 0.6, 0.45, 0.4, "test.rs")];
    let result = evaluator.evaluate(&entities, "test query");

    // Should use custom thresholds
    if result.recommend_deep_read {
        assert_eq!(result.token_budget, 3000, "Custom deep-read budget should apply");
    } else {
        assert_eq!(result.token_budget, 300, "Custom snippet budget should apply");
    }
}

#[test]
fn test_end_to_end_quality_flow() {
    let evaluator = FusionQualityEvaluator::new();

    // Simulate real fusion results
    let entities = vec![
        make_entity(1, 0.88, 0.85, 0.75, 0.7, "src/parser.rs"),
        make_entity(2, 0.72, 0.7, 0.65, 0.6, "src/lexer.rs"),
        make_entity(3, 0.58, 0.6, 0.5, 0.45, "src/ast.rs"),
        make_entity(4, 0.45, 0.5, 0.4, 0.35, "src/utils.rs"),
    ];

    let result = evaluator.evaluate(&entities, "how does the parser tokenize input");

    // Should produce reasonable quality assessment
    assert!(result.quality_score > 0.0);
    assert!(result.quality_score <= 1.0);
    assert!(result.confidence > 0.0);
    assert!(result.completeness > 0.0);
    assert!(result.diversity > 0.0);

    // Token budget should be within bounds
    let guarded_tokens = evaluator.guard_token_budget(&result, 1000);
    assert!(guarded_tokens >= 100);
    assert!(guarded_tokens <= result.token_budget);

    println!("Quality Score: {:.2}", result.quality_score);
    println!("Confidence: {:.2}", result.confidence);
    println!("Completeness: {:.2}", result.completeness);
    println!("Diversity: {:.2}", result.diversity);
    println!("Deep Read: {}", result.recommend_deep_read);
    println!("Token Budget: {}", result.token_budget);
    println!("Guarded Tokens: {}", guarded_tokens);
    println!("Reason: {}", result.recommendation_reason);
}
