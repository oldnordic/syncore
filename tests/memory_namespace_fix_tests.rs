//! APEX 2.0-M-FIX: Memory Namespace Isolation Tests
//!
//! TDD tests for namespace-aware memory operations.
//! Tests MUST pass after implementation.

use anyhow::Result;
use syncore::memory::{Memory, MemoryConfig};
use tempfile::NamedTempFile;

#[test]
fn test_query_with_namespace_isolates_keys() -> Result<()> {
    let temp = NamedTempFile::new()?;

    let memory = Memory::new(temp.path().to_str().unwrap())?;

    // Store same key in two namespaces
    memory.store_with_metadata("shared", "value_ns1", "ns1", &[], 0.5)?;
    memory.store_with_metadata("shared", "value_ns2", "ns2", &[], 0.5)?;

    // Query with namespace should return correct value
    let r1 = memory.query_with_namespace("shared", Some("ns1"))?;
    let r2 = memory.query_with_namespace("shared", Some("ns2"))?;

    assert_eq!(r1, Some("value_ns1".to_string()));
    assert_eq!(r2, Some("value_ns2".to_string()));

    Ok(())
}

#[test]
fn test_query_with_namespace_none_uses_default() -> Result<()> {
    let temp = NamedTempFile::new()?;

    let memory = Memory::new(temp.path().to_str().unwrap())?;

    // Store in default namespace via store()
    memory.store("mykey", "myvalue")?;

    // Query with None should find it
    let result = memory.query_with_namespace("mykey", None)?;
    assert_eq!(result, Some("myvalue".to_string()));

    Ok(())
}

#[test]
fn test_delete_with_namespace_isolates_keys() -> Result<()> {
    let temp = NamedTempFile::new()?;

    let memory = Memory::new(temp.path().to_str().unwrap())?;

    // Store same key in two namespaces
    memory.store_with_metadata("shared", "value_ns1", "ns1", &[], 0.5)?;
    memory.store_with_metadata("shared", "value_ns2", "ns2", &[], 0.5)?;

    // Delete from ns1 only
    memory.delete_with_namespace("shared", Some("ns1"))?;

    // ns1 should be gone, ns2 should remain
    let r1 = memory.query_with_namespace("shared", Some("ns1"))?;
    let r2 = memory.query_with_namespace("shared", Some("ns2"))?;

    assert_eq!(r1, None);
    assert_eq!(r2, Some("value_ns2".to_string()));

    Ok(())
}

#[test]
fn test_delete_with_namespace_none_uses_default() -> Result<()> {
    let temp = NamedTempFile::new()?;

    let memory = Memory::new(temp.path().to_str().unwrap())?;

    memory.store("mykey", "myvalue")?;
    memory.delete_with_namespace("mykey", None)?;

    let result = memory.query("mykey")?;
    assert_eq!(result, None);

    Ok(())
}

#[test]
fn test_store_backward_compat_uses_default_namespace() -> Result<()> {
    let temp = NamedTempFile::new()?;

    let memory = Memory::new(temp.path().to_str().unwrap())?;

    // Old API: store() should write to default namespace
    memory.store("key1", "value1")?;

    // Should be queryable via default namespace
    let result = memory.query_with_namespace("key1", Some("default"))?;
    assert_eq!(result, Some("value1".to_string()));

    Ok(())
}

#[test]
fn test_query_backward_compat_uses_default_namespace() -> Result<()> {
    let temp = NamedTempFile::new()?;

    let memory = Memory::new(temp.path().to_str().unwrap())?;

    // Store via new API in default namespace
    memory.store_with_metadata("key1", "value1", "default", &[], 0.5)?;

    // Old API: query() should find it
    let result = memory.query("key1")?;
    assert_eq!(result, Some("value1".to_string()));

    Ok(())
}

#[test]
fn test_delete_backward_compat_uses_default_namespace() -> Result<()> {
    let temp = NamedTempFile::new()?;

    let memory = Memory::new(temp.path().to_str().unwrap())?;

    memory.store("key1", "value1")?;
    memory.delete("key1")?;

    let result = memory.query("key1")?;
    assert_eq!(result, None);

    Ok(())
}

#[test]
fn test_namespace_allows_duplicate_keys() -> Result<()> {
    let temp = NamedTempFile::new()?;

    let memory = Memory::new(temp.path().to_str().unwrap())?;

    // Store same key in 3 different namespaces
    memory.store_with_metadata("dup", "v1", "ns1", &[], 0.5)?;
    memory.store_with_metadata("dup", "v2", "ns2", &[], 0.5)?;
    memory.store_with_metadata("dup", "v3", "default", &[], 0.5)?;

    // All should be retrievable independently
    assert_eq!(
        memory.query_with_namespace("dup", Some("ns1"))?,
        Some("v1".to_string())
    );
    assert_eq!(
        memory.query_with_namespace("dup", Some("ns2"))?,
        Some("v2".to_string())
    );
    assert_eq!(
        memory.query_with_namespace("dup", Some("default"))?,
        Some("v3".to_string())
    );

    Ok(())
}

#[test]
fn test_update_overwrites_within_namespace() -> Result<()> {
    let temp = NamedTempFile::new()?;

    let memory = Memory::new(temp.path().to_str().unwrap())?;

    // Store and update in same namespace
    memory.store_with_metadata("key", "v1", "ns1", &[], 0.5)?;
    memory.store_with_metadata("key", "v2_updated", "ns1", &[], 0.6)?;

    let result = memory.query_with_namespace("key", Some("ns1"))?;
    assert_eq!(result, Some("v2_updated".to_string()));

    Ok(())
}

#[test]
fn test_cache_respects_namespace() -> Result<()> {
    let temp = NamedTempFile::new()?;

    let memory = Memory::new(temp.path().to_str().unwrap())?;

    // Store same key in two namespaces
    memory.store_with_metadata("cached", "ns1_val", "ns1", &[], 0.5)?;
    memory.store_with_metadata("cached", "ns2_val", "ns2", &[], 0.5)?;

    // Query twice (first hits DB, second hits cache)
    let r1a = memory.query_with_namespace("cached", Some("ns1"))?;
    let r1b = memory.query_with_namespace("cached", Some("ns1"))?;
    let r2a = memory.query_with_namespace("cached", Some("ns2"))?;
    let r2b = memory.query_with_namespace("cached", Some("ns2"))?;

    // Both should be consistent
    assert_eq!(r1a, Some("ns1_val".to_string()));
    assert_eq!(r1b, Some("ns1_val".to_string()));
    assert_eq!(r2a, Some("ns2_val".to_string()));
    assert_eq!(r2b, Some("ns2_val".to_string()));

    Ok(())
}
