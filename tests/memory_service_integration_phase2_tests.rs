//! TDD Integration Tests for MemoryService Phase 2 (RAM + LTM)

use syncore::db::DbManager;
use syncore::memory_service::{MemoryEntry, MemoryService};

fn setup_test_service_with_ltm() -> MemoryService {
    // Create in-memory databases for testing
    let db_manager =
        DbManager::new(":memory:", ":memory:").expect("Failed to create test DbManager");

    // Create MemoryService with RAM cache + LTM adapter (Mock backend)
    MemoryService::new_with_ltm(128, 10, db_manager)
        .expect("Failed to create MemoryService with LTM")
}

#[test]
fn test_memory_service_store_uses_ram_and_ltm() {
    // Test that store() writes to both RAM and LTM
    let mut service = setup_test_service_with_ltm();

    let entry = MemoryEntry {
        id: "test_entry".to_string(),
        summary: "Test entry".to_string(),
        importance: 0.8,
        tags: vec!["test".to_string()],
        embedding: vec![0.5; 128],
    };

    let result = service.store(entry);
    assert!(result.is_ok(), "Store should succeed");

    // Verify stats show entry in both RAM and LTM
    let stats = service.stats();
    assert_eq!(stats.ram_size, 1, "Should have 1 entry in RAM");
    assert_eq!(stats.ltm_nodes, 1, "Should have 1 entry in LTM SQL");
}

#[test]
fn test_memory_service_retrieve_merges_ram_and_ltm() {
    // Test that retrieve() merges results from RAM and LTM
    let mut service = setup_test_service_with_ltm();

    // Store 3 entries in both RAM and LTM
    for i in 0..3 {
        let mut emb = vec![i as f32, 0.0, 0.0, 0.0];
        emb.extend(vec![0.0; 124]);

        let entry = MemoryEntry {
            id: format!("entry{}", i),
            summary: format!("Entry {}", i),
            importance: 0.5,
            tags: vec![],
            embedding: emb,
        };
        service.store(entry).expect("Store should succeed");
    }

    // Query with k=5 (more than we have, so all results returned)
    let mut query = vec![0.5, 0.0, 0.0, 0.0];
    query.extend(vec![0.0; 124]);
    let results = service.retrieve(&query, 5);

    // Should get results from both RAM and LTM, but deduplicated
    assert!(results.len() >= 3, "Should have at least 3 results");
    assert!(
        results.len() <= 6,
        "Should not exceed 2x due to duplication"
    );
}

#[test]
fn test_memory_service_retrieve_deduplicates_results() {
    // Test that retrieve() deduplicates entries present in both RAM and LTM
    let mut service = setup_test_service_with_ltm();

    let entry = MemoryEntry {
        id: "duplicate_entry".to_string(),
        summary: "Entry in both RAM and LTM".to_string(),
        importance: 0.7,
        tags: vec![],
        embedding: vec![0.5; 128],
    };

    service.store(entry).expect("Store should succeed");

    let query = vec![0.5; 128];
    let results = service.retrieve(&query, 10);

    // Should only appear once despite being in both RAM and LTM
    let duplicate_count = results.iter().filter(|e| e.id == "duplicate_entry").count();
    assert_eq!(
        duplicate_count, 1,
        "Entry should appear exactly once (deduplicated)"
    );
}

#[test]
fn test_memory_service_retrieve_stable_sort() {
    // Test that retrieve() sorts results deterministically
    let mut service = setup_test_service_with_ltm();

    // Store entries with similar embeddings to test tie-breaking
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

    service.store(entry1).expect("Store should succeed");
    service.store(entry2).expect("Store should succeed");

    let query = vec![0.5; 128];

    // Query multiple times
    let results1 = service.retrieve(&query, 5);
    let results2 = service.retrieve(&query, 5);
    let results3 = service.retrieve(&query, 5);

    // Results should be identical across queries (deterministic)
    assert_eq!(results1.len(), results2.len());
    assert_eq!(results1.len(), results3.len());

    for i in 0..results1.len() {
        assert_eq!(results1[i].id, results2[i].id);
        assert_eq!(results1[i].id, results3[i].id);
    }

    // With equal similarities, should sort by ID
    if results1.len() >= 2 {
        let ids: Vec<String> = results1.iter().map(|e| e.id.clone()).collect();
        // a_entry should come before b_entry (alphabetical)
        let a_pos = ids.iter().position(|id| id == "a_entry");
        let b_pos = ids.iter().position(|id| id == "b_entry");

        if let (Some(a), Some(b)) = (a_pos, b_pos) {
            assert!(a < b, "a_entry should come before b_entry");
        }
    }
}

#[test]
fn test_memory_service_stats_combines_ram_and_ltm() {
    // Test that stats() returns combined statistics from RAM and LTM
    let mut service = setup_test_service_with_ltm();

    // Initially empty
    let stats = service.stats();
    assert_eq!(stats.ram_size, 0);
    assert_eq!(stats.ltm_nodes, 0);

    // Store 5 entries
    for i in 0..5 {
        let entry = MemoryEntry {
            id: format!("entry{}", i),
            summary: format!("Entry {}", i),
            importance: 0.5,
            tags: vec![],
            embedding: vec![i as f32 / 10.0; 128],
        };
        service.store(entry).expect("Store should succeed");
    }

    // Check combined stats
    let stats = service.stats();
    assert_eq!(stats.ram_size, 5, "Should have 5 entries in RAM");
    assert_eq!(stats.ltm_nodes, 5, "Should have 5 entries in LTM SQL");
}

#[test]
fn test_memory_service_retrieve_respects_k_with_ltm() {
    // Test that retrieve() respects k parameter with merged RAM+LTM results
    let mut service = setup_test_service_with_ltm();

    // Store 10 entries
    for i in 0..10 {
        let entry = MemoryEntry {
            id: format!("entry{}", i),
            summary: format!("Entry {}", i),
            importance: 0.5,
            tags: vec![],
            embedding: vec![i as f32 / 10.0; 128],
        };
        service.store(entry).expect("Store should succeed");
    }

    // Query with k=3
    let query = vec![0.5; 128];
    let results = service.retrieve(&query, 3);

    // After deduplication, should have exactly 3 results
    assert_eq!(results.len(), 3, "Should return exactly 3 results");
}

#[test]
fn test_memory_service_ltm_only_after_ram_eviction() {
    // Test that entries evicted from RAM can still be retrieved from LTM
    let mut service = setup_test_service_with_ltm();

    // Store more entries than RAM capacity (capacity = 10)
    for i in 0..15 {
        let entry = MemoryEntry {
            id: format!("entry{}", i),
            summary: format!("Entry {}", i),
            importance: 0.5,
            tags: vec![],
            embedding: vec![i as f32 / 10.0; 128],
        };
        service.store(entry).expect("Store should succeed");
    }

    // RAM should have only 10 entries (capacity limit)
    let stats = service.stats();
    assert_eq!(stats.ram_size, 10, "RAM should have capacity limit entries");
    assert_eq!(stats.ltm_nodes, 15, "LTM should have all 15 entries");

    // Query for early entries (likely evicted from RAM)
    let mut query = vec![0.0, 0.0, 0.0, 0.0];
    query.extend(vec![0.0; 124]);

    let results = service.retrieve(&query, 5);

    // Should still find entries from LTM even if not in RAM
    assert!(results.len() > 0, "Should retrieve entries from LTM");

    // entry0 should be in results (even if evicted from RAM)
    let has_entry0 = results.iter().any(|e| e.id == "entry0");
    assert!(
        has_entry0,
        "Should find entry0 via LTM even if evicted from RAM"
    );
}

#[test]
fn test_memory_service_without_ltm_still_works() {
    // Test that MemoryService works without LTM (Phase 1 compatibility)
    let mut service = MemoryService::new(128, 10);

    let entry = MemoryEntry {
        id: "ram_only".to_string(),
        summary: "RAM only entry".to_string(),
        importance: 0.5,
        tags: vec![],
        embedding: vec![0.5; 128],
    };

    service.store(entry).expect("Store should succeed");

    let query = vec![0.5; 128];
    let results = service.retrieve(&query, 5);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "ram_only");

    let stats = service.stats();
    assert_eq!(stats.ram_size, 1);
    assert_eq!(stats.ltm_nodes, 0, "LTM nodes should be 0 without LTM");
}
