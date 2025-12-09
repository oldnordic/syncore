// Simplified Indexer Path Resolution TDD Tests
//
// These tests focus on the core glob pattern and path resolution issues
// without requiring internal module access.

use std::fs;
use tempfile::TempDir;

#[test]
fn test_basic_glob_patterns() {
    // Test basic glob pattern matching to understand the root cause
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Create test files
    fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();
    fs::write(src_dir.join("lib.rs"), "pub fn lib() {}").unwrap();

    let sub_dir = src_dir.join("utils");
    fs::create_dir_all(&sub_dir).unwrap();
    fs::write(sub_dir.join("helper.rs"), "pub fn helper() {}").unwrap();

    // Change to temp directory to test relative patterns
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Test different pattern formats
    let test_cases = vec![
        ("src/**/*.rs", "Recursive pattern"),
        ("src/*.rs", "Non-recursive pattern"),
        ("src/**/main.rs", "Specific file recursive"),
        ("*/main.rs", "Wildcard directory"),
    ];

    for (pattern, description) in test_cases {
        println!("\nTesting {}: {}", description, pattern);

        let paths: Vec<String> = glob::glob(pattern)
            .unwrap()
            .filter_map(Result::ok)
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        println!("  Found {} files: {:?}", paths.len(), paths);

        if paths.is_empty() {
            println!("  ❌ No files found - this could be the issue!");
        } else {
            println!("  ✅ Found files");
        }
    }

    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_pattern_wrapping_issue() {
    // Test the specific issue where executor wraps user patterns
    let directory = "src";
    let user_pattern = "*.rs";

    // This is what the executor does: src/macro_tools/executor_real/executors/code_parser_executor.rs:261
    let wrapped_pattern = format!("{}/**/{}", directory, user_pattern);

    println!("Directory: {}", directory);
    println!("User pattern: {}", user_pattern);
    println!("Executor wrapped pattern: {}", wrapped_pattern);

    // Test if this pattern works in a real directory structure
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();
    fs::write(src_dir.join("lib.rs"), "pub fn lib() {}").unwrap();

    let sub_dir = src_dir.join("submodule");
    fs::create_dir_all(&sub_dir).unwrap();
    fs::write(sub_dir.join("mod.rs"), "pub fn test() {}").unwrap();

    // Change to temp dir
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Test the wrapped pattern
    let wrapped_paths: Vec<String> = glob::glob(&wrapped_pattern)
        .unwrap()
        .filter_map(Result::ok)
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    // Test what the user probably intended
    let intended_pattern = "src/**/*.rs";
    let intended_paths: Vec<String> = glob::glob(intended_pattern)
        .unwrap()
        .filter_map(Result::ok)
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    println!("Wrapped pattern found: {} files", wrapped_paths.len());
    println!("Intended pattern found: {} files", intended_paths.len());

    if wrapped_paths.len() < intended_paths.len() {
        println!("❌ PATTERN WRAPPING ISSUE DETECTED!");
        println!("   Wrapped pattern finds fewer files than intended");
    }

    // Restore directory
    std::env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_double_pattern_issue() {
    // Test when user provides a pattern that already includes the directory
    let directory = "src";
    let user_pattern = "src/**/*.rs"; // User might provide this

    // Executor wraps it: src/**/src/**/*.rs (WRONG!)
    let wrapped_pattern = format!("{}/**/{}", directory, user_pattern);

    println!("This demonstrates the DOUBLE PATTERN issue:");
    println!("  Directory: '{}'", directory);
    println!("  User pattern: '{}'", user_pattern);
    println!("  Wrapped pattern: '{}'", wrapped_pattern);
    println!("  This pattern looks for files in: src/**/src/**/*.rs");

    // The correct pattern should just be what the user provided
    let correct_pattern = user_pattern;

    // Test if the patterns are different
    assert_ne!(wrapped_pattern, correct_pattern);
    assert!(wrapped_pattern.contains("src/**/src"), "Wrapped pattern has duplicate src");

    println!("❌ This double pattern issue would cause files to not be found!");
}

#[test]
fn test_current_working_directory_effects() {
    // Test how current working directory affects glob patterns
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(src_dir.join("test.rs"), "pub fn test() {}").unwrap();

    let original_dir = std::env::current_dir().unwrap();
    println!("Original directory: {:?}", original_dir);

    // Test 1: From temp directory (should work with relative pattern)
    std::env::set_current_dir(&temp_dir).unwrap();
    println!("Changed to temp directory: {:?}", std::env::current_dir());

    let relative_pattern = "src/**/*.rs";
    let relative_paths: Vec<String> = glob::glob(relative_pattern)
        .unwrap()
        .filter_map(Result::ok)
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    println!("Relative pattern '{}' found {} files", relative_pattern, relative_paths.len());

    // Test 2: From original directory with absolute pattern
    std::env::set_current_dir(&original_dir).unwrap();
    println!("Changed back to original directory: {:?}", std::env::current_dir());

    let absolute_pattern = temp_dir.path().join("src/**/*.rs").to_string_lossy().to_string();
    let absolute_paths: Vec<String> = glob::glob(&absolute_pattern)
        .unwrap()
        .filter_map(Result::ok)
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    println!("Absolute pattern '{}' found {} files", absolute_pattern, absolute_paths.len());

    // At least one should work
    let total_found = relative_paths.len() + absolute_paths.len();
    println!("Total files found: {}", total_found);

    if total_found == 0 {
        println!("❌ WORKING DIRECTORY ISSUE: No files found from any directory!");
    } else {
        println!("✅ Files can be found when using the correct working directory");
    }

    assert!(total_found > 0, "Should find test files from some directory");
}

#[test]
fn test_path_canonicalization() {
    // Test file path canonicalization that might be failing
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    let test_file = src_dir.join("main.rs");
    fs::write(&test_file, "fn main() {}").unwrap();

    println!("Testing path canonicalization:");
    println!("  Original path: {:?}", test_file);

    // This is what CodeGraph::index_file_internal does
    match test_file.canonicalize() {
        Ok(canonical_path) => {
            println!("  Canonical path: {:?}", canonical_path);
            println!("  ✅ Canonicalization works");
        }
        Err(e) => {
            println!("  ❌ Canonicalization failed: {}", e);
            println!("  This could be the root cause of 0 files indexed!");
        }
    }

    // Test with relative path (common in indexing)
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    let relative_path = Path::new("src/main.rs");
    println!("  Relative path: {:?}", relative_path);

    match relative_path.canonicalize() {
        Ok(canonical_path) => {
            println!("  Canonical from relative: {:?}", canonical_path);
            println!("  ✅ Relative path canonicalization works");
        }
        Err(e) => {
            println!("  ❌ Relative path canonicalization failed: {}", e);
            println!("  This is likely the root cause!");
        }
    }

    std::env::set_current_dir(original_dir).unwrap();
}

extern crate glob;
use std::path::Path;