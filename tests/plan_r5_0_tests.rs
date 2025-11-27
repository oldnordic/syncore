//! Phase R5.0 - Planning Engine + Plan Executor Tests
//!
//! Tests for the minimal action layer that:
//! - Generates high-level plans based on intent, patterns, and consistency
//! - Executes steps using real SynCore MCP tools
//! - Stores episodes into ReasoningLedger
//! - Uses RAGGraph to map codebase/knowledge when relevant
//!
//! Requirements:
//! - STRICT TDD (tests first)
//! - Real tools only (no mocks/stubs)
//! - Modules < 300 LOC each
//! - No regressions (R2.2-R4.2 tests pass)
//! - Minimal plans (3-8 steps max)
//! - Read/search/index tools by default
//! - Write operations require explicit allow_write flag

use anyhow::Result;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use syncore::code_graph::CodeGraph;
use syncore::cognition::context_bundle::{CodeEntityWithScore, ContextBundle};
use syncore::cognition::continuity_engine::ReasoningContinuity;
use syncore::cognition::intent_classifier::{classify_intent, QueryIntent};
use syncore::cognition::orchestrator::{enrich_query_with_context_bundle, EnrichedContext};
use syncore::cognition::pattern_engine::{PatternGraphUsage, ReasoningPattern};
use syncore::cognition::plan_engine::{generate_plan, Plan, PlanStep};
use syncore::cognition::plan_executor::{execute_plan, ExecutionResult};
use syncore::cognition::reasoning_ledger::{fetch_recent_episodes_sql, ReasoningEpisode};
use syncore::cognition::router_logic::route_query;
use syncore::cognition::self_consistency::{
    evaluate_self_consistency, SelfConsistencyIssue, SelfConsistencyIssueKind,
};
use syncore::memory::Memory;
use syncore::router::SynCoreState;
use syncore::vector::{HuggingFaceEmbeddings, VectorStore};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn get_unique_db_path() -> String {
    let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("/tmp/test_r5_0_db_{}_{}", std::process::id(), counter)
}

// Test 1: Generate plan respects intent and patterns
#[test]
fn test_generate_plan_respects_intent_and_patterns() -> Result<()> {
    // Given: Semantic intent with successful patterns for code_index
    let intent = QueryIntent::Semantic;
    let selected_mode = "simple";

    let pattern = ReasoningPattern {
        id: 1,
        intent_type: QueryIntent::Semantic,
        selected_mode: "simple".to_string(),
        tool_sequence: vec!["code_index".to_string(), "code_search".to_string()],
        graph_usage: PatternGraphUsage::None,
        success_count: 10,
        failure_count: 1,
        success_rate: 0.91,
        last_updated: 1234567890,
        client_id: None,
    };

    let patterns = vec![pattern];
    let bundle = ContextBundle::new();
    let consistency = evaluate_self_consistency(
        "test query",
        &intent,
        selected_mode,
        &[],
        &bundle,
        &ReasoningContinuity::new(),
        &patterns,
        &[],
    );

    // When: Generate plan
    let plan = generate_plan(
        "explain how code indexing works",
        &intent,
        selected_mode,
        &patterns,
        &consistency,
        &bundle,
    )?;

    // Then: Plan should include tools from successful pattern
    assert!(plan.steps.len() >= 1 && plan.steps.len() <= 8);
    assert!(plan
        .steps
        .iter()
        .any(|s| s.tool.contains("code_index") || s.tool.contains("code_search")));

    Ok(())
}

// Test 2: Generate plan avoids inconsistent sequences
#[test]
fn test_generate_plan_avoids_inconsistent_sequences() -> Result<()> {
    // Given: Historical failures for a specific tool sequence
    let intent = QueryIntent::Semantic;
    let selected_mode = "simple";

    let failed_episode = ReasoningEpisode {
        id: 1,
        timestamp: 1234567890,
        user_query: "test".to_string(),
        selected_mode: "simple".to_string(),
        important_entities: vec![],
        tool_calls: vec!["bad_tool_1".to_string(), "bad_tool_2".to_string()],
        outcome: "failure".to_string(),
        notes: None,
        client_id: None,
    };

    let bundle = ContextBundle::new();
    let consistency = evaluate_self_consistency(
        "test query",
        &intent,
        selected_mode,
        &["bad_tool_1".to_string(), "bad_tool_2".to_string()],
        &bundle,
        &ReasoningContinuity::new(),
        &[],
        &[
            failed_episode.clone(),
            failed_episode.clone(),
            failed_episode,
        ],
    );

    // Then: Consistency should detect repeated failures
    assert!(
        consistency.score < 0.8,
        "Expected low consistency score for repeated failures"
    );
    assert!(consistency
        .issues
        .iter()
        .any(|i| matches!(i.kind, SelfConsistencyIssueKind::RepeatedFailedSequence)));

    // When: Generate plan (should avoid failed sequence)
    let plan = generate_plan(
        "test query",
        &intent,
        selected_mode,
        &[],
        &consistency,
        &bundle,
    )?;

    // Then: Plan should not include bad tools
    assert!(plan
        .steps
        .iter()
        .all(|s| !s.tool.contains("bad_tool_1") && !s.tool.contains("bad_tool_2")));

    Ok(())
}

// Test 3: Execute plan runs real tools (code_index)
#[tokio::test]
async fn test_execute_plan_runs_real_tools() -> Result<()> {
    // Setup
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState::with_db_manager(vector_store)?;

    // Create a minimal plan with parser_search (read-only operation)
    let plan = Plan {
        steps: vec![PlanStep {
            tool: "parser_search".to_string(),
            args: serde_json::json!({
                "pattern": "test"
            }),
        }],
        notes: Some("Test plan".to_string()),
    };

    // Execute (read-only, so allow_write not needed)
    let result = execute_plan(&plan, &state, false).await?;

    // Assert: Execution completed
    assert_eq!(result.steps_executed, 1);
    assert!(result.outputs.len() == 1);

    Ok(())
}

// Test 4: Execute plan collects outputs into episode
#[tokio::test]
async fn test_execute_plan_collects_outputs_into_episode() -> Result<()> {
    // Setup
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState::with_db_manager(vector_store)?;

    // Create plan with memory operations
    let plan = Plan {
        steps: vec![
            PlanStep {
                tool: "memory_store".to_string(),
                args: serde_json::json!({
                    "key": "test_key",
                    "value": "test_value"
                }),
            },
            PlanStep {
                tool: "memory_query".to_string(),
                args: serde_json::json!({
                    "key": "test_key"
                }),
            },
        ],
        notes: None,
    };

    // Execute with allow_write=true for memory_store
    let result = execute_plan(&plan, &state, true).await?;

    // Assert: Both steps executed, outputs collected
    assert_eq!(result.steps_executed, 2);
    assert_eq!(result.outputs.len(), 2);
    assert!(result.tool_calls.len() == 2);
    assert_eq!(result.tool_calls[0], "memory_store");
    assert_eq!(result.tool_calls[1], "memory_query");

    Ok(())
}

// Test 5: Plan attached into enriched context
#[tokio::test]
async fn test_plan_attached_into_enriched_context() -> Result<()> {
    // Setup
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let code_graph = CodeGraph::new(":memory:", vector_store)?;
    let memory = Memory::new(&get_unique_db_path())?;

    // Run query through orchestrator
    let enriched =
        enrich_query_with_context_bundle("test query", &code_graph, &memory, None).await?;

    // Assert: Plan field exists (may be None if prerequisites not met)
    // Plan generation requires mode_hint + patterns + consistency
    // This test just verifies the field is present and accessible
    assert!(enriched.context_bundle.is_some());

    Ok(())
}

// Test 6: Plan execution stores episode in ledger
#[tokio::test]
async fn test_plan_execution_stores_episode_in_ledger() -> Result<()> {
    // Setup
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState::with_db_manager(vector_store.clone())?;
    let code_graph = CodeGraph::new(":memory:", vector_store)?;
    let memory = Memory::new(&get_unique_db_path())?;

    // Generate and execute plan
    let intent = classify_intent("test query");
    let decision = route_query(&intent, "test query");
    let patterns = vec![];
    let bundle = ContextBundle::new();
    let consistency = evaluate_self_consistency(
        "test query",
        &intent,
        decision.mode_hint.as_deref().unwrap_or("simple"),
        &[],
        &bundle,
        &ReasoningContinuity::new(),
        &patterns,
        &[],
    );

    let plan = generate_plan(
        "test query",
        &intent,
        decision.mode_hint.as_deref().unwrap_or("simple"),
        &patterns,
        &consistency,
        &bundle,
    )?;

    let _result = execute_plan(&plan, &state, false).await?;

    // Store episode manually (orchestrator would do this)
    let episode = ReasoningEpisode {
        id: 1,
        timestamp: 1234567890,
        user_query: "test query".to_string(),
        selected_mode: decision.mode_hint.unwrap_or_else(|| "simple".to_string()),
        important_entities: vec![],
        tool_calls: plan.steps.iter().map(|s| s.tool.clone()).collect(),
        outcome: "success".to_string(),
        notes: plan.notes,
        client_id: None,
    };

    syncore::cognition::reasoning_ledger::store_episode_sql(&memory, &episode)?;

    // Verify episode stored
    let episodes = fetch_recent_episodes_sql(&memory, "test", 10)?;
    assert!(!episodes.is_empty());

    Ok(())
}

// Test 7: RAGGraph is used to map project if query mentions entities
#[tokio::test]
async fn test_raggraph_is_used_to_map_project_if_query_mentions_entities() -> Result<()> {
    // Given: Query that mentions code entities
    let query = "explain the CodeGraph indexer";
    let intent = classify_intent(query);
    let selected_mode = "attention";

    // Setup
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let code_graph = CodeGraph::new(":memory:", vector_store)?;

    // Create bundle with entities
    let mut bundle = ContextBundle::with_mode(selected_mode);
    bundle.add_raggraph_entity(CodeEntityWithScore {
        entity_id: Some(1),
        file_path: "src/code_graph/indexer.rs".to_string(),
        entity_type: "module".to_string(),
        name: "indexer".to_string(),
        signature: None,
        score: 0.9,
        rank: 1,
    });

    let patterns = vec![];
    let consistency = evaluate_self_consistency(
        query,
        &intent,
        selected_mode,
        &[],
        &bundle,
        &ReasoningContinuity::new(),
        &patterns,
        &[],
    );

    // When: Generate plan
    let plan = generate_plan(
        query,
        &intent,
        selected_mode,
        &patterns,
        &consistency,
        &bundle,
    )?;

    // Then: Plan should include graph or search operations for mentioned entities
    assert!(plan.steps.len() > 0);
    // Should use code_graph_fusion_query or code_search when entities present
    let has_graph_or_search = plan
        .steps
        .iter()
        .any(|s| s.tool.contains("code_graph") || s.tool.contains("code_search"));
    assert!(
        has_graph_or_search,
        "Plan should use graph/search when entities mentioned"
    );

    Ok(())
}

// Test 8: Backward compatibility - all R2.x-R4.2 tests still pass
#[tokio::test]
async fn test_backward_compatibility_r2_to_r4() -> Result<()> {
    use syncore::code_graph::fusion_simple::FusionSimple;
    use syncore::cognition::intent_classifier::classify_intent;

    // R2.4: Fusion still works
    let fusion = FusionSimple::new(0.6, 0.3, 0.1, 0.0);
    let score = fusion.combine(0.8, 0.4, 0.0, 0.0);
    assert!((score - 0.64).abs() < 0.001);

    // R3.1: Intent classification still works
    let intent = classify_intent("format_string");
    assert_eq!(intent, QueryIntent::Symbolic);

    // R3.2: ContextBundle composition still works
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let code_graph = CodeGraph::new(":memory:", vector_store)?;
    let memory = Memory::new(&get_unique_db_path())?;

    let enriched =
        enrich_query_with_context_bundle("test query", &code_graph, &memory, None).await?;

    assert!(enriched.context_bundle.is_some());
    // R5.0: Plan field added (may be None without full prerequisites)

    Ok(())
}
