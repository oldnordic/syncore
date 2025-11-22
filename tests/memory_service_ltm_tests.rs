//! TDD Tests for LTM Adapter

use std::sync::{Arc, Mutex};
use syncore::db::DbManager;
use syncore::memory_service::ltm_adapter::{LongTermStore, LtmAdapter, LtmStats};
use syncore::memory_service::{MemoryEntry, MemoryError};

fn setup_test_adapter() -> LtmAdapter {
    // Use in-memory databases for testing
    let db_manager =
        DbManager::new(":memory:", ":memory:").expect("Failed to create test DbManager");

    // Create LtmAdapter with Mock backend (no real Neo4j/HNSW needed for Phase 2 tests)
    LtmAdapter::new_with_mock(db_manager, 128).expect("Failed to create LtmAdapter")
}

#[test]
fn test_ltm_store_creates_node_and_sql_record() {
    // Test that ltm_store creates both a graph node and SQL record
    let mut adapter = setup_test_adapter();

    let entry = MemoryEntry {
        id: "test_entry_1".to_string(),
        summary: "Test memory entry".to_string(),
        importance: 0.8,
        tags: vec!["test".to_string(), "memory".to_string()],
        embedding: vec![0.5; 128],
    };

    let result = adapter.ltm_store(&entry);
    assert!(result.is_ok(), "ltm_store should succeed");

    let node_id = result.unwrap();
    assert!(!node_id.is_empty(), "node_id should not be empty");

    // Verify stats show the entry was stored
    let stats = adapter.ltm_stats().expect("stats should succeed");
    assert_eq!(stats.sql_rows, 1, "Should have 1 SQL row");
}

#[test]
fn test_ltm_store_validates_dimension() {
    // Test that ltm_store rejects wrong dimension
    let mut adapter = setup_test_adapter();

    let entry = MemoryEntry {
        id: "bad_dimension".to_string(),
        summary: "Entry with wrong dimension".to_string(),
        importance: 0.5,
        tags: vec![],
        embedding: vec![0.5; 64], // Wrong dimension (expected 128)
    };

    let result = adapter.ltm_store(&entry);
    assert!(
        result.is_err(),
        "ltm_store should fail with wrong dimension"
    );

    match result {
        Err(MemoryError::DimensionMismatch) => {} // Expected error
        _ => panic!("Expected DimensionMismatch error"),
    }
}

#[test]
fn test_ltm_query_returns_relevant_nodes() {
    // Test that ltm_query returns entries similar to query
    let mut adapter = setup_test_adapter();

    // Store several entries with distinct embeddings
    let entry1 = MemoryEntry {
        id: "entry1".to_string(),
        summary: "First entry about cats".to_string(),
        importance: 0.8,
        tags: vec!["animals".to_string()],
        embedding: {
            let mut emb = vec![1.0, 0.0, 0.0, 0.0];
            emb.extend(vec![0.0; 124]);
            emb
        },
    };

    let entry2 = MemoryEntry {
        id: "entry2".to_string(),
        summary: "Second entry about dogs".to_string(),
        importance: 0.6,
        tags: vec!["animals".to_string()],
        embedding: {
            let mut emb = vec![0.0, 1.0, 0.0, 0.0];
            emb.extend(vec![0.0; 124]);
            emb
        },
    };

    let entry3 = MemoryEntry {
        id: "entry3".to_string(),
        summary: "Third entry about programming".to_string(),
        importance: 0.9,
        tags: vec!["tech".to_string()],
        embedding: {
            let mut emb = vec![0.0, 0.0, 1.0, 0.0];
            emb.extend(vec![0.0; 124]);
            emb
        },
    };

    adapter
        .ltm_store(&entry1)
        .expect("Store entry1 should succeed");
    adapter
        .ltm_store(&entry2)
        .expect("Store entry2 should succeed");
    adapter
        .ltm_store(&entry3)
        .expect("Store entry3 should succeed");

    // Query with embedding similar to entry1
    let mut query = vec![1.0, 0.0, 0.0, 0.0];
    query.extend(vec![0.0; 124]);

    let results = adapter.ltm_query(&query, 2).expect("Query should succeed");

    assert_eq!(results.len(), 2, "Should return 2 results");
    assert_eq!(
        results[0].id, "entry1",
        "Most similar entry should be first"
    );
}

#[test]
fn test_ltm_query_respects_k() {
    // Test that ltm_query respects the k parameter
    let mut adapter = setup_test_adapter();

    // Store 5 entries
    for i in 0..5 {
        let entry = MemoryEntry {
            id: format!("entry{}", i),
            summary: format!("Entry {}", i),
            importance: 0.5,
            tags: vec![],
            embedding: vec![i as f32 / 10.0; 128],
        };
        adapter.ltm_store(&entry).expect("Store should succeed");
    }

    // Query with k=3
    let query = vec![0.5; 128];
    let results = adapter.ltm_query(&query, 3).expect("Query should succeed");

    assert_eq!(results.len(), 3, "Should return exactly 3 results");
}

#[test]
fn test_ltm_query_deterministic_ordering() {
    // Test that ltm_query returns deterministic results across multiple queries
    let mut adapter = setup_test_adapter();

    // Store entries with identical embeddings to test tie-breaking
    let entry1 = MemoryEntry {
        id: "b_entry".to_string(),
        summary: "Entry B".to_string(),
        importance: 0.5,
        tags: vec![],
        embedding: vec![0.5; 128],
    };

    let entry2 = MemoryEntry {
        id: "a_entry".to_string(),
        summary: "Entry A".to_string(),
        importance: 0.5,
        tags: vec![],
        embedding: vec![0.5; 128],
    };

    adapter.ltm_store(&entry1).expect("Store should succeed");
    adapter.ltm_store(&entry2).expect("Store should succeed");

    let query = vec![0.5; 128];

    // Query multiple times
    let results1 = adapter.ltm_query(&query, 2).expect("Query should succeed");
    let results2 = adapter.ltm_query(&query, 2).expect("Query should succeed");
    let results3 = adapter.ltm_query(&query, 2).expect("Query should succeed");

    // All results should be identical (deterministic)
    assert_eq!(results1.len(), 2);
    assert_eq!(results1[0].id, results2[0].id);
    assert_eq!(results1[1].id, results2[1].id);
    assert_eq!(results2[0].id, results3[0].id);
    assert_eq!(results2[1].id, results3[1].id);

    // Should be sorted by ID for tie-breaking
    assert_eq!(results1[0].id, "a_entry");
    assert_eq!(results1[1].id, "b_entry");
}

#[test]
fn test_ltm_stats_counts_nodes_edges_sql() {
    // Test that ltm_stats returns accurate counts
    let mut adapter = setup_test_adapter();

    // Initially empty
    let stats = adapter.ltm_stats().expect("stats should succeed");
    assert_eq!(stats.sql_rows, 0);

    // Store some entries
    for i in 0..3 {
        let entry = MemoryEntry {
            id: format!("entry{}", i),
            summary: format!("Entry {}", i),
            importance: 0.5,
            tags: vec![],
            embedding: vec![i as f32 / 10.0; 128],
        };
        adapter.ltm_store(&entry).expect("Store should succeed");
    }

    // Check stats updated
    let stats = adapter.ltm_stats().expect("stats should succeed");
    assert_eq!(stats.sql_rows, 3, "Should have 3 SQL rows");
}

#[test]
fn test_ltm_query_empty_returns_empty() {
    // Test that querying empty adapter returns empty results
    let adapter = setup_test_adapter();

    let query = vec![0.5; 128];
    let results = adapter.ltm_query(&query, 5).expect("Query should succeed");

    assert_eq!(
        results.len(),
        0,
        "Empty adapter should return empty results"
    );
}

#[test]
fn test_ltm_store_preserves_all_fields() {
    // Test that all MemoryEntry fields are preserved through store/query
    let mut adapter = setup_test_adapter();

    let entry = MemoryEntry {
        id: "full_entry".to_string(),
        summary: "Complete entry with all fields".to_string(),
        importance: 0.75,
        tags: vec!["tag1".to_string(), "tag2".to_string(), "tag3".to_string()],
        embedding: vec![0.3; 128],
    };

    adapter.ltm_store(&entry).expect("Store should succeed");

    let query = vec![0.3; 128];
    let results = adapter.ltm_query(&query, 1).expect("Query should succeed");

    assert_eq!(results.len(), 1);
    let retrieved = &results[0];

    assert_eq!(retrieved.id, "full_entry");
    assert_eq!(retrieved.summary, "Complete entry with all fields");
    assert_eq!(retrieved.importance, 0.75);
    assert_eq!(retrieved.tags.len(), 3);
    assert!(retrieved.tags.contains(&"tag1".to_string()));
    assert!(retrieved.tags.contains(&"tag2".to_string()));
    assert!(retrieved.tags.contains(&"tag3".to_string()));
    assert_eq!(retrieved.embedding.len(), 128);
}
