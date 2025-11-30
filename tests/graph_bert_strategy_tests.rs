//! TDD Tests for Graph-BERT Strategy Implementation (APEX 2.x-GB)
//!
//! These tests define the contract for GraphBertModel before implementation.
//! All tests should FAIL initially, then PASS after PHASE C implementation.
//!
//! Test Coverage:
//! 1. GraphBertModel loads ONNX model successfully
//! 2. Embedding dimensions match GRAPH domain config (384 dims)
//! 3. Embeddings are deterministic (same input → same output)
//! 4. Graph features influence embedding output
//! 5. GraphEmbeddingService uses GraphBertModel when enabled

use syncore::code_graph::graph_embeddings::{GraphEmbeddingStrategy, GraphFeatures};

// ============================================================================
// PHASE B: TDD Test Cases (Written BEFORE Implementation)
// ============================================================================

#[test]
fn test_graph_bert_strategy_loads_model() {
    // Test that GraphBertModel can be instantiated with ONNX model
    //
    // Expected behavior:
    // - GraphBertModel::new() succeeds if ONNX model file exists
    // - Returns error if model not found or corrupted
    // - Loads model into memory for inference

    use syncore::code_graph::graph_bert::GraphBertModel;

    let model = GraphBertModel::new();
    assert!(model.is_ok(), "GraphBertModel should load successfully");
}

#[test]
fn test_graph_bert_strategy_embedding_dimension_matches_config() {
    // Test that GraphBertModel respects GRAPH domain dimension (384)
    //
    // Expected behavior:
    // - embed_with_graph() returns Vec<f32> with length 384
    // - Matches EmbeddingDomain::Graph.default_dimension()

    use syncore::code_graph::graph_bert::GraphBertModel;

    let model = GraphBertModel::new().unwrap();
    let code_emb = vec![0.5; 384];
    let features = GraphFeatures::empty();

    let graph_emb = model.embed_with_graph(&code_emb, &features);
    assert_eq!(graph_emb.len(), 384, "GRAPH domain must use 384 dims");
}

#[test]
fn test_graph_bert_strategy_deterministic_output() {
    // Test that GraphBertModel produces deterministic embeddings
    //
    // Expected behavior:
    // - Same input (code_embedding + graph_features) → same output
    // - No random seed or non-deterministic operations
    // - Critical for reproducible tests and production reliability

    use syncore::code_graph::graph_bert::GraphBertModel;

    let model = GraphBertModel::new().unwrap();
    let code_emb = vec![0.5; 384];
    let mut features = GraphFeatures::empty();
    features.degree_in = 5;
    features.degree_out = 10;

    let emb1 = model.embed_with_graph(&code_emb, &features);
    let emb2 = model.embed_with_graph(&code_emb, &features);

    assert_eq!(emb1, emb2, "Embeddings must be deterministic");
}

#[test]
fn test_graph_bert_strategy_respects_graph_features() {
    // Test that GraphBertModel actually uses graph features in embedding
    //
    // Expected behavior:
    // - Different graph features → different embeddings
    // - degree_in, degree_out, edge_types should influence output
    // - Not just returning code_embedding unchanged

    use syncore::code_graph::graph_bert::GraphBertModel;

    let model = GraphBertModel::new().unwrap();
    let code_emb = vec![0.5; 384];

    // Embedding with no graph features
    let features_empty = GraphFeatures::empty();
    let emb_empty = model.embed_with_graph(&code_emb, &features_empty);

    // Embedding with graph features
    let mut features_rich = GraphFeatures::empty();
    features_rich.degree_in = 10;
    features_rich.degree_out = 20;
    let emb_rich = model.embed_with_graph(&code_emb, &features_rich);

    assert_ne!(emb_empty, emb_rich, "Graph features must influence embedding");
}

#[test]
fn test_graph_embedding_service_uses_graph_bert_when_enabled() {
    // Test that GraphBertModel can be used via the GraphEmbeddingStrategy trait
    //
    // Expected behavior:
    // - GraphBertModel implements GraphEmbeddingStrategy
    // - Can be used interchangeably with SimpleFeatureCombiner
    // - GRAPH domain routing remains intact

    use syncore::code_graph::graph_bert::GraphBertModel;
    use syncore::code_graph::graph_embeddings::GraphEmbeddingStrategy;
    use syncore::vector::domain::{EmbeddingConfig, EmbeddingDomain};

    // Verify GRAPH domain config
    let graph_config = EmbeddingConfig::for_graph();
    assert_eq!(graph_config.dimension, 384);
    assert_eq!(graph_config.domain, EmbeddingDomain::Graph);

    // Verify GraphBertModel implements GraphEmbeddingStrategy
    let graph_bert: Box<dyn GraphEmbeddingStrategy> = Box::new(GraphBertModel::new().unwrap());
    let code_emb = vec![0.5; 384];
    let features = GraphFeatures::empty();

    let graph_emb = graph_bert.embed_with_graph(&code_emb, &features);
    assert_eq!(graph_emb.len(), 384, "GraphBertModel produces 384-dim embeddings");
}

// ============================================================================
// Backward Compatibility Tests (Must NOT Regress)
// ============================================================================

#[test]
fn test_simple_feature_combiner_still_works() {
    // Ensure SimpleFeatureCombiner continues working after Graph-BERT addition
    //
    // This test should PASS immediately (no implementation needed)
    use syncore::code_graph::graph_embeddings::SimpleFeatureCombiner;

    let combiner = SimpleFeatureCombiner;
    let code_emb = vec![0.5; 384];
    let features = GraphFeatures::empty();

    let graph_emb = combiner.embed_with_graph(&code_emb, &features);
    assert_eq!(graph_emb.len(), 384, "SimpleFeatureCombiner must still work");
}

#[test]
fn test_code_and_general_domains_unchanged() {
    // Ensure CODE and GENERAL domains are NOT affected by Graph-BERT
    //
    // This test should PASS immediately (APEX 2.0-E behavior preserved)
    use syncore::vector::domain::{EmbeddingDomain, EmbeddingConfig};

    let code_config = EmbeddingConfig::for_code();
    let general_config = EmbeddingConfig::for_general();

    // APEX 2.0-E: Both use BGE-M3 (1024 dims)
    assert_eq!(code_config.model_name, "bge-m3");
    assert_eq!(code_config.dimension, 1024);

    assert_eq!(general_config.model_name, "bge-m3");
    assert_eq!(general_config.dimension, 1024);

    // GRAPH domain unchanged: all-MiniLM-L6-v2 (384 dims)
    let graph_config = EmbeddingConfig::for_graph();
    assert_eq!(graph_config.model_name, "all-MiniLM-L6-v2");
    assert_eq!(graph_config.dimension, 384);
}
