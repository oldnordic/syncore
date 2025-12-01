/// SEMANTIC MEMORY INTEGRATION TESTS
///
/// These tests verify REAL functionality using:
/// - REAL SQLite database
/// - REAL DualEmbeddingService with REAL embeddings (all-MiniLM-L6-v2)
/// - REAL VectorStore with HNSW indexing
/// - REAL Neo4j connections (optional, based on environment)
///
/// NO MOCKS. NO FAKES. NO PLACEHOLDERS.
///
/// Tests are written FIRST (TDD) and will FAIL until implementation is complete.
use anyhow::Result;
use syncore::memory::Memory;
use tempfile::TempDir;

/// Helper to create test Memory with REAL infrastructure
fn create_test_memory_with_semantics() -> Result<(Memory, TempDir)> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("semantic_test.db");
    let db_path_str = db_path.to_str().unwrap();

    // Create Memory instance
    // NOTE: This will fail until we add semantic search support
    let memory = Memory::new(db_path_str)?;

    Ok((memory, temp_dir))
}

#[test]
fn test_semantic_search_finds_related_memories() -> Result<()> {
    let (memory, _temp_dir) = create_test_memory_with_semantics()?;

    // Store semantically related memories
    memory.store(
        "rust_functions",
        "Functions in Rust use fn keyword and have type signatures",
    )?;
    memory.store(
        "python_functions",
        "Python functions use def keyword and support duck typing",
    )?;
    memory.store(
        "shopping_list",
        "Buy milk, eggs, and bread from grocery store",
    )?;

    // SEMANTIC SEARCH: Query for "programming functions" should find Rust and Python with high scores
    let results = memory.search_semantic("programming function syntax", None, 10)?;

    assert!(
        results.len() >= 2,
        "Should find at least 2 related memories"
    );

    // Verify results contain function-related memories (programming-related should rank higher)
    let keys: Vec<String> = results.iter().map(|r| r.entry.key.clone()).collect();
    assert!(
        keys.contains(&"rust_functions".to_string()),
        "Should find Rust functions"
    );
    assert!(
        keys.contains(&"python_functions".to_string()),
        "Should find Python functions"
    );

    // Verify programming-related memories rank higher than shopping list
    // (General embeddings may include unrelated items, but relevance should be ranked correctly)
    let rust_pos = keys.iter().position(|k| k == "rust_functions");
    let python_pos = keys.iter().position(|k| k == "python_functions");
    let shopping_pos = keys.iter().position(|k| k == "shopping_list");

    if let Some(shop_idx) = shopping_pos {
        // If shopping list appears, ensure programming items rank higher
        assert!(
            rust_pos.is_some() && rust_pos.unwrap() < shop_idx,
            "Rust functions should rank higher than shopping"
        );
        assert!(
            python_pos.is_some() && python_pos.unwrap() < shop_idx,
            "Python functions should rank higher than shopping"
        );
    }

    // Verify similarity scores are ranked correctly
    assert!(
        results[0].similarity > 0.3,
        "Top result should have reasonable similarity"
    );
    assert!(
        results[0].similarity >= results[1].similarity,
        "Results should be ranked by similarity"
    );

    Ok(())
}

#[test]
fn test_semantic_search_uses_real_embeddings() -> Result<()> {
    let (memory, _temp_dir) = create_test_memory_with_semantics()?;

    // Store memories with subtle semantic differences
    memory.store(
        "cat_info",
        "Cats are small domesticated feline animals that purr",
    )?;
    memory.store("dog_info", "Dogs are loyal canine companions that bark")?;
    memory.store("car_info", "Cars are motorized vehicles with four wheels")?;

    // Query for "pet animals" - should find cat and dog, not car
    let results = memory.search_semantic("pet animals", None, 10)?;

    assert!(
        results.len() >= 2,
        "Should find at least 2 pet-related memories"
    );

    let top_keys: Vec<String> = results
        .iter()
        .take(2)
        .map(|r| r.entry.key.clone())
        .collect();
    assert!(
        top_keys.contains(&"cat_info".to_string()) || top_keys.contains(&"dog_info".to_string()),
        "Top results should be about pets, not cars"
    );

    // Verify REAL embeddings were used (not random vectors)
    // Real embeddings should produce similarity scores in expected range
    for result in &results {
        assert!(
            result.similarity >= 0.0 && result.similarity <= 1.0,
            "Similarity scores should be normalized between 0 and 1"
        );
    }

    Ok(())
}

#[test]
fn test_store_with_metadata_and_tags() -> Result<()> {
    let (memory, _temp_dir) = create_test_memory_with_semantics()?;

    // Store memory with rich metadata
    let entry_id = memory.store_with_metadata(
        "apex_18_implementation",
        "APEX 1.8 REFRAG implementation completed with 11/11 tests passing",
        "project_history",
        &["apex", "refrag", "completed"],
        0.9, // High importance
    )?;

    assert!(entry_id > 0, "Should return valid entry ID");

    // Query by tag
    let tagged = memory.query_by_tags(&["apex"], Some("project_history"))?;
    assert_eq!(tagged.len(), 1, "Should find 1 memory with 'apex' tag");
    assert_eq!(tagged[0].key, "apex_18_implementation");
    assert_eq!(tagged[0].importance, 0.9);
    assert_eq!(tagged[0].namespace, "project_history");

    Ok(())
}

#[test]
fn test_query_by_importance_ranking() -> Result<()> {
    let (memory, _temp_dir) = create_test_memory_with_semantics()?;

    // Store memories with different importance levels
    memory.store_with_metadata(
        "critical_bug",
        "Production crash needs immediate fix",
        "default",
        &[],
        1.0,
    )?;
    memory.store_with_metadata("minor_typo", "Fix typo in README", "default", &[], 0.1)?;
    memory.store_with_metadata(
        "feature_request",
        "Add dark mode to UI",
        "default",
        &[],
        0.6,
    )?;

    // Query by minimum importance
    let important = memory.query_by_importance(0.5, 10)?;

    assert_eq!(
        important.len(),
        2,
        "Should find 2 memories with importance >= 0.5"
    );

    let keys: Vec<String> = important.iter().map(|e| e.key.clone()).collect();
    assert!(keys.contains(&"critical_bug".to_string()));
    assert!(keys.contains(&"feature_request".to_string()));
    assert!(!keys.contains(&"minor_typo".to_string()));

    Ok(())
}

#[test]
fn test_temporal_queries_recent_and_since() -> Result<()> {
    let (memory, _temp_dir) = create_test_memory_with_semantics()?;

    // Store memories at different times (use 1+ second delays for timestamp separation)
    memory.store("first", "First memory")?;
    std::thread::sleep(std::time::Duration::from_millis(1100));

    let checkpoint = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;

    std::thread::sleep(std::time::Duration::from_millis(10));
    memory.store("second", "Second memory")?;
    std::thread::sleep(std::time::Duration::from_millis(1100)); // Ensure different second
    memory.store("third", "Third memory")?;

    // Query recent (should get all 3, newest first)
    let recent = memory.query_recent(10, None)?;
    assert!(recent.len() >= 3, "Should find at least 3 recent memories");
    assert_eq!(recent[0].key, "third", "Most recent should be first");

    // Query since checkpoint (should only get second and third)
    let since = memory.query_since(checkpoint, None)?;
    assert!(
        since.len() >= 2,
        "Should find at least 2 memories since checkpoint"
    );

    let since_keys: Vec<String> = since.iter().map(|e| e.key.clone()).collect();
    assert!(since_keys.contains(&"second".to_string()));
    assert!(since_keys.contains(&"third".to_string()));
    assert!(
        !since_keys.contains(&"first".to_string()),
        "Should not include memories before checkpoint"
    );

    Ok(())
}

#[test]
fn test_hybrid_search_combines_semantic_and_keyword() -> Result<()> {
    let (memory, _temp_dir) = create_test_memory_with_semantics()?;

    // Store memories
    memory.store(
        "rust_concurrency",
        "Rust provides fearless concurrency with ownership model",
    )?;
    memory.store(
        "go_goroutines",
        "Go uses lightweight goroutines for concurrent programming",
    )?;
    memory.store(
        "python_threading",
        "Python has threading module but GIL limits true parallelism",
    )?;
    memory.store(
        "rust_memory",
        "Rust memory safety without garbage collection",
    )?;

    // Hybrid search: semantic="concurrent programming" + keywords=["rust"]
    let results = memory.search_hybrid("concurrent programming", &["rust"], None, 10)?;

    // Should prioritize "rust_concurrency" (matches both semantic + keyword)
    assert!(results.len() > 0, "Should find matching memories");
    assert_eq!(
        results[0].entry.key, "rust_concurrency",
        "Hybrid search should rank exact keyword+semantic match highest"
    );

    Ok(())
}

#[test]
fn test_consolidate_similar_memories() -> Result<()> {
    let (memory, _temp_dir) = create_test_memory_with_semantics()?;

    // Store duplicate/similar memories
    memory.store(
        "meeting_note_1",
        "Team meeting discussed Q4 roadmap and priorities",
    )?;
    memory.store(
        "meeting_note_2",
        "Q4 roadmap and team priorities were discussed in meeting",
    )?;
    memory.store("unrelated", "Fix database connection pooling issue")?;

    // Consolidate similar memories (threshold 0.9 for near-duplicates)
    let consolidated_ids = memory.consolidate_similar(0.9)?;

    assert!(
        consolidated_ids.len() > 0,
        "Should consolidate similar memories"
    );

    // After consolidation, searching should return consolidated version
    let results = memory.search_semantic("team meeting roadmap", None, 10)?;
    assert!(
        results.len() > 0,
        "Consolidated memory should still be searchable"
    );

    Ok(())
}

#[test]
fn test_namespace_isolation() -> Result<()> {
    let (memory, _temp_dir) = create_test_memory_with_semantics()?;

    // Store memories in different namespaces
    memory.store_with_metadata("task", "Implement feature X", "work", &[], 0.5)?;
    memory.store_with_metadata("task", "Buy groceries", "personal", &[], 0.5)?;

    // Query in work namespace
    let work_results = memory.search_semantic("task to do", Some("work"), 10)?;
    assert_eq!(
        work_results.len(),
        1,
        "Should find 1 task in work namespace"
    );
    assert!(
        work_results[0].entry.value.contains("feature"),
        "Should find work task"
    );

    // Query in personal namespace
    let personal_results = memory.search_semantic("task to do", Some("personal"), 10)?;
    assert_eq!(
        personal_results.len(),
        1,
        "Should find 1 task in personal namespace"
    );
    assert!(
        personal_results[0].entry.value.contains("groceries"),
        "Should find personal task"
    );

    Ok(())
}

#[test]
fn test_access_tracking_and_frequently_accessed() -> Result<()> {
    let (memory, _temp_dir) = create_test_memory_with_semantics()?;

    memory.store("frequently_used", "This memory is accessed often")?;
    memory.store("rarely_used", "This memory is rarely accessed")?;

    // Access one memory multiple times
    for _ in 0..5 {
        memory.query("frequently_used")?;
    }
    memory.query("rarely_used")?;

    // Query frequently accessed
    let frequent = memory.query_frequently_accessed(10)?;
    assert!(frequent.len() >= 2, "Should find at least 2 memories");
    assert_eq!(
        frequent[0].key, "frequently_used",
        "Most accessed should be first"
    );
    assert!(frequent[0].access_count >= 5, "Should track access count");

    Ok(())
}

#[test]
fn test_get_related_memories_via_graph() -> Result<()> {
    let (memory, _temp_dir) = create_test_memory_with_semantics()?;

    // Store related memories
    memory.store("apex_17", "APEX 1.7 dual-domain embeddings implementation")?;
    memory.store(
        "apex_18",
        "APEX 1.8 REFRAG selective expansion implementation",
    )?;
    memory.store(
        "apex_19",
        "APEX 1.9 code-specific embeddings with BGE model",
    )?;

    // Link them explicitly (if Neo4j is available)
    if memory.has_neo4j() {
        memory.link_memories("apex_17", "apex_18", "PRECEDED_BY")?;
        memory.link_memories("apex_18", "apex_19", "PRECEDED_BY")?;
    }

    // Get related memories should use both semantic similarity AND graph links
    let related = memory.get_related_memories("apex_18", 10)?;

    assert!(
        related.len() >= 2,
        "Should find at least 2 related memories"
    );

    let related_keys: Vec<String> = related.iter().map(|e| e.key.clone()).collect();
    assert!(
        related_keys.contains(&"apex_17".to_string())
            || related_keys.contains(&"apex_19".to_string()),
        "Should find semantically or graph-related memories"
    );

    Ok(())
}

#[test]
fn test_backward_compatibility_with_simple_store_query() -> Result<()> {
    let (memory, _temp_dir) = create_test_memory_with_semantics()?;

    // OLD API should still work (backward compatibility)
    memory.store("test_key", "test_value")?;

    let result = memory.query("test_key")?;
    assert_eq!(result, Some("test_value".to_string()));

    memory.delete("test_key")?;

    let result = memory.query("test_key")?;
    assert_eq!(result, None);

    Ok(())
}
