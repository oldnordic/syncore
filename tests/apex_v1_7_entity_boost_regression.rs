//! APEX v1.7 Entity Boosting Regression Tests
//!
//! Validates that entity type boosting is correctly applied in code search results.
//! Functions should score higher than imports when semantic similarity is equal.

use anyhow::Result;
use std::sync::{Arc, Mutex};
use syncore::code_graph::CodeGraph;
use syncore::vector::{HuggingFaceEmbeddings, VectorStore};
use tempfile::NamedTempFile;

#[test]
fn test_entity_boost_applied_in_search() -> Result<()> {
    // Setup
    let temp_db = NamedTempFile::new()?;
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    let mut code_graph = CodeGraph::new(temp_db.path().to_str().unwrap(), vector_store.clone())?;

    // Create test file with function and import
    let test_file = tempfile::Builder::new().suffix(".rs").tempfile()?;
    std::fs::write(
        test_file.path(),
        r#"
use std::collections::HashMap;

/// Parse configuration file
fn parse_config() -> HashMap<String, String> {
    HashMap::new()
}
"#,
    )?;

    // Index the file
    code_graph.index_file(test_file.path())?;

    // Search for "parse configuration"
    let results = code_graph.search_code("parse configuration", 10)?;

    // Verify entity boosting is applied
    assert!(!results.is_empty(), "Should return search results");

    // Find function and import in results
    let function_result = results.iter().find(|m| m.entity.entity_type.as_str() == "function");

    let import_result = results.iter().find(|m| m.entity.entity_type.as_str() == "import");

    // If both exist, function should score higher than import
    // (function boost = 1.35, import boost = 0.65)
    if let (Some(func), Some(imp)) = (function_result, import_result) {
        assert!(
            func.score > imp.score,
            "Function (score={}) should score higher than Import (score={}) due to entity boosting",
            func.score,
            imp.score
        );
    }

    Ok(())
}

#[test]
fn test_body_snippet_boost_applied() -> Result<()> {
    // Setup
    let temp_db = NamedTempFile::new()?;
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    let mut code_graph = CodeGraph::new(temp_db.path().to_str().unwrap(), vector_store)?;

    // Create test file with detailed function
    let test_file = tempfile::Builder::new().suffix(".rs").tempfile()?;
    std::fs::write(
        test_file.path(),
        r#"
/// Calculate database connection timeout
fn calculate_timeout() -> u64 {
    // Default timeout for database connections
    let base_timeout = 30;
    // Add retry buffer
    let retry_buffer = 5;
    base_timeout + retry_buffer
}
"#,
    )?;

    // Index the file
    code_graph.index_file(test_file.path())?;

    // Search for "database connection timeout"
    let results = code_graph.search_code("database connection timeout", 5)?;

    assert!(!results.is_empty(), "Should return search results");

    // Verify the function with body_snippet has boosted score
    let func_match = results.iter().find(|m| m.entity.name == "calculate_timeout");

    if let Some(func) = func_match {
        // Entity should have body_snippet populated
        assert!(func.entity.body_snippet.is_some(), "Function should have body_snippet");

        // Score should be boosted (base * 1.35 function * 1.15 body)
        // Without boost, semantic score would be lower
        assert!(
            func.score > 0.5,
            "Function with body_snippet should have boosted score > 0.5, got {}",
            func.score
        );
    }

    Ok(())
}

#[test]
fn test_boost_multipliers_correct() {
    use syncore::code_graph::entity_boost::{compute_body_boost, compute_entity_type_boost};

    // Verify boost multipliers match actual implementation
    // Implementation category (Function, Class, Method, Struct, Impl) = 1.35
    assert!((compute_entity_type_boost("Function") - 1.35).abs() < 0.01);
    assert!((compute_entity_type_boost("Method") - 1.35).abs() < 0.01);
    assert!((compute_entity_type_boost("Class") - 1.35).abs() < 0.01);
    assert!((compute_entity_type_boost("Struct") - 1.35).abs() < 0.01);

    // Secondary category (Trait, Enum, Constant) = 1.18
    assert!((compute_entity_type_boost("Trait") - 1.18).abs() < 0.01);

    // Neutral category (Module, Unknown) = 1.0
    assert!((compute_entity_type_boost("Module") - 1.0).abs() < 0.01);

    // Import category = 0.65 (de-boost)
    assert!((compute_entity_type_boost("Import") - 0.65).abs() < 0.01);

    assert!((compute_body_boost(true) - 1.15).abs() < 0.01);
    assert!((compute_body_boost(false) - 1.0).abs() < 0.01);
}
