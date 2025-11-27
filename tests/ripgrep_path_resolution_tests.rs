//! TDD Tests for Bug #1: Path Resolution in RipgrepSearcher
//!
//! ISSUE: RipgrepSearcher::search() fails when given file paths instead of directories
//!
//! Root Cause Analysis:
//! - Ripgrep exit codes: 0=matches, 1=no matches, 2=error (file not found)
//! - When relative file path doesn't exist in CWD → exit code 2
//! - Current code treats exit code 2 as error, not as "path needs resolution"
//! - Syncore runs from ~/.config/syncore/, not project root
//! - Relative paths like "indexer.rs" don't exist in syncore's CWD
//!
//! EXPECTED: These tests FAIL initially, then PASS after fix

use anyhow::Result;
use std::fs;
use std::io::Write as IoWrite;
use std::path::Path;
use syncore::parser::RipgrepSearcher;
use tempfile::TempDir;

/// Test that RipgrepSearcher works with absolute file paths
#[test]
fn test_search_with_absolute_file_path() -> Result<()> {
    // Arrange: Create a test file with known content
    let temp_dir = TempDir::new()?;
    let test_file = temp_dir.path().join("test.rs");

    let mut file = fs::File::create(&test_file)?;
    writeln!(file, "use std::collections::HashMap;")?;
    writeln!(file, "use std::collections::HashSet;")?;
    writeln!(file, "fn test_function() {{}}")?;
    drop(file);

    // Act: Search with absolute file path
    let results = RipgrepSearcher::search("HashSet", &test_file, 0)?;

    // Assert: Should find match
    assert_eq!(
        results.len(),
        1,
        "Should find 1 match for 'HashSet' in absolute file path"
    );
    assert_eq!(results[0].line_number, 2);
    assert!(results[0].line_content.contains("HashSet"));

    Ok(())
}

/// Test that RipgrepSearcher works with relative file paths
#[test]
fn test_search_with_relative_file_path() -> Result<()> {
    // Arrange: Create test file in current directory
    let test_file = "test_relative_search.rs";

    {
        let mut file = fs::File::create(test_file)?;
        writeln!(file, "use std::collections::HashMap;")?;
        writeln!(file, "use std::collections::HashSet;")?;
        writeln!(file, "fn test_function() {{}}")?;
    }

    // Act: Search with relative file path
    let results = RipgrepSearcher::search("HashSet", Path::new(test_file), 0)?;

    // Assert: Should find match
    assert_eq!(
        results.len(),
        1,
        "Should find 1 match for 'HashSet' in relative file path"
    );

    // Cleanup
    fs::remove_file(test_file)?;

    Ok(())
}

/// Test that RipgrepSearcher works with directory paths
#[test]
fn test_search_with_directory_path() -> Result<()> {
    // Arrange: Create directory with multiple files
    let temp_dir = TempDir::new()?;

    let file1 = temp_dir.path().join("file1.rs");
    let mut f = fs::File::create(&file1)?;
    writeln!(f, "use std::collections::HashSet;")?;
    drop(f);

    let file2 = temp_dir.path().join("file2.rs");
    let mut f = fs::File::create(&file2)?;
    writeln!(f, "use std::collections::HashMap;")?;
    writeln!(f, "use std::collections::HashSet;")?;
    drop(f);

    // Act: Search directory
    let results = RipgrepSearcher::search("HashSet", temp_dir.path(), 0)?;

    // Assert: Should find matches from both files
    assert_eq!(
        results.len(),
        2,
        "Should find 2 matches for 'HashSet' across directory"
    );

    Ok(())
}

/// Test that RipgrepSearcher handles non-existent file gracefully
#[test]
fn test_search_with_nonexistent_file() -> Result<()> {
    // Arrange: Non-existent file path
    let nonexistent = Path::new("/tmp/nonexistent_file_xyz123.rs");

    // Act: Search should return error or empty results
    let result = RipgrepSearcher::search("pattern", nonexistent, 0);

    // Assert: Should either error gracefully OR return empty results
    // Current behavior: Returns error with exit code 2
    // After fix: Should return Ok(Vec::new()) or clear error message
    match result {
        Ok(results) => {
            assert_eq!(
                results.len(),
                0,
                "Non-existent file should return empty results"
            );
        }
        Err(e) => {
            let msg = e.to_string();
            assert!(
                msg.contains("not found") || msg.contains("No such file"),
                "Error message should indicate file not found: {}",
                msg
            );
        }
    }

    Ok(())
}

/// Test that RipgrepSearcher handles file with no matches
#[test]
fn test_search_with_no_matches() -> Result<()> {
    // Arrange: Create file without the search pattern
    let temp_dir = TempDir::new()?;
    let test_file = temp_dir.path().join("no_match.rs");

    let mut file = fs::File::create(&test_file)?;
    writeln!(file, "use std::collections::HashMap;")?;
    writeln!(file, "fn test_function() {{}}")?;
    drop(file);

    // Act: Search for pattern that doesn't exist
    let results = RipgrepSearcher::search("NONEXISTENT_PATTERN", &test_file, 0)?;

    // Assert: Should return empty results (exit code 1)
    assert_eq!(
        results.len(),
        0,
        "File with no matches should return empty results"
    );

    Ok(())
}

/// Regression test: Verify parser_search tool integration
#[test]
fn test_integration_with_parser_search() -> Result<()> {
    // Arrange: Create test file
    let temp_dir = TempDir::new()?;
    let test_file = temp_dir.path().join("integration.rs");

    let mut file = fs::File::create(&test_file)?;
    writeln!(file, "pub struct MyStruct {{}}")?;
    writeln!(file, "impl MyStruct {{}}")?;
    drop(file);

    // Act: Use RipgrepSearcher directly (simulates parser_search MCP tool)
    let results = RipgrepSearcher::search("struct", &test_file, 0)?;

    // Assert: Should find the struct definition
    assert!(
        results.len() > 0,
        "parser_search should find struct definitions in files"
    );
    assert_eq!(results[0].line_number, 1);

    Ok(())
}

/// Real-world test: Search in actual syncore source file
#[test]
fn test_search_in_syncore_source() -> Result<()> {
    // Arrange: Use actual syncore source file (if exists)
    let parser_rs = Path::new("/home/feanor/Projects/SynCore/syncore/src/parser.rs");

    if !parser_rs.exists() {
        eprintln!("Skipping test: parser.rs not found at expected location");
        return Ok(());
    }

    // Act: Search for known pattern in parser.rs
    let results = RipgrepSearcher::search("RipgrepSearcher", parser_rs, 0)?;

    // Assert: Should find RipgrepSearcher struct definition
    assert!(
        results.len() > 0,
        "Should find 'RipgrepSearcher' in parser.rs"
    );

    // Verify we got actual line numbers
    for result in &results {
        assert!(
            result.line_number > 0,
            "Line number should be positive: {}",
            result.line_number
        );
        assert!(
            result.line_content.contains("RipgrepSearcher"),
            "Line content should contain pattern: {}",
            result.line_content
        );
    }

    Ok(())
}
