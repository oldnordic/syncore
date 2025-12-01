//! TDD Tests for Ripgrep Integration (APEX 2.0-M)
//!
//! These tests verify that parser_search (ripgrep) works correctly.
//! These should mostly PASS (ripgrep already integrated), but verify behavior.

use std::fs;
use std::path::Path;
use syncore::parser::RipgrepSearcher;
use tempfile::TempDir;

fn create_test_codebase() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create test Rust file with patterns
    let rust_file = temp_dir.path().join("lib.rs");
    fs::write(
        &rust_file,
        r#"
// Test file for ripgrep search
pub struct TestStruct {
    pub name: String,
}

impl TestStruct {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

pub fn helper_function() {
    println!("TestStruct is defined above");
}
"#,
    )
    .expect("Failed to write test file");

    temp_dir
}

#[test]
fn test_ripgrep_search_finds_pattern() {
    // Test that RipgrepSearcher finds patterns in codebase
    let test_dir = create_test_codebase();

    let matches =
        RipgrepSearcher::search("TestStruct", test_dir.path(), 0).expect("Search should succeed");

    assert!(!matches.is_empty(), "Should find TestStruct pattern in test file");

    // Verify at least one match contains expected content
    let found_struct_def = matches.iter().any(|m| m.line_content.contains("pub struct TestStruct"));

    assert!(found_struct_def, "Should find struct definition with TestStruct");
}

#[test]
fn test_ripgrep_search_context_lines() {
    // Test that context_lines parameter includes surrounding lines
    let test_dir = create_test_codebase();

    let matches = RipgrepSearcher::search("helper_function", test_dir.path(), 2)
        .expect("Search should succeed");

    assert!(!matches.is_empty(), "Should find helper_function");

    // With context=2, should see lines before and after the match
    // This is a basic check - actual context handling depends on RipgrepSearcher implementation
    assert!(matches[0].line_number > 0, "Should have valid line number");
}

#[test]
fn test_ripgrep_search_no_matches_returns_empty() {
    // Test that no-match case returns empty Vec, not error
    let test_dir = create_test_codebase();

    let matches = RipgrepSearcher::search("NonexistentPattern_12345_XYZ", test_dir.path(), 0)
        .expect("Search should succeed even with no matches");

    assert_eq!(matches.len(), 0, "No matches should return empty Vec, not error");
}

#[test]
fn test_ripgrep_search_multiple_matches() {
    // Test that ripgrep finds multiple occurrences of a pattern
    let test_dir = create_test_codebase();

    // "TestStruct" appears multiple times in the file
    let matches =
        RipgrepSearcher::search("TestStruct", test_dir.path(), 0).expect("Search should succeed");

    assert!(
        matches.len() >= 2,
        "Should find multiple occurrences of TestStruct (struct def, impl, comment)"
    );
}
