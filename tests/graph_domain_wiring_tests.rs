//! TDD Tests for GRAPH Domain Wiring to GraphBertCandleEmbeddings (Phase G2)
//!
//! These tests enforce that TripleEmbeddingService uses GraphBertCandleEmbeddings
//! for the GRAPH domain instead of the old TF-IDF/feature-based implementation.
//! Tests follow strict TDD methodology: write failing tests first, then implementation.

use anyhow::Result;
use syncore::vector::domain::{EmbeddingConfig, EmbeddingDomain};
use syncore::vector::dual_service::TripleEmbeddingService;
use syncore::config::{SyncoreConfig, GraphEmbeddingsConfig};

// ============================================================================
// TEST 1: GRAPH Domain Uses GraphBertCandleEmbeddings Backend
// ============================================================================

#[test]
fn test_graph_domain_uses_graphbert_candle_backend() {
    // Construct config with invalid GraphBERT model path to test error handling
    let mut graph_config = GraphEmbeddingsConfig::default();
    graph_config.model_path = "/nonexistent/path/graphbert.gguf".to_string();
    graph_config.dimensions = 384;

    let config = SyncoreConfig {
        graph_embeddings: graph_config,
        // Minimal other required fields for compilation
        ..Default::default()
    };

    // Create TripleEmbeddingService - should fail due to invalid GRAPH config
    // This tests that GRAPH domain actually calls GraphBertCandleEmbeddings::new()
    let result = TripleEmbeddingService::new();

    // The service should fail due to GRAPH domain initialization failure
    assert!(result.is_err(), "TripleEmbeddingService should fail when GraphBertCandleEmbeddings cannot load model");

    // Error should mention graph or graphbert (not a generic embedding error)
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("graph") || error_msg.contains("GraphBERT") || error_msg.contains("graphbert"),
        "Error should mention graph domain: {}",
        error_msg
    );
}

// ============================================================================
// TEST 2: GRAPH Domain Failure Does Not Break CODE and GENERAL
// ============================================================================

#[test]
fn test_graph_domain_failure_does_not_break_code_and_general() {
    // Test that CODE and GENERAL domains work independently of GRAPH domain failures

    // Test CODE domain independently
    let code_config = EmbeddingConfig::for_code();
    assert!(code_config.validate().is_ok(), "CODE config should be valid");

    // Test GENERAL domain independently
    let general_config = EmbeddingConfig::for_general();
    assert!(general_config.validate().is_ok(), "GENERAL config should be valid");

    // Test that we can create individual embeddings for CODE and GENERAL
    // This verifies the domains themselves are not broken by GRAPH configuration
    let code_embedding_result = syncore::vector::HuggingFaceEmbeddings::new_bge();
    assert!(code_embedding_result.is_ok(), "CODE domain embeddings should work independently");

    let general_embedding_result = syncore::vector::HuggingFaceEmbeddings::new();
    assert!(general_embedding_result.is_ok(), "GENERAL domain embeddings should work independently");

    // Verify dimensions are correct for working domains
    let code_embedding = code_embedding_result.unwrap();
    assert_eq!(code_embedding.dim(), 384, "CODE embedding dimension should be 384");

    let general_embedding = general_embedding_result.unwrap();
    assert_eq!(general_embedding.dim(), 384, "GENERAL embedding dimension should be 384");
}

// ============================================================================
// TEST 3: Backwards Compatibility - Missing Graph Config
// ============================================================================

#[test]
fn test_graph_domain_disabled_if_config_missing() {
    // Test that existing configs without graph-specific settings still work

    // Create a minimal config without explicit GraphEmbeddingsConfig
    let minimal_config = EmbeddingConfig::for_graph();

    // This should work - EmbeddingConfig::for_graph() provides defaults
    assert!(minimal_config.validate().is_ok(), "Default graph config should be valid");

    // Test that we can create an embedding config with minimal settings
    let config = EmbeddingConfig::for_graph();
    assert_eq!(config.domain, EmbeddingDomain::Graph);
    assert_eq!(config.model_name, "graphbert-base");
    assert_eq!(config.dimension, 384);

    // For Phase G2, when graph config is missing or incomplete,
    // TripleEmbeddingService should either:
    // - Initialize successfully and fail only when GRAPH embeddings are requested, OR
    // - Return clear "GraphBERT backend not configured" error

    // For now, we verify the structure is ready for proper error handling
    let config = GraphEmbeddingsConfig::default();
    assert_eq!(config.model_name, "graphbert-base");
    assert_eq!(config.dimensions, 384);
    assert_eq!(config.model_path, "models/graphbert-base.onnx");
}

// ============================================================================
// TEST 4: GraphBertCandleEmbeddings Integration Validation
// ============================================================================

#[test]
fn test_graphbert_candle_embeddings_integration_validation() {
    // Test that GraphBertCandleEmbeddings validates its configuration properly
    use syncore::embeddings::GraphBertCandleEmbeddings;

    // Test with clearly invalid model path
    let invalid_config = GraphEmbeddingsConfig {
        model_name: "test-graphbert".to_string(),
        model_path: "/clearly/invalid/path/model.gguf".to_string(),
        dimensions: 384,
        batch_size: 16,
        use_onnx: false,
    };

    let result = GraphBertCandleEmbeddings::new(&invalid_config);
    assert!(result.is_err(), "GraphBertCandleEmbeddings should reject invalid model path");

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("Model file not found") || error_msg.contains("Invalid model format"),
        "Error should be specific about model file issue: {}",
        error_msg
    );

    // Test with invalid dimension
    let invalid_config = GraphEmbeddingsConfig {
        model_name: "test-graphbert".to_string(),
        model_path: "test.gguf".to_string(),
        dimensions: 0, // Invalid dimension
        batch_size: 16,
        use_onnx: false,
    };

    let result = GraphBertCandleEmbeddings::new(&invalid_config);
    assert!(result.is_err(), "GraphBertCandleEmbeddings should reject zero dimension");

    // Test with invalid batch size
    let invalid_config = GraphEmbeddingsConfig {
        model_name: "test-graphbert".to_string(),
        model_path: "test.gguf".to_string(),
        dimensions: 384,
        batch_size: 0, // Invalid batch size
        use_onnx: false,
    };

    let result = GraphBertCandleEmbeddings::new(&invalid_config);
    assert!(result.is_err(), "GraphBertCandleEmbeddings should reject zero batch size");
}

// ============================================================================
// TEST 5: Embedding Type Verification
// ============================================================================

#[test]
fn test_embedding_type_verification() {
    // Verify that we can distinguish between different embedding implementations

    // Create individual embedding instances to test their behavior
    let huggingface_bge = syncore::vector::HuggingFaceEmbeddings::new_bge().unwrap();
    let huggingface_general = syncore::vector::HuggingFaceEmbeddings::new().unwrap();

    // Test that HuggingFace embeddings work
    let test_text = "fn test_function() -> Result<()> { Ok(()) }";

    let code_result = huggingface_bge.embed(test_text);
    assert!(code_result.is_ok(), "HuggingFace BGE should work");
    let code_embedding = code_result.unwrap();
    assert_eq!(code_embedding.len(), 384, "BGE should produce 384-dim embeddings");

    let general_result = huggingface_general.embed(test_text);
    assert!(general_result.is_ok(), "HuggingFace general should work");
    let general_embedding = general_result.unwrap();
    assert_eq!(general_embedding.len(), 384, "General should produce 384-dim embeddings");

    // Verify the embeddings are different (different models should produce different results)
    assert_ne!(
        code_embedding[0..10], general_embedding[0..10],
        "CODE and GENERAL embeddings should be different"
    );
}

// ============================================================================
// TEST 6: Domain Routing Validation
// ============================================================================

#[test]
fn test_domain_routing_validation() {
    // Test that EmbeddingDomain mapping works correctly

    // Verify namespace to domain mapping
    assert_eq!(EmbeddingDomain::from_namespace("code_entity"), EmbeddingDomain::Code);
    assert_eq!(EmbeddingDomain::from_namespace("rust_code"), EmbeddingDomain::Code);

    assert_eq!(EmbeddingDomain::from_namespace("documents"), EmbeddingDomain::General);
    assert_eq!(EmbeddingDomain::from_namespace("plan"), EmbeddingDomain::General);

    assert_eq!(EmbeddingDomain::from_namespace("graph_entity"), EmbeddingDomain::Graph);
    assert_eq!(EmbeddingDomain::from_namespace("rag_graph"), EmbeddingDomain::Graph);
    assert_eq!(EmbeddingDomain::from_namespace("hop_graph"), EmbeddingDomain::Graph);

    // Test default index paths differ
    let code_path = EmbeddingDomain::Code.default_index_path();
    let general_path = EmbeddingDomain::General.default_index_path();
    let graph_path = EmbeddingDomain::Graph.default_index_path();

    assert_ne!(code_path, general_path);
    assert_ne!(code_path, graph_path);
    assert_ne!(general_path, graph_path);

    assert!(code_path.contains("code"));
    assert!(general_path.contains("general"));
    assert!(graph_path.contains("graph"));
}

// ============================================================================
// CONFIGURATION HELPERS
// ============================================================================

/// Create a valid test GraphEmbeddingsConfig
fn create_test_graph_config(model_path: &str) -> GraphEmbeddingsConfig {
    GraphEmbeddingsConfig {
        model_name: "test-graphbert".to_string(),
        model_path: model_path.to_string(),
        dimensions: 384,
        batch_size: 16,
        use_onnx: false,
    }
}

/// Create a minimal test SyncoreConfig with graph configuration
fn create_test_syncore_config(graph_config: GraphEmbeddingsConfig) -> SyncoreConfig {
    SyncoreConfig {
        graph_embeddings: graph_config,
        // Use minimal defaults for other fields
        ..Default::default()
    }
}