//! TDD Tests for Memory Backward Compatibility (APEX 2.0-M)
//!
//! These tests verify that existing Memory APIs continue working after refactoring.
//! All tests should PASS immediately (no breaking changes).

use syncore::memory::Memory;
use tempfile::NamedTempFile;

#[test]
fn test_memory_new_still_works() {
    // Test that Memory::new() backward compatibility is maintained
    let temp_db = NamedTempFile::new().expect("Failed to create temp db");
    let db_path = temp_db.path().to_str().unwrap();

    // Old API should still work
    let memory = Memory::new(db_path).expect("Failed to create Memory with old API");

    // Should use default configuration
    memory
        .store("test", "value")
        .expect("Failed to store with old API");

    let result = memory.query("test").expect("Failed to query with old API");

    assert_eq!(result, Some("value".to_string()));
}

#[test]
fn test_deprecated_apis_still_functional() {
    // Test that store()/query()/delete() continue working (even if deprecated)
    let temp_db = NamedTempFile::new().expect("Failed to create temp db");
    let db_path = temp_db.path().to_str().unwrap();

    let memory = Memory::new(db_path).expect("Failed to create Memory");

    // These methods may be marked deprecated but must still work
    memory
        .store("key1", "value1")
        .expect("Deprecated store() should still work");

    let result = memory
        .query("key1")
        .expect("Deprecated query() should still work");

    assert_eq!(result, Some("value1".to_string()));

    memory
        .delete("key1")
        .expect("Deprecated delete() should still work");

    let result = memory
        .query("key1")
        .expect("Query after delete should work");

    assert_eq!(result, None, "Entry should be deleted");
}

#[test]
fn test_migration_006_namespace_isolation_intact() {
    // Test that Migration 006 (namespace isolation) still works correctly
    let temp_db = NamedTempFile::new().expect("Failed to create temp db");
    let db_path = temp_db.path().to_str().unwrap();

    // Run migrations
    syncore::db::ensure_schema(db_path).expect("Failed to run migrations");

    let memory = Memory::new(db_path).expect("Failed to create Memory");

    // Store same key in different namespaces
    memory
        .store_with_metadata("shared", "v1", "ns1", &[], 0.5)
        .expect("Failed to store in ns1");
    memory
        .store_with_metadata("shared", "v2", "ns2", &[], 0.5)
        .expect("Failed to store in ns2");

    // Should NOT raise unique constraint violation
    // (Migration 006 creates composite unique index on (k, namespace))

    let r1 = memory
        .query_with_namespace("shared", Some("ns1"))
        .expect("Failed to query ns1");
    let r2 = memory
        .query_with_namespace("shared", Some("ns2"))
        .expect("Failed to query ns2");

    assert_eq!(r1, Some("v1".to_string()));
    assert_eq!(r2, Some("v2".to_string()));
}
