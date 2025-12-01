//! Tests for Index Application Multilanguage Functionality

use anyhow::Result;
use std::fs;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

use syncore::code_graph::{IndexApplication, LanguageParser};
use syncore::vector::{HuggingFaceEmbeddings, VectorStore};

/// Helper function to ensure code graph schema exists
fn ensure_code_graph_schema(conn: &rusqlite::Connection) -> Result<()> {
    // Create code_entities table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS code_entities (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_path TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            name TEXT NOT NULL,
            signature TEXT,
            line_start INTEGER NOT NULL,
            line_end INTEGER NOT NULL,
            docstring TEXT,
            language TEXT NOT NULL,
            indexed_at INTEGER NOT NULL,
            UNIQUE(file_path, entity_type, name, line_start)
        )",
        [],
    )?;

    // Create code_edges table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS code_edges (
            src_entity_id INTEGER NOT NULL,
            dst_entity_id INTEGER NOT NULL,
            edge_type TEXT NOT NULL,
            PRIMARY KEY (src_entity_id, dst_entity_id, edge_type)
        )",
        [],
    )?;

    // Create code_embeddings table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS code_embeddings (
            entity_id INTEGER PRIMARY KEY,
            vector_id INTEGER NOT NULL,
            model_version TEXT NOT NULL,
            created_at INTEGER NOT NULL
        )",
        [],
    )?;

    Ok(())
}

fn create_test_index_app(db_path: &str) -> Result<IndexApplication> {
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    IndexApplication::new(db_path, vector_store)
}

#[test]
fn test_language_detection_rust() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");
    let app = create_test_index_app(db_path.to_str().unwrap())?;

    // Test various Rust file extensions
    assert_eq!(app.detect_language(std::path::Path::new("test.rs"))?, "rust");
    assert_eq!(app.detect_language(std::path::Path::new("lib.rs"))?, "rust");
    assert_eq!(app.detect_language(std::path::Path::new("main.rs"))?, "rust");
    assert_eq!(app.detect_language(std::path::Path::new("/path/to/module.rs"))?, "rust");

    Ok(())
}

#[test]
fn test_language_detection_python() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");
    let app = create_test_index_app(db_path.to_str().unwrap())?;

    // Test various Python file extensions
    assert_eq!(app.detect_language(std::path::Path::new("test.py"))?, "python");
    assert_eq!(app.detect_language(std::path::Path::new("main.py"))?, "python");
    assert_eq!(app.detect_language(std::path::Path::new("script.py"))?, "python");
    assert_eq!(app.detect_language(std::path::Path::new("/path/to/module.py"))?, "python");

    Ok(())
}

#[test]
fn test_language_detection_unsupported() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");
    let app = create_test_index_app(db_path.to_str().unwrap())?;

    // Test unsupported file extensions
    assert!(app.detect_language(std::path::Path::new("test.js")).is_err());
    assert!(app.detect_language(std::path::Path::new("test.ts")).is_err());
    assert!(app.detect_language(std::path::Path::new("test.java")).is_err());
    assert!(app.detect_language(std::path::Path::new("test.cpp")).is_err());
    assert!(app.detect_language(std::path::Path::new("test")).is_err());
    assert!(app.detect_language(std::path::Path::new("")).is_err());

    Ok(())
}

#[test]
fn test_index_rust_file() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");
    let mut app = create_test_index_app(db_path.to_str().unwrap())?;

    // Create test Rust file
    let rust_file = temp_dir.path().join("test.rs");
    let rust_code = r#"
/// A simple test function
pub fn add_numbers(a: i32, b: i32) -> i32 {
    a + b
}

/// A test struct
pub struct Calculator {
    value: i32,
}

impl Calculator {
    pub fn new(value: i32) -> Self {
        Self { value }
    }
    
    pub fn add(&mut self, other: i32) {
        self.value += other;
    }
}

use std::collections::HashMap;
"#;
    fs::write(&rust_file, rust_code)?;

    let count = app.index_file(&rust_file)?;

    // Should index: 1 function + 1 struct + 2 methods + 1 import = 5 entities
    assert!(count >= 5);

    // Verify entities were stored in database
    let conn = rusqlite::Connection::open(&db_path)?;
    ensure_code_graph_schema(&conn)?;

    let entity_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM code_entities WHERE file_path = ?",
        [rust_file.to_string_lossy().as_ref()],
        |row| row.get(0),
    )?;
    assert!(entity_count >= 5);

    let rust_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM code_entities WHERE language = 'rust'", [], |row| {
            row.get(0)
        })?;
    assert!(rust_count >= 5);

    Ok(())
}

#[test]
fn test_index_python_file() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");
    let mut app = create_test_index_app(db_path.to_str().unwrap())?;

    // Create test Python file
    let py_file = temp_dir.path().join("test.py");
    let python_code = r#"
"""A simple test module."""

import os
import sys as system
from typing import List, Dict

def add_numbers(a: int, b: int) -> int:
    """Add two numbers together."""
    return a + b

class Calculator:
    """A simple calculator class."""
    
    def __init__(self, initial_value: int = 0):
        """Initialize calculator."""
        self.value = initial_value
    
    def add(self, other: int) -> int:
        """Add value to calculator."""
        self.value += other
        return self.value
    
    def get_value(self) -> int:
        """Get current value."""
        return self.value
"#;
    fs::write(&py_file, python_code)?;

    let count = app.index_file(&py_file)?;

    // Debug: Print actual count
    println!("Actual Python entity count: {}", count);

    // Should index: 1 function + 1 class + 3 methods + 2 imports = 7 entities
    assert!(count >= 7, "Expected at least 7 entities, got {}", count);

    // Verify entities were stored in database
    let conn = rusqlite::Connection::open(&db_path)?;
    ensure_code_graph_schema(&conn)?;

    let entity_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM code_entities WHERE file_path = ?",
        [py_file.to_string_lossy().as_ref()],
        |row| row.get(0),
    )?;
    assert!(entity_count >= 7);

    let python_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM code_entities WHERE language = 'python'",
        [],
        |row| row.get(0),
    )?;
    assert!(python_count >= 7);

    Ok(())
}

#[test]
fn test_index_multiple_files_mixed_languages() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");
    let mut app = create_test_index_app(db_path.to_str().unwrap())?;

    // Create Rust file
    let rust_file = temp_dir.path().join("rust_module.rs");
    let rust_code = r#"
pub fn rust_function() -> String {
    "Hello from Rust".to_string()
}
"#;
    fs::write(&rust_file, rust_code)?;

    // Create Python file
    let py_file = temp_dir.path().join("python_module.py");
    let python_code = r#"
def python_function():
    return "Hello from Python"
"#;
    fs::write(&py_file, python_code)?;

    // Index both files
    let files = vec![&rust_file, &py_file];
    let total_count = app.index_files(&files)?;

    // Should index at least 2 entities (one from each file)
    assert!(total_count >= 2);

    // Verify both languages are represented
    let conn = rusqlite::Connection::open(&db_path)?;
    ensure_code_graph_schema(&conn)?;

    let rust_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM code_entities WHERE language = 'rust'", [], |row| {
            row.get(0)
        })?;
    let python_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM code_entities WHERE language = 'python'",
        [],
        |row| row.get(0),
    )?;

    assert!(rust_count >= 1);
    assert!(python_count >= 1);

    Ok(())
}

#[test]
fn test_index_file_incremental_behavior() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");
    let mut app = create_test_index_app(db_path.to_str().unwrap())?;

    // Create initial file
    let test_file = temp_dir.path().join("test.rs");
    let initial_code = r#"
pub fn original_function() {
    println!("Original");
}
"#;
    fs::write(&test_file, initial_code)?;

    // Index first time
    let count1 = app.index_file(&test_file)?;
    assert!(count1 >= 1);

    // Modify file
    let modified_code = r#"
pub fn original_function() {
    println!("Original");
}

pub fn new_function() {
    println!("New");
}
"#;
    fs::write(&test_file, modified_code)?;

    // Index again (should detect changes and re-index)
    let count2 = app.index_file(&test_file)?;
    assert!(count2 >= 2); // Should have more entities now

    // Verify database reflects the changes
    let conn = rusqlite::Connection::open(&db_path)?;
    ensure_code_graph_schema(&conn)?;

    let entity_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM code_entities WHERE file_path = ?",
        [test_file.to_string_lossy().as_ref()],
        |row| row.get(0),
    )?;
    assert!(entity_count >= 2);

    Ok(())
}

#[test]
fn test_code_graph_access() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");
    let mut app = create_test_index_app(db_path.to_str().unwrap())?;

    // Test that we can access the underlying CodeGraph
    let _code_graph = app.code_graph();

    // Test that we can access mutable CodeGraph
    let _code_graph_mut = app.code_graph_mut();

    Ok(())
}

#[test]
fn test_manual_database_seeding_verification() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");

    // Create database connection and ensure schema
    let conn = rusqlite::Connection::open(&db_path)?;
    ensure_code_graph_schema(&conn)?;

    // Manually insert test data for both languages
    conn.execute(
        "INSERT INTO code_entities 
         (file_path, entity_type, name, signature, line_start, line_end, docstring, language, indexed_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params![
            "/test/rust_file.rs",
            "function",
            "rust_function",
            "rust_function()",
            1,
            3,
            "Rust function",
            "rust",
            chrono::Utc::now().timestamp(),
        ],
    )?;

    conn.execute(
        "INSERT INTO code_entities 
         (file_path, entity_type, name, signature, line_start, line_end, docstring, language, indexed_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params![
            "/test/python_file.py",
            "function",
            "python_function",
            "python_function()",
            1,
            3,
            "Python function",
            "python",
            chrono::Utc::now().timestamp(),
        ],
    )?;

    // Verify data was inserted correctly
    let rust_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM code_entities WHERE language = 'rust'", [], |row| {
            row.get(0)
        })?;
    assert_eq!(rust_count, 1);

    let python_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM code_entities WHERE language = 'python'",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(python_count, 1);

    let total_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM code_entities", [], |row| row.get(0))?;
    assert_eq!(total_count, 2);

    Ok(())
}
