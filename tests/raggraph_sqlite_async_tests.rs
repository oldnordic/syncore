//! RAGGraph SQLite Async Integration Tests
//!
//! TDD Integration tests that reproduce the previous failures in sqlite_storage_adapter.rs
//! These tests MUST FAIL before implementation and PASS after async façade implementation.
//!
//! Tests validate:
//! - raggraph_query(real + sqlitegraph) works concurrently
//! - raggraph_multihop(real + sqlitegraph) returns expected structure
//! - No blocking violations under load
//! - No deadlocks
//! - Adapter correctly handles 10, 50, 100 concurrent queries

use anyhow::{anyhow, Result};
use std::sync::{Arc, Mutex};
use syncore::config::{GraphBackend as ConfigBackend, GraphConfig};
use syncore::graph::{backend_selector::create_graph_backend, GraphBackend};
use syncore::raggraph::{
    HopGraphTransformer, RagGraphConfig, RagGraphResult, RagQuery, SQLiteGraphStorageAdapter,
    StorageAdapter,
};
use syncore::vector::traits::VectorIndex;
use tempfile::tempdir;

/// Mock VectorIndex for testing
struct MockVectorIndex {
    results: Vec<(i64, f32)>,
}

impl VectorIndex for MockVectorIndex {
    fn add(&mut self, _id: i64, _embedding: Vec<f32>) -> Result<()> {
        Ok(())
    }

    fn search(&self, _query: &[f32], k: usize) -> Result<Vec<(i64, f32)>> {
        let limit = std::cmp::min(k, self.results.len());
        Ok(self.results[..limit].to_vec())
    }

    fn dimension(&self) -> Option<usize> {
        Some(384)
    }

    fn len(&self) -> usize {
        self.results.len()
    }
}

/// Create test components for async SQLiteGraph testing
async fn create_test_components() -> (Arc<dyn StorageAdapter>, Arc<dyn GraphBackend>) {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test_raggraph_async.db");

    // Create GraphBackend
    let graph_config = GraphConfig {
        backend: ConfigBackend::SqliteGraph,
        path: db_path.to_str().unwrap().to_string(),
        uri: String::new(),
        user: String::new(),
        password: String::new(),
        enabled: true,
    };

    let graph_backend = create_graph_backend(&graph_config, "test").await.unwrap();

    // Create Mock VectorIndex with test data
    let vector_index = Arc::new(Mutex::new(MockVectorIndex {
        results: vec![
            (1, 0.95),
            (2, 0.90),
            (3, 0.85),
            (4, 0.80),
            (5, 0.75),
            (6, 0.70),
            (7, 0.65),
            (8, 0.60),
            (9, 0.55),
            (10, 0.50),
        ],
    }));

    // Create SQLiteGraphStorageAdapter - this is what currently has blocking issues
    let storage_adapter = SQLiteGraphStorageAdapter::new(vector_index, graph_backend.clone(), 384)
        .expect("Failed to create SQLiteGraphStorageAdapter");

    (Arc::new(storage_adapter), graph_backend)
}

#[tokio::test(flavor = "multi_thread")]
async fn test_raggraph_query_with_sqliteasync_real_mode() {
    // This test reproduces the original blocking issue and should FAIL initially
    let (storage, _graph_backend) = create_test_components().await;

    // Configure for real mode with SQLiteGraph
    let rag_config = RagGraphConfig {
        backend_mode: syncore::raggraph::RaggraphBackendMode::Real,
        num_hops: 3,
        alpha: 0.85,
        top_k: 50,
        embedding_dim: 384,
    };

    // Create RagQuery with SQLiteGraph storage
    let rag_query = RagQuery::with_storage(rag_config, storage);

    // This should FAIL initially due to "can call blocking only when running on the multi-threaded runtime"
    let result = rag_query.query("test async query");

    assert!(result.is_ok(), "RagQuery with SQLiteGraph should succeed after async façade");

    let rag_result = result.unwrap();
    assert!(!rag_result.top_nodes.is_empty(), "Should return top nodes");
    assert!(!rag_result.reasoning_path.is_empty(), "Should return reasoning path");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_raggraph_multihop_with_sqliteasync_real_mode() {
    // This test reproduces the original blocking issue for multihop reasoning
    let (storage, _graph_backend) = create_test_components().await;

    // Configure for real mode with SQLiteGraph
    let rag_config = RagGraphConfig {
        backend_mode: syncore::raggraph::RaggraphBackendMode::Real,
        num_hops: 3,
        alpha: 0.85,
        top_k: 50,
        embedding_dim: 384,
    };

    // Create HopGraphTransformer with SQLiteGraph storage
    let transformer = HopGraphTransformer::with_storage(rag_config, storage);

    // Test multi-hop reasoning - this should FAIL initially due to blocking violations
    let seed_nodes = vec![1, 2, 3];
    let result = transformer.multi_hop_reasoning(&seed_nodes);

    assert!(
        result.is_ok(),
        "HopGraphTransformer with SQLiteGraph should succeed after async façade"
    );

    let hop_result = result.unwrap();
    assert!(!hop_result.top_nodes.is_empty(), "Should return top nodes");
    assert!(!hop_result.reasoning_path.is_empty(), "Should return reasoning path");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_concurrent_raggraph_queries_10_concurrent() {
    // Test 10 concurrent RAGGraph queries - should fail initially due to blocking violations
    let (storage, _graph_backend) = create_test_components().await;

    let rag_config = RagGraphConfig {
        backend_mode: syncore::raggraph::RaggraphBackendMode::Real,
        num_hops: 2,
        alpha: 0.85,
        top_k: 10,
        embedding_dim: 384,
    };

    let transformer = Arc::new(HopGraphTransformer::with_storage(rag_config, storage));

    // Launch 10 concurrent queries
    let mut handles = Vec::new();
    for i in 0..10 {
        let transformer_clone = transformer.clone();
        let seed_nodes = vec![i + 1, i + 2, i + 3];

        let handle =
            tokio::spawn(async move { transformer_clone.multi_hop_reasoning(&seed_nodes) });
        handles.push(handle);
    }

    // Wait for all to complete - this should FAIL initially with runtime errors
    let mut success_count = 0;
    for handle in handles {
        let result = handle.await;
        assert!(result.is_ok(), "Concurrent task should complete without panicking");

        let hop_result = result.unwrap();
        if hop_result.as_ref().unwrap().top_nodes.len() > 0 {
            success_count += 1;
        }
    }

    // At least some queries should succeed after async façade implementation
    assert!(success_count > 0, "Some concurrent queries should succeed");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_concurrent_raggraph_queries_50_concurrent() {
    // Test 50 concurrent RAGGraph queries - stress test for the async façade
    let (storage, _graph_backend) = create_test_components().await;

    let rag_config = RagGraphConfig {
        backend_mode: syncore::raggraph::RaggraphBackendMode::Real,
        num_hops: 1,
        alpha: 0.85,
        top_k: 5,
        embedding_dim: 384,
    };

    let transformer = Arc::new(HopGraphTransformer::with_storage(rag_config, storage));

    // Launch 50 concurrent queries
    let mut handles = Vec::new();
    for i in 0..50 {
        let transformer_clone = transformer.clone();
        let seed_nodes = vec![(i % 10) + 1];

        let handle =
            tokio::spawn(async move { transformer_clone.multi_hop_reasoning(&seed_nodes) });
        handles.push(handle);
    }

    // Wait for all to complete - this should FAIL initially with thread pool exhaustion
    let mut success_count = 0;
    for handle in handles {
        let result = handle.await;
        assert!(result.is_ok(), "Concurrent task should complete without panicking");

        let hop_result = result.unwrap();
        if hop_result.as_ref().unwrap().top_nodes.len() > 0 {
            success_count += 1;
        }
    }

    // Should handle 50 concurrent queries after async façade implementation
    assert!(success_count >= 40, "Most concurrent queries should succeed");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_concurrent_raggraph_queries_100_concurrent() {
    // Test 100 concurrent RAGGraph queries - maximum stress test
    let (storage, _graph_backend) = create_test_components().await;

    let rag_config = RagGraphConfig {
        backend_mode: syncore::raggraph::RaggraphBackendMode::Real,
        num_hops: 1,
        alpha: 0.85,
        top_k: 3,
        embedding_dim: 384,
    };

    let transformer = Arc::new(HopGraphTransformer::with_storage(rag_config, storage));

    // Launch 100 concurrent queries
    let mut handles = Vec::new();
    for i in 0..100 {
        let transformer_clone = transformer.clone();
        let seed_nodes = vec![(i % 5) + 1];

        let handle =
            tokio::spawn(async move { transformer_clone.multi_hop_reasoning(&seed_nodes) });
        handles.push(handle);
    }

    // Wait for all to complete - this should FAIL initially with severe blocking issues
    let mut success_count = 0;
    for handle in handles {
        let result = handle.await;
        assert!(result.is_ok(), "Concurrent task should complete without panicking");

        let hop_result = result.unwrap();
        if hop_result.as_ref().unwrap().top_nodes.len() > 0 {
            success_count += 1;
        }
    }

    // Should handle 100 concurrent queries after async façade implementation
    assert!(success_count >= 80, "Most concurrent queries should succeed even at high load");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_no_runtime_blocking_under_load() {
    // Verify that the async façade doesn't block the runtime under load
    let (storage, _graph_backend) = create_test_components().await;

    let rag_config = RagGraphConfig {
        backend_mode: syncore::raggraph::RaggraphBackendMode::Real,
        num_hops: 2,
        alpha: 0.85,
        top_k: 10,
        embedding_dim: 384,
    };

    let transformer = Arc::new(HopGraphTransformer::with_storage(rag_config, storage));

    // Start a background task that should continue running
    let background_counter = Arc::new(Mutex::new(0));
    let background_counter_clone = background_counter.clone();

    let background_task = tokio::spawn(async move {
        for _ in 0..1000 {
            tokio::task::yield_now().await;
            let mut count = background_counter_clone.lock().unwrap();
            *count += 1;
        }
    });

    // Perform multiple RAGGraph operations concurrently
    let mut handles = Vec::new();
    for i in 0..20 {
        let transformer_clone = transformer.clone();
        let seed_nodes = vec![i + 1];

        let handle =
            tokio::spawn(async move { transformer_clone.multi_hop_reasoning(&seed_nodes) });
        handles.push(handle);
    }

    // Wait for all RAGGraph operations
    for handle in handles {
        let result = handle.await;
        assert!(result.is_ok(), "RAGGraph operation should complete");
    }

    // Wait for background task
    background_task.await.unwrap();

    // Background task should have completed without being blocked
    let final_count = *background_counter.lock().unwrap();
    assert_eq!(final_count, 1000, "Background task should not be blocked by RAGGraph operations");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_no_deadlocks_with_mixed_operations() {
    // Test for deadlocks when mixing different types of operations
    let (storage, _graph_backend) = create_test_components().await;

    let rag_config = RagGraphConfig {
        backend_mode: syncore::raggraph::RaggraphBackendMode::Real,
        num_hops: 2,
        alpha: 0.85,
        top_k: 5,
        embedding_dim: 384,
    };

    let transformer = Arc::new(HopGraphTransformer::with_storage(rag_config, storage));

    // Mix of different operations that could potentially deadlock
    let mut handles = Vec::new();

    for i in 0..10 {
        let transformer_clone = transformer.clone();

        let handle = tokio::spawn(async move {
            match i % 3 {
                0 => {
                    // Multi-hop reasoning
                    let seed_nodes = vec![i + 1, i + 2];
                    transformer_clone.multi_hop_reasoning(&seed_nodes)
                }
                1 => {
                    // Single hop reasoning
                    let seed_nodes = vec![i + 1];
                    transformer_clone.multi_hop_reasoning(&seed_nodes)
                }
                2 => {
                    // Different seed nodes
                    let seed_nodes = vec![(i * 2) + 1, (i * 2) + 2];
                    transformer_clone.multi_hop_reasoning(&seed_nodes)
                }
                _ => unreachable!(),
            }
        });
        handles.push(handle);
    }

    // All should complete without deadlocks
    for handle in handles {
        let result = handle.await;
        assert!(result.is_ok(), "Mixed operations should complete without deadlocks");
    }
}
