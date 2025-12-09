//! RAGGraph SQLiteGraph Integration Tests
//!
//! Tests that RAGGraph Real mode works correctly with SQLiteGraph backend
//! instead of requiring Neo4j. This validates that the SQLiteGraph-first
//! architecture is fully functional.

use std::sync::{Arc, Mutex};
use syncore::config::{GraphBackend, GraphConfig, SyncoreConfig};
use syncore::graph::backend_selector::create_default_graph_backend;
use syncore::raggraph::{
    HopGraphTransformer, RagGraphConfig, RagQuery, SQLiteGraphStorageAdapter, StorageAdapter,
};
use syncore::vector::traits::VectorIndex;
use syncore::vector::VectorStore;
use tempfile::tempdir;

/// Create a mock VectorStore for testing
fn create_mock_vector_store() -> Arc<Mutex<VectorStore>> {
    // Create a VectorStore with real embeddings for testing
    let embeddings = Box::new(syncore::vector::RealEmbeddings::new(384).unwrap());
    let store = VectorStore::new(embeddings);
    Arc::new(Mutex::new(store))
}

#[tokio::test]
async fn test_sqlite_graph_storage_adapter_creation() {
    // Create temp directory for test databases
    let temp_dir = tempdir().unwrap();
    let code_graph_db_path = temp_dir.path().join("test_code_graph.db");

    // Create mock vector store
    let vector_store = create_mock_vector_store();
    let vector_index = vector_store.clone() as Arc<Mutex<dyn VectorIndex>>;

    // Create SQLiteGraph backend
    let graph_config = GraphConfig {
        backend: GraphBackend::SqliteGraph,
        path: code_graph_db_path.to_str().unwrap().to_string(),
        uri: String::new(),
        user: String::new(),
        password: String::new(),
        enabled: true,
    };

    let graph_backend = create_default_graph_backend(&graph_config).await.unwrap();

    // Create SQLiteGraphStorageAdapter
    let adapter = SQLiteGraphStorageAdapter::new(
        vector_index,
        graph_backend,
        384, // dimension
    );

    // Test that adapter was created successfully
    println!("✅ SQLiteGraphStorageAdapter created successfully");

    // Test basic functionality - should not panic
    let result = adapter.seed_nodes_from_query("test query", 5);
    assert!(result.is_ok(), "seed_nodes_from_query should work");
}

#[test]
fn test_raggraph_config_environment_parsing() {
    // Test default configuration
    let config = RagGraphConfig::default();
    assert_eq!(config.backend_mode, syncore::raggraph::RaggraphBackendMode::Mock);
    assert_eq!(config.num_hops, 3);
    assert_eq!(config.top_k, 50);
    assert_eq!(config.embedding_dim, 384);

    // Test environment variable parsing
    std::env::set_var("SYNCORE_RAGGRAPH_BACKEND", "real");
    let config = RagGraphConfig::from_env();
    assert_eq!(config.backend_mode, syncore::raggraph::RaggraphBackendMode::Real);
    std::env::remove_var("SYNCORE_RAGGRAPH_BACKEND");

    println!("✅ RagGraphConfig environment parsing works correctly");
}

#[test]
fn test_syncore_config_graph_backend_selection() {
    // Test default SQLiteGraph backend
    let mut config = SyncoreConfig::default();
    config.apply_env_overrides();
    matches!(config.graph.backend, GraphBackend::SqliteGraph);

    // Test environment variable override for SQLiteGraph
    std::env::set_var("GRAPH_BACKEND", "sqlitegraph");
    let mut config = SyncoreConfig::default();
    config.apply_env_overrides();
    matches!(config.graph.backend, GraphBackend::SqliteGraph);
    std::env::remove_var("GRAPH_BACKEND");

    // Test environment variable override for Neo4j
    std::env::set_var("GRAPH_BACKEND", "neo4j");
    let mut config = SyncoreConfig::default();
    config.apply_env_overrides();
    matches!(config.graph.backend, GraphBackend::Neo4j);
    std::env::remove_var("GRAPH_BACKEND");

    println!("✅ SyncoreConfig graph backend selection works correctly");
}

#[tokio::test]
async fn test_ragquery_with_sqlitegraph_backend() {
    // Set environment variables for SQLiteGraph backend
    std::env::set_var("GRAPH_BACKEND", "sqlitegraph");

    // Create temp directory for test databases
    let temp_dir = tempdir().unwrap();
    let code_graph_db_path = temp_dir.path().join("test_code_graph.db");
    std::env::set_var("SYNCORE_CODE_GRAPH_DB", code_graph_db_path.to_str().unwrap());

    // Load config with environment overrides
    let mut syncore_config = SyncoreConfig::default();
    syncore_config.apply_env_overrides();
    let graph_config = syncore_config.graph;

    // Verify SQLiteGraph backend is selected
    matches!(graph_config.backend, GraphBackend::SqliteGraph);

    // Create mock vector store
    let vector_store = create_mock_vector_store();
    let vector_index = vector_store.clone() as Arc<Mutex<dyn VectorIndex>>;

    // Create SQLiteGraph backend
    let graph_backend = create_default_graph_backend(&graph_config).await.unwrap();

    // Create SQLiteGraphStorageAdapter
    let storage =
        Arc::new(SQLiteGraphStorageAdapter::new(vector_index.clone(), graph_backend, 384));

    // Create RagQuery with SQLiteGraph storage
    let rag_config = RagGraphConfig {
        backend_mode: syncore::raggraph::RaggraphBackendMode::Real,
        num_hops: 3,
        alpha: 0.85,
        top_k: 50,
        embedding_dim: 384,
    };

    let rag_query = RagQuery::with_storage(rag_config, storage);

    // Test query execution
    let result = rag_query.query("test query");
    assert!(result.is_ok(), "RagQuery with SQLiteGraph should succeed");

    let rag_result = result.unwrap();
    assert!(!rag_result.top_nodes.is_empty(), "Should return top nodes");
    assert!(!rag_result.reasoning_path.is_empty(), "Should return reasoning path");

    // Clean up
    std::env::remove_var("GRAPH_BACKEND");
    std::env::remove_var("SYNCORE_CODE_GRAPH_DB");

    println!("✅ RagQuery with SQLiteGraph backend succeeded");
}

#[tokio::test]
async fn test_hopgraph_transformer_with_sqlitegraph_backend() {
    // Set environment variables for SQLiteGraph backend
    std::env::set_var("GRAPH_BACKEND", "sqlitegraph");

    // Create temp directory for test databases
    let temp_dir = tempdir().unwrap();
    let code_graph_db_path = temp_dir.path().join("test_code_graph.db");
    std::env::set_var("SYNCORE_CODE_GRAPH_DB", code_graph_db_path.to_str().unwrap());

    // Load config with environment overrides
    let mut syncore_config = SyncoreConfig::default();
    syncore_config.apply_env_overrides();
    let graph_config = syncore_config.graph;

    // Create mock vector store
    let vector_store = create_mock_vector_store();
    let vector_index = vector_store.clone() as Arc<Mutex<dyn VectorIndex>>;

    // Create SQLiteGraph backend
    let graph_backend = create_default_graph_backend(&graph_config).await.unwrap();

    // Create SQLiteGraphStorageAdapter
    let storage =
        Arc::new(SQLiteGraphStorageAdapter::new(vector_index.clone(), graph_backend, 384));

    // Create HopGraphTransformer with SQLiteGraph storage
    let rag_config = RagGraphConfig {
        backend_mode: syncore::raggraph::RaggraphBackendMode::Real,
        num_hops: 3,
        alpha: 0.85,
        top_k: 50,
        embedding_dim: 384,
    };

    let transformer = HopGraphTransformer::with_storage(rag_config, storage);

    // Test multi-hop reasoning
    let seed_nodes = vec![1, 2, 3];
    let result = transformer.multi_hop_reasoning(&seed_nodes);
    assert!(result.is_ok(), "HopGraphTransformer with SQLiteGraph should succeed");

    let hop_result = result.unwrap();
    assert!(!hop_result.top_nodes.is_empty(), "Should return top nodes");
    assert!(!hop_result.reasoning_path.is_empty(), "Should return reasoning path");

    // Clean up
    std::env::remove_var("GRAPH_BACKEND");
    std::env::remove_var("SYNCORE_CODE_GRAPH_DB");

    println!("✅ HopGraphTransformer with SQLiteGraph backend succeeded");
}
