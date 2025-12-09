//! GraphBERT Integration TDD Tests
//!
//! Test-Driven Development for GraphBERT integration into the triple-domain pipeline.
//! These tests were written to fail before implementation and should now pass.

use anyhow::Result;
use syncore::vector::domain::{EmbeddingConfig, EmbeddingDomain, EmbeddingService};
use syncore::vector::dual_service::TripleEmbeddingService;
use syncore::config::{SyncoreConfig, GraphEmbeddingsConfig, EmbeddingsConfig};

/// Test 1: EmbeddingConfig::for_graph() returns GraphBERT-compatible configuration
#[test]
fn test_embedding_config_for_graph_uses_graphbert() {
    // Test: EmbeddingConfig::for_graph() should return a configuration
    // with GraphBERT model identifier and dedicated graph index path

    let config = EmbeddingConfig::for_graph();

    // Should be Graph domain
    assert_eq!(config.domain, EmbeddingDomain::Graph);

    // Should use GraphBERT model (not BGE or all-MiniLM)
    assert!(config.model_name.contains("graphbert"),
           "Expected graphbert model, got: {}", config.model_name);

    // Should use dedicated graph index path
    assert!(config.index_path.contains("graph"),
           "Expected graph index path, got: {}", config.index_path);

    // Should have graph-specific dimension (384 or configurable)
    assert!(config.dimension > 0, "Graph embedding dimension must be > 0");

    println!("✅ EmbeddingConfig::for_graph() uses GraphBERT: model={}, path={}, dim={}",
             config.model_name, config.index_path, config.dimension);
}

/// Test 2: SyncoreConfig supports graph_embeddings configuration
#[test]
fn test_syncore_config_graph_embeddings_section() {
    // Test: SyncoreConfig should have graph_embeddings section
    // with model_name, model_path, and other GraphBERT-specific settings

    let config_str = r#"
[embeddings]
model = "semantic"
dimensions = 384
batch_size = 32

[graph_embeddings]
model_name = "custom-graphbert-large"
model_path = "/models/custom-graphbert-large.onnx"
dimensions = 768
use_onnx = true
"#;

    let config: Result<SyncoreConfig, _> = toml::from_str(config_str);
    assert!(config.is_ok(), "Should parse graph_embeddings section");

    let config = config.unwrap();

    // Verify graph_embeddings configuration exists and has correct values
    assert_eq!(config.graph_embeddings.model_name, "custom-graphbert-large");
    assert_eq!(config.graph_embeddings.model_path, "/models/custom-graphbert-large.onnx");
    assert_eq!(config.graph_embeddings.dimensions, 768);
    assert_eq!(config.graph_embeddings.use_onnx, true);
    assert_eq!(config.graph_embeddings.batch_size, 16); // default value

    println!("✅ SyncoreConfig supports graph_embeddings configuration");
}

/// Test 3: TripleEmbeddingService uses GraphBertModel for Graph domain
#[test]
fn test_triple_embedding_service_uses_graphbert_for_graph() -> Result<()> {
    // Test: TripleEmbeddingService should use GraphBertModel for Graph domain
    // and HuggingFaceEmbeddings for Code/General domains

    let service = TripleEmbeddingService::new()?;

    // Graph domain should use GraphBERT embeddings
    let graph_config = service.config(EmbeddingDomain::Graph);
    assert_eq!(graph_config.domain, EmbeddingDomain::Graph);
    assert!(graph_config.model_name.contains("graphbert"),
           "Graph domain should use GraphBERT, got: {}", graph_config.model_name);

    // Code domain should still use BGE
    let code_config = service.config(EmbeddingDomain::Code);
    assert_eq!(code_config.domain, EmbeddingDomain::Code);
    assert!(code_config.model_name.contains("bge"),
           "Code domain should use BGE, got: {}", code_config.model_name);

    // General domain should still use all-MiniLM
    let general_config = service.config(EmbeddingDomain::General);
    assert_eq!(general_config.domain, EmbeddingDomain::General);
    assert!(general_config.model_name.contains("minilm"),
           "General domain should use all-MiniLM, got: {}", general_config.model_name);

    // Dimensions may differ by domain
    println!("✅ TripleEmbeddingService uses distinct models per domain:");
    println!("  Code: {} (dim={})", code_config.model_name, code_config.dimension);
    println!("  General: {} (dim={})", general_config.model_name, general_config.dimension);
    println!("  Graph: {} (dim={})", graph_config.model_name, graph_config.dimension);

    Ok(())
}

/// Test 4: Graph domain embeddings produce different results than Code domain
#[test]
fn test_graph_domain_embeddings_different_from_code() -> Result<()> {
    // Test: Embedding same text in Graph vs Code domains should produce
    // different vectors (demonstrating GraphBERT vs BGE)

    let service = TripleEmbeddingService::new()?;
    let text = "function calculate_sum(a, b) { return a + b; }";

    // Embed in Code domain (BGE)
    let code_embedding = service.embed(text, EmbeddingDomain::Code)?;

    // Embed in Graph domain (GraphBERT)
    let graph_embedding = service.embed(text, EmbeddingDomain::Graph)?;

    // Both should have valid dimensions
    assert!(!code_embedding.is_empty(), "Code embedding should not be empty");
    assert!(!graph_embedding.is_empty(), "Graph embedding should not be empty");

    // Should have same dimensions (both 384)
    assert_eq!(code_embedding.len(), graph_embedding.len(),
              "Code and Graph embeddings should have same dimensions");

    // But different values (GraphBERT incorporates graph features)
    let mut differences = 0;
    for (i, (code_val, graph_val)) in code_embedding.iter().zip(graph_embedding.iter()).enumerate() {
        if (code_val - graph_val).abs() > 0.001 { // Significant difference
            differences += 1;
            if differences <= 5 { // Log first 5 differences
                println!("  Dim {}: Code={:.6}, Graph={:.6}, Diff={:.6}",
                        i, code_val, graph_val, (code_val - graph_val).abs());
            }
        }
    }

    assert!(differences > 0, "GraphBERT should produce different embeddings than BGE");
    println!("✅ GraphBERT produces {} different dimensions out of {}", differences, code_embedding.len());

    Ok(())
}

/// Test 5: TripleEmbeddingService domain isolation works correctly
#[test]
fn test_triple_embedding_service_domain_isolation() -> Result<()> {
    // Test: Each domain should have separate VectorStores and configurations

    let service = TripleEmbeddingService::new()?;

    // Get stores for each domain
    let code_store = service.store_for_domain(EmbeddingDomain::Code);
    let general_store = service.store_for_domain(EmbeddingDomain::General);
    let graph_store = service.store_for_domain(EmbeddingDomain::Graph);

    // Verify stores are distinct Arc<Mutex<VectorStore>> instances
    let code_ptr = Arc::as_ptr(&code_store);
    let general_ptr = Arc::as_ptr(&general_store);
    let graph_ptr = Arc::as_ptr(&graph_store);

    assert_ne!(code_ptr, general_ptr, "Code and General stores should be different instances");
    assert_ne!(code_ptr, graph_ptr, "Code and Graph stores should be different instances");
    assert_ne!(general_ptr, graph_ptr, "General and Graph stores should be different instances");

    // Verify each store starts empty (using VectorStore::len)
    let code_len = {
        let store = code_store.lock().unwrap();
        store.len()
    };
    let general_len = {
        let store = general_store.lock().unwrap();
        store.len()
    };
    let graph_len = {
        let store = graph_store.lock().unwrap();
        store.len()
    };

    assert_eq!(code_len, 0, "Code store should start empty");
    assert_eq!(general_len, 0, "General store should start empty");
    assert_eq!(graph_len, 0, "Graph store should start empty");

    println!("✅ TripleEmbeddingService maintains separate VectorStores per domain");
    println!("  Code store: {} embeddings", code_len);
    println!("  General store: {} embeddings", general_len);
    println!("  Graph store: {} embeddings", graph_len);

    Ok(())
}

/// Test 6: Graph domain embeddings work with real VectorStore operations
#[test]
fn test_graph_domain_real_vector_store_operations() -> Result<()> {
    // Test: Graph domain should work with actual VectorStore operations
    // This test uses the real VectorStore API (insert_text, search)

    let service = TripleEmbeddingService::new()?;

    // Get graph store
    let graph_store = service.store_for_domain(EmbeddingDomain::Graph);

    // Test text for graph entity
    let graph_text = "Node: main_function, Type: Function, Calls: [calculate_sum], UsedBy: [entry_point]";
    let namespace = "graph_entity";

    // Insert into graph store using real VectorStore API
    {
        let mut store = graph_store.lock().unwrap();
        let id = store.insert_text(namespace, graph_text, "test_graph_entity")?;
        println!("✅ Inserted graph entity with ID: {}", id);
    }

    // Verify it was inserted
    let graph_len = {
        let store = graph_store.lock().unwrap();
        store.len()
    };
    assert_eq!(graph_len, 1, "Graph store should have 1 embedding");

    // Search for the text in graph store
    let search_results = {
        let store = graph_store.lock().unwrap();
        store.search(graph_text, 5)?
    };

    assert!(!search_results.is_empty(), "Graph store should find the inserted embedding");
    println!("✅ Graph store search found {} results", search_results.len());

    // Verify code and general stores are still empty
    let code_len = {
        let store = service.code_store().lock().unwrap();
        store.len()
    };
    let general_len = {
        let store = service.general_store().lock().unwrap();
        store.len()
    };

    assert_eq!(code_len, 0, "Code store should remain empty");
    assert_eq!(general_len, 0, "General store should remain empty");

    println!("✅ Graph domain operations are isolated from other domains");
    println!("  Graph: {} embeddings, Code: {} embeddings, General: {} embeddings",
             graph_len, code_len, general_len);

    Ok(())
}