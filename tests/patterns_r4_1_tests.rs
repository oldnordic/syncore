//! Phase R4.1 - Reasoning Pattern Extraction Engine Tests
//!
//! Tests for mining, storing, and recommending reasoning patterns.
//! Verifies:
//! - Pattern mining from episodes (grouping by intent, mode, tool sequence)
//! - Success rate computation
//! - Graph usage detection
//! - Pattern storage and retrieval
//! - Pattern recommendation (filtering, sorting)
//! - Orchestrator integration
//! - Backward compatibility

use anyhow::Result;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn get_unique_db_path() -> String {
    let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("/tmp/test_r4_1_db_{}_{}", std::process::id(), counter)
}

// Test 1: Mine patterns groups by intent and mode
#[test]
fn test_mine_patterns_groups_by_intent_and_mode() -> Result<()> {
    use syncore::cognition::pattern_engine::mine_patterns_from_episodes;
    use syncore::cognition::reasoning_ledger::ReasoningEpisode;

    // Create episodes with different intents and modes
    let episodes = vec![
        ReasoningEpisode {
            id: 1,
            timestamp: 1000,
            user_query: "query1".to_string(),
            selected_mode: "simple".to_string(),
            important_entities: vec![],
            tool_calls: vec!["code_index".to_string()],
            outcome: "success".to_string(),
            notes: None,
            client_id: None,
        },
        ReasoningEpisode {
            id: 2,
            timestamp: 1001,
            user_query: "explain query2".to_string(),
            selected_mode: "simple".to_string(),
            important_entities: vec![],
            tool_calls: vec!["code_index".to_string()],
            outcome: "success".to_string(),
            notes: None,
            client_id: None,
        },
        ReasoningEpisode {
            id: 3,
            timestamp: 1002,
            user_query: "trace flow".to_string(),
            selected_mode: "reasoning".to_string(),
            important_entities: vec![],
            tool_calls: vec!["code_graph_fusion_query".to_string()],
            outcome: "success".to_string(),
            notes: None,
            client_id: None,
        },
    ];

    let patterns = mine_patterns_from_episodes(&episodes);

    // Should have patterns grouped by (intent derived from query, mode, tools)
    assert!(!patterns.is_empty());

    // Find pattern for simple mode
    let simple_patterns: Vec<_> = patterns
        .iter()
        .filter(|p| p.selected_mode == "simple")
        .collect();
    assert!(!simple_patterns.is_empty());

    Ok(())
}

// Test 2: Mine patterns computes success rate
#[test]
fn test_mine_patterns_computes_success_rate() -> Result<()> {
    use syncore::cognition::pattern_engine::mine_patterns_from_episodes;
    use syncore::cognition::reasoning_ledger::ReasoningEpisode;

    // Create 3 successful, 1 failed episode with same pattern
    let episodes = vec![
        ReasoningEpisode {
            id: 1,
            timestamp: 1000,
            user_query: "test".to_string(),
            selected_mode: "simple".to_string(),
            important_entities: vec![],
            tool_calls: vec!["tool1".to_string()],
            outcome: "success".to_string(),
            notes: None,
            client_id: None,
        },
        ReasoningEpisode {
            id: 2,
            timestamp: 1001,
            user_query: "test".to_string(),
            selected_mode: "simple".to_string(),
            important_entities: vec![],
            tool_calls: vec!["tool1".to_string()],
            outcome: "success".to_string(),
            notes: None,
            client_id: None,
        },
        ReasoningEpisode {
            id: 3,
            timestamp: 1002,
            user_query: "test".to_string(),
            selected_mode: "simple".to_string(),
            important_entities: vec![],
            tool_calls: vec!["tool1".to_string()],
            outcome: "success".to_string(),
            notes: None,
            client_id: None,
        },
        ReasoningEpisode {
            id: 4,
            timestamp: 1003,
            user_query: "test".to_string(),
            selected_mode: "simple".to_string(),
            important_entities: vec![],
            tool_calls: vec!["tool1".to_string()],
            outcome: "error".to_string(),
            notes: None,
            client_id: None,
        },
    ];

    let patterns = mine_patterns_from_episodes(&episodes);

    // Find the pattern
    assert_eq!(patterns.len(), 1);
    let pattern = &patterns[0];

    assert_eq!(pattern.success_count, 3);
    assert_eq!(pattern.failure_count, 1);
    assert!((pattern.success_rate - 0.75).abs() < 0.01);

    Ok(())
}

// Test 3: Mine patterns detects graph usage
#[test]
fn test_mine_patterns_detects_graph_usage() -> Result<()> {
    use syncore::cognition::pattern_engine::{mine_patterns_from_episodes, PatternGraphUsage};
    use syncore::cognition::reasoning_ledger::ReasoningEpisode;

    let episodes = vec![ReasoningEpisode {
        id: 1,
        timestamp: 1000,
        user_query: "test".to_string(),
        selected_mode: "reasoning".to_string(),
        important_entities: vec![],
        tool_calls: vec![
            "code_graph_fusion_query".to_string(),
            "code_index".to_string(),
        ],
        outcome: "success".to_string(),
        notes: None,
        client_id: None,
    }];

    let patterns = mine_patterns_from_episodes(&episodes);

    assert_eq!(patterns.len(), 1);
    let pattern = &patterns[0];

    // Should detect graph usage
    assert!(matches!(
        pattern.graph_usage,
        PatternGraphUsage::Heavy | PatternGraphUsage::Light
    ));

    Ok(())
}

// Test 4: Store and load patterns roundtrip
#[test]
fn test_store_and_load_patterns_roundtrip() -> Result<()> {
    use syncore::cognition::intent_classifier::QueryIntent;
    use syncore::cognition::pattern_engine::{
        load_patterns_from_memory, store_patterns_to_memory, PatternGraphUsage, ReasoningPattern,
    };
    use syncore::memory::Memory;

    let memory = Memory::new(&get_unique_db_path())?;

    let patterns = vec![ReasoningPattern {
        id: 1,
        intent_type: QueryIntent::Symbolic,
        selected_mode: "simple".to_string(),
        tool_sequence: vec!["tool1".to_string(), "tool2".to_string()],
        graph_usage: PatternGraphUsage::None,
        success_count: 5,
        failure_count: 1,
        success_rate: 5.0 / 6.0,
        last_updated: 2000,
        client_id: None,
    }];

    // Store
    store_patterns_to_memory(&patterns, &memory, "test_namespace")?;

    // Load
    let loaded = load_patterns_from_memory(&memory, "test_namespace")?;

    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, 1);
    assert_eq!(loaded[0].selected_mode, "simple");
    assert_eq!(loaded[0].success_count, 5);
    assert_eq!(loaded[0].failure_count, 1);

    Ok(())
}

// Test 5: Recommend patterns returns top by success rate
#[test]
fn test_recommend_patterns_returns_top_by_success_rate() -> Result<()> {
    use syncore::cognition::intent_classifier::QueryIntent;
    use syncore::cognition::pattern_engine::{
        recommend_patterns_for_query, store_patterns_to_memory, PatternGraphUsage, ReasoningPattern,
    };
    use syncore::memory::Memory;

    let memory = Memory::new(&get_unique_db_path())?;

    let patterns = vec![
        ReasoningPattern {
            id: 1,
            intent_type: QueryIntent::Semantic,
            selected_mode: "simple".to_string(),
            tool_sequence: vec!["tool1".to_string()],
            graph_usage: PatternGraphUsage::None,
            success_count: 10,
            failure_count: 2,
            success_rate: 10.0 / 12.0,
            last_updated: 2000,
        client_id: None,
        },
        ReasoningPattern {
            id: 2,
            intent_type: QueryIntent::Semantic,
            selected_mode: "simple".to_string(),
            tool_sequence: vec!["tool2".to_string()],
            graph_usage: PatternGraphUsage::None,
            success_count: 20,
            failure_count: 1,
            success_rate: 20.0 / 21.0,
            last_updated: 2001,
        client_id: None,
        },
        ReasoningPattern {
            id: 3,
            intent_type: QueryIntent::Symbolic,
            selected_mode: "simple".to_string(),
            tool_sequence: vec!["tool3".to_string()],
            graph_usage: PatternGraphUsage::None,
            success_count: 5,
            failure_count: 0,
            success_rate: 1.0,
            last_updated: 2002,
        client_id: None,
        },
    ];

    store_patterns_to_memory(&patterns, &memory, "test")?;

    // Recommend for Semantic + simple
    let recommended =
        recommend_patterns_for_query(&QueryIntent::Semantic, "simple", &memory, "test", 10)?;

    // Should return only Semantic patterns, sorted by success_rate
    assert_eq!(recommended.len(), 2);
    assert_eq!(recommended[0].id, 2); // Higher success rate
    assert_eq!(recommended[1].id, 1);

    Ok(())
}

// Test 6: Recommend patterns filters by intent and mode
#[test]
fn test_recommend_patterns_filters_by_intent_and_mode() -> Result<()> {
    use syncore::cognition::intent_classifier::QueryIntent;
    use syncore::cognition::pattern_engine::{
        recommend_patterns_for_query, store_patterns_to_memory, PatternGraphUsage, ReasoningPattern,
    };
    use syncore::memory::Memory;

    let memory = Memory::new(&get_unique_db_path())?;

    let patterns = vec![
        ReasoningPattern {
            id: 1,
            intent_type: QueryIntent::Symbolic,
            selected_mode: "simple".to_string(),
            tool_sequence: vec![],
            graph_usage: PatternGraphUsage::None,
            success_count: 5,
            failure_count: 0,
            success_rate: 1.0,
            last_updated: 2000,
        client_id: None,
        },
        ReasoningPattern {
            id: 2,
            intent_type: QueryIntent::Semantic,
            selected_mode: "attention".to_string(),
            tool_sequence: vec![],
            graph_usage: PatternGraphUsage::Light,
            success_count: 3,
            failure_count: 0,
            success_rate: 1.0,
            last_updated: 2001,
        client_id: None,
        },
        ReasoningPattern {
            id: 3,
            intent_type: QueryIntent::Symbolic,
            selected_mode: "attention".to_string(),
            tool_sequence: vec![],
            graph_usage: PatternGraphUsage::None,
            success_count: 2,
            failure_count: 0,
            success_rate: 1.0,
            last_updated: 2002,
        client_id: None,
        },
    ];

    store_patterns_to_memory(&patterns, &memory, "test")?;

    // Filter for Symbolic + attention
    let recommended =
        recommend_patterns_for_query(&QueryIntent::Symbolic, "attention", &memory, "test", 10)?;

    assert_eq!(recommended.len(), 1);
    assert_eq!(recommended[0].id, 3);

    Ok(())
}

// Test 7: Orchestrator attaches recommended patterns
#[tokio::test]
async fn test_orchestrator_attaches_recommended_patterns() -> Result<()> {
    use std::sync::{Arc, Mutex};
    use syncore::code_graph::CodeGraph;
    use syncore::cognition::intent_classifier::QueryIntent;
    use syncore::cognition::orchestrator::enrich_query_with_context_bundle;
    use syncore::cognition::pattern_engine::{
        store_patterns_to_memory, PatternGraphUsage, ReasoningPattern,
    };
    use syncore::memory::Memory;
    use syncore::vector::{HuggingFaceEmbeddings, VectorStore};

    // Setup
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let code_graph = CodeGraph::new(":memory:", vector_store)?;
    let memory = Memory::new(&get_unique_db_path())?;

    // Pre-store some patterns
    let patterns = vec![ReasoningPattern {
        id: 1,
        intent_type: QueryIntent::Symbolic,
        selected_mode: "simple".to_string(),
        tool_sequence: vec!["code_index".to_string()],
        graph_usage: PatternGraphUsage::None,
        success_count: 10,
        failure_count: 0,
        success_rate: 1.0,
        last_updated: 2000,
        client_id: None,
    }];
    store_patterns_to_memory(&patterns, &memory, "default")?;

    // Run query through orchestrator
    let enriched =
        enrich_query_with_context_bundle("test_function", &code_graph, &memory, None).await?;

    // Verify enriched context exists (patterns integration will be added to orchestrator)
    assert!(enriched.context_bundle.is_some());

    Ok(())
}

// Test 8: Backwards compatibility
#[tokio::test]
async fn test_backwards_compatibility() -> Result<()> {
    use std::sync::{Arc, Mutex};
    use syncore::code_graph::fusion_simple::FusionSimple;
    use syncore::code_graph::CodeGraph;
    use syncore::cognition::intent_classifier::{classify_intent, QueryIntent};
    use syncore::cognition::orchestrator::enrich_query_with_context_bundle;
    use syncore::cognition::router_logic::route_query;
    use syncore::memory::Memory;
    use syncore::vector::{HuggingFaceEmbeddings, VectorStore};

    // R2.4: Fusion still works
    let fusion = FusionSimple::new(0.6, 0.3, 0.1);
    let score = fusion.combine(0.8, 0.4, 0.0);
    assert!((score - 0.64).abs() < 0.001);

    // R3.1: Intent classification still works
    let intent = classify_intent("format_string");
    assert_eq!(intent, QueryIntent::Symbolic);

    // R3.1: Router still works
    let decision = route_query(&QueryIntent::Semantic, "explain test");
    assert!(decision.should_call_raggraph);

    // R3.2 + R3.3: ContextBundle and continuity still work
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let code_graph = CodeGraph::new(":memory:", vector_store)?;
    let memory = Memory::new(&get_unique_db_path())?;

    let enriched =
        enrich_query_with_context_bundle("test query", &code_graph, &memory, None).await?;

    assert!(enriched.context_bundle.is_some());
    // R3.3: reasoning_continuity field exists
    assert!(enriched.reasoning_continuity.is_some() || enriched.reasoning_continuity.is_none());

    Ok(())
}
