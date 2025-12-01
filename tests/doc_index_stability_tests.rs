//! Document indexing stability tests for Phase 4
//! Tests that doc_index respects boundaries, doesn't hang, and doesn't pollute filesystem

use anyhow::Result;
use std::fs;
use std::path::Path;
use syncore::document_indexer::{DocumentIndexer, IndexerConfig};
use tempfile::TempDir;

/// Test that doc_index_respects_ignored_directories
#[tokio::test]
async fn test_doc_index_respects_ignored_directories() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path();

    // Create test directory structure
    fs::create_dir_all(root.join("subdir"))?;
    fs::write(root.join("doc.txt"), "This should be indexed")?;
    fs::write(root.join("subdir/nested.txt"), "This should also be indexed")?;

    // Create ignored directories with files
    fs::create_dir_all(root.join(".git"))?;
    fs::write(root.join(".git/secret.txt"), "This should NOT be indexed")?;

    fs::create_dir_all(root.join("target"))?;
    fs::write(root.join("target/temp.bin"), "This should NOT be indexed")?;

    fs::create_dir_all(root.join("node_modules"))?;
    fs::write(root.join("node_modules/package.json"), "This should NOT be indexed")?;

    fs::create_dir_all(root.join(".vscode"))?;
    fs::write(root.join(".vscode/settings.json"), "This should NOT be indexed")?;

    // Create indexer with default excluded dirs
    let config = IndexerConfig::default();
    let indexer = DocumentIndexer::new(config);

    // Scan directory
    let documents = indexer.scan_directory(root)?;

    // Verify that only non-ignored files are found
    let file_paths: Vec<String> =
        documents.iter().map(|doc| doc.path.to_string_lossy().to_string()).collect();

    assert!(file_paths.iter().any(|p| p.contains("doc.txt")), "Should include doc.txt");
    assert!(file_paths.iter().any(|p| p.contains("nested.txt")), "Should include nested.txt");

    // Verify ignored directories are NOT included
    assert!(!file_paths.iter().any(|p| p.contains(".git")), "Should NOT include .git files");
    assert!(!file_paths.iter().any(|p| p.contains("target")), "Should NOT include target files");
    assert!(
        !file_paths.iter().any(|p| p.contains("node_modules")),
        "Should NOT include node_modules files"
    );
    assert!(!file_paths.iter().any(|p| p.contains(".vscode")), "Should NOT include .vscode files");

    Ok(())
}

/// Test that doc_index_does_not_escape_project_root
#[tokio::test]
async fn test_doc_index_does_not_escape_project_root() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path();

    // Create a file outside temp directory
    let outside_file = temp_dir.path().parent().unwrap().join("outside.txt");
    fs::write(&outside_file, "This should NOT be indexed")?;

    // Create a symlink pointing outside the project root
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&outside_file, root.join("symlink_outside.txt"))?;
    }

    // Create normal file inside
    fs::write(root.join("inside.txt"), "This should be indexed")?;

    // Create indexer
    let config = IndexerConfig::default();
    let indexer = DocumentIndexer::new(config);

    // Scan directory
    let documents = indexer.scan_directory(root)?;

    // Verify that only the inside file is found
    let file_paths: Vec<String> =
        documents.iter().map(|doc| doc.path.to_string_lossy().to_string()).collect();

    assert!(file_paths.iter().any(|p| p.contains("inside.txt")), "Should include inside.txt");

    // The symlink should either be ignored or not resolve outside the root
    let outside_found = file_paths.iter().any(|p| p.contains("outside.txt"));
    assert!(!outside_found, "Should NOT include files outside project root via symlink");

    Ok(())
}

/// Test that doc_index_does_not_create_spurious_directories
#[tokio::test]
async fn test_doc_index_does_not_create_spurious_directories() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path();

    // Record initial directory structure
    let initial_dirs = get_directory_contents(root)?;

    // Create a simple file to index
    fs::write(root.join("test.txt"), "Test content")?;

    // Create indexer
    let config = IndexerConfig::default();
    let indexer = DocumentIndexer::new(config);

    // Scan directory (this should NOT create new directories)
    let _documents = indexer.scan_directory(root)?;

    // Check that no new directories were created in the project root
    let final_dirs = get_directory_contents(root)?;

    // Should only have the new file, no new directories
    let new_dirs: Vec<String> = final_dirs
        .iter()
        .filter(|&(_, is_dir)| *is_dir)
        .map(|(name, _)| name)
        .filter(|name| !initial_dirs.iter().any(|(init_name, _)| init_name == *name))
        .map(|name| name.clone())
        .collect();

    assert!(new_dirs.is_empty(), "doc_index should not create directories: {:?}", new_dirs);

    Ok(())
}

/// Test that doc_index_completes_on_small_tree
#[tokio::test]
async fn test_doc_index_timeout_resilience() -> Result<()> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path().to_path_buf();
    let content = "test content";

    // Create a small but reasonable directory tree
    for i in 0..10 {
        fs::create_dir_all(root.join(format!("dir{}", i))).expect("Failed to create dir");
        for j in 0..5 {
            let file_path = root.join(format!("dir{}/file{}.txt", i, j));
            fs::write(file_path, content).expect("Failed to write file");
        }
    }

    // Create indexer
    let config = IndexerConfig::default();
    let indexer = DocumentIndexer::new(config);

    // Use timeout to ensure completion within reasonable time
    let scan_result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        tokio::task::spawn_blocking(move || indexer.scan_directory(&root)),
    )
    .await;

    // Should complete within 5 seconds
    match scan_result {
        Ok(Ok(documents)) => {
            // Should find all 50 files
            let docs = documents.expect("Failed to get documents");
            assert_eq!(docs.len(), 50, "Should find all 50 files");
        }
        Ok(Err(e)) => {
            return Err(e.into());
        }
        Err(_) => {
            panic!("doc_index scan timed out after 5 seconds");
        }
    }

    Ok(())
}

/// Test that doc_index handles hidden files correctly
#[tokio::test]
async fn test_doc_index_handles_hidden_files() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path();

    // Create hidden and non-hidden files
    fs::write(root.join("visible.txt"), "Visible file")?;
    fs::write(root.join(".hidden.txt"), "Hidden file")?;

    fs::create_dir_all(root.join(".hidden_dir"))?;
    fs::write(root.join(".hidden_dir/content.txt"), "Hidden dir content")?;

    // Create indexer with default config (skip_hidden = true)
    let config = IndexerConfig::default();
    let indexer = DocumentIndexer::new(config);

    // Scan directory
    let documents = indexer.scan_directory(root)?;

    let file_paths: Vec<String> =
        documents.iter().map(|doc| doc.path.to_string_lossy().to_string()).collect();

    // Should include visible file
    assert!(file_paths.iter().any(|p| p.contains("visible.txt")), "Should include visible.txt");

    // Should NOT include hidden files when skip_hidden = true
    assert!(
        !file_paths.iter().any(|p| p.contains(".hidden.txt")),
        "Should NOT include .hidden.txt"
    );
    assert!(
        !file_paths.iter().any(|p| p.contains(".hidden_dir")),
        "Should NOT include .hidden_dir"
    );

    Ok(())
}

/// Helper function to get directory contents (files and dirs)
fn get_directory_contents(path: &Path) -> Result<Vec<(String, bool)>> {
    let mut contents = Vec::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.file_type()?.is_dir();
        contents.push((name, is_dir));
    }

    contents.sort();
    Ok(contents)
}
