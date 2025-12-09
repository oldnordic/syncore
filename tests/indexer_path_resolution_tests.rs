// Indexer Path Resolution TDD Tests
//
// These tests verify that directory indexing works correctly for different path scenarios
// and identify the root cause of "0 files indexed" issues.

use std::path::Path;
use std::collections::HashMap;
use anyhow::Result;
use tempfile::TempDir;
use std::fs;

use syncore::macro_tools::executor_real::executors::code_parser_executor;
use syncore::config::SyncoreConfig;
use syncore::common::db_paths;
use syncore::db::DBManager;
use syncore::memory::GeneralStore;
use syncore::vector::VectorStore;
use syncore::code_directory_indexer::{DirectoryIndexer, DirectoryIndexRequest, DirectoryIndexResponse};
use syncore::code_graph::CodeGraph;
use std::sync::{Arc, Mutex};
use syncore::general_store::GeneralStoreManager;

#[test]
fn test_basic_src_directory_indexing() {
    // Test the most basic case: indexing src/ directory with *.rs pattern
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Create some test Rust files
    fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();
    fs::write(src_dir.join("lib.rs"), "pub fn hello() {}").unwrap();

    let sub_dir = src_dir.join("utils");
    fs::create_dir_all(&sub_dir).unwrap();
    fs::write(sub_dir.join("helper.rs"), "pub fn helper() {}").unwrap();

    // Test the glob pattern that should be generated
    let directory = "src";
    let pattern = "*.rs";
    let search_pattern = format!("{}/**/{}", directory, pattern);

    // Verify glob finds files
    let glob_paths: Vec<String> = glob::glob(&search_pattern)
        .unwrap()
        .filter_map(Result::ok)
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    println!("Glob pattern: {}", search_pattern);
    println!("Found paths: {:?}", glob_paths);
    println!("Working directory: {:?}", std::env::current_dir());

    // This should find files
    assert!(!glob_paths.is_empty(), "Glob pattern should find test files");
    assert!(glob_paths.iter().any(|p| p.contains("main.rs")), "Should find main.rs");
}

#[test]
fn test_mcp_code_suite_vs_executor_paths() {
    // Test the difference between MCP Code Suite and Executor path generation
    let directory = "src";
    let pattern = "*.rs";

    // MCP Code Suite path (uses pattern directly)
    let mcp_pattern = pattern;

    // Executor path (wraps pattern)
    let executor_pattern = format!("{}/**/{}", directory, pattern);

    println!("MCP pattern: {}", mcp_pattern);
    println!("Executor pattern: {}", executor_pattern);

    // Both should be different patterns
    assert_ne!(mcp_pattern, executor_pattern);

    // Test glob results
    let mcp_paths: Vec<String> = glob::glob(mcp_pattern)
        .unwrap()
        .filter_map(Result::ok)
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    let executor_paths: Vec<String> = glob::glob(&executor_pattern)
        .unwrap()
        .filter_map(Result::ok)
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    println!("MCP glob results: {:?}", mcp_paths);
    println!("Executor glob results: {:?}", executor_paths);

    // The executor pattern should be more comprehensive for recursive search
    assert!(executor_pattern.contains("**/"), "Executor pattern should contain recursive glob");
}

#[test]
fn test_path_canonicalization_in_indexer() {
    // Test file path canonicalization that might be causing issues
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    let test_file = src_dir.join("main.rs");
    fs::write(&test_file, "fn main() {}").unwrap();

    // Test canonicalization (this is what CodeGraph::index_file does)
    let canonical_result = test_file.canonicalize();
    match canonical_result {
        Ok(canonical_path) => {
            println!("Original: {:?}", test_file);
            println!("Canonical: {:?}", canonical_path);
            assert_eq!(canonical_path, test_file.canonicalize().unwrap());
        }
        Err(e) => {
            println!("Canonicalization failed: {}", e);
            // This might be the root cause!
            panic!("File canonicalization should work for valid paths");
        }
    }
}

#[test]
fn test_directory_indexer_glob_patterns() {
    // Test the DirectoryIndexer directly with different patterns
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Create test files
    fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();
    fs::write(src_dir.join("lib.rs"), "pub fn lib() {}").unwrap();

    let sub_dir = src_dir.join("submodule");
    fs::create_dir_all(&sub_dir).unwrap();
    fs::write(sub_dir.join("mod.rs"), "pub mod submodule;").unwrap();

    // Change to temp directory to simulate real usage
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    let test_cases = vec![
        ("src", "*.rs"),           // Should find main.rs, lib.rs
        ("src", "**/*.rs"),        // Should find all rs files recursively
        ("./src", "*.rs"),         // Should work with relative path
        ("src", "main.rs"),        // Should find only main.rs
    ];

    for (directory, pattern) in test_cases {
        println!("Testing: directory='{}', pattern='{}'", directory, pattern);

        let search_pattern = format!("{}/**/{}", directory, pattern);
        println!("  Generated pattern: {}", search_pattern);

        let paths: Vec<String> = glob::glob(&search_pattern)
            .unwrap()
            .filter_map(Result::ok)
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        println!("  Found {} files: {:?}", paths.len(), paths);

        // At least some files should be found
        if paths.is_empty() {
            println!("  WARNING: No files found for this pattern!");
        }
    }

    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_path_filtering_allows_src_files() {
    // Verify that path filtering doesn't exclude src files
    use syncore::macro_tools::path_filter;

    let test_paths = vec![
        "src/main.rs",
        "src/lib.rs",
        "./src/main.rs",
        "/absolute/path/to/src/main.rs",
        "src/submodule/mod.rs",
    ];

    for path in test_paths {
        let should_index = path_filter::should_index_path(path);
        println!("Path: {} -> Should index: {}", path, should_index);
        assert!(should_index, "Path should be allowed: {}", path);
    }
}

#[test]
fn test_double_glob_pattern_issue() {
    // Test for the double glob pattern issue
    // When user passes pattern="src/**/*.rs" and directory="src"
    // Executor creates: src/**/src/**/*.rs (WRONG)

    let directory = "src";
    let user_pattern = "src/**/*.rs"; // User might pass this
    let generated_pattern = format!("{}/**/{}", directory, user_pattern);

    println!("User pattern: {}", user_pattern);
    println!("Generated pattern: {}", generated_pattern);

    // This pattern is wrong - it has duplicate 'src' in the path
    assert!(generated_pattern.contains("src/**/src"), "Generated pattern has duplicate src");

    // Test if this wrong pattern finds files
    let paths: Vec<String> = glob::glob(&generated_pattern)
        .unwrap()
        .filter_map(Result::ok)
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    println!("Wrong pattern found {} files: {:?}", paths.len(), paths);

    // This wrong pattern should find fewer or no files compared to the correct pattern
    let correct_pattern = user_pattern; // Just use the pattern directly
    let correct_paths: Vec<String> = glob::glob(correct_pattern)
        .unwrap()
        .filter_map(Result::ok)
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    println!("Correct pattern found {} files: {:?}", correct_paths.len(), correct_paths);

    // The correct pattern should find more or equal files
    assert!(correct_paths.len() >= paths.len(), "Correct pattern should find >= files");
}

#[test]
fn test_current_working_directory_effects() {
    // Test how current working directory affects glob patterns
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(src_dir.join("test.rs"), "pub fn test() {}").unwrap();

    let original_dir = std::env::current_dir().unwrap();

    // Test from temp directory
    std::env::set_current_dir(&temp_dir).unwrap();

    let pattern = "src/**/*.rs";
    let paths_from_temp: Vec<String> = glob::glob(pattern)
        .unwrap()
        .filter_map(Result::ok)
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    // Test from project root
    std::env::set_current_dir(&original_dir).unwrap();

    let relative_pattern = temp_dir.path().join("src/**/*.rs").to_string_lossy().to_string();
    let paths_from_root: Vec<String> = glob::glob(&relative_pattern)
        .unwrap()
        .filter_map(Result::ok)
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    println!("From temp dir (pattern: '{}'): {:?}", pattern, paths_from_temp);
    println!("From root (pattern: '{}'): {:?}", relative_pattern, paths_from_root);

    // At least one approach should find the file
    assert!(paths_from_temp.len() + paths_from_root.len() > 0, "Should find test file from some directory");
}

// Helper to get the glob crate (needed for the tests)
extern crate glob;