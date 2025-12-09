//! Basic GraphBERT Integration Tests
//!
//! Simple tests to verify GraphBERT functionality works

use syncore::config::{SyncoreConfig, GraphEmbeddingsConfig};
use syncore::vector::domain::{EmbeddingConfig, EmbeddingDomain};
use syncore::vector::{Embeddings, GraphBertEmbeddings};

#[test]
fn test_graphbert_config_parsing() {
    // Test that GraphEmbeddingsConfig can be parsed from TOML
    let config_str = r#"
[graph_embeddings]
model_name = "graphbert-base"
model_path = "/models/graphbert-base.onnx"
dimensions = 384
use_onnx = false
batch_size = 16
"#;

    let config: Result<SyncoreConfig, _> = toml::from_str(config_str);
    assert!(config.is_ok(), "Should parse graph_embeddings section");

    let config = config.unwrap();
    assert_eq!(config.graph_embeddings.model_name, "graphbert-base");
    assert_eq!(config.graph_embeddings.dimensions, 384);
    assert!(!config.graph_embeddings.use_onnx);

    println!("✅ GraphEmbeddingsConfig parsing works");
}

#[test]
fn test_embedding_config_for_graph() {
    // Test that EmbeddingConfig::for_graph() returns GraphBERT settings
    let config = EmbeddingConfig::for_graph();

    assert_eq!(config.domain, EmbeddingDomain::Graph);
    assert!(config.model_name.contains("graphbert"));
    assert_eq!(config.dimension, 384);
    assert!(config.index_path.contains("graph"));

    println!("✅ EmbeddingConfig::for_graph() uses GraphBERT: model={}, dim={}",
             config.model_name, config.dimension);
}

#[test]
fn test_graphbert_embeddings_creation() -> anyhow::Result<()> {
    // Test that GraphBertEmbeddings can be created and used
    let embeddings = GraphBertEmbeddings::new()?;

    assert_eq!(embeddings.dim(), 384);
    assert!(embeddings.model_name().contains("graphbert"));

    // Test embedding some text
    let text = "Node: main_function, Type: Function, Calls: [calculate_sum], UsedBy: [entry_point]";
    let vector = embeddings.embed(text)?;

    assert_eq!(vector.len(), 384);
    assert!(!vector.iter().all(|&x| x == 0.0)); // Should have non-zero values

    println!("✅ GraphBertEmbeddings produces {}-dim vectors", vector.len());

    Ok(())
}

#[test]
fn test_graphbert_embeddings_with_config() -> anyhow::Result<()> {
    // Test creating GraphBertEmbeddings from configuration
    let config = GraphEmbeddingsConfig {
        model_name: "custom-graphbert-large".to_string(),
        model_path: "/models/custom-graphbert-large.onnx".to_string(),
        dimensions: 768,
        use_onnx: false,
        batch_size: 32,
    };

    let embeddings = GraphBertEmbeddings::from_config(&config)?;

    assert_eq!(embeddings.dim(), 768);
    assert_eq!(embeddings.model_name(), "custom-graphbert-large");

    // Test embedding with custom dimension
    let text = "Test graph entity with custom model";
    let vector = embeddings.embed(text)?;

    assert_eq!(vector.len(), 768);

    println!("✅ Custom GraphBertEmbeddings produces {}-dim vectors", vector.len());

    Ok(())
}