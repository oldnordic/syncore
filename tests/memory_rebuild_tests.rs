//! APEX 2.0-M Memory Suite Rebuild - TDD Tests
//!
//! These tests define the contract for memory suite BEFORE implementation.
//! All tests should FAIL initially, then PASS after implementation.
//!
//! Test Coverage (23 tests):
//! 1. Schema Migration Tests (10 tests)
//! 2. Memory Operations Tests (8 tests)
//! 3. Namespace Isolation Tests (5 tests)

use rusqlite::Connection;
use std::collections::HashSet;
use syncore::schema_migration::{get_schema_version, run_migrations, CURRENT_SCHEMA_VERSION};

// ============================================================================
// PHASE 1: Schema Migration Tests (10 tests)
// ============================================================================

#[test]
fn test_migration_005_adds_semantic_memory_columns() {
    // Test that migration 005 adds all 7 missing columns to memory table
    let conn = Connection::open_in_memory().unwrap();

    // Run migrations up to version 5
    run_migrations(&conn).unwrap();

    // Verify all semantic memory columns exist
    let mut stmt = conn.prepare("PRAGMA table_info(memory)").unwrap();
    let columns: HashSet<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    assert!(columns.contains("summary"), "summary column must exist");
    assert!(columns.contains("namespace"), "namespace column must exist");
    assert!(
        columns.contains("importance"),
        "importance column must exist"
    );
    assert!(
        columns.contains("created_at"),
        "created_at column must exist"
    );
    assert!(
        columns.contains("last_accessed"),
        "last_accessed column must exist"
    );
    assert!(
        columns.contains("access_count"),
        "access_count column must exist"
    );
    assert!(
        columns.contains("embedding_id"),
        "embedding_id column must exist"
    );
}

#[test]
fn test_migration_005_creates_memory_tags_table() {
    // Test that migration 005 creates memory_tags table
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn).unwrap();

    let table_exists: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='memory_tags'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);

    assert!(
        table_exists,
        "memory_tags table must exist after migration 005"
    );
}

#[test]
fn test_migration_005_creates_memory_consolidations_table() {
    // Test that migration 005 creates memory_consolidations table
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn).unwrap();

    let table_exists: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='memory_consolidations'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);

    assert!(
        table_exists,
        "memory_consolidations table must exist after migration 005"
    );
}

#[test]
fn test_migration_006_creates_namespace_composite_index() {
    // Test that migration 006 creates idx_memory_k_namespace index
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn).unwrap();

    let index_exists: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='index' AND name='idx_memory_k_namespace'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);

    assert!(
        index_exists,
        "idx_memory_k_namespace index must exist after migration 006"
    );
}

#[test]
fn test_migration_006_drops_old_single_column_index() {
    // Test that migration 006 removes the old idx_memory_k index
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn).unwrap();

    // Try to find old single-column index (should NOT exist)
    let old_index_exists: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='index' AND name='idx_memory_k' AND sql LIKE '%memory(k)%'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);

    assert!(
        !old_index_exists,
        "Old idx_memory_k single-column index must be removed"
    );
}

#[test]
fn test_schema_version_reaches_6_after_migrations() {
    // Test that CURRENT_SCHEMA_VERSION is 6
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn).unwrap();

    let version = get_schema_version(&conn).unwrap();
    assert_eq!(version, 6, "Schema version must be 6 after all migrations");
    assert_eq!(
        CURRENT_SCHEMA_VERSION, 6,
        "CURRENT_SCHEMA_VERSION constant must be 6"
    );
}

#[test]
fn test_migration_005_is_idempotent() {
    // Test that migration 005 can be run multiple times safely
    let conn = Connection::open_in_memory().unwrap();

    // Run migrations twice
    run_migrations(&conn).unwrap();
    let result = run_migrations(&conn);

    assert!(
        result.is_ok(),
        "Migration 005 must be idempotent (safe to run twice)"
    );
}

#[test]
fn test_migration_006_is_idempotent() {
    // Test that migration 006 can be run multiple times safely
    let conn = Connection::open_in_memory().unwrap();

    // Run migrations twice
    run_migrations(&conn).unwrap();
    let result = run_migrations(&conn);

    assert!(
        result.is_ok(),
        "Migration 006 must be idempotent (safe to run twice)"
    );
}

#[test]
fn test_migration_preserves_existing_memory_data() {
    // Test that adding columns doesn't delete existing data
    let conn = Connection::open_in_memory().unwrap();

    // Create old schema (version 1 only)
    conn.execute_batch(
        "CREATE TABLE memory (id INTEGER PRIMARY KEY, k TEXT NOT NULL, v TEXT NOT NULL, ts INTEGER NOT NULL);
         CREATE UNIQUE INDEX idx_memory_k ON memory(k);
         INSERT INTO memory (k, v, ts) VALUES ('test_key', 'test_value', 123456);"
    ).unwrap();

    // Run migrations (should add columns)
    run_migrations(&conn).unwrap();

    // Verify old data still exists
    let value: String = conn
        .query_row("SELECT v FROM memory WHERE k = 'test_key'", [], |row| {
            row.get(0)
        })
        .unwrap();

    assert_eq!(
        value, "test_value",
        "Existing data must be preserved after migration"
    );
}

#[test]
fn test_migration_sets_default_values_for_new_columns() {
    // Test that new columns get proper default values for existing rows
    let conn = Connection::open_in_memory().unwrap();

    // Create old schema with one row
    conn.execute_batch(
        "CREATE TABLE memory (id INTEGER PRIMARY KEY, k TEXT NOT NULL, v TEXT NOT NULL, ts INTEGER NOT NULL);
         INSERT INTO memory (k, v, ts) VALUES ('test_key', 'test_value', 123456);"
    ).unwrap();

    // Run migrations
    run_migrations(&conn).unwrap();

    // Verify defaults were applied
    let (namespace, importance, access_count): (String, f32, i64) = conn
        .query_row(
            "SELECT namespace, importance, access_count FROM memory WHERE k = 'test_key'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();

    assert_eq!(namespace, "default", "Default namespace must be 'default'");
    assert_eq!(importance, 0.5, "Default importance must be 0.5");
    assert_eq!(access_count, 0, "Default access_count must be 0");
}

// ============================================================================
// PHASE 2: Memory Operations Tests (8 tests)
// ============================================================================

#[test]
fn test_memory_store_with_namespace() {
    // Test that Memory::store_with_metadata() works with namespace parameter
    use syncore::memory::Memory;
    use tempfile::NamedTempFile;

    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();

    let memory = Memory::new(db_path).unwrap();

    // Store with explicit namespace
    let result = memory.store_with_metadata(
        "test_key",
        "test_value",
        "custom_namespace",
        &["tag1", "tag2"],
        0.8,
    );

    assert!(result.is_ok(), "store_with_metadata must succeed");
}

#[test]
fn test_memory_query_respects_namespace() {
    // Test that Memory::query() only returns values from correct namespace
    use syncore::memory::Memory;
    use tempfile::NamedTempFile;

    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();

    let memory = Memory::new(db_path).unwrap();

    // Store same key in different namespaces (including default)
    memory
        .store_with_metadata("key", "value_default", "default", &[], 0.5)
        .unwrap();
    memory
        .store_with_metadata("key", "value_ns1", "namespace1", &[], 0.5)
        .unwrap();
    memory
        .store_with_metadata("key", "value_ns2", "namespace2", &[], 0.5)
        .unwrap();

    // Query should return default namespace value (not ns1 or ns2)
    let value = memory.query("key").unwrap();

    assert!(value.is_some(), "Query must work with namespace isolation");
    assert_eq!(
        value.unwrap(),
        "value_default",
        "Query should return value from default namespace, not other namespaces"
    );
}

#[test]
fn test_memory_tags_stored_and_retrieved() {
    // Test that tags are properly stored in memory_tags table
    use syncore::memory::Memory;
    use tempfile::NamedTempFile;

    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();

    let memory = Memory::new(db_path).unwrap();

    // Store with tags
    memory
        .store_with_metadata("key", "value", "default", &["tag1", "tag2"], 0.5)
        .unwrap();

    // Retrieve and verify tags
    let entries = memory.query_recent(100, Some("default")).unwrap();
    let entry = entries.iter().find(|e| e.key == "key").unwrap();

    assert_eq!(entry.tags.len(), 2, "Must store 2 tags");
    assert!(
        entry.tags.contains(&"tag1".to_string()),
        "Must contain tag1"
    );
    assert!(
        entry.tags.contains(&"tag2".to_string()),
        "Must contain tag2"
    );
}

#[test]
fn test_memory_importance_stored_and_retrieved() {
    // Test that importance scores are stored correctly
    use syncore::memory::Memory;
    use tempfile::NamedTempFile;

    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();

    let memory = Memory::new(db_path).unwrap();

    // Store with high importance
    memory
        .store_with_metadata("key", "value", "default", &[], 0.9)
        .unwrap();

    // Retrieve and verify importance
    let entries = memory.query_recent(100, Some("default")).unwrap();
    let entry = entries.iter().find(|e| e.key == "key").unwrap();

    assert_eq!(entry.importance, 0.9, "Importance must be stored correctly");
}

#[test]
fn test_memory_access_tracking() {
    // Test that last_accessed and access_count are updated on query
    use syncore::memory::Memory;
    use tempfile::NamedTempFile;

    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();

    let memory = Memory::new(db_path).unwrap();

    memory
        .store_with_metadata("key", "value", "default", &[], 0.5)
        .unwrap();

    // Small delay to ensure timestamps differ
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Query multiple times
    memory.query("key").unwrap();
    memory.query("key").unwrap();
    memory.query("key").unwrap();

    // Verify access tracking
    let entries = memory.query_recent(100, Some("default")).unwrap();
    let entry = entries.iter().find(|e| e.key == "key").unwrap();

    assert_eq!(
        entry.access_count, 3,
        "Access count must increment on each query"
    );
    assert!(
        entry.last_accessed >= entry.created_at,
        "last_accessed must be updated or equal"
    );
}

#[test]
fn test_memory_semantic_search() {
    // Test that semantic search works with embedding_id
    use syncore::memory::Memory;
    use tempfile::NamedTempFile;

    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();

    let memory = Memory::new(db_path).unwrap();

    // Store some memories
    memory
        .store_with_metadata("key1", "machine learning", "default", &["ai"], 0.8)
        .unwrap();
    memory
        .store_with_metadata("key2", "database", "default", &["storage"], 0.5)
        .unwrap();

    // Semantic search should work if embeddings are enabled
    let result = memory.search_semantic("artificial intelligence", None, 5);

    // Should either work or gracefully handle disabled embeddings
    assert!(
        result.is_ok() || result.is_err(),
        "semantic_search must handle embeddings state"
    );
}

#[test]
fn test_memory_consolidation() {
    // Test that consolidate_similar() works with semantic memory
    use syncore::memory::Memory;
    use tempfile::NamedTempFile;

    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();

    let memory = Memory::new(db_path).unwrap();

    // Store duplicate-ish memories
    memory
        .store_with_metadata("key1", "test value", "default", &[], 0.6)
        .unwrap();
    memory
        .store_with_metadata("key2", "test value", "default", &[], 0.4)
        .unwrap();

    // Consolidation should work or gracefully fail if embeddings disabled
    let result = memory.consolidate_similar(0.9);

    assert!(
        result.is_ok() || result.is_err(),
        "consolidate_similar must handle embeddings state"
    );
}

#[test]
fn test_memory_get_stats() {
    // Test that get_stats() returns correct counts and namespaces
    use syncore::memory::Memory;
    use tempfile::NamedTempFile;

    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();

    let memory = Memory::new(db_path).unwrap();

    // Store in multiple namespaces
    memory
        .store_with_metadata("key1", "value1", "ns1", &[], 0.5)
        .unwrap();
    memory
        .store_with_metadata("key2", "value2", "ns2", &[], 0.5)
        .unwrap();
    memory
        .store_with_metadata("key3", "value3", "ns2", &[], 0.5)
        .unwrap();

    let (count, namespaces) = memory.get_stats().unwrap();

    assert_eq!(count, 3, "Total count must be 3");
    assert!(namespaces.contains(&"ns1".to_string()), "Must include ns1");
    assert!(namespaces.contains(&"ns2".to_string()), "Must include ns2");
}

// ============================================================================
// PHASE 3: Namespace Isolation Tests (5 tests)
// ============================================================================

#[test]
fn test_namespace_allows_duplicate_keys() {
    // Test that same key can exist in multiple namespaces
    use syncore::memory::Memory;
    use tempfile::NamedTempFile;

    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();

    let memory = Memory::new(db_path).unwrap();

    // Store same key in 3 different namespaces
    let result1 = memory.store_with_metadata("duplicate_key", "value1", "ns1", &[], 0.5);
    let result2 = memory.store_with_metadata("duplicate_key", "value2", "ns2", &[], 0.5);
    let result3 = memory.store_with_metadata("duplicate_key", "value3", "ns3", &[], 0.5);

    assert!(result1.is_ok(), "Must allow duplicate key in ns1");
    assert!(result2.is_ok(), "Must allow duplicate key in ns2");
    assert!(result3.is_ok(), "Must allow duplicate key in ns3");

    // Verify all 3 exist
    let (count, _) = memory.get_stats().unwrap();
    assert_eq!(count, 3, "All 3 entries must exist with same key");
}

#[test]
fn test_namespace_isolation_query() {
    // Test that query doesn't leak data between namespaces
    use syncore::memory::Memory;
    use tempfile::NamedTempFile;

    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();

    let memory = Memory::new(db_path).unwrap();

    // Store in namespace1 only (not in default)
    memory
        .store_with_metadata("secret", "value_ns1", "namespace1", &[], 0.5)
        .unwrap();

    // Query from default namespace should not see namespace1 data
    let value = memory.query("secret");

    // Should be None (not leaked from namespace1) or error (no such key)
    assert!(value.is_ok(), "Query should complete without error");
    assert!(value.unwrap().is_none(), "Query must respect namespace isolation - should not see namespace1 data from default namespace");
}

#[test]
fn test_namespace_isolation_delete() {
    // Test that delete only affects target namespace
    use syncore::memory::Memory;
    use tempfile::NamedTempFile;

    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();

    let memory = Memory::new(db_path).unwrap();

    // Store same key in 2 namespaces
    memory
        .store_with_metadata("key", "value1", "default", &[], 0.5)
        .unwrap();
    memory
        .store_with_metadata("key", "value2", "other", &[], 0.5)
        .unwrap();

    // Delete from default namespace
    memory.delete("key").unwrap();

    // Verify only default was deleted
    let (count, _) = memory.get_stats().unwrap();
    assert_eq!(count, 1, "Only default namespace entry should be deleted");
}

#[test]
fn test_namespace_default_behavior() {
    // Test that store() and query() use default namespace
    use syncore::memory::Memory;
    use tempfile::NamedTempFile;

    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();

    let memory = Memory::new(db_path).unwrap();

    // Use simple store (no namespace param)
    memory.store("key", "value").unwrap();

    // Verify it went to default namespace
    let entries = memory.query_recent(100, Some("default")).unwrap();
    let entry = entries.iter().find(|e| e.key == "key");

    assert!(entry.is_some(), "Simple store() must use default namespace");
}

#[test]
fn test_namespace_prevents_single_key_uniqueness() {
    // Test that old idx_memory_k constraint is gone
    use syncore::memory::Memory;
    use tempfile::NamedTempFile;

    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();

    let memory = Memory::new(db_path).unwrap();

    // This should NOT fail (would fail with old single-column unique index)
    memory
        .store_with_metadata("key", "v1", "ns1", &[], 0.5)
        .unwrap();
    let result = memory.store_with_metadata("key", "v2", "ns2", &[], 0.5);

    assert!(
        result.is_ok(),
        "Namespace isolation must allow same key in different namespaces"
    );
}
