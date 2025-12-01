//! TDD Tests for MemoryService Core

use syncore::memory_service::{MemoryEntry, MemoryService};

#[test]
fn test_memory_service_store_and_retrieve_basic() {
    // Test basic store and retrieve workflow
    let mut service = MemoryService::new(128, 10);

    // Store 3 entries
    let mut emb1 = vec![1.0, 0.0, 0.0, 0.0];
    emb1.extend(vec![0.0; 124]);
    let entry1 = MemoryEntry {
        id: "entry1".to_string(),
        summary: "First entry about cats".to_string(),
        importance: 0.8,
        tags: vec!["animals".to_string()],
        embedding: emb1,
    };

    let mut emb2 = vec![0.0, 1.0, 0.0, 0.0];
    emb2.extend(vec![0.0; 124]);
    let entry2 = MemoryEntry {
        id: "entry2".to_string(),
        summary: "Second entry about dogs".to_string(),
        importance: 0.6,
        tags: vec!["animals".to_string()],
        embedding: emb2,
    };

    let mut emb3 = vec![0.0, 0.0, 1.0, 0.0];
    emb3.extend(vec![0.0; 124]);
    let entry3 = MemoryEntry {
        id: "entry3".to_string(),
        summary: "Third entry about programming".to_string(),
        importance: 0.9,
        tags: vec!["tech".to_string()],
        embedding: emb3,
    };

    service.store(entry1).expect("Store should succeed");
    service.store(entry2).expect("Store should succeed");
    service.store(entry3).expect("Store should succeed");

    // Retrieve entries similar to entry1
    let mut query = vec![1.0, 0.0, 0.0, 0.0];
    query.extend(vec![0.0; 124]);
    let results = service.retrieve(&query, 2);

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].id, "entry1"); // Most similar
}

#[test]
fn test_memory_service_dimension_getter() {
    // Test that dimension getter returns the configured dimension
    let dimension = 256;
    let capacity = 50;
    let service = MemoryService::new(dimension, capacity);

    assert_eq!(
        service.dimension(),
        dimension,
        "Dimension getter should return configured dimension"
    );

    // Test that stats include dimension
    let stats = service.stats();
    assert_eq!(stats.dimension, dimension, "Stats should include dimension");
}

#[test]
fn test_memory_service_empty_results() {
    // Test retrieve on empty service
    let service = MemoryService::new(128, 10);

    let query = vec![0.5; 128];
    let results = service.retrieve(&query, 5);

    assert_eq!(results.len(), 0);
}

#[test]
fn test_memory_service_importance_field_preserved() {
    // Test that importance field is preserved through store/retrieve
    let mut service = MemoryService::new(4, 10);

    let entry = MemoryEntry {
        id: "test".to_string(),
        summary: "Test entry".to_string(),
        importance: 0.75,
        tags: vec![],
        embedding: vec![1.0, 0.0, 0.0, 0.0],
    };

    service.store(entry).expect("Store should succeed");

    let query = vec![1.0, 0.0, 0.0, 0.0];
    let results = service.retrieve(&query, 1);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].importance, 0.75);
}

#[test]
fn test_memory_service_handles_multiple_tags() {
    // Test that multiple tags are preserved
    let mut service = MemoryService::new(4, 10);

    let entry = MemoryEntry {
        id: "multi_tag".to_string(),
        summary: "Entry with multiple tags".to_string(),
        importance: 0.5,
        tags: vec!["tag1".to_string(), "tag2".to_string(), "tag3".to_string()],
        embedding: vec![0.5; 4],
    };

    service.store(entry).expect("Store should succeed");

    let query = vec![0.5; 4];
    let results = service.retrieve(&query, 1);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].tags.len(), 3);
    assert!(results[0].tags.contains(&"tag1".to_string()));
    assert!(results[0].tags.contains(&"tag2".to_string()));
    assert!(results[0].tags.contains(&"tag3".to_string()));
}

#[test]
fn test_memory_service_stats() {
    // Test statistics reporting
    let mut service = MemoryService::new(4, 10);

    // Initially empty
    let stats = service.stats();
    assert_eq!(stats.ram_size, 0);

    // After storing entries
    for i in 0..5 {
        let entry = MemoryEntry {
            id: format!("entry{}", i),
            summary: format!("Entry {}", i),
            importance: 0.5,
            tags: vec![],
            embedding: vec![i as f32 / 10.0; 4],
        };
        service.store(entry).expect("Store should succeed");
    }

    let stats = service.stats();
    assert_eq!(stats.ram_size, 5);
}

#[test]
fn test_memory_service_retrieve_respects_k() {
    // Test that retrieve respects the k parameter
    let mut service = MemoryService::new(4, 20);

    // Store 10 entries
    for i in 0..10 {
        let entry = MemoryEntry {
            id: format!("e{}", i),
            summary: format!("Entry {}", i),
            importance: 0.5,
            tags: vec![],
            embedding: vec![i as f32 / 10.0; 4],
        };
        service.store(entry).expect("Store should succeed");
    }

    // Retrieve with k=3
    let query = vec![0.5; 4];
    let results = service.retrieve(&query, 3);

    assert_eq!(results.len(), 3);
}
