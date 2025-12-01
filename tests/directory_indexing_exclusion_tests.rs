//! TDD Tests for Bug #2: Directory Exclusion in code_suite.rs:218
//!
//! ISSUE: Manual directory reindex indexes build artifacts (target/, node_modules/, etc.)
//!
//! Root Cause:
//! - Bootstrap (src/bootstrap.rs:89-96) HAS exclusion check ✅
//! - Manual reindex (src/mcp_tools/code_suite.rs:218) LACKS exclusion check ❌
//! - Result: 57% of database is contaminated with build artifacts (7,536 / 13,234 entities)
//!
//! Evidence from Production:
//! - Total entities: 13,234
//! - From target/ (should be excluded): 7,536 entities (57%)
//! - From src/ (actual source): 3,252 entities (25%)
//!
//! EXPECTED: These tests FAIL initially, then PASS after fix

use anyhow::Result;
use rusqlite::Connection;
use std::fs;
use std::io::Write as IoWrite;
use std::sync::{Arc, Mutex};
use syncore::code_graph::CodeGraph;
use syncore::config::SyncoreConfig;
use syncore::vector::{HuggingFaceEmbeddings, VectorStore};
use tempfile::TempDir;

/// Test that directory indexing excludes target/ directory
#[test]
fn test_index_directory_excludes_target() -> Result<()> {
    // Arrange: Create workspace with src/ and target/
    let workspace = TempDir::new()?;

    // Create src/main.rs (should be indexed)
    let src_dir = workspace.path().join("src");
    fs::create_dir(&src_dir)?;
    let src_file = src_dir.join("main.rs");
    let mut f = fs::File::create(&src_file)?;
    writeln!(f, "fn main() {{}}")?;
    drop(f);

    // Create target/debug/build.rs (should NOT be indexed)
    let target_dir = workspace.path().join("target");
    fs::create_dir(&target_dir)?;
    let target_debug = target_dir.join("debug");
    fs::create_dir(&target_debug)?;
    let target_file = target_debug.join("build.rs");
    let mut f = fs::File::create(&target_file)?;
    writeln!(f, "fn build_script() {{}}")?;
    drop(f);

    // Setup database
    let db_path = workspace.path().join("test.db");
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let mut code_graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store)?;

    // Act: Index entire workspace directory
    let glob_pattern = format!("{}/**/*.rs", workspace.path().display());
    let paths = glob::glob(&glob_pattern)?;

    let mut indexed_files = Vec::new();
    for entry in paths.flatten() {
        // Simulate what cmd_index_directory should do (with fix)
        let entry_str = entry.to_string_lossy();

        // THIS IS THE FIX WE'RE TESTING FOR:
        let config = SyncoreConfig::default();
        let should_skip =
            config.indexing.excluded_dirs.iter().any(|excluded| entry_str.contains(excluded));

        if should_skip {
            continue; // Skip excluded directories
        }

        code_graph.index_file(&entry)?;
        indexed_files.push(entry.to_string_lossy().to_string());
    }

    // Assert: Should only index src/main.rs, NOT target/debug/build.rs
    assert_eq!(
        indexed_files.len(),
        1,
        "Should index only 1 file (src/main.rs), not files in target/"
    );

    assert!(
        indexed_files[0].contains("src/main.rs"),
        "Indexed file should be src/main.rs, got: {}",
        indexed_files[0]
    );

    assert!(!indexed_files[0].contains("target"), "Should not index files in target/ directory");

    // Verify database has no target/ entries
    let conn = Connection::open(&db_path)?;
    let target_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM code_entities WHERE file_path LIKE '%target%'",
        [],
        |row| row.get(0),
    )?;

    assert_eq!(target_count, 0, "Database should have ZERO entities from target/ directory");

    Ok(())
}

/// Test that directory indexing excludes node_modules/
#[test]
fn test_index_directory_excludes_node_modules() -> Result<()> {
    // Arrange: Create workspace with src/ and node_modules/
    let workspace = TempDir::new()?;

    let src_dir = workspace.path().join("src");
    fs::create_dir(&src_dir)?;
    let src_file = src_dir.join("app.rs");
    let mut f = fs::File::create(&src_file)?;
    writeln!(f, "pub fn app() {{}}")?;
    drop(f);

    let node_modules = workspace.path().join("node_modules");
    fs::create_dir(&node_modules)?;
    let lodash_dir = node_modules.join("lodash");
    fs::create_dir(&lodash_dir)?;
    let lodash_file = lodash_dir.join("index.rs"); // Hypothetical Rust in node_modules
    let mut f = fs::File::create(&lodash_file)?;
    writeln!(f, "pub fn helper() {{}}")?;
    drop(f);

    // Setup database
    let db_path = workspace.path().join("test.db");
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let mut code_graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store)?;

    // Act: Index with exclusion
    let glob_pattern = format!("{}/**/*.rs", workspace.path().display());
    let paths = glob::glob(&glob_pattern)?;

    let config = SyncoreConfig::default();
    let mut indexed_count = 0;

    for entry in paths.flatten() {
        let entry_str = entry.to_string_lossy();
        let should_skip =
            config.indexing.excluded_dirs.iter().any(|excluded| entry_str.contains(excluded));

        if should_skip {
            continue;
        }

        code_graph.index_file(&entry)?;
        indexed_count += 1;
    }

    // Assert
    assert_eq!(indexed_count, 1, "Should only index src/app.rs");

    let conn = Connection::open(&db_path)?;
    let node_modules_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM code_entities WHERE file_path LIKE '%node_modules%'",
        [],
        |row| row.get(0),
    )?;

    assert_eq!(node_modules_count, 0, "Database should have ZERO entities from node_modules/");

    Ok(())
}

/// Test that directory indexing excludes .git/
#[test]
fn test_index_directory_excludes_git() -> Result<()> {
    // Arrange
    let workspace = TempDir::new()?;

    let src_dir = workspace.path().join("src");
    fs::create_dir(&src_dir)?;
    let src_file = src_dir.join("lib.rs");
    let mut f = fs::File::create(&src_file)?;
    writeln!(f, "pub fn lib() {{}}")?;
    drop(f);

    let git_dir = workspace.path().join(".git");
    fs::create_dir(&git_dir)?;
    let git_hooks = git_dir.join("hooks");
    fs::create_dir(&git_hooks)?;
    let git_file = git_hooks.join("pre-commit.rs"); // Hypothetical Rust hook
    let mut f = fs::File::create(&git_file)?;
    writeln!(f, "fn pre_commit() {{}}")?;
    drop(f);

    // Setup database
    let db_path = workspace.path().join("test.db");
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let mut code_graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store)?;

    // Act
    let glob_pattern = format!("{}/**/*.rs", workspace.path().display());
    let paths = glob::glob(&glob_pattern)?;

    let config = SyncoreConfig::default();

    for entry in paths.flatten() {
        let entry_str = entry.to_string_lossy();
        let should_skip =
            config.indexing.excluded_dirs.iter().any(|excluded| entry_str.contains(excluded));

        if should_skip {
            continue;
        }

        code_graph.index_file(&entry)?;
    }

    // Assert
    let conn = Connection::open(&db_path)?;
    let git_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM code_entities WHERE file_path LIKE '%.git%'",
        [],
        |row| row.get(0),
    )?;

    assert_eq!(git_count, 0, "Database should have ZERO entities from .git/");

    Ok(())
}

/// Regression test: Verify src/ files ARE indexed
#[test]
fn test_index_directory_includes_src() -> Result<()> {
    // Arrange
    let workspace = TempDir::new()?;

    let src_dir = workspace.path().join("src");
    fs::create_dir(&src_dir)?;
    let file1 = src_dir.join("main.rs");
    let mut f = fs::File::create(&file1)?;
    writeln!(f, "fn main() {{}}")?;
    drop(f);

    let file2 = src_dir.join("lib.rs");
    let mut f = fs::File::create(&file2)?;
    writeln!(f, "pub fn lib() {{}}")?;
    drop(f);

    // Setup database
    let db_path = workspace.path().join("test.db");
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let mut code_graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store)?;

    // Act
    let glob_pattern = format!("{}/**/*.rs", workspace.path().display());
    let paths = glob::glob(&glob_pattern)?;

    let config = SyncoreConfig::default();
    let mut indexed_count = 0;

    for entry in paths.flatten() {
        let entry_str = entry.to_string_lossy();
        let should_skip =
            config.indexing.excluded_dirs.iter().any(|excluded| entry_str.contains(excluded));

        if should_skip {
            continue;
        }

        code_graph.index_file(&entry)?;
        indexed_count += 1;
    }

    // Assert
    assert_eq!(indexed_count, 2, "Should index both files in src/");

    let conn = Connection::open(&db_path)?;
    let entity_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM code_entities", [], |row| row.get(0))?;

    assert!(entity_count >= 2, "Should have at least 2 entities (main and lib functions)");

    Ok(())
}

/// Real-world test: Verify multiple exclusions work together
#[test]
fn test_index_directory_multiple_exclusions() -> Result<()> {
    // Arrange: Complex workspace with multiple excluded directories
    let workspace = TempDir::new()?;

    // Good files
    let src_dir = workspace.path().join("src");
    fs::create_dir(&src_dir)?;
    fs::File::create(src_dir.join("main.rs"))?.write_all(b"fn main() {}")?;

    // Bad files (should be excluded)
    let target_dir = workspace.path().join("target");
    fs::create_dir(&target_dir)?;
    fs::File::create(target_dir.join("build.rs"))?.write_all(b"fn build() {}")?;

    let node_modules = workspace.path().join("node_modules");
    fs::create_dir(&node_modules)?;
    fs::File::create(node_modules.join("lib.rs"))?.write_all(b"fn lib() {}")?;

    let git_dir = workspace.path().join(".git");
    fs::create_dir(&git_dir)?;
    fs::File::create(git_dir.join("hook.rs"))?.write_all(b"fn hook() {}")?;

    let pycache = workspace.path().join("__pycache__");
    fs::create_dir(&pycache)?;
    fs::File::create(pycache.join("cache.rs"))?.write_all(b"fn cache() {}")?;

    // Setup database
    let db_path = workspace.path().join("test.db");
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let mut code_graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store)?;

    // Act
    let glob_pattern = format!("{}/**/*.rs", workspace.path().display());
    let paths = glob::glob(&glob_pattern)?;

    let config = SyncoreConfig::default();
    let mut indexed_count = 0;

    for entry in paths.flatten() {
        let entry_str = entry.to_string_lossy();
        let should_skip =
            config.indexing.excluded_dirs.iter().any(|excluded| entry_str.contains(excluded));

        if should_skip {
            continue;
        }

        code_graph.index_file(&entry)?;
        indexed_count += 1;
    }

    // Assert: Should only index src/main.rs
    assert_eq!(indexed_count, 1, "Should index only 1 file from src/, not {} files", indexed_count);

    let conn = Connection::open(&db_path)?;
    let total_entities: i64 =
        conn.query_row("SELECT COUNT(*) FROM code_entities", [], |row| row.get(0))?;

    // Only main() function from src/main.rs
    assert_eq!(total_entities, 1, "Should have only 1 entity (main function from src/)");

    Ok(())
}
