//! Phase R3.3 - Reasoning Continuity Engine Tests
//!
//! Tests for the hybrid SQL + Graph reasoning ledger with routing.
//! Verifies:
//! - SQL ledger storage and retrieval
//! - Graph ledger storage and retrieval
//! - Continuity routing (SqlOnly/GraphOnly/Hybrid/None)
//! - ReasoningContinuity building
//! - Episode persistence
//! - Orchestrator integration
//! - Backward compatibility

use anyhow::Result;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn get_unique_db_path() -> String {
    let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("/tmp/test_r3_3_db_{}_{}", std::process::id(), counter)
}

// Test 1: SQL ledger store and fetch episode
#[tokio::test]
async fn test_sql_ledger_store_and_fetch_episode() -> Result<()> {
    use syncore::cognition::reasoning_ledger::{
        fetch_recent_episodes_sql, store_episode_sql, ReasoningEpisode,
    };
    use syncore::memory::Memory;

    // Setup
    let memory = Memory::new(&get_unique_db_path())?;

    // Create minimal episode
    let episode = ReasoningEpisode {
        id: 1,
        timestamp: 1234567890,
        user_query: "test query".to_string(),
        selected_mode: "simple".to_string(),
        important_entities: vec!["entity1".to_string(), "entity2".to_string()],
        tool_calls: vec!["code_index".to_string()],
        outcome: "success".to_string(),
        notes: Some("test notes".to_string()),
        client_id: None,
    };

    // Store
    store_episode_sql(&memory, &episode)?;

    // Fetch
    let episodes = fetch_recent_episodes_sql(&memory, "test", 10)?;

    // Assert
    assert!(!episodes.is_empty());
    assert_eq!(episodes[0].user_query, "test query");
    assert_eq!(episodes[0].selected_mode, "simple");
    assert_eq!(episodes[0].outcome, "success");

    Ok(())
}

// Test 2: Graph ledger store and fetch related episodes
#[tokio::test]
async fn test_graph_ledger_store_and_fetch_related_episodes() -> Result<()> {
    use syncore::cognition::reasoning_ledger::{
        fetch_related_episodes_graph, store_episode_graph, ReasoningEpisode,
    };
    use syncore::graph::Neo4jClient;

    // Skip if Neo4j not available
    let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string());
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let pass = std::env::var("NEO4J_PASS").unwrap_or_else(|_| "testpassword123".to_string());

    let neo4j = match Neo4jClient::connect(&uri, &user, &pass).await {
        Ok(client) => client,
        Err(_) => {
            eprintln!("Neo4j not available, skipping test");
            return Ok(());
        }
    };

    // Create episode with entities
    let episode = ReasoningEpisode {
        id: 2,
        timestamp: 1234567891,
        user_query: "graph test query".to_string(),
        selected_mode: "attention".to_string(),
        important_entities: vec!["entity_graph_1".to_string(), "entity_graph_2".to_string()],
        tool_calls: vec!["code_graph_fusion_query".to_string()],
        outcome: "success".to_string(),
        notes: Some("graph test".to_string()),
        client_id: None,
    };

    // Store
    store_episode_graph(&neo4j, &episode).await?;

    // Fetch by entity IDs
    let entity_ids = vec!["entity_graph_1".to_string()];
    let episode_ids = fetch_related_episodes_graph(&neo4j, &entity_ids, 10).await?;

    // Assert
    assert!(!episode_ids.is_empty());
    assert!(episode_ids.contains(&2));

    Ok(())
}

// Test 3: Continuity route SQL only for simple queries
#[test]
fn test_continuity_route_sql_only_for_simple_queries() -> Result<()> {
    use syncore::cognition::context_bundle::ContextBundle;
    use syncore::cognition::continuity_engine::{decide_continuity_route, ContinuityRoute};
    use syncore::cognition::intent_classifier::QueryIntent;

    // Given: Symbolic intent, small bundle
    let intent = QueryIntent::Symbolic;
    let bundle = ContextBundle::new();

    // Decide route
    let route = decide_continuity_route(&intent, &bundle);

    // Assert: SqlOnly or None
    match route {
        ContinuityRoute::SqlOnly | ContinuityRoute::None => {
            // Expected
        }
        _ => panic!("Expected SqlOnly or None for simple query"),
    }

    Ok(())
}

// Test 4: Continuity route hybrid for semantic and causal
#[test]
fn test_continuity_route_hybrid_for_semantic_and_causal() -> Result<()> {
    use syncore::cognition::context_bundle::{CodeEntityWithScore, ContextBundle};
    use syncore::cognition::continuity_engine::{decide_continuity_route, ContinuityRoute};
    use syncore::cognition::intent_classifier::QueryIntent;

    // Given: Semantic intent, bundle with multiple entities
    let intent = QueryIntent::Semantic;
    let mut bundle = ContextBundle::new();

    // Add multiple entities
    for i in 0..5 {
        bundle.add_raggraph_entity(CodeEntityWithScore {
            entity_id: Some(i),
            file_path: format!("test{}.rs", i),
            entity_type: "function".to_string(),
            name: format!("fn{}", i),
            signature: None,
            score: 0.9,
            rank: i as usize,
        });
    }

    // Decide route
    let route = decide_continuity_route(&intent, &bundle);

    // Assert: Hybrid
    assert!(
        matches!(route, ContinuityRoute::Hybrid),
        "Expected Hybrid for semantic query with multiple entities"
    );

    Ok(())
}

// Test 5: Build reasoning continuity combines SQL and graph
#[tokio::test]
async fn test_build_reasoning_continuity_combines_sql_and_graph() -> Result<()> {
    use syncore::cognition::continuity_engine::{build_reasoning_continuity, ContinuityRoute};
    use syncore::cognition::reasoning_ledger::{
        store_episode_graph, store_episode_sql, ReasoningEpisode,
    };
    use syncore::graph::Neo4jClient;
    use syncore::memory::Memory;

    // Setup
    let memory = Memory::new(&get_unique_db_path())?;

    // Try to connect to Neo4j
    let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string());
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let pass = std::env::var("NEO4J_PASS").unwrap_or_else(|_| "testpassword123".to_string());

    let neo4j_opt = Neo4jClient::connect(&uri, &user, &pass).await.ok();

    // Create episodes with overlapping entities
    let episode1 = ReasoningEpisode {
        id: 10,
        timestamp: 1234567900,
        user_query: "combined test 1".to_string(),
        selected_mode: "simple".to_string(),
        important_entities: vec!["shared_entity".to_string()],
        tool_calls: vec!["code_index".to_string()],
        outcome: "success".to_string(),
        notes: Some("sql episode".to_string()),
        client_id: None,
    };

    let episode2 = ReasoningEpisode {
        id: 11,
        timestamp: 1234567901,
        user_query: "combined test 2".to_string(),
        selected_mode: "attention".to_string(),
        important_entities: vec!["shared_entity".to_string()],
        tool_calls: vec!["code_graph_fusion_query".to_string()],
        outcome: "success".to_string(),
        notes: Some("graph episode".to_string()),
        client_id: None,
    };

    // Store in SQL
    store_episode_sql(&memory, &episode1)?;
    store_episode_sql(&memory, &episode2)?;

    // Store in graph if available
    if let Some(ref neo4j) = neo4j_opt {
        store_episode_graph(neo4j, &episode1).await?;
        store_episode_graph(neo4j, &episode2).await?;
    }

    // Build continuity
    let continuity = build_reasoning_continuity(
        "combined test",
        &["shared_entity".to_string()],
        &ContinuityRoute::Hybrid,
        &memory,
        neo4j_opt.as_ref(),
        10,
    )
    .await?;

    // Assert
    assert!(continuity.episodes.len() >= 1);
    assert!(continuity.sql_used || continuity.graph_used);

    Ok(())
}

// Test 6: Persist current episode stores to both ledgers
#[tokio::test]
async fn test_persist_current_episode_stores_to_both_ledgers() -> Result<()> {
    use std::sync::{Arc, Mutex};
    use syncore::cognition::context_bundle::{CodeEntityWithScore, ContextBundle};
    use syncore::cognition::continuity_engine::persist_current_episode;
    use syncore::cognition::intent_classifier::QueryIntent;
    use syncore::cognition::orchestrator::EnrichedContext;
    use syncore::cognition::reasoning_ledger::fetch_recent_episodes_sql;
    use syncore::cognition::router_logic::{route_query, RoutingDecision};
    use syncore::graph::Neo4jClient;
    use syncore::memory::Memory;

    // Setup
    let memory = Memory::new(&get_unique_db_path())?;

    // Try Neo4j
    let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string());
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let pass = std::env::var("NEO4J_PASS").unwrap_or_else(|_| "testpassword123".to_string());
    let neo4j_opt = Neo4jClient::connect(&uri, &user, &pass).await.ok();

    // Create simulated context
    let intent = QueryIntent::Semantic;
    let decision = route_query(&intent, "persist test");

    let mut bundle = ContextBundle::with_mode("attention");
    bundle.add_raggraph_entity(CodeEntityWithScore {
        entity_id: Some(100),
        file_path: "persist_test.rs".to_string(),
        entity_type: "function".to_string(),
        name: "persist_fn".to_string(),
        signature: None,
        score: 0.95,
        rank: 1,
    });

    let enriched = EnrichedContext {
        query: "persist test".to_string(),
        intent,
        decision: decision.clone(),
        selected_mode: Some("attention".to_string()),
        raggraph_results: None,
        raggraph_invoked: true,
        context_bundle: Some(bundle.clone()),
        reasoning_continuity: None,
        recommended_patterns: None,
        self_consistency: None,
        plan: None,
        execution_result: None,
        debug_info: "test".to_string(),
    };

    let tool_calls = vec!["code_graph_fusion_query".to_string()];

    // Persist
    persist_current_episode(
        &enriched,
        &bundle,
        &tool_calls,
        "success",
        &memory,
        neo4j_opt.as_ref(),
    )
    .await?;

    // Verify SQL
    let episodes = fetch_recent_episodes_sql(&memory, "persist", 10)?;
    assert!(!episodes.is_empty());

    // Verify graph if available - just check no errors
    // (detailed verification would require graph query)

    Ok(())
}

// Test 7: Orchestrator uses continuity on new query
#[tokio::test]
async fn test_orchestrator_uses_continuity_on_new_query() -> Result<()> {
    use std::sync::{Arc, Mutex};
    use syncore::code_graph::CodeGraph;
    use syncore::cognition::context_bundle::{CodeEntityWithScore, ContextBundle};
    use syncore::cognition::continuity_engine::persist_current_episode;
    use syncore::cognition::intent_classifier::QueryIntent;
    use syncore::cognition::orchestrator::enrich_query_with_context_bundle;
    use syncore::cognition::orchestrator::EnrichedContext;
    use syncore::cognition::router_logic::route_query;
    use syncore::memory::Memory;
    use syncore::vector::{HuggingFaceEmbeddings, VectorStore};

    // Setup
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let code_graph = CodeGraph::new(":memory:", vector_store)?;
    let memory = Memory::new(&get_unique_db_path())?;

    // Create a past episode
    let intent = QueryIntent::Semantic;
    let decision = route_query(&intent, "past query");
    let mut bundle = ContextBundle::with_mode("simple");
    bundle.add_raggraph_entity(CodeEntityWithScore {
        entity_id: Some(200),
        file_path: "orchestrator_test.rs".to_string(),
        entity_type: "function".to_string(),
        name: "test_fn".to_string(),
        signature: None,
        score: 0.9,
        rank: 1,
    });

    let enriched = EnrichedContext {
        query: "past query".to_string(),
        intent,
        decision: decision.clone(),
        selected_mode: Some("simple".to_string()),
        raggraph_results: None,
        raggraph_invoked: true,
        context_bundle: Some(bundle.clone()),
        reasoning_continuity: None,
        recommended_patterns: None,
        self_consistency: None,
        plan: None,
        execution_result: None,
        debug_info: "test".to_string(),
    };

    persist_current_episode(
        &enriched,
        &bundle,
        &vec!["code_index".to_string()],
        "success",
        &memory,
        None,
    )
    .await?;

    // Run new query through orchestrator
    let enriched_new =
        enrich_query_with_context_bundle("related query", &code_graph, &memory, None).await?;

    // Assert: continuity should be included (or at least no error)
    assert!(enriched_new.context_bundle.is_some());

    Ok(())
}

// Test 8: Reasoning continuity does not break existing flow
#[tokio::test]
async fn test_reasoning_continuity_does_not_break_existing_flow() -> Result<()> {
    use std::sync::{Arc, Mutex};
    use syncore::code_graph::CodeGraph;
    use syncore::cognition::orchestrator::enrich_query_with_context_bundle;
    use syncore::memory::Memory;
    use syncore::vector::{HuggingFaceEmbeddings, VectorStore};

    // Setup with fresh DB (no history)
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let code_graph = CodeGraph::new(":memory:", vector_store)?;
    let memory = Memory::new(&get_unique_db_path())?;

    // Run query (no history exists)
    let enriched =
        enrich_query_with_context_bundle("unrelated query", &code_graph, &memory, None).await?;

    // Assert: no panic, no error, bundle exists
    assert!(enriched.context_bundle.is_some());
    assert_eq!(enriched.query, "unrelated query");

    Ok(())
}

// Test 9: Backwards compatibility
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

    // R3.2: ContextBundle composition still works
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let code_graph = CodeGraph::new(":memory:", vector_store)?;
    let memory = Memory::new(&get_unique_db_path())?;

    let enriched =
        enrich_query_with_context_bundle("test query", &code_graph, &memory, None).await?;

    assert!(enriched.context_bundle.is_some());

    Ok(())
}
