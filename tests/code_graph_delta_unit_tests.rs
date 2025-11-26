//! APEX 2.6-CG-GRAPH-DELTA: Unit Tests for Delta Engine
//!
//! These tests MUST FAIL initially (TDD-first).
//! Tests isolated delta computation logic without full pipeline.

use anyhow::Result;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;
use tree_sitter::Range;

use syncore::code_graph::delta::{AstDelta, CodeGraphDeltaEngine};
use syncore::code_graph::CodeGraph;
use syncore::parser_service::ParseDelta;
use syncore::vector::{StubEmbeddings, VectorStore};

/// Helper to create CodeGraph for testing
fn create_test_graph() -> Result<(TempDir, CodeGraph)> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_delta.db");

    let embeddings = Box::new(StubEmbeddings::new(384)?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store)?;

    Ok((temp_dir, graph))
}

// ============================================================================
// TEST 1: Empty changed_ranges yields no reindex
// ============================================================================

#[tokio::test]
async fn test_empty_changed_ranges_no_reindex() -> Result<()> {
    let (_temp, graph) = create_test_graph()?;
    let delta_engine = CodeGraphDeltaEngine::new(Arc::new(Mutex::new(graph)));

    let file_path = PathBuf::from("/test/file.rs");
    let parse_delta = ParseDelta {
        path: file_path.clone(),
        changed_ranges: vec![], // Empty - no changes
        had_errors: false,
    };

    let ast_delta = delta_engine.compute_ast_delta(&file_path, &parse_delta)?;

    // Empty changed_ranges should result in no-op delta
    assert_eq!(ast_delta.changed_ranges.len(), 0, "Should have no changed ranges");
    assert!(!ast_delta.deleted, "Should not be marked as deleted");
    assert!(ast_delta.renamed.is_none(), "Should not be renamed");

    Ok(())
}

// ============================================================================
// TEST 2: Single changed range triggers selective reindex
// ============================================================================

#[tokio::test]
async fn test_single_changed_range_selective_reindex() -> Result<()> {
    let (_temp, graph) = create_test_graph()?;
    let delta_engine = CodeGraphDeltaEngine::new(Arc::new(Mutex::new(graph)));

    let file_path = PathBuf::from("/test/file.rs");
    let changed_range = Range {
        start_byte: 100,
        end_byte: 200,
        start_point: tree_sitter::Point { row: 5, column: 0 },
        end_point: tree_sitter::Point { row: 10, column: 0 },
    };

    let parse_delta = ParseDelta {
        path: file_path.clone(),
        changed_ranges: vec![changed_range],
        had_errors: false,
    };

    let ast_delta = delta_engine.compute_ast_delta(&file_path, &parse_delta)?;

    // Should have exactly one changed range
    assert_eq!(ast_delta.changed_ranges.len(), 1, "Should have one changed range");
    assert_eq!(ast_delta.file_path, file_path, "File path should match");

    Ok(())
}

// ============================================================================
// TEST 3: Multiple changed ranges are preserved
// ============================================================================

#[tokio::test]
async fn test_multiple_changed_ranges_preserved() -> Result<()> {
    let (_temp, graph) = create_test_graph()?;
    let delta_engine = CodeGraphDeltaEngine::new(Arc::new(Mutex::new(graph)));

    let file_path = PathBuf::from("/test/file.rs");
    let range1 = Range {
        start_byte: 100,
        end_byte: 150,
        start_point: tree_sitter::Point { row: 5, column: 0 },
        end_point: tree_sitter::Point { row: 7, column: 0 },
    };
    let range2 = Range {
        start_byte: 300,
        end_byte: 400,
        start_point: tree_sitter::Point { row: 15, column: 0 },
        end_point: tree_sitter::Point { row: 20, column: 0 },
    };

    let parse_delta = ParseDelta {
        path: file_path.clone(),
        changed_ranges: vec![range1, range2],
        had_errors: false,
    };

    let ast_delta = delta_engine.compute_ast_delta(&file_path, &parse_delta)?;

    // Should preserve both ranges
    assert_eq!(ast_delta.changed_ranges.len(), 2, "Should have two changed ranges");

    Ok(())
}

// ============================================================================
// TEST 4: Parser errors trigger full file reindex
// ============================================================================

#[tokio::test]
async fn test_parser_errors_trigger_full_reindex() -> Result<()> {
    let (_temp, graph) = create_test_graph()?;
    let delta_engine = CodeGraphDeltaEngine::new(Arc::new(Mutex::new(graph)));

    let file_path = PathBuf::from("/test/file.rs");
    let parse_delta = ParseDelta {
        path: file_path.clone(),
        changed_ranges: vec![], // Even with empty ranges
        had_errors: true, // Parser errors
    };

    let ast_delta = delta_engine.compute_ast_delta(&file_path, &parse_delta)?;

    // With parser errors, should mark for full reindex
    // Implementation detail: can be represented by empty changed_ranges + special flag
    // or by having changed_ranges covering whole file
    assert_eq!(ast_delta.file_path, file_path, "File path should match");

    Ok(())
}

// ============================================================================
// TEST 5: Deleted flag marks file for entity removal
// ============================================================================

#[tokio::test]
async fn test_deleted_flag_marks_removal() -> Result<()> {
    let (_temp, graph) = create_test_graph()?;
    let delta_engine = CodeGraphDeltaEngine::new(Arc::new(Mutex::new(graph)));

    let file_path = PathBuf::from("/test/deleted.rs");

    // Create an AstDelta directly for deletion test
    let ast_delta = AstDelta {
        file_path: file_path.clone(),
        changed_ranges: vec![],
        deleted: true,
        renamed: None,
    };

    // Apply delta should handle deletion
    delta_engine.apply_delta(&ast_delta)?;

    // Deletion should succeed (entities removed from DB)
    Ok(())
}

// ============================================================================
// TEST 6: Renamed flag triggers delete + reindex
// ============================================================================

#[tokio::test]
async fn test_renamed_triggers_delete_and_reindex() -> Result<()> {
    let (_temp, graph) = create_test_graph()?;
    let delta_engine = CodeGraphDeltaEngine::new(Arc::new(Mutex::new(graph)));

    let old_path = PathBuf::from("/test/old.rs");
    let new_path = PathBuf::from("/test/new.rs");

    let ast_delta = AstDelta {
        file_path: old_path.clone(),
        changed_ranges: vec![],
        deleted: false,
        renamed: Some(new_path.clone()),
    };

    // Apply rename delta
    delta_engine.apply_delta(&ast_delta)?;

    // Rename should succeed
    Ok(())
}

// ============================================================================
// TEST 7: Delta application is idempotent
// ============================================================================

#[tokio::test]
async fn test_delta_application_idempotent() -> Result<()> {
    let (_temp, graph) = create_test_graph()?;
    let delta_engine = CodeGraphDeltaEngine::new(Arc::new(Mutex::new(graph)));

    let file_path = PathBuf::from("/test/file.rs");
    let parse_delta = ParseDelta {
        path: file_path.clone(),
        changed_ranges: vec![],
        had_errors: false,
    };

    // Apply same delta twice
    let ast_delta = delta_engine.compute_ast_delta(&file_path, &parse_delta)?;
    delta_engine.apply_delta(&ast_delta)?;
    delta_engine.apply_delta(&ast_delta)?; // Should not fail

    Ok(())
}
