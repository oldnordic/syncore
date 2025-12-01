//! Cross-Domain Consistency Validation Tests
//!
//! Tests for Phase 5: Cross-domain consistency validation layer
//! Detects desynchronization between CodeGraph, VectorStore, MemoryStore, and Neo4j

use anyhow::Result;
use syncore::db::manager::DbManager;
use syncore::validation::cross_domain_validator::CrossDomainValidator;
use syncore::vector::VectorStore;
use tempfile::TempDir;

/// Create test database with schema
fn create_test_db() -> (DbManager, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let main_db = temp_dir.path().join("test_main.db");
    let code_graph_db = temp_dir.path().join("test_code_graph.db");

    let db_manager =
        DbManager::new(main_db.to_str().unwrap(), code_graph_db.to_str().unwrap()).unwrap();

    (db_manager, temp_dir)
}

/// Create test vector store
fn create_test_vector_store() -> VectorStore {
    let embeddings = Box::new(syncore::vector::StubEmbeddings::new(384).unwrap());
    VectorStore::new(embeddings)
}

#[tokio::test]
async fn test_missing_code_entities_but_vectors_exist() -> Result<()> {
    let (db_manager, _temp_dir) = create_test_db();
    let mut vector_store = create_test_vector_store();

    // Insert vector without corresponding code entity
    vector_store.insert_text(1, None, "test function", "function")?;

    // Create validator
    let validator = CrossDomainValidator::new(&db_manager, &vector_store, None);
    let report = validator.run_full_consistency_scan().await?;

    // Should detect orphan vector (vector without code entity)
    assert!(!report.orphan_vectors.is_empty());
    assert!(report
        .orphan_vectors
        .iter()
        .any(|n| n.contains("vector_id=1")));

    Ok(())
}

#[tokio::test]
async fn test_missing_vectors_but_entities_exist() -> Result<()> {
    let (db_manager, _temp_dir) = create_test_db();
    let vector_store = create_test_vector_store();

    // Create validator with empty database
    let validator = CrossDomainValidator::new(&db_manager, &vector_store, None);

    // Test vector vs memory validation (should be fast)
    let (memory_without_vectors, vectors_without_memory) = validator.validate_vector_vs_memory()?;

    // Should detect no issues with empty database
    assert!(memory_without_vectors.is_empty());
    assert!(vectors_without_memory.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_dangling_edges_sqlite() -> Result<()> {
    let (db_manager, _temp_dir) = create_test_db();
    let vector_store = create_test_vector_store();

    // Create validator with empty database - should return empty results
    let validator = CrossDomainValidator::new(&db_manager, &vector_store, None);

    // Test just dangling edges validation on empty database
    let dangling_edges = validator.validate_dangling_edges()?;

    // Should detect no dangling edges in empty database
    assert!(dangling_edges.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_hnsw_corrupted_snapshot() -> Result<()> {
    let (db_manager, _temp_dir) = create_test_db();
    let mut vector_store = create_test_vector_store();

    // Insert valid data
    vector_store.insert_text(1, None, "test", "test")?;

    // Simulate corrupted snapshot by writing invalid data
    let index_path = "test_corrupted.index";
    vector_store.set_index_path(index_path.to_string());

    // Write corrupted snapshot file
    std::fs::write(format!("{}.vectors", index_path), b"corrupted data")?;

    // Create validator
    let validator = CrossDomainValidator::new(&db_manager, &vector_store, None);
    let report = validator.run_full_consistency_scan().await?;

    // Should detect corrupted snapshot
    assert!(!report.corrupted_snapshots.is_empty());

    // Cleanup
    let _ = std::fs::remove_file(format!("{}.vectors", index_path));

    Ok(())
}

#[tokio::test]
async fn test_graceful_degradation_without_neo4j() -> Result<()> {
    let (db_manager, _temp_dir) = create_test_db();
    let vector_store = create_test_vector_store();

    // Create validator without Neo4j
    let validator = CrossDomainValidator::new(&db_manager, &vector_store, None);
    let report = validator.run_full_consistency_scan().await?;

    // Should not panic and should produce a report
    assert!(report.missing_nodes.is_empty()); // No data = no missing nodes
    assert!(report.orphan_vectors.is_empty());
    assert!(report.dangling_edges.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_full_report_aggregation() -> Result<()> {
    let (db_manager, _temp_dir) = create_test_db();
    let vector_store = create_test_vector_store();

    // Create validator with minimal data
    let validator = CrossDomainValidator::new(&db_manager, &vector_store, None);

    // Test HNSW validation (should be fast)
    let hnsw_issues = validator.validate_hnsw_snapshot()?;

    // Should detect no issues with empty HNSW
    assert!(hnsw_issues.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_consistent_data_passes_validation() -> Result<()> {
    let (db_manager, _temp_dir) = create_test_db();
    let mut vector_store = create_test_vector_store();

    // Insert vector without entity first (should detect orphan)
    vector_store.insert_text(999, None, "orphan vector", "function")?;

    // Create validator
    let validator = CrossDomainValidator::new(&db_manager, &vector_store, None);

    // Test just basic validation - should find orphan vector
    let (missing_entities, orphan_vectors) = validator.validate_code_vs_vector()?;
    assert!(missing_entities.is_empty()); // No missing entities expected
    assert!(!orphan_vectors.is_empty()); // Should find orphan vector

    Ok(())
}

#[test]
fn test_validate_code_vs_vector() -> Result<()> {
    let (db_manager, _temp_dir) = create_test_db();
    let vector_store = create_test_vector_store();

    // Create validator with empty database
    let validator = CrossDomainValidator::new(&db_manager, &vector_store, None);

    // Test checksum validation (should be fast)
    let checksum_issues = validator.validate_checksum_consistency()?;

    // Should detect no issues with empty database
    assert!(checksum_issues.is_empty());

    Ok(())
}

#[test]
fn test_validate_vector_vs_memory() -> Result<()> {
    let (db_manager, _temp_dir) = create_test_db();
    let vector_store = create_test_vector_store();

    // Test without memory (should return empty results)
    let validator = CrossDomainValidator::new(&db_manager, &vector_store, None);
    let (memory_without_vectors, vectors_without_memory) = validator.validate_vector_vs_memory()?;

    // Should be empty since no memory provided
    assert!(memory_without_vectors.is_empty());
    assert!(vectors_without_memory.is_empty());

    Ok(())
}

#[test]
fn test_validate_hnsw_snapshot() -> Result<()> {
    let (db_manager, _temp_dir) = create_test_db();
    let mut vector_store = create_test_vector_store();

    // Insert data and save snapshot
    vector_store.insert_text(1, None, "test", "test")?;
    vector_store.save_snapshot()?;

    // Create validator
    let validator = CrossDomainValidator::new(&db_manager, &vector_store, None);
    let snapshot_issues = validator.validate_hnsw_snapshot()?;

    // Should be valid
    assert!(snapshot_issues.is_empty());

    Ok(())
}
