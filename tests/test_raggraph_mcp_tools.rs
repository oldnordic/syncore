// Integration test for RagGraph MCP tools

use syncore::raggraph::{HopGraphTransformer, RagGraphConfig, RagQuery};

#[test]
fn test_raggraph_query_tool_backend() {
    // Test the RagQuery engine that backs the raggraph_query MCP tool
    let query_engine = RagQuery::new();

    let result = query_engine.query("test query about AI").expect("Query should succeed");

    // Verify result structure
    assert!(!result.top_nodes.is_empty(), "Should return top nodes");
    assert_eq!(result.context_embedding.len(), 384, "Should have 384-dim embedding");
    assert!(!result.reasoning_path.is_empty(), "Should have reasoning path");
}

#[test]
fn test_raggraph_multihop_tool_backend() {
    // Test the HopGraphTransformer that backs the raggraph_multihop MCP tool
    let config = RagGraphConfig::default();
    let transformer = HopGraphTransformer::new(config);

    let seed_nodes = vec![1, 2, 3];
    let result = transformer.multi_hop_reasoning(&seed_nodes).expect("Multi-hop should succeed");

    // Verify result structure
    assert!(!result.top_nodes.is_empty(), "Should return top nodes");
    assert_eq!(result.context_embedding.len(), 384, "Should have 384-dim embedding");
    assert!(!result.reasoning_path.is_empty(), "Should have reasoning path");
}

#[test]
fn test_raggraph_query_empty_query() {
    // Test error handling for empty query
    let query_engine = RagQuery::new();

    let result = query_engine.query("");
    assert!(result.is_err(), "Empty query should return error");
    assert!(result.unwrap_err().to_string().contains("empty"), "Error should mention empty query");
}

#[test]
fn test_raggraph_multihop_empty_seeds() {
    // Test error handling for empty seed nodes
    let config = RagGraphConfig::default();
    let transformer = HopGraphTransformer::new(config);

    let empty_seeds: Vec<i64> = vec![];
    let result = transformer.multi_hop_reasoning(&empty_seeds);
    assert!(result.is_err(), "Empty seeds should return error");
    assert!(result.unwrap_err().to_string().contains("empty"), "Error should mention empty seeds");
}

#[test]
fn test_raggraph_query_deterministic() {
    // Verify deterministic behavior (same query = same seeds)
    let query_engine = RagQuery::new();

    let result1 = query_engine.query("AI research").expect("First query should succeed");
    let result2 = query_engine.query("AI research").expect("Second query should succeed");

    // Same query should produce same seed nodes, which leads to same set of top nodes
    // (ordering may vary due to HashMap iteration order, but set should be identical)
    use std::collections::HashSet;
    let set1: HashSet<_> = result1.top_nodes.iter().collect();
    let set2: HashSet<_> = result2.top_nodes.iter().collect();
    assert_eq!(set1, set2, "Same query should produce same set of nodes");

    // Also verify count is the same
    assert_eq!(
        result1.top_nodes.len(),
        result2.top_nodes.len(),
        "Same query should produce same node count"
    );
}

// ========================================
// Real Backend Integration Tests
// ========================================

use std::sync::{Arc, Mutex};
use syncore::vector::{RealEmbeddings, VectorIndex, VectorMeta, VectorStore};

#[test]
fn test_vector_store_implements_vectorindex() {
    // Test that VectorStore correctly implements VectorIndex trait
    // (Required for Real backend integration in MCP server)
    let dim = 384;
    let embeddings = Box::new(RealEmbeddings::new(dim).expect("Failed to create embeddings"));
    let meta = VectorMeta {
        dim,
        m: 32,
        ef_construction: 200,
        ef_search: 100,
    };
    let mut vector_store = VectorStore::with_meta(embeddings, meta);

    // Add test vectors
    vector_store.add(1, vec![0.5; dim]).expect("Failed to add vector");

    // Verify VectorIndex trait methods work
    assert_eq!(VectorIndex::dimension(&vector_store), Some(dim));
    assert_eq!(VectorIndex::len(&vector_store), 1);

    // Verify VectorStore can be cast to VectorIndex trait object (as MCP server does)
    let _vector_index: Arc<Mutex<dyn VectorIndex>> = Arc::new(Mutex::new(vector_store));
}

#[test]
fn test_vector_store_search_functionality() {
    // Test that VectorStore search works correctly (required for Real backend seed generation)
    let dim = 128;
    let embeddings = Box::new(RealEmbeddings::new(dim).expect("Failed to create embeddings"));
    let meta = VectorMeta {
        dim,
        m: 16,
        ef_construction: 100,
        ef_search: 50,
    };
    let mut vector_store = VectorStore::with_meta(embeddings, meta);

    // Add test vectors
    for i in 1..=10 {
        let vec = vec![i as f32 / 10.0; dim];
        vector_store.add(i as i64, vec).expect(&format!("Failed to add vector {}", i));
    }

    // Test vector search via VectorIndex trait (as Real backend uses it)
    let query_vec = vec![0.5; dim];
    let results = VectorIndex::search(&vector_store, &query_vec, 5).expect("Search should succeed");

    // Verify search results
    assert!(results.len() <= 5, "Should return at most 5 results");
    assert!(!results.is_empty(), "Should return at least one result");

    // Verify scores are valid (cosine similarity: -1 to 1, but typically 0 to 1 for normalized vectors)
    for (_node_id, score) in results {
        assert!(score >= -1.0 && score <= 1.0, "Score should be in [-1, 1] range");
    }
}

#[test]
fn test_vector_store_trait_methods() {
    // Test all VectorIndex trait methods on VectorStore (as MCP server uses them)
    let dim = 256;
    let embeddings = Box::new(RealEmbeddings::new(dim).expect("Failed to create embeddings"));
    let meta = VectorMeta {
        dim,
        m: 24,
        ef_construction: 150,
        ef_search: 75,
    };
    let mut vector_store = VectorStore::with_meta(embeddings, meta);

    // Test dimension() before and after adding vectors
    assert_eq!(VectorIndex::dimension(&vector_store), Some(dim));

    // Test len() with empty store
    assert_eq!(VectorIndex::len(&vector_store), 0);

    // Add test vectors
    for i in 1..=5 {
        let vec = vec![i as f32 / 5.0; dim];
        VectorIndex::add(&mut vector_store, i as i64, vec).expect("Add should succeed");
    }

    // Test len() after adding
    assert_eq!(VectorIndex::len(&vector_store), 5);

    // Test search()
    let query = vec![0.5; dim];
    let results = VectorIndex::search(&vector_store, &query, 3).expect("Search should succeed");
    assert_eq!(results.len(), 3, "Should return exactly 3 results");

    // Test dimension() after adding (should remain constant)
    assert_eq!(VectorIndex::dimension(&vector_store), Some(dim));
}

#[test]
fn test_mcp_server_cast_pattern() {
    // Test the exact casting pattern used in MCP server for Real backend integration
    let dim = 384;
    let embeddings = Box::new(RealEmbeddings::new(dim).expect("Failed to create embeddings"));
    let meta = VectorMeta {
        dim,
        m: 32,
        ef_construction: 200,
        ef_search: 100,
    };
    let vector_store = VectorStore::with_meta(embeddings, meta);

    // Wrap in Arc<Mutex<>> first (simulating SynCoreState.vector_store)
    let wrapped = Arc::new(Mutex::new(vector_store));

    // Cast to trait object (as mcp_server.rs does)
    let vector_index: Arc<Mutex<dyn VectorIndex>> = wrapped.clone() as Arc<Mutex<dyn VectorIndex>>;

    // Test that we can still use VectorIndex methods through the trait object
    let dimension = {
        let store = vector_index.lock().unwrap();
        VectorIndex::dimension(&*store)
    };

    assert_eq!(dimension, Some(dim), "Dimension should be accessible through trait object");
}
