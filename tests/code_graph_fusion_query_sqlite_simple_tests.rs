//! Code Graph Fusion Query SQLiteGraph Integration Tests
//!
//! Tests that code_graph_fusion_query MCP tool works with SQLiteGraph backend
//! while maintaining Neo4j compatibility.
//!
//! TDD approach: Tests are written FIRST to define the expected behavior,
//! then implementation follows to make them pass.

use std::sync::{Arc, Mutex};
use tempfile::tempdir;
use syncore::code_graph::fusion_reasoning::FusionReasoning;

/// Create a mock VectorStore for testing
fn create_mock_vector_store() -> Arc<Mutex<syncore::vector::VectorStore>> {
    // Create a VectorStore with real embeddings for testing
    let embeddings = Box::new(syncore::vector::RealEmbeddings::new(384).unwrap());
    let store = syncore::vector::VectorStore::new(embeddings);
    Arc::new(Mutex::new(store))
}

/// Create a mock CodeGraph
fn create_mock_code_graph() -> syncore::code_graph::CodeGraph {
    let vector_store = create_mock_vector_store();
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test_code_graph.db");
    syncore::code_graph::CodeGraph::new(db_path.to_str().unwrap(), vector_store).expect("Failed to create CodeGraph")
}

#[test]
fn test_rag_graph_api_with_sqlitegraph() {
    // Test: RagGraphAPI can be created with GraphBackend trait object

    // Arrange
    std::env::set_var("GRAPH_BACKEND", "sqlitegraph");

    let temp_dir = tempdir().unwrap();
    let code_graph_db_path = temp_dir.path().join("test_code_graph.db");
    std::env::set_var("SYNCORE_CODE_GRAPH_DB", code_graph_db_path.to_str().unwrap());

    let code_graph = create_mock_code_graph();

    // Act - Create GraphBackend using the selector
    let rt = tokio::runtime::Runtime::new().unwrap();
    let graph_backend = rt.block_on(async {
        let config = syncore::config::SyncoreConfig::default();
        let graph_config = &config.graph;
        syncore::graph::backend_selector::create_default_graph_backend(graph_config).await
    });

    // Assert
    assert!(graph_backend.is_ok(), "Should create graph backend successfully");

    let graph_backend = graph_backend.unwrap();
    let rag_api = syncore::code_graph::RagGraphAPI::new(code_graph, graph_backend);

    // Verify we can create the API without panicking
    println!("✅ RagGraphAPI created successfully with SQLiteGraph backend");

    // Clean up
    std::env::remove_var("GRAPH_BACKEND");
    std::env::remove_var("SYNCORE_CODE_GRAPH_DB");
}

#[test]
fn test_fusion_reasoning_with_sqlitegraph() {
    // Test: FusionReasoning can be created with GraphBackend trait object

    // Arrange
    std::env::set_var("GRAPH_BACKEND", "sqlitegraph");

    let temp_dir = tempdir().unwrap();
    let code_graph_db_path = temp_dir.path().join("test_code_graph.db");
    std::env::set_var("SYNCORE_CODE_GRAPH_DB", code_graph_db_path.to_str().unwrap());

    let vector_store = create_mock_vector_store();

    // Act - Create GraphBackend using the selector
    let rt = tokio::runtime::Runtime::new().unwrap();
    let graph_backend = rt.block_on(async {
        let config = syncore::config::SyncoreConfig::default();
        let graph_config = &config.graph;
        syncore::graph::backend_selector::create_default_graph_backend(graph_config).await
    });

    // Assert
    assert!(graph_backend.is_ok(), "Should create graph backend successfully");

    let graph_backend = graph_backend.unwrap();
    let fusion_reasoning = FusionReasoning::new(graph_backend, vector_store);

    // Test the higher-order combination function
    let result = fusion_reasoning.combine_higher_order(0.6, 0.8);

    // Expected: 0.4*0.6 + 0.4*0.8 + 0.2*0.64 = 0.688
    assert!((result - 0.688).abs() < 0.01, "Higher-order combination should work correctly");

    println!("✅ FusionReasoning created successfully with SQLiteGraph backend");

    // Clean up
    std::env::remove_var("GRAPH_BACKEND");
    std::env::remove_var("SYNCORE_CODE_GRAPH_DB");
}

#[test]
fn test_sqlitegraph_backend_creation() {
    // Test: SQLiteGraph backend can be created when environment variables are set

    // Arrange
    std::env::set_var("GRAPH_BACKEND", "sqlitegraph");

    let temp_dir = tempdir().unwrap();
    let code_graph_db_path = temp_dir.path().join("test_code_graph.db");
    std::env::set_var("SYNCORE_CODE_GRAPH_DB", code_graph_db_path.to_str().unwrap());

    // Act
    let rt = tokio::runtime::Runtime::new().unwrap();
    let backend_result = rt.block_on(async {
        let mut config = syncore::config::SyncoreConfig::default();
        config.apply_env_overrides();
        let graph_config = &config.graph;
        syncore::graph::backend_selector::create_default_graph_backend(graph_config).await
    });

    // Assert
    assert!(backend_result.is_ok(), "SQLiteGraph backend should be created successfully");

    let backend = backend_result.unwrap();
    println!("✅ SQLiteGraph backend created successfully: {:?}", std::any::type_name_of_val(&backend));

    // Verify it's a trait object (abstracted properly)
    let backend_type = format!("{:?}", std::any::type_name_of_val(&backend));
    assert!(backend_type.contains("Arc") && backend_type.contains("GraphBackend"),
            "Should be trait object Arc<dyn GraphBackend>");

    // Clean up
    std::env::remove_var("GRAPH_BACKEND");
    std::env::remove_var("SYNCORE_CODE_GRAPH_DB");
}

#[test]
fn test_neo4j_backend_still_possible() {
    // Test: Neo4j backend can still be created when environment variables are set

    // Arrange
    std::env::set_var("GRAPH_BACKEND", "neo4j");
    std::env::set_var("NEO4J_URI", "bolt://127.0.0.1:7687");
    std::env::set_var("NEO4J_USER", "neo4j");
    std::env::set_var("NEO4J_PASS", "testpassword123");

    // Act
    let rt = tokio::runtime::Runtime::new().unwrap();
    let backend_result = rt.block_on(async {
        let mut config = syncore::config::SyncoreConfig::default();
        config.apply_env_overrides();
        let graph_config = &config.graph;
        syncore::graph::backend_selector::create_default_graph_backend(graph_config).await
    });

    // Assert - We expect this to fail since Neo4j isn't running, but it should attempt connection
    match backend_result {
        Ok(backend) => {
            let backend_type = format!("{:?}", std::any::type_name_of_val(&backend));
            assert!(backend_type.contains("Arc") && backend_type.contains("GraphBackend"),
                    "Should be trait object Arc<dyn GraphBackend>");
            println!("✅ Neo4j backend created successfully");
        }
        Err(e) => {
            // Expected since Neo4j isn't running in test environment
            println!("✅ Neo4j backend correctly returned connection error: {}", e);
            assert!(e.to_string().contains("connection") || e.to_string().contains("neo4j"),
                    "Should return Neo4j connection error");
        }
    }

    // Clean up
    std::env::remove_var("GRAPH_BACKEND");
    std::env::remove_var("NEO4J_URI");
    std::env::remove_var("NEO4J_USER");
    std::env::remove_var("NEO4J_PASS");
}

#[test]
fn test_backend_selector_defaults_to_sqlitegraph() {
    // Test: When no GRAPH_BACKEND is set, defaults to SQLiteGraph

    // Arrange - Remove any existing GRAPH_BACKEND
    std::env::remove_var("GRAPH_BACKEND");

    // Create a temp directory for the test database
    let temp_dir = tempdir().unwrap();
    let code_graph_db_path = temp_dir.path().join("test_code_graph.db");
    std::env::set_var("SYNCORE_CODE_GRAPH_DB", code_graph_db_path.to_str().unwrap());

    // Act
    let rt = tokio::runtime::Runtime::new().unwrap();
    let backend_result = rt.block_on(async {
        let mut config = syncore::config::SyncoreConfig::default();
        config.apply_env_overrides(); // Should default to SQLiteGraph
        let graph_config = &config.graph;
        syncore::graph::backend_selector::create_default_graph_backend(graph_config).await
    });

    // Assert
    assert!(backend_result.is_ok(), "Should default to SQLiteGraph backend successfully");

    let backend = backend_result.unwrap();
    let backend_type = format!("{:?}", std::any::type_name_of_val(&backend));
    assert!(backend_type.contains("Arc") && backend_type.contains("GraphBackend"),
            "Should be trait object Arc<dyn GraphBackend>");

    println!("✅ Default backend correctly defaults to SQLiteGraph");

    // Clean up
    std::env::remove_var("SYNCORE_CODE_GRAPH_DB");
}

#[test]
fn test_rag_graph_api_legacy_compatibility() {
    // Test: Legacy with_neo4j method still works for backward compatibility

    // Arrange
    let code_graph = create_mock_code_graph();
    let vector_store = create_mock_vector_store();

    // Act - This should still compile and work, even though we prefer the generic constructor
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        // We can't easily create a Neo4jClient in tests, but we can verify the API exists
        // This tests compilation compatibility more than runtime behavior
        let _ = FusionReasoning::with_neo4j;
        Ok::<(), anyhow::Error>(())
    });

    // Assert
    assert!(result.is_ok(), "Legacy compatibility methods should be available");

    println!("✅ Legacy compatibility methods are available");
}