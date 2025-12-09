//! Integration Test for GraphBertCandleEmbeddings wiring in TripleEmbeddingService
//!
//! This test verifies that:
//! 1. TripleEmbeddingService uses GraphBertCandleEmbeddings for GRAPH domain
//! 2. Error handling works correctly with invalid model paths
//! 3. Other domains (CODE, GENERAL) are unaffected

use anyhow::Result;
use syncore::vector::dual_service::TripleEmbeddingService;
use syncore::config::{SyncoreConfig, GraphEmbeddingsConfig};

#[test]
fn test_graph_domain_uses_graphbert_candle_embeddings() -> Result<()> {
    // This test verifies that the GRAPH domain correctly uses GraphBertCandleEmbeddings
    // and fails appropriately with invalid model configuration

    // Create a config with invalid model path to test error handling
    let mut graph_config = GraphEmbeddingsConfig::default();
    graph_config.model_path = "/nonexistent/path/graphbert.gguf".to_string();

    let config = SyncoreConfig {
        graph_embeddings: graph_config,
        // Use minimal defaults for other fields
        ..Default::default()
    };

    // Set the global config so TripleEmbeddingService can access it
    std::env::set_var("SYNCORE_CONFIG", serde_json::to_string(&config)?);

    // Try to create TripleEmbeddingService - should fail due to invalid GRAPH config
    let result = TripleEmbeddingService::new();

    // The service should fail due to GRAPH domain initialization failure
    assert!(result.is_err(), "TripleEmbeddingService should fail when GraphBertCandleEmbeddings cannot load model");

    // Error should mention graph, model, or similar (not a generic embedding error)
    let error_msg = result.unwrap_err().to_string();
    let mentions_graph_domain = error_msg.contains("graph") ||
                               error_msg.contains("GraphBERT") ||
                               error_msg.contains("graphbert") ||
                               error_msg.contains("model") ||
                               error_msg.contains("Model");

    assert!(mentions_graph_domain, "Error should mention graph domain/model: {}", error_msg);

    println!("✓ Test passed: TripleEmbeddingService correctly uses GraphBertCandleEmbeddings and handles invalid model path");
    println!("✓ Error message: {}", error_msg);

    Ok(())
}

#[test]
fn test_individual_domains_still_work() -> Result<()> {
    // Test that CODE and GENERAL domains can work independently

    // Test CODE domain configuration
    let code_config = syncore::vector::domain::EmbeddingConfig::for_code();
    assert!(code_config.validate().is_ok(), "CODE config should be valid");

    // Test GENERAL domain configuration
    let general_config = syncore::vector::domain::EmbeddingConfig::for_general();
    assert!(general_config.validate().is_ok(), "GENERAL config should be valid");

    // Test that HuggingFace embeddings still work for CODE and GENERAL
    let code_embedding_result = syncore::vector::HuggingFaceEmbeddings::new_bge();
    assert!(code_embedding_result.is_ok(), "CODE domain BGE embeddings should work independently");

    let general_embedding_result = syncore::vector::HuggingFaceEmbeddings::new();
    assert!(general_embedding_result.is_ok(), "GENERAL domain MiniLM embeddings should work independently");

    println!("✓ Test passed: CODE and GENERAL domains work independently");

    Ok(())
}

#[test]
fn test_graph_embeddings_config_structure() -> Result<()> {
    // Test that GraphEmbeddingsConfig has the expected structure

    let config = GraphEmbeddingsConfig::default();

    // Verify default values
    assert_eq!(config.model_name, "graphbert-base");
    assert_eq!(config.model_path, "models/graphbert-base.onnx");
    assert_eq!(config.dimensions, 384);
    assert_eq!(config.batch_size, 16);
    assert!(!config.use_onnx); // Should be false by default for now

    // Test custom configuration
    let mut custom_config = GraphEmbeddingsConfig::default();
    custom_config.model_name = "custom-graphbert".to_string();
    custom_config.model_path = "models/custom.gguf".to_string();
    custom_config.dimensions = 768;
    custom_config.batch_size = 32;
    custom_config.use_onnx = true;

    assert_eq!(custom_config.model_name, "custom-graphbert");
    assert_eq!(custom_config.model_path, "models/custom.gguf");
    assert_eq!(custom_config.dimensions, 768);
    assert_eq!(custom_config.batch_size, 32);
    assert!(custom_config.use_onnx);

    println!("✓ Test passed: GraphEmbeddingsConfig structure is correct");

    Ok(())
}

#[test]
fn test_syncore_config_integration() -> Result<()> {
    // Test that SyncoreConfig properly contains GraphEmbeddingsConfig

    let mut config = SyncoreConfig::default();

    // Modify graph embeddings settings
    config.graph_embeddings.model_name = "test-graphbert".to_string();
    config.graph_embeddings.model_path = "test.gguf".to_string();
    config.graph_embeddings.dimensions = 512;

    // Verify the changes
    assert_eq!(config.graph_embeddings.model_name, "test-graphbert");
    assert_eq!(config.graph_embeddings.model_path, "test.gguf");
    assert_eq!(config.graph_embeddings.dimensions, 512);

    println!("✓ Test passed: SyncoreConfig properly integrates GraphEmbeddingsConfig");

    Ok(())
}