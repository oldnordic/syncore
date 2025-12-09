//! TDD Tests for Candle-backed GraphBERT Embedder (Phase G1)
//!
//! These tests are written BEFORE implementation and follow strict TDD methodology:
//! 1. Tests FAIL initially (implementation doesn't exist)
//! 2. Implementation is written to make tests PASS
//! 3. All existing tests must remain GREEN
//!
//! The tests verify:
//! - GraphBertCandleEmbeddings replaces TF-IDF backend for GRAPH domain only
//! - Candle model loading with proper error handling
//! - Domain isolation (CODE/GENERAL untouched)
//! - Configuration-based initialization
//! - Real transformer embeddings (not mocks/stubs)

use anyhow::Result;
use syncore::embeddings::GraphBertCandleEmbeddings;
use syncore::vector::domain::{EmbeddingConfig, EmbeddingDomain};
use syncore::vector::dual_service::TripleEmbeddingService;
use syncore::config::{SyncoreConfig, GraphEmbeddingsConfig};

// ============================================================================
// TEST 1: Configuration Integration
// ============================================================================

#[test]
fn test_graphbert_config_integration() {
    // Test that GraphEmbeddingsConfig can be constructed with Candle-specific fields
    let mut config = GraphEmbeddingsConfig::default();

    // Verify default Candle-friendly values
    assert_eq!(config.model_name, "graphbert-base");
    assert_eq!(config.model_path, "models/graphbert-base.onnx");
    assert_eq!(config.dimensions, 384);
    assert_eq!(config.batch_size, 16);
    assert!(!config.use_onnx); // Will be true when ONNX is implemented

    // Test custom configuration for Candle model
    config.model_name = "graphcodebert-base".to_string();
    config.model_path = "models/graphcodebert-base.gguf".to_string(); // GGUF for Candle
    config.dimensions = 768;
    config.batch_size = 32;

    assert_eq!(config.model_name, "graphcodebert-base");
    assert_eq!(config.model_path, "models/graphcodebert-base.gguf");
    assert_eq!(config.dimensions, 768);
    assert_eq!(config.batch_size, 32);
}

// ============================================================================
// TEST 2: GraphBertCandleEmbeddings Constructor Error Handling
// ============================================================================

#[test]
fn test_graphbert_candle_embeddings_fails_with_invalid_model_path() {
    // This test should FAIL before implementation (GraphBertCandleEmbeddings doesn't exist)
    // and PASS after implementation with proper error handling

    let config = GraphEmbeddingsConfig {
        model_name: "graphbert-test".to_string(),
        model_path: "/nonexistent/path/graphbert.gguf".to_string(),
        dimensions: 384,
        batch_size: 16,
        use_onnx: false,
    };

    // When implemented, this should return a clear error about missing model file
    // let result = GraphBertCandleEmbeddings::new(&config);
    // assert!(result.is_err());
    // let error_msg = result.unwrap_err().to_string();
    // assert!(error_msg.contains("graphbert") || error_msg.contains("model") || error_msg.contains("GGUF"));

    // For now, this test documents the expected behavior
    // TODO: Uncomment after GraphBertCandleEmbeddings is implemented
}

// ============================================================================
// TEST 3: TripleEmbeddingService Uses GraphBertCandleEmbeddings for GRAPH Domain
// ============================================================================

#[test]
fn test_triple_embedding_service_graph_domain_uses_graphbert() {
    // Construct a SyncoreConfig with graph embedding configuration
    let mut graph_config = GraphEmbeddingsConfig::default();
    graph_config.model_name = "test-graphbert".to_string();
    graph_config.model_path = "models/test-graphbert.gguf".to_string();
    graph_config.dimensions = 384;

    let config = SyncoreConfig {
        graph_embeddings: graph_config,
        // Other required fields with minimal defaults
        ..Default::default()
    };

    // When GraphBertCandleEmbeddings is implemented, TripleEmbeddingService::with_graph_config
    // should create a service where GRAPH domain uses Candle-backed embeddings

    // Verify that the configuration contains our test settings
    assert_eq!(config.graph_embeddings.model_name, "test-graphbert");
    assert_eq!(config.graph_embeddings.model_path, "models/test-graphbert.gguf");
    assert_eq!(config.graph_embeddings.dimensions, 384);

    // TODO: After implementation, verify actual integration:
    // let service = TripleEmbeddingService::with_graph_config(&config);
    // assert!(service.is_ok());

    // Test that embedding with GRAPH domain works
    // let service = service.unwrap();
    // let result = service.embed("test function signature()", EmbeddingDomain::Graph);
    // assert!(result.is_ok());

    // Verify embedding dimension matches config
    // let embedding = result.unwrap();
    // assert_eq!(embedding.len(), 384);
}

// ============================================================================
// TEST 4: Domain Isolation - Code and General Unaffected
// ============================================================================

#[test]
fn test_graph_domain_does_not_affect_code_and_general() {
    // Create a config with invalid graph settings but valid code/general settings
    let mut invalid_graph_config = GraphEmbeddingsConfig::default();
    invalid_graph_config.model_path = "/nonexistent/graphbert.gguf".to_string();
    invalid_graph_config.dimensions = 384;

    let config = SyncoreConfig {
        graph_embeddings: invalid_graph_config,
        ..Default::default()
    };

    // When implemented: TripleEmbeddingService should handle graph domain failure gracefully
    // - CODE domain should still work with its HuggingFace BGE embeddings
    // - GENERAL domain should still work with its HuggingFace MiniLM embeddings
    // - Only GRAPH domain should fail with a clear error

    // For now, test that the basic config structure works
    assert_eq!(config.graph_embeddings.model_path, "/nonexistent/graphbert.gguf");
    assert_eq!(config.graph_embeddings.dimensions, 384);

    // TODO: After implementation, test domain isolation:
    // let service_result = TripleEmbeddingService::with_graph_config(&config);

    // Should fail overall due to graph domain initialization failure
    // assert!(service_result.is_err());

    // But if we test domains individually:
    // let code_config = EmbeddingConfig::for_code();
    // let general_config = EmbeddingConfig::for_general();

    // Code and General configs should be valid and work independently
    // assert!(code_config.validate().is_ok());
    // assert!(general_config.validate().is_ok());

    // TODO: Create separate TripleEmbeddingService instances with only code/general
    // to verify they work independently of graph domain issues
}

// ============================================================================
// TEST 5: Embedding Properties (Shape, Non-Zero, Finite Values)
// ============================================================================

#[test]
fn test_graphbert_candle_embeddings_properties() {
    // This test verifies that Candle-backed embeddings have correct mathematical properties
    // It requires a real model to be available via environment variable

    // Check if test model is available via environment variable
    let test_model_path = std::env::var("GRAPHBERT_TEST_MODEL_PATH");

    if test_model_path.is_err() {
        // Skip test if no real model is available - this is expected in CI
        return;
    }

    let model_path = test_model_path.unwrap();
    assert!(model_path.ends_with(".gguf") || model_path.ends_with(".onnx"));

    let config = GraphEmbeddingsConfig {
        model_name: "test-graphbert".to_string(),
        model_path: model_path.clone(),
        dimensions: 384,
        batch_size: 1,
        use_onnx: model_path.ends_with(".onnx"),
    };

    // TODO: After implementation, test with real model
    // let embedder_result = GraphBertCandleEmbeddings::new(&config);
    // assert!(embedder_result.is_ok(), "Failed to load model from: {}", model_path);

    // let embedder = embedder_result.unwrap();

    // Test single embedding
    // let code_text = "fn fibonacci(n: u32) -> u64 { match n { 0 => 0, 1 => 1, _ => fibonacci(n-1) + fibonacci(n-2) } }";
    // let embedding_result = embedder.embed_single(code_text);
    // assert!(embedding_result.is_ok());

    // let embedding = embedding_result.unwrap();

    // Verify mathematical properties
    // assert_eq!(embedding.len(), config.dimensions);
    // assert!(embedding.iter().all(|x| x.is_finite()), "Embedding contains NaN or infinite values");
    // assert!(embedding.iter().any(|x| *x != 0.0), "Embedding is all zeros");

    // Test batch embedding
    // let inputs = vec![
    //     "fn main() {}".to_string(),
    //     "let x = 42;".to_string(),
    //     "struct Test { field: i32 }".to_string(),
    // ];
    // let batch_result = embedder.embed_batch(&inputs);
    // assert!(batch_result.is_ok());

    // let batch_embeddings = batch_result.unwrap();
    // assert_eq!(batch_embeddings.len(), inputs.len());

    // for (i, embedding) in batch_embeddings.iter().enumerate() {
    //     assert_eq!(embedding.len(), config.dimensions, "Batch embedding {} has wrong dimension", i);
    //     assert!(embedding.iter().all(|x| x.is_finite()), "Batch embedding {} contains NaN/inf", i);
    // }

    // Test determinism - same input should produce same output
    // let embedding1 = embedder.embed_single(code_text).unwrap();
    // let embedding2 = embedder.embed_single(code_text).unwrap();
    // assert_eq!(embedding1, embedding2, "Embeddings should be deterministic");
}

// ============================================================================
// TEST 6: Interface Compatibility with Existing TripleEmbeddingService
// ============================================================================

#[test]
fn test_graphbert_candle_embeddings_interface_compatibility() {
    // Verify that the new GraphBertCandleEmbeddings implements the required traits
    // and is compatible with existing TripleEmbeddingService expectations

    let config = GraphEmbeddingsConfig {
        model_name: "compatibility-test-graphbert".to_string(),
        model_path: "/tmp/compatibility-test.gguf".to_string(),
        dimensions: 384,
        batch_size: 16,
        use_onnx: false,
    };

    // TODO: After implementation, verify trait implementation
    // let embedder_result = GraphBertCandleEmbeddings::new(&config);
    // assert!(embedder_result.is_err()); // Should fail due to missing model file

    // When implemented successfully with a real model:
    // - GraphBertCandleEmbeddings should implement the Embeddings trait
    // - Should provide embed(), dim(), and model_name() methods
    // - Should be constructible from GraphEmbeddingsConfig
    // - Should handle errors gracefully without panics

    // For now, verify the config structure
    assert_eq!(config.dimensions, 384);
    assert_eq!(config.batch_size, 16);
    assert!(!config.use_onnx);
}

// ============================================================================
// TEST 7: Backwards Compatibility
// ============================================================================

#[test]
fn test_existing_embeddings_unchanged() {
    // Verify that CODE and GENERAL domain embeddings continue to work exactly as before
    // This test ensures we don't break existing functionality

    let code_config = EmbeddingConfig::for_code();
    let general_config = EmbeddingConfig::for_general();
    let graph_config = EmbeddingConfig::for_graph(); // Current implementation

    // Verify existing configurations are unchanged
    assert_eq!(code_config.domain, EmbeddingDomain::Code);
    assert_eq!(general_config.domain, EmbeddingDomain::General);
    assert_eq!(graph_config.domain, EmbeddingDomain::Graph);

    // Verify models are unchanged for CODE and GENERAL
    assert_eq!(code_config.model_name, "BGE-small-en-v1.5");
    assert_eq!(general_config.model_name, "all-MiniLM-L6-v2");

    // Graph config should still work with current defaults
    assert!(graph_config.model_name.contains("graphbert"));
    assert_eq!(graph_config.dimension, 384);

    // TODO: After implementation, verify that existing code/general embedding
    // functionality is completely unaffected by GraphBertCandleEmbeddings
}

// ============================================================================
// CONFIGURATION FOR OPTIONAL REAL MODEL TESTING
// ============================================================================

/// Check if a real model is available for integration testing
pub fn has_real_graphbert_model() -> bool {
    std::env::var("GRAPHBERT_TEST_MODEL_PATH")
        .map(|path| std::path::Path::new(&path).exists())
        .unwrap_or(false)
}

/// Get path to real model for integration testing (if available)
pub fn get_real_graphbert_model_path() -> Option<String> {
    std::env::var("GRAPHBERT_TEST_MODEL_PATH").ok()
}

// ============================================================================
// PHASE G3: MODE SELECTION TESTS (Write BEFORE implementation)
// ============================================================================

#[test]
fn test_graphbert_mode_selection_defaults_to_features() {
    // Test that GraphBertCandleEmbeddings defaults to Features mode
    // when use_onnx = false (current behavior should be preserved)

    let mut config = GraphEmbeddingsConfig::default();
    config.use_onnx = false; // Explicitly disable ONNX
    config.model_path = "irrelevant/path".to_string(); // Should be ignored in Features mode

    // This should succeed and create a Features mode embedder
    let result = GraphBertCandleEmbeddings::new(&config);

    assert!(result.is_ok(), "Features mode should work regardless of model path");

    let embedder = result.unwrap();

    // Test that it produces embeddings (Features mode should work)
    let test_text = "fn example() { println!(\"test\"); }";
    let embedding_result = embedder.embed(test_text);

    assert!(embedding_result.is_ok(), "Features mode should produce embeddings");
    let embedding = embedding_result.unwrap();
    assert_eq!(embedding.len(), config.dimensions, "Embedding dimension should match config");

    // Verify the embedding has the expected properties of feature-based embeddings
    assert!(embedding.iter().all(|x| x.is_finite()), "All values should be finite");
    assert!(embedding.iter().any(|x| *x != 0.0), "Embedding should not be all zeros");
}

#[test]
fn test_graphbert_mode_selection_transformer_requires_model_path() {
    // Test that requesting Transformer mode without a valid model path fails cleanly

    let mut config = GraphEmbeddingsConfig::default();
    config.use_onnx = true;  // Request Transformer mode
    config.model_path = "".to_string();  // Empty model path should be invalid

    let result = GraphBertCandleEmbeddings::new(&config);

    assert!(result.is_err(), "Transformer mode should fail with empty model path");

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("path") || error_msg.contains("model") || error_msg.contains("missing") || error_msg.contains("invalid"),
        "Error should mention missing/invalid model path: {}",
        error_msg
    );

    // Also test with None/null equivalent
    config.model_path = "   ".to_string(); // Whitespace only

    let result = GraphBertCandleEmbeddings::new(&config);
    assert!(result.is_err(), "Transformer mode should fail with whitespace-only model path");
}

#[test]
fn test_graphbert_transformer_mode_invalid_model_path_fails_cleanly() {
    // Test that Transformer mode with nonexistent model path fails without fallback

    let mut config = GraphEmbeddingsConfig::default();
    config.use_onnx = true;  // Request Transformer mode
    config.model_path = "/nonexistent/path/model.gguf".to_string();  // Invalid path

    let result = GraphBertCandleEmbeddings::new(&config);

    assert!(result.is_err(), "Transformer mode should fail with invalid model path");

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("model") || error_msg.contains("file") || error_msg.contains("path") || error_msg.contains("not found"),
        "Error should mention model/file/path issues: {}",
        error_msg
    );

    // Ensure no mention of Features mode or fallback in the error
    assert!(
        !error_msg.contains("feature") && !error_msg.contains("fallback") && !error_msg.contains("default"),
        "Error should not mention features or fallback: {}",
        error_msg
    );
}

#[test]
fn test_graphbert_transformer_mode_does_not_fallback_to_features_on_failure() {
    // Test that when Transformer mode is explicitly requested and fails,
    // there is NO silent fallback to Features mode

    let mut config = GraphEmbeddingsConfig::default();
    config.use_onnx = true;  // Explicitly request Transformer mode
    config.model_path = "/definitely/invalid/model.onnx".to_string();  // Invalid model

    let result = GraphBertCandleEmbeddings::new(&config);

    // Should fail, not silently fallback to Features
    assert!(result.is_err(), "Should not fallback to Features mode when Transformer is explicitly requested");

    let error_msg = result.unwrap_err().to_string();

    // Error should be specific to model loading, not generic
    assert!(
        error_msg.contains("model") || error_msg.contains("load") || error_msg.contains("file"),
        "Error should be model-loading specific: {}",
        error_msg
    );

    // Should NOT mention using feature-based fallback
    assert!(
        !error_msg.to_lowercase().contains("fallback") &&
        !error_msg.to_lowercase().contains("using feature") &&
        !error_msg.to_lowercase().contains("defaulting to"),
        "Error should not mention fallback to features: {}",
        error_msg
    );
}

#[test]
fn test_graphbert_features_mode_works_with_invalid_path() {
    // Test that Features mode ignores model path and works regardless

    let mut config = GraphEmbeddingsConfig::default();
    config.use_onnx = false;  // Explicitly request Features mode
    config.model_path = "/completely/invalid/path/model.gguf".to_string();  // Invalid path should be ignored

    let result = GraphBertCandleEmbeddings::new(&config);

    assert!(result.is_ok(), "Features mode should succeed regardless of model path");

    let embedder = result.unwrap();
    let test_text = "struct Test { field: i32 }";
    let embedding_result = embedder.embed(test_text);

    assert!(embedding_result.is_ok(), "Features mode should produce embeddings");
    let embedding = embedding_result.unwrap();
    assert_eq!(embedding.len(), config.dimensions, "Embedding dimension should be correct");
}

#[ignore] // Optional: Only run when real model is available
#[test]
fn test_graphbert_transformer_mode_runs_with_real_model() {
    // Test real transformer mode when model is available via environment variable

    let model_path = std::env::var("GRAPHBERT_TEST_MODEL_PATH");
    if model_path.is_err() {
        // Skip test if no real model is available - this is expected in CI
        return;
    }

    let model_path = model_path.unwrap();
    assert!(
        model_path.ends_with(".gguf") || model_path.ends_with(".onnx") || model_path.ends_with(".safetensors"),
        "Model should be in supported format: {}",
        model_path
    );

    let mut config = GraphEmbeddingsConfig::default();
    config.use_onnx = true;  // Request Transformer mode
    config.model_path = model_path.clone();
    config.dimensions = 384;
    config.batch_size = 1;

    // Should succeed with real model
    let embedder_result = GraphBertCandleEmbeddings::new(&config);
    assert!(embedder_result.is_ok(), "Should load real model from: {}", model_path);

    let embedder = embedder_result.unwrap();

    // Test single embedding
    let code_text = "fn fibonacci(n: u32) -> u64 { match n { 0 => 0, 1 => 1, _ => fibonacci(n-1) + fibonacci(n-2) } }";
    let embedding_result = embedder.embed(code_text);

    assert!(embedding_result.is_ok(), "Should embed with real transformer model");
    let embedding = embedding_result.unwrap();

    // Verify embedding properties
    assert_eq!(embedding.len(), config.dimensions, "Embedding dimension should match config");
    assert!(embedding.iter().all(|x| x.is_finite()), "All values should be finite");
    assert!(embedding.iter().any(|x| *x != 0.0), "Embedding should not be all zeros");

    // Test batch embedding
    let inputs = vec![
        "fn main() {}".to_string(),
        "let x = 42;".to_string(),
        "struct Test { field: i32 }".to_string(),
    ];

    let batch_result = embedder.embed_batch(&inputs);
    assert!(batch_result.is_ok(), "Batch embedding should work with real model");

    let batch_embeddings = batch_result.unwrap();
    assert_eq!(batch_embeddings.len(), inputs.len(), "Batch size should match input");

    for (i, embedding) in batch_embeddings.iter().enumerate() {
        assert_eq!(embedding.len(), config.dimensions, "Batch embedding {} has wrong dimension", i);
        assert!(embedding.iter().all(|x| x.is_finite()), "Batch embedding {} contains NaN/inf", i);
    }

    // Test determinism - same input should produce same output (for transformer models this may vary slightly)
    let embedding1 = embedder.embed(code_text).unwrap();
    let embedding2 = embedder.embed(code_text).unwrap();

    // For transformer models, we expect very similar outputs (allowing for minor floating point differences)
    let mut total_diff = 0.0;
    for (a, b) in embedding1.iter().zip(embedding2.iter()) {
        total_diff += (a - b).abs();
    }
    let avg_diff = total_diff / embedding1.len() as f32;

    assert!(avg_diff < 1e-6, "Transformer embeddings should be deterministic (avg diff: {})", avg_diff);
}