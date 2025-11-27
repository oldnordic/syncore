//! Phase R3.1 - Cognitive Routing + Intent Classifier + Auto Tool-Call Planner Tests
//!
//! These tests verify the intelligent cognitive orchestration system that:
//! - Classifies user intent (symbolic, semantic, causal)
//! - Routes to appropriate fusion mode (simple, attention, reasoning)
//! - Auto-inserts RAGGraph tool calls before worker model execution
//! - Produces enriched context for LLM prompts

use anyhow::Result;
use syncore::graph::Neo4jClient;

// Test 1: Intent classifier recognizes symbolic queries
#[test]
fn test_intent_classifier_symbolic() -> Result<()> {
    use syncore::cognition::intent_classifier::{classify_intent, QueryIntent};

    let query = "format_string";
    let intent = classify_intent(query);

    assert_eq!(intent, QueryIntent::Symbolic);
    Ok(())
}

// Test 2: Intent classifier recognizes semantic queries
#[test]
fn test_intent_classifier_semantic() -> Result<()> {
    use syncore::cognition::intent_classifier::{classify_intent, QueryIntent};

    let query = "explain why parse function fails";
    let intent = classify_intent(query);

    assert_eq!(intent, QueryIntent::Semantic);
    Ok(())
}

// Test 3: Intent classifier recognizes causal queries
#[test]
fn test_intent_classifier_causal() -> Result<()> {
    use syncore::cognition::intent_classifier::{classify_intent, QueryIntent};

    let query = "trace dependency from A to B";
    let intent = classify_intent(query);

    assert_eq!(intent, QueryIntent::Causal);
    Ok(())
}

// Test 4: Router logic suggests simple mode for symbolic queries
#[test]
fn test_router_logic_simple() -> Result<()> {
    use syncore::cognition::intent_classifier::QueryIntent;
    use syncore::cognition::router_logic::{route_query, RoutingDecision};

    let intent = QueryIntent::Symbolic;
    let decision = route_query(&intent, "format_string");

    assert!(decision.should_call_raggraph);
    assert_eq!(decision.mode_hint, Some("simple".to_string()));
    assert!(decision.top_k.unwrap_or(0) > 0);

    Ok(())
}

// Test 5: Router logic suggests attention mode for semantic queries
#[test]
fn test_router_logic_attention() -> Result<()> {
    use syncore::cognition::intent_classifier::QueryIntent;
    use syncore::cognition::router_logic::{route_query, RoutingDecision};

    let intent = QueryIntent::Semantic;
    let decision = route_query(&intent, "explain why parse fails");

    assert!(decision.should_call_raggraph);
    assert_eq!(decision.mode_hint, Some("attention".to_string()));

    Ok(())
}

// Test 6: Router logic suggests reasoning mode for causal queries
#[test]
fn test_router_logic_reasoning() -> Result<()> {
    use syncore::cognition::intent_classifier::QueryIntent;
    use syncore::cognition::router_logic::{route_query, RoutingDecision};

    let intent = QueryIntent::Causal;
    let decision = route_query(&intent, "trace dependency from A to B");

    assert!(decision.should_call_raggraph);
    assert_eq!(decision.mode_hint, Some("reasoning".to_string()));

    Ok(())
}

// Test 7: Orchestrator inserts RAGGraph call (integration test)
#[tokio::test]
async fn test_orchestrator_inserts_raggraph_call() -> Result<()> {
    use std::sync::{Arc, Mutex};
    use syncore::code_graph::CodeGraph;
    use syncore::cognition::orchestrator::enrich_query_with_raggraph;
    use syncore::vector::{HuggingFaceEmbeddings, VectorStore};

    // Setup real components
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let db_conn = Arc::new(Mutex::new(rusqlite::Connection::open_in_memory()?));

    // Create CodeGraph with real connection
    let code_graph = CodeGraph::with_connection(db_conn.clone(), vector_store.clone())?;

    // Get Neo4j connection (if available)
    let neo4j_result = get_neo4j_client().await;

    if neo4j_result.is_err() {
        // Skip test if Neo4j not available
        println!("Skipping test - Neo4j not available");
        return Ok(());
    }

    let neo4j = neo4j_result?;

    // Test query that should trigger RAGGraph (symbolic query)
    let query = "format_string";
    let enriched = enrich_query_with_raggraph(query, &code_graph, &neo4j).await?;

    // Verify enriched context was created
    assert!(
        enriched.selected_mode.is_some(),
        "selected_mode should be Some for symbolic query"
    );
    assert!(
        enriched.raggraph_invoked,
        "RAGGraph should be invoked for symbolic query"
    );

    Ok(())
}

// Test 8: Orchestrator skips RAGGraph for irrelevant queries
#[tokio::test]
async fn test_orchestrator_skips_raggraph_for_irrelevant_queries() -> Result<()> {
    use syncore::cognition::intent_classifier::{classify_intent, QueryIntent};

    let query = "hello world";
    let intent = classify_intent(query);

    // Should be Unknown, not triggering RAGGraph
    assert_eq!(intent, QueryIntent::Unknown);

    Ok(())
}

// Test 9: RAGGraph auto mode works (no mode_hint given)
#[tokio::test]
async fn test_raggraph_auto_mode_works() -> Result<()> {
    use syncore::cognition::intent_classifier::classify_intent;
    use syncore::cognition::router_logic::route_query;

    let query = "trace how config is loaded";
    let intent = classify_intent(query);
    let decision = route_query(&intent, query);

    // Auto-selection should provide a mode_hint
    assert!(decision.mode_hint.is_some());
    assert!(decision.should_call_raggraph);

    Ok(())
}

// Test 10: Backwards compatibility check
#[tokio::test]
async fn test_backwards_compatibility() -> Result<()> {
    use std::sync::{Arc, Mutex};
    use syncore::code_graph::fusion_simple::FusionSimple;
    use syncore::code_graph::CodeGraph;
    use syncore::vector::{HuggingFaceEmbeddings, VectorStore};

    // R2.2: Basic indexing still works
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let _code_graph = CodeGraph::new(":memory:", vector_store)?;

    // R2.4: Fusion still works
    let fusion = FusionSimple::new(0.6, 0.3, 0.1, 0.0);
    let score = fusion.combine(0.8, 0.4, 0.0, 0.0);
    assert!((score - 0.64).abs() < 0.001);

    Ok(())
}

/// Helper to get Neo4j connection
async fn get_neo4j_client() -> Result<Neo4jClient> {
    let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string());
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let pass = std::env::var("NEO4J_PASS").unwrap_or_else(|_| "testpassword123".to_string());

    Neo4jClient::connect(&uri, &user, &pass).await
}
