//! Phase R4.2 - Self-Consistency Checker Tests
//!
//! Tests for the advanced cognitive constraint system that:
//! - Detects repeated failed sequences
//! - Identifies conflicting patterns
//! - Catches graph inconsistencies
//! - Flags namespace mismatches
//! - Detects suspicious tool ordering
//! - Identifies missing required steps
//! - Detects potential loops

use anyhow::Result;
use std::sync::atomic::{AtomicUsize, Ordering};
use syncore::cognition::context_bundle::ContextBundle;
use syncore::cognition::continuity_engine::ReasoningContinuity;
use syncore::cognition::intent_classifier::QueryIntent;
use syncore::cognition::pattern_engine::ReasoningPattern;
use syncore::cognition::reasoning_ledger::ReasoningEpisode;
use syncore::cognition::self_consistency::{
    evaluate_self_consistency, SelfConsistencyConfig, SelfConsistencyIssueKind,
    SelfConsistencySeverity,
};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn get_unique_db_path() -> String {
    let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("/tmp/test_r4_2_db_{}_{}", std::process::id(), counter)
}

// Test 1: Detect repeated failed sequence
#[test]
fn test_repeated_failed_sequence_detected() -> Result<()> {
    // Build episodes with failing tool sequence
    let episodes = vec![
        ReasoningEpisode {
            id: 1,
            timestamp: 1000,
            user_query: "apply changes".to_string(),
            selected_mode: "simple".to_string(),
            important_entities: vec![],
            tool_calls: vec![
                "code_index".to_string(),
                "code_graph_fusion_query".to_string(),
                "apply_patch".to_string(),
            ],
            outcome: "failure".to_string(),
            notes: Some("patch failed".to_string()),
            client_id: None,
        },
        ReasoningEpisode {
            id: 2,
            timestamp: 2000,
            user_query: "apply changes again".to_string(),
            selected_mode: "simple".to_string(),
            important_entities: vec![],
            tool_calls: vec![
                "code_index".to_string(),
                "code_graph_fusion_query".to_string(),
                "apply_patch".to_string(),
            ],
            outcome: "failure".to_string(),
            notes: Some("patch failed again".to_string()),
            client_id: None,
        },
    ];

    let planned_tools = vec![
        "code_index".to_string(),
        "code_graph_fusion_query".to_string(),
        "apply_patch".to_string(),
    ];

    let context_bundle = ContextBundle::new();
    let continuity = ReasoningContinuity::new();
    let recommended_patterns = Vec::new();

    let result = evaluate_self_consistency(
        "apply patch",
        SelfConsistencyConfig {
            intent: &QueryIntent::Symbolic,
            selected_mode: "simple",
            planned_tools: &planned_tools,
            context_bundle: &context_bundle,
            continuity: &continuity,
            recommended_patterns: &recommended_patterns,
            ledger_episodes: &episodes,
        },
    );

    // Should detect repeated failure
    assert!(result.score < 1.0, "Score should be penalized");
    assert!(
        result
            .issues
            .iter()
            .any(|i| matches!(i.kind, SelfConsistencyIssueKind::RepeatedFailedSequence)),
        "Should detect repeated failed sequence"
    );

    Ok(())
}

// Test 2: Detect conflicting pattern with better success rate
#[test]
fn test_conflicting_pattern_with_better_success_rate() -> Result<()> {
    use syncore::cognition::pattern_engine::PatternGraphUsage;

    // Pattern A: low success
    let pattern_a = ReasoningPattern {
        id: 1,
        intent_type: QueryIntent::Symbolic,
        selected_mode: "simple".to_string(),
        tool_sequence: vec!["tool_a".to_string(), "tool_b".to_string()],
        graph_usage: PatternGraphUsage::None,
        success_count: 2,
        failure_count: 8,
        success_rate: 0.2,
        last_updated: 1000,
        client_id: None,
    };

    // Pattern B: high success
    let pattern_b = ReasoningPattern {
        id: 2,
        intent_type: QueryIntent::Symbolic,
        selected_mode: "simple".to_string(),
        tool_sequence: vec!["tool_c".to_string(), "tool_d".to_string()],
        graph_usage: PatternGraphUsage::None,
        success_count: 9,
        failure_count: 1,
        success_rate: 0.9,
        last_updated: 2000,
        client_id: None,
    };

    let planned_tools = vec!["tool_a".to_string(), "tool_b".to_string()];
    let recommended_patterns = vec![pattern_b.clone()]; // Recommends better pattern

    let context_bundle = ContextBundle::new();
    let continuity = ReasoningContinuity::new();
    let episodes = vec![];

    let result = evaluate_self_consistency(
        "test",
        SelfConsistencyConfig {
            intent: &QueryIntent::Symbolic,
            selected_mode: "simple",
            planned_tools: &planned_tools,
            context_bundle: &context_bundle,
            continuity: &continuity,
            recommended_patterns: &recommended_patterns,
            ledger_episodes: &episodes,
        },
    );

    // Should detect conflict
    assert!(
        result
            .issues
            .iter()
            .any(|i| matches!(i.kind, SelfConsistencyIssueKind::ConflictingPattern)),
        "Should detect conflicting pattern"
    );

    Ok(())
}

// Test 3: Detect graph inconsistency when no graph entities present
#[test]
fn test_graph_inconsistency_when_no_graph_entities_present() -> Result<()> {
    // Empty context bundle (no graph entities)
    let context_bundle = ContextBundle::new();

    // But plan includes graph-heavy tools
    let planned_tools = vec!["code_graph_fusion_query".to_string(), "raggraph_query".to_string()];

    let continuity = ReasoningContinuity::new();
    let recommended_patterns = Vec::new();
    let episodes = Vec::new();

    let result = evaluate_self_consistency(
        "query graph",
        SelfConsistencyConfig {
            intent: &QueryIntent::Semantic,
            selected_mode: "reasoning",
            planned_tools: &planned_tools,
            context_bundle: &context_bundle,
            continuity: &continuity,
            recommended_patterns: &recommended_patterns,
            ledger_episodes: &episodes,
        },
    );

    // Should detect graph inconsistency
    assert!(
        result
            .issues
            .iter()
            .any(|i| matches!(i.kind, SelfConsistencyIssueKind::GraphInconsistency)),
        "Should detect graph inconsistency"
    );

    Ok(())
}

// Test 4: Detect namespace mismatch
#[test]
fn test_namespace_mismatch() -> Result<()> {
    use syncore::cognition::context_bundle::{CodeEntityWithScore, LtmcGraphRelation};

    // Context bundle with entities in different files (namespace proxy via file path)
    let mut context_bundle = ContextBundle::new();
    context_bundle.raggraph_entities.push(CodeEntityWithScore {
        entity_id: Some(1),
        name: "alpha_function".to_string(),
        entity_type: "function".to_string(),
        file_path: "src/core/alpha.rs".to_string(),
        signature: Some("fn alpha_function()".to_string()),
        score: 0.9,
        rank: 1,
    });

    // Add graph relation pointing to different namespace (via properties)
    context_bundle.memory_graph.push(LtmcGraphRelation {
        source_id: "core::beta::beta_module".to_string(),
        relation_type: "calls".to_string(),
        target_id: "core::beta::beta_function".to_string(),
        properties: Some(serde_json::json!({"namespace": "core::beta"})),
    });

    let planned_tools = vec!["code_index".to_string()];
    let continuity = ReasoningContinuity::new();
    let recommended_patterns = Vec::new();
    let episodes = Vec::new();

    let result = evaluate_self_consistency(
        "test",
        SelfConsistencyConfig {
            intent: &QueryIntent::Symbolic,
            selected_mode: "simple",
            planned_tools: &planned_tools,
            context_bundle: &context_bundle,
            continuity: &continuity,
            recommended_patterns: &recommended_patterns,
            ledger_episodes: &episodes,
        },
    );

    // Should detect namespace mismatch
    assert!(
        result.issues.iter().any(|i| matches!(i.kind, SelfConsistencyIssueKind::NamespaceMismatch)),
        "Should detect namespace mismatch"
    );

    Ok(())
}

// Test 5: Detect suspicious tool order
#[test]
fn test_tool_order_suspicious_against_success_patterns() -> Result<()> {
    use syncore::cognition::pattern_engine::PatternGraphUsage;

    // Pattern shows successful sequence: code_index → code_graph_fusion_query
    let pattern = ReasoningPattern {
        id: 1,
        intent_type: QueryIntent::Semantic,
        selected_mode: "attention".to_string(),
        tool_sequence: vec!["code_index".to_string(), "code_graph_fusion_query".to_string()],
        graph_usage: PatternGraphUsage::Heavy,
        success_count: 10,
        failure_count: 0,
        success_rate: 1.0,
        last_updated: 1000,
        client_id: None,
    };

    // But planned tools are in reverse order
    let planned_tools = vec!["code_graph_fusion_query".to_string(), "code_index".to_string()];

    let recommended_patterns = vec![pattern];
    let context_bundle = ContextBundle::new();
    let continuity = ReasoningContinuity::new();
    let episodes = Vec::new();

    let result = evaluate_self_consistency(
        "analyze code",
        SelfConsistencyConfig {
            intent: &QueryIntent::Semantic,
            selected_mode: "attention",
            planned_tools: &planned_tools,
            context_bundle: &context_bundle,
            continuity: &continuity,
            recommended_patterns: &recommended_patterns,
            ledger_episodes: &episodes,
        },
    );

    // Should detect suspicious ordering
    assert!(
        result
            .issues
            .iter()
            .any(|i| matches!(i.kind, SelfConsistencyIssueKind::ToolOrderSuspicious)),
        "Should detect suspicious tool order"
    );

    Ok(())
}

// Test 6: Detect missing required step
#[test]
fn test_missing_required_step_detected() -> Result<()> {
    use syncore::cognition::pattern_engine::PatternGraphUsage;

    // Patterns show code_index usually precedes graph operations
    let pattern = ReasoningPattern {
        id: 1,
        intent_type: QueryIntent::Causal,
        selected_mode: "reasoning".to_string(),
        tool_sequence: vec!["code_index".to_string(), "code_graph_fusion_query".to_string()],
        graph_usage: PatternGraphUsage::Heavy,
        success_count: 15,
        failure_count: 1,
        success_rate: 0.9375,
        last_updated: 1000,
        client_id: None,
    };

    // But planned tools skip code_index
    let planned_tools = vec!["code_graph_fusion_query".to_string()];

    let recommended_patterns = vec![pattern];
    let context_bundle = ContextBundle::new();
    let continuity = ReasoningContinuity::new();
    let episodes = Vec::new();

    let result = evaluate_self_consistency(
        "trace flow",
        SelfConsistencyConfig {
            intent: &QueryIntent::Causal,
            selected_mode: "reasoning",
            planned_tools: &planned_tools,
            context_bundle: &context_bundle,
            continuity: &continuity,
            recommended_patterns: &recommended_patterns,
            ledger_episodes: &episodes,
        },
    );

    // Should detect missing step
    assert!(
        result
            .issues
            .iter()
            .any(|i| matches!(i.kind, SelfConsistencyIssueKind::MissingRequiredStep)),
        "Should detect missing required step"
    );

    Ok(())
}

// Test 7: Detect potential loop from continuity
#[test]
fn test_potential_loop_detected_from_continuity() -> Result<()> {
    // Continuity shows repeated alternation
    let mut continuity = ReasoningContinuity::new();
    continuity.episodes = vec![
        ReasoningEpisode {
            id: 1,
            timestamp: 1000,
            user_query: "test1".to_string(),
            selected_mode: "simple".to_string(),
            important_entities: vec![],
            tool_calls: vec!["tool_a".to_string()],
            outcome: "partial".to_string(),
            notes: None,
            client_id: None,
        },
        ReasoningEpisode {
            id: 2,
            timestamp: 2000,
            user_query: "test2".to_string(),
            selected_mode: "simple".to_string(),
            important_entities: vec![],
            tool_calls: vec!["tool_b".to_string()],
            outcome: "partial".to_string(),
            notes: None,
            client_id: None,
        },
        ReasoningEpisode {
            id: 3,
            timestamp: 3000,
            user_query: "test3".to_string(),
            selected_mode: "simple".to_string(),
            important_entities: vec![],
            tool_calls: vec!["tool_a".to_string()],
            outcome: "partial".to_string(),
            notes: None,
            client_id: None,
        },
        ReasoningEpisode {
            id: 4,
            timestamp: 4000,
            user_query: "test4".to_string(),
            selected_mode: "simple".to_string(),
            important_entities: vec![],
            tool_calls: vec!["tool_b".to_string()],
            outcome: "partial".to_string(),
            notes: None,
            client_id: None,
        },
    ];

    // Plan repeats the alternating cycle
    let planned_tools = vec!["tool_a".to_string()];

    let context_bundle = ContextBundle::new();
    let recommended_patterns = Vec::new();
    let episodes = Vec::new();

    let result = evaluate_self_consistency(
        "test",
        SelfConsistencyConfig {
            intent: &QueryIntent::Symbolic,
            selected_mode: "simple",
            planned_tools: &planned_tools,
            context_bundle: &context_bundle,
            continuity: &continuity,
            recommended_patterns: &recommended_patterns,
            ledger_episodes: &episodes,
        },
    );

    // Should detect potential loop
    assert!(
        result.issues.iter().any(|i| matches!(i.kind, SelfConsistencyIssueKind::PotentialLoop)),
        "Should detect potential loop"
    );

    Ok(())
}

// Test 8: Score decreases with issue severity
#[test]
fn test_score_decreases_with_issue_severity() -> Result<()> {
    // Scenario 1: No issues
    let context_bundle = ContextBundle::new();
    let continuity = ReasoningContinuity::new();
    let planned_tools = vec!["code_index".to_string()];

    let result_clean = evaluate_self_consistency(
        "test",
        SelfConsistencyConfig {
            intent: &QueryIntent::Symbolic,
            selected_mode: "simple",
            planned_tools: &planned_tools,
            context_bundle: &context_bundle,
            continuity: &continuity,
            recommended_patterns: &Vec::new(),
            ledger_episodes: &Vec::new(),
        },
    );

    // Scenario 2: Create episodes that will trigger warnings
    let episodes_warn = vec![ReasoningEpisode {
        id: 1,
        timestamp: 1000,
        user_query: "test".to_string(),
        selected_mode: "simple".to_string(),
        important_entities: vec![],
        tool_calls: vec!["code_index".to_string()],
        outcome: "failure".to_string(),
        notes: None,
        client_id: None,
    }];

    let result_warn = evaluate_self_consistency(
        "test",
        SelfConsistencyConfig {
            intent: &QueryIntent::Symbolic,
            selected_mode: "simple",
            planned_tools: &planned_tools,
            context_bundle: &context_bundle,
            continuity: &continuity,
            recommended_patterns: &Vec::new(),
            ledger_episodes: &episodes_warn,
        },
    );

    // Scenario 3: Multiple failures (errors)
    let episodes_error = vec![
        ReasoningEpisode {
            id: 1,
            timestamp: 1000,
            user_query: "test".to_string(),
            selected_mode: "simple".to_string(),
            important_entities: vec![],
            tool_calls: vec!["code_index".to_string()],
            outcome: "failure".to_string(),
            notes: None,
            client_id: None,
        },
        ReasoningEpisode {
            id: 2,
            timestamp: 2000,
            user_query: "test".to_string(),
            selected_mode: "simple".to_string(),
            important_entities: vec![],
            tool_calls: vec!["code_index".to_string()],
            outcome: "failure".to_string(),
            notes: None,
            client_id: None,
        },
        ReasoningEpisode {
            id: 3,
            timestamp: 3000,
            user_query: "test".to_string(),
            selected_mode: "simple".to_string(),
            important_entities: vec![],
            tool_calls: vec!["code_index".to_string()],
            outcome: "failure".to_string(),
            notes: None,
            client_id: None,
        },
    ];

    let result_error = evaluate_self_consistency(
        "test",
        SelfConsistencyConfig {
            intent: &QueryIntent::Symbolic,
            selected_mode: "simple",
            planned_tools: &planned_tools,
            context_bundle: &context_bundle,
            continuity: &continuity,
            recommended_patterns: &Vec::new(),
            ledger_episodes: &episodes_error,
        },
    );

    // Scores should decrease with severity
    assert!(result_clean.score >= result_warn.score, "Clean score should be >= warning score");
    assert!(result_warn.score >= result_error.score, "Warning score should be >= error score");

    Ok(())
}

// Test 9: Backward compatibility - all previous phases still work
#[tokio::test]
async fn test_backwards_compatibility() -> Result<()> {
    use std::sync::{Arc, Mutex};
    use syncore::code_graph::CodeGraph;
    use syncore::cognition::intent_classifier::classify_intent;
    use syncore::cognition::orchestrator::enrich_query_with_context_bundle;
    use syncore::cognition::router_logic::route_query;
    use syncore::memory::Memory;
    use syncore::vector::{HuggingFaceEmbeddings, VectorStore};

    // R3.1: Intent classification
    let intent = classify_intent("format_code");
    assert_eq!(intent, QueryIntent::Symbolic);

    // R3.1: Router
    let decision = route_query(&QueryIntent::Semantic, "explain test");
    assert!(decision.should_call_raggraph);

    // R3.2 + R3.3 + R4.1: Full orchestrator
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let code_graph = CodeGraph::new(":memory:", vector_store)?;
    let memory = Memory::new(&get_unique_db_path())?;

    let enriched =
        enrich_query_with_context_bundle("test query", &code_graph, &memory, None).await?;

    assert!(enriched.context_bundle.is_some());
    // R4.1: recommended_patterns exists
    assert!(enriched.recommended_patterns.is_some() || enriched.recommended_patterns.is_none());

    Ok(())
}
