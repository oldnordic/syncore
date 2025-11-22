//! TDD Tests for Memory Service RAM Cache

use syncore::memory_service::{MemoryEntry, MemoryError, RamCache};

#[test]
fn test_ram_cache_insert_and_len() {
    // Test basic insert and length tracking
    let mut cache = RamCache::new(128, 10);

    assert_eq!(cache.len(), 0);

    let entry = MemoryEntry {
        id: "test1".to_string(),
        summary: "Test summary".to_string(),
        importance: 0.5,
        tags: vec!["tag1".to_string()],
        embedding: vec![0.1; 128],
    };

    cache.insert(entry).expect("Insert should succeed");
    assert_eq!(cache.len(), 1);
}

#[test]
fn test_ram_cache_search_by_cosine_similarity() {
    // Test vector similarity search with hand-crafted embeddings
    let mut cache = RamCache::new(4, 10);

    // Entry 1: [1, 0, 0, 0]
    let entry1 = MemoryEntry {
        id: "e1".to_string(),
        summary: "Entry 1".to_string(),
        importance: 0.5,
        tags: vec![],
        embedding: vec![1.0, 0.0, 0.0, 0.0],
    };

    // Entry 2: [0, 1, 0, 0]
    let entry2 = MemoryEntry {
        id: "e2".to_string(),
        summary: "Entry 2".to_string(),
        importance: 0.5,
        tags: vec![],
        embedding: vec![0.0, 1.0, 0.0, 0.0],
    };

    // Entry 3: [0.7071, 0.7071, 0, 0] (similar to both e1 and e2)
    let entry3 = MemoryEntry {
        id: "e3".to_string(),
        summary: "Entry 3".to_string(),
        importance: 0.5,
        tags: vec![],
        embedding: vec![0.7071, 0.7071, 0.0, 0.0],
    };

    cache.insert(entry1).unwrap();
    cache.insert(entry2).unwrap();
    cache.insert(entry3).unwrap();

    // Query similar to entry1 [1, 0, 0, 0]
    let query = vec![1.0, 0.0, 0.0, 0.0];
    let results = cache.search(&query, 3);

    // Entry 1 should be first (exact match, similarity = 1.0)
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].id, "e1");
}

#[test]
fn test_ram_cache_respects_k() {
    // Test that search returns at most k results
    let mut cache = RamCache::new(4, 20);

    // Insert 10 entries
    for i in 0..10 {
        let entry = MemoryEntry {
            id: format!("e{}", i),
            summary: format!("Entry {}", i),
            importance: 0.5,
            tags: vec![],
            embedding: vec![i as f32 / 10.0; 4],
        };
        cache.insert(entry).unwrap();
    }

    // Search with k=5
    let query = vec![0.5; 4];
    let results = cache.search(&query, 5);

    assert_eq!(results.len(), 5);
}

#[test]
fn test_ram_cache_dimension_mismatch() {
    // Test dimension mismatch error
    let mut cache = RamCache::new(128, 10);

    let wrong_dim_entry = MemoryEntry {
        id: "wrong".to_string(),
        summary: "Wrong dimension".to_string(),
        importance: 0.5,
        tags: vec![],
        embedding: vec![0.1; 256], // Wrong dimension!
    };

    let result = cache.insert(wrong_dim_entry);
    assert!(result.is_err());

    match result {
        Err(MemoryError::DimensionMismatch) => { /* Expected */ }
        _ => panic!("Expected DimensionMismatch error"),
    }

    // Cache should remain empty
    assert_eq!(cache.len(), 0);
}

#[test]
fn test_ram_cache_deterministic_ordering() {
    // Test deterministic ordering when similarities are equal
    let mut cache = RamCache::new(4, 10);

    // Insert entries with identical embeddings (will have same similarity to any query)
    let embedding = vec![0.5; 4];

    let ids = vec!["id_c", "id_a", "id_b"]; // Intentionally unsorted
    for id in ids {
        let entry = MemoryEntry {
            id: id.to_string(),
            summary: format!("Entry {}", id),
            importance: 0.5,
            tags: vec![],
            embedding: embedding.clone(),
        };
        cache.insert(entry).unwrap();
    }

    // Query multiple times
    let query = vec![0.5; 4];
    let results1 = cache.search(&query, 3);
    let results2 = cache.search(&query, 3);

    // Results should be identical across multiple queries
    assert_eq!(results1.len(), results2.len());
    for i in 0..results1.len() {
        assert_eq!(results1[i].id, results2[i].id);
    }

    // Ordering should be deterministic (sorted by ID for ties)
    let result_ids: Vec<String> = results1.iter().map(|e| e.id.clone()).collect();
    let mut sorted_ids = result_ids.clone();
    sorted_ids.sort();
    assert_eq!(result_ids, sorted_ids);
}

#[test]
fn test_ram_cache_empty_search() {
    // Test search on empty cache returns empty results
    let cache = RamCache::new(128, 10);

    let query = vec![0.5; 128];
    let results = cache.search(&query, 5);

    assert_eq!(results.len(), 0);
}

#[test]
fn test_ram_cache_capacity_eviction() {
    // Test that oldest entries are evicted when capacity exceeded
    let mut cache = RamCache::new(4, 3); // Capacity of 3

    let e1 = MemoryEntry {
        id: "e1".to_string(),
        summary: "First".to_string(),
        importance: 0.5,
        tags: vec![],
        embedding: vec![0.1; 4],
    };

    let e2 = MemoryEntry {
        id: "e2".to_string(),
        summary: "Second".to_string(),
        importance: 0.5,
        tags: vec![],
        embedding: vec![0.2; 4],
    };

    let e3 = MemoryEntry {
        id: "e3".to_string(),
        summary: "Third".to_string(),
        importance: 0.5,
        tags: vec![],
        embedding: vec![0.3; 4],
    };

    let e4 = MemoryEntry {
        id: "e4".to_string(),
        summary: "Fourth".to_string(),
        importance: 0.5,
        tags: vec![],
        embedding: vec![0.4; 4],
    };

    cache.insert(e1).unwrap();
    cache.insert(e2).unwrap();
    cache.insert(e3).unwrap();

    assert_eq!(cache.len(), 3);

    // Insert 4th entry - should evict oldest (e1)
    cache.insert(e4).unwrap();

    assert_eq!(cache.len(), 3);

    // Search to verify e1 is gone, e2/e3/e4 remain
    let query = vec![0.25; 4];
    let results = cache.search(&query, 10);

    let ids: Vec<String> = results.iter().map(|e| e.id.clone()).collect();
    assert!(!ids.contains(&"e1".to_string()));
    assert!(ids.contains(&"e2".to_string()));
    assert!(ids.contains(&"e3".to_string()));
    assert!(ids.contains(&"e4".to_string()));
}
