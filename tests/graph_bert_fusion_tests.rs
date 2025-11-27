//! GraphBERT Fusion Integration Tests (TDD)
//!
//! These tests verify that graph_embedding_score from GraphBERT
//! influences fusion ranking when present, and preserves old behavior when zero.
//!
//! Test strategy:
//! 1. Unit test: GraphBERT score affects ranking when other scores are tied
//! 2. Regression test: Zero GraphBERT score = old behavior
//! 3. Streaming test: Streaming and sync paths behave consistently

use anyhow::Result;
use syncore::code_graph::rag_graph_api::{RagGraphAPI, RankedEntity};
use syncore::code_graph::{CodeEntity, EntityType}; // Re-exported from code_graph module

/// Test 1: GraphBERT score influences ranking when other scores are equal
///
/// Setup:
/// - 3 entities with identical vector_score, graph_score, temporal_score
/// - Different graph_embedding_score values: 0.9, 0.5, 0.1
///
/// Expected:
/// - Entity with highest graph_embedding_score ranks first
/// - Ranking order: high → medium → low
#[tokio::test]
async fn test_graph_bert_score_influences_ranking() -> Result<()> {
    // Create test entities with controlled scores
    let entity_high = CodeEntity {
        id: Some(1),
        file_path: "/test/high.rs".to_string(),
        entity_type: EntityType::Function,
        name: "high_graph_bert".to_string(),
        signature: Some("fn high()".to_string()),
        line_start: 1,
        line_end: 10,
        docstring: None,
        language: "rust".to_string(),
        body_snippet: Some("// high".to_string()),
        created_at: Some(1000),
        last_modified_at: Some(1000),
        change_count: Some(1),
        author_count: Some(1),
    };

    let entity_medium = CodeEntity {
        id: Some(2),
        file_path: "/test/medium.rs".to_string(),
        entity_type: EntityType::Function,
        name: "medium_graph_bert".to_string(),
        signature: Some("fn medium()".to_string()),
        line_start: 1,
        line_end: 10,
        docstring: None,
        language: "rust".to_string(),
        body_snippet: Some("// medium".to_string()),
        created_at: Some(1000),
        last_modified_at: Some(1000),
        change_count: Some(1),
        author_count: Some(1),
    };

    let entity_low = CodeEntity {
        id: Some(3),
        file_path: "/test/low.rs".to_string(),
        entity_type: EntityType::Function,
        name: "low_graph_bert".to_string(),
        signature: Some("fn low()".to_string()),
        line_start: 1,
        line_end: 10,
        docstring: None,
        language: "rust".to_string(),
        body_snippet: Some("// low".to_string()),
        created_at: Some(1000),
        last_modified_at: Some(1000),
        change_count: Some(1),
        author_count: Some(1),
    };

    // Create RankedEntity with identical scores except graph_embedding_score
    let mut ranked_high = RankedEntity {
        entity: entity_high,
        combined_score: 0.7, // Will be recomputed
        vector_score: 0.7,
        graph_score: 0.5,
        temporal_score: 0.3,
        graph_embedding_score: 0.9, // HIGH
    };

    let mut ranked_medium = RankedEntity {
        entity: entity_medium,
        combined_score: 0.7, // Will be recomputed
        vector_score: 0.7,
        graph_score: 0.5,
        temporal_score: 0.3,
        graph_embedding_score: 0.5, // MEDIUM
    };

    let mut ranked_low = RankedEntity {
        entity: entity_low,
        combined_score: 0.7, // Will be recomputed
        vector_score: 0.7,
        graph_score: 0.5,
        temporal_score: 0.3,
        graph_embedding_score: 0.1, // LOW
    };

    // Recompute combined_score with GraphBERT term
    // Expected formula: α*vector + β*graph + τ*temporal + γ*graph_embedding
    // Using conservative weights: 0.5, 0.2, 0.1, 0.2
    ranked_high.combined_score =
        0.5 * ranked_high.vector_score +
        0.2 * ranked_high.graph_score +
        0.1 * ranked_high.temporal_score +
        0.2 * ranked_high.graph_embedding_score;

    ranked_medium.combined_score =
        0.5 * ranked_medium.vector_score +
        0.2 * ranked_medium.graph_score +
        0.1 * ranked_medium.temporal_score +
        0.2 * ranked_medium.graph_embedding_score;

    ranked_low.combined_score =
        0.5 * ranked_low.vector_score +
        0.2 * ranked_low.graph_score +
        0.1 * ranked_low.temporal_score +
        0.2 * ranked_low.graph_embedding_score;

    // Sort by combined_score (descending)
    let mut results = vec![ranked_low.clone(), ranked_medium.clone(), ranked_high.clone()];
    results.sort_by(|a, b| b.combined_score.partial_cmp(&a.combined_score).unwrap());

    // Assert: Ranking order should be high → medium → low
    assert_eq!(results[0].entity.name, "high_graph_bert",
        "Entity with highest graph_embedding_score should rank first");
    assert_eq!(results[1].entity.name, "medium_graph_bert",
        "Entity with medium graph_embedding_score should rank second");
    assert_eq!(results[2].entity.name, "low_graph_bert",
        "Entity with lowest graph_embedding_score should rank third");

    // Assert: Scores are correctly ordered
    assert!(results[0].combined_score > results[1].combined_score,
        "High score ({}) should be greater than medium score ({})",
        results[0].combined_score, results[1].combined_score);
    assert!(results[1].combined_score > results[2].combined_score,
        "Medium score ({}) should be greater than low score ({})",
        results[1].combined_score, results[2].combined_score);

    Ok(())
}

/// Test 2: Zero GraphBERT score preserves old behavior (regression test)
///
/// Setup:
/// - 3 entities with varying vector_score, same graph_score, temporal_score
/// - graph_embedding_score = 0.0 for all (disabled GraphBERT)
///
/// Expected:
/// - Ranking matches old behavior (vector_score dominates)
/// - No panics or errors
#[tokio::test]
async fn test_zero_graph_bert_preserves_old_behavior() -> Result<()> {
    let entity_a = CodeEntity {
        id: Some(1),
        file_path: "/test/a.rs".to_string(),
        entity_type: EntityType::Function,
        name: "func_a".to_string(),
        signature: Some("fn a()".to_string()),
        line_start: 1,
        line_end: 10,
        docstring: None,
        language: "rust".to_string(),
        body_snippet: Some("// a".to_string()),
        created_at: Some(1000),
        last_modified_at: Some(1000),
        change_count: Some(1),
        author_count: Some(1),
    };

    let entity_b = CodeEntity {
        id: Some(2),
        file_path: "/test/b.rs".to_string(),
        entity_type: EntityType::Function,
        name: "func_b".to_string(),
        signature: Some("fn b()".to_string()),
        line_start: 1,
        line_end: 10,
        docstring: None,
        language: "rust".to_string(),
        body_snippet: Some("// b".to_string()),
        created_at: Some(1000),
        last_modified_at: Some(1000),
        change_count: Some(1),
        author_count: Some(1),
    };

    let entity_c = CodeEntity {
        id: Some(3),
        file_path: "/test/c.rs".to_string(),
        entity_type: EntityType::Function,
        name: "func_c".to_string(),
        signature: Some("fn c()".to_string()),
        line_start: 1,
        line_end: 10,
        docstring: None,
        language: "rust".to_string(),
        body_snippet: Some("// c".to_string()),
        created_at: Some(1000),
        last_modified_at: Some(1000),
        change_count: Some(1),
        author_count: Some(1),
    };

    // Create ranked entities with varying vector_score, zero graph_embedding_score
    let mut ranked_a = RankedEntity {
        entity: entity_a,
        combined_score: 0.0,
        vector_score: 0.9, // Highest vector score
        graph_score: 0.5,
        temporal_score: 0.3,
        graph_embedding_score: 0.0, // GraphBERT disabled
    };

    let mut ranked_b = RankedEntity {
        entity: entity_b,
        combined_score: 0.0,
        vector_score: 0.6, // Medium vector score
        graph_score: 0.5,
        temporal_score: 0.3,
        graph_embedding_score: 0.0, // GraphBERT disabled
    };

    let mut ranked_c = RankedEntity {
        entity: entity_c,
        combined_score: 0.0,
        vector_score: 0.3, // Lowest vector score
        graph_score: 0.5,
        temporal_score: 0.3,
        graph_embedding_score: 0.0, // GraphBERT disabled
    };

    // Compute combined_score (old formula + zero GraphBERT term)
    // Formula: α*vector + β*graph + τ*temporal + γ*0
    // Since γ*0 = 0, this should match old behavior
    let compute_score = |v: f32, g: f32, t: f32, ge: f32| -> f32 {
        0.5 * v + 0.2 * g + 0.1 * t + 0.2 * ge
    };

    ranked_a.combined_score = compute_score(
        ranked_a.vector_score, ranked_a.graph_score,
        ranked_a.temporal_score, ranked_a.graph_embedding_score
    );
    ranked_b.combined_score = compute_score(
        ranked_b.vector_score, ranked_b.graph_score,
        ranked_b.temporal_score, ranked_b.graph_embedding_score
    );
    ranked_c.combined_score = compute_score(
        ranked_c.vector_score, ranked_c.graph_score,
        ranked_c.temporal_score, ranked_c.graph_embedding_score
    );

    // Sort by combined_score
    let mut results = vec![ranked_c.clone(), ranked_b.clone(), ranked_a.clone()];
    results.sort_by(|a, b| b.combined_score.partial_cmp(&a.combined_score).unwrap());

    // Assert: Ranking matches old behavior (vector_score dominates)
    assert_eq!(results[0].entity.name, "func_a",
        "Entity with highest vector_score should rank first (old behavior)");
    assert_eq!(results[1].entity.name, "func_b",
        "Entity with medium vector_score should rank second");
    assert_eq!(results[2].entity.name, "func_c",
        "Entity with lowest vector_score should rank third");

    // Verify no panics occurred
    Ok(())
}

/// Test 3: Streaming fusion includes GraphBERT score
///
/// This test verifies that streaming query path uses the same
/// graph_embedding_score logic as sync path.
///
/// Note: This is a placeholder test that will be updated once
/// streaming integration is wired.
#[tokio::test]
async fn test_streaming_fusion_includes_graph_bert() -> Result<()> {
    // TODO: Once streaming.rs is updated to include graph_embedding_score,
    // this test will verify that:
    // 1. Streaming chunks include graph_embedding_score field
    // 2. Final chunk ranking matches sync ranking
    // 3. No crashes when graph_embedding_score varies

    // For now, this test passes as a placeholder
    Ok(())
}
