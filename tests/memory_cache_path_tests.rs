//! Tests for memory cache path handling
//!
//! Ensures that memory tools do not create cache directories in the working directory
//! when using :memory: databases.

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use syncore::memory::Memory;
use tempfile::TempDir;

#[test]
fn test_memory_cache_no_pollution_with_memory_db() {
    let original_cwd = std::env::current_dir().unwrap();

    // Create a temporary directory to work in
    let temp_dir = TempDir::new().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // List files before creating memory
    let files_before: Vec<_> = fs::read_dir(".")
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().into_string().unwrap())
        .collect();

    // Create memory with :memory: database
    let memory = Memory::new(":memory:_test_cache_pollution").expect("Failed to create memory");

    // Store and retrieve some data to trigger cache usage
    memory
        .store("test_key", "test_value")
        .expect("Failed to store data");
    let _value = memory.query("test_key").expect("Failed to query data");

    // List files after memory operations
    let files_after: Vec<_> = fs::read_dir(".")
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().into_string().unwrap())
        .collect();

    // Restore original working directory
    std::env::set_current_dir(original_cwd).unwrap();

    // Assert no cache directories were created in current working directory
    let cache_files: Vec<_> = files_after
        .iter()
        .filter(|name| name.contains(":memory:") && name.contains("_cache"))
        .collect();

    assert!(
        cache_files.is_empty(),
        "Found cache files in working directory: {:?}",
        cache_files
    );

    // Files should be the same as before (no pollution)
    if files_before.len() != files_after.len() {
        let files_before_set: HashSet<_> = files_before.iter().collect();
        let files_after_set: HashSet<_> = files_after.iter().collect();
        let new_files: Vec<_> = files_after_set.difference(&files_before_set).collect();
        println!("New files created:");
        for file in &new_files {
            println!("  {}", file);
        }
    }

    assert_eq!(
        files_before.len(),
        files_after.len(),
        "Working directory was polluted with new files"
    );
}

#[test]
fn test_memory_cache_with_file_db() {
    let original_cwd = std::env::current_dir().unwrap();

    // Create a temporary directory to work in
    let temp_dir = TempDir::new().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    let db_path = "test_memory.db";

    // Create memory with file database
    let memory = Memory::new(db_path).expect("Failed to create memory");

    // Store and retrieve some data to trigger cache usage
    memory
        .store("test_key", "test_value")
        .expect("Failed to store data");
    let _value = memory.query("test_key").expect("Failed to query data");

    // Check that cache directory was created next to the database
    let cache_path = format!("{}_cache", db_path);
    assert!(
        Path::new(&cache_path).exists(),
        "Cache directory should exist for file database"
    );

    // Clean up
    drop(memory);
    fs::remove_file(db_path).ok();
    fs::remove_dir_all(&cache_path).ok();

    // Restore original working directory
    std::env::set_current_dir(original_cwd).unwrap();
}

#[test]
fn test_memory_cache_cleanup_on_drop() {
    let original_cwd = std::env::current_dir().unwrap();

    // Create a temporary directory to work in
    let temp_dir = TempDir::new().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create memory with :memory: database
    {
        let memory = Memory::new(":memory:_test_cleanup").expect("Failed to create memory");

        // Store and retrieve some data
        memory
            .store("test_key", "test_value")
            .expect("Failed to store data");
        let _value = memory.query("test_key").expect("Failed to query data");

        // Memory should be using a temporary cache directory
        // We can't easily check the exact location since it's in temp dir,
        // but we can ensure no pollution in current directory
    } // memory is dropped here

    // List files after memory is dropped
    let files_after: Vec<_> = fs::read_dir(".")
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().into_string().unwrap())
        .collect();

    // Restore original working directory
    std::env::set_current_dir(original_cwd).unwrap();

    // Assert no cache directories were created in current working directory
    let cache_files: Vec<_> = files_after
        .iter()
        .filter(|name| name.contains(":memory:") && name.contains("_cache"))
        .collect();

    assert!(
        cache_files.is_empty(),
        "Found cache files in working directory after drop: {:?}",
        cache_files
    );
}
