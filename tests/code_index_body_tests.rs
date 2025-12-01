//! Code indexing with function body support tests
//!
//! Tests that code indexing captures and indexes function bodies for semantic search

use anyhow::Result;
use std::path::Path;
use std::sync::{Arc, Mutex};
use syncore::code_graph::CodeGraph;
use syncore::vector::{HuggingFaceEmbeddings, VectorStore};
use tempfile::NamedTempFile;

#[test]
fn test_function_bodies_are_indexed() -> Result<()> {
    // Index the fixture project
    let fixture_path = Path::new("tests/fixtures/body_index_project");
    assert!(fixture_path.exists(), "Fixture project should exist");

    // Create temporary database and vector store
    let temp_db = NamedTempFile::new()?;
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = VectorStore::new(embeddings);
    let mut code_graph = CodeGraph::new(
        temp_db.path().to_str().unwrap(),
        Arc::new(Mutex::new(vector_store)),
    )?;

    // Index the fixture file with unique function body
    let unique_file = fixture_path.join("src/unique_feature.rs");
    code_graph.index_file(&unique_file)?;

    // Search for "cosmic alignment" - a unique phrase in the function body
    let results = code_graph.search_code("cosmic alignment", 5)?;

    // Assert that results are not empty
    assert!(
        !results.is_empty(),
        "Should find matches for 'cosmic alignment'"
    );

    // Assert that calculate_cosmic_alignment function is in top results
    let top_match = &results[0];
    assert!(
        top_match.entity.name.contains("calculate_cosmic_alignment"),
        "Top result should be calculate_cosmic_alignment function, got: {}",
        top_match.entity.name
    );

    Ok(())
}

#[test]
fn test_body_indexing_does_not_break_existing_entity_indexing() -> Result<()> {
    // This test ensures that when we add body indexing,
    // existing entity-level indexing (function names, imports, etc.) still works

    // Create temporary database and vector store
    let temp_db = NamedTempFile::new()?;
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = VectorStore::new(embeddings);
    let mut code_graph = CodeGraph::new(
        temp_db.path().to_str().unwrap(),
        Arc::new(Mutex::new(vector_store)),
    )?;

    // Index a simple Rust file
    let fixture_path = Path::new("tests/fixtures/body_index_project");
    let unique_file = fixture_path.join("src/unique_feature.rs");
    code_graph.index_file(&unique_file)?;

    // Verify entities were indexed (basic entity extraction still works)
    let results = code_graph.search_code("calculate", 10)?;
    assert!(
        !results.is_empty(),
        "Should find entities with 'calculate' in name"
    );

    // Verify we can find entities by function name (not just body content)
    let has_cosmic_fn = results
        .iter()
        .any(|r| r.entity.name.contains("calculate_cosmic_alignment"));
    assert!(
        has_cosmic_fn,
        "Should find calculate_cosmic_alignment by name"
    );

    Ok(())
}

#[test]
fn test_code_search_prefers_implementation_over_reexport() -> Result<()> {
    // Index the fixture project
    let fixture_path = Path::new("tests/fixtures/body_index_project");
    assert!(fixture_path.exists(), "Fixture project should exist");

    // Create temporary database and vector store
    let temp_db = NamedTempFile::new()?;
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = VectorStore::new(embeddings);
    let mut code_graph = CodeGraph::new(
        temp_db.path().to_str().unwrap(),
        Arc::new(Mutex::new(vector_store)),
    )?;

    // Index both implementation and re-export files
    let unique_file = fixture_path.join("src/unique_feature.rs");
    let imports_file = fixture_path.join("src/imports.rs");

    code_graph.index_file(&unique_file)?;
    code_graph.index_file(&imports_file)?;

    // Search for "calculate cosmic alignment implementation"
    // The implementation (with body) should rank higher than re-export
    let results = code_graph.search_code("calculate cosmic alignment implementation", 5)?;

    assert!(!results.is_empty(), "Should find matches");

    // Find which result is the implementation vs re-export
    let impl_result = results
        .iter()
        .find(|r| r.entity.file_path.contains("unique_feature.rs"));
    let reexport_result = results
        .iter()
        .find(|r| r.entity.file_path.contains("imports.rs"));

    // Both should be found
    assert!(impl_result.is_some(), "Should find implementation");
    assert!(reexport_result.is_some(), "Should find re-export");

    // Implementation should have higher score than re-export
    // (This will be true once we implement body_snippet scoring in Phase 3.3)
    if let (Some(impl_match), Some(reexport_match)) = (impl_result, reexport_result) {
        assert!(
            impl_match.score >= reexport_match.score,
            "Implementation (score: {}) should rank >= re-export (score: {})",
            impl_match.score,
            reexport_match.score
        );
    }

    Ok(())
}
