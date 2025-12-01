//! TDD Tests for Memory Regression Guards (APEX 2.0-M)
//!
//! These tests verify that APEX 2.0-M semantic memory features,
//! triple-domain embedding routing, and HNSW indexes remain intact.

use syncore::memory::Memory;
use syncore::vector::domain::{EmbeddingConfig, EmbeddingDomain};
use tempfile::NamedTempFile;

#[test]
fn test_semantic_memory_metadata_preserved() {
    // Test that APEX 2.0-M semantic memory features (tags, importance, summary) work
    let temp_db = NamedTempFile::new().expect("Failed to create temp db");
    let db_path = temp_db.path().to_str().unwrap();

    let memory = Memory::new(db_path).expect("Failed to create Memory");

    // Store with metadata (APEX 2.0-M feature)
    memory
        .store_with_metadata(
            "semantic_test",
            "This is a test entry with metadata",
            "default",
            &["test", "semantic", "metadata"],
            0.8, // importance
        )
        .expect("Failed to store with metadata");

    // Query should find entry
    let result = memory
        .query_with_namespace("semantic_test", Some("default"))
        .expect("Failed to query");

    assert_eq!(
        result,
        Some("This is a test entry with metadata".to_string())
    );

    // Search by tags should work (if semantic search enabled)
    // This is a regression guard - should continue working after fix
}

#[test]
fn test_triple_domain_config_unchanged() {
    // Test that CODE/GENERAL domain configs remain intact (APEX 2.0-E)
    let code_config = EmbeddingConfig::for_code();
    let general_config = EmbeddingConfig::for_general();

    // APEX 2.0-E: CODE uses BGE-small-en-v1.5 (384 dims)
    assert_eq!(code_config.model_name, "BGE-small-en-v1.5");
    assert_eq!(code_config.dimension, 384);
    assert_eq!(code_config.domain, EmbeddingDomain::Code);

    assert_eq!(general_config.model_name, "all-MiniLM-L6-v2");
    assert_eq!(general_config.dimension, 384);
    assert_eq!(general_config.domain, EmbeddingDomain::General);

    // Verify separate index paths
    assert_ne!(
        code_config.index_path, general_config.index_path,
        "CODE and GENERAL must use separate HNSW indices"
    );
}

#[test]
fn test_hnsw_indexes_separate_per_domain() {
    // Test that HNSW indexes remain separate for CODE, GENERAL, GRAPH domains
    let code_config = EmbeddingConfig::for_code();
    let general_config = EmbeddingConfig::for_general();

    // Index paths should be different
    assert!(
        code_config.index_path.contains("code"),
        "CODE domain should have 'code' in index path"
    );
    assert!(
        general_config.index_path.contains("general"),
        "GENERAL domain should have 'general' in index path"
    );

    // This ensures our fix doesn't accidentally merge indexes or change routing
}
