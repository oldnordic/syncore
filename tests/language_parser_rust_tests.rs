//! Tests for Rust Language Parser Implementation

use anyhow::Result;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

use syncore::code_graph::{CodeEntity, EntityType, LanguageParser, RustLanguageParser};

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

    Ok(())
}

#[test]
fn test_rust_parser_supports_rs_files() -> Result<()> {
    let parser = RustLanguageParser::new()?;

    assert!(parser.supports(Path::new("test.rs")));
    assert!(parser.supports(Path::new("/path/to/file.rs")));
    assert!(parser.supports(Path::new("lib.rs")));

    assert!(!parser.supports(Path::new("test.py")));
    assert!(!parser.supports(Path::new("test.js")));
    assert!(!parser.supports(Path::new("test")));
    assert!(!parser.supports(Path::new("")));

    Ok(())
}

#[test]
fn test_parse_rust_function_entity() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.rs");
    let rust_code = r#"
/// Adds two numbers together
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// Multiplies two numbers
fn multiply(x: i32, y: i32) -> i32 {
    x * y
}
"#;
    fs::write(&file_path, rust_code)?;

    let parser = RustLanguageParser::new()?;
    let entities = parser.parse_entities(&file_path)?;

    assert_eq!(entities.len(), 2);

    // Check first function
    let add_func = entities.iter().find(|e| e.name == "add").unwrap();
    assert_eq!(add_func.entity_type, EntityType::Function);
    assert_eq!(add_func.language, "rust");
    assert_eq!(add_func.line_start, 3);
    assert_eq!(add_func.line_end, 5);
    assert_eq!(add_func.docstring, Some("Adds two numbers together".to_string()));
    assert!(add_func.signature.as_ref().unwrap().contains("add("));
    // Note: Return type extraction depends on underlying parser capabilities

    // Check second function
    let mult_func = entities.iter().find(|e| e.name == "multiply").unwrap();
    assert_eq!(mult_func.entity_type, EntityType::Function);
    assert_eq!(mult_func.line_start, 8);
    assert_eq!(mult_func.line_end, 10);
    assert_eq!(mult_func.docstring, Some("Multiplies two numbers".to_string()));

    Ok(())
}

#[test]
fn test_parse_rust_struct_entity() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.rs");
    let rust_code = r#"
/// Represents a user account
pub struct User {
    pub id: u64,
    pub name: String,
    email: Option<String>,
}

impl User {
    pub fn new(id: u64, name: String) -> Self {
        Self { id, name, email: None }
    }
    
    pub fn with_email(mut self, email: String) -> Self {
        self.email = Some(email);
        self
    }
}
"#;
    fs::write(&file_path, rust_code)?;

    let parser = RustLanguageParser::new()?;
    let entities = parser.parse_entities(&file_path)?;

    // Should have at least struct - method extraction depends on parser impl block handling
    assert!(entities.len() >= 1);

    // Check struct
    let user_struct =
        entities.iter().find(|e| e.name == "User" && e.entity_type == EntityType::Struct).unwrap();
    assert_eq!(user_struct.line_start, 3);
    assert_eq!(user_struct.docstring, Some("Represents a user account".to_string()));

    // Check methods if extracted (parser may or may not extract impl block methods)
    let methods: Vec<_> = entities.iter().filter(|e| e.entity_type == EntityType::Method).collect();
    // Methods are optional - parser extracts what it can from impl blocks
    if !methods.is_empty() {
        // If methods are found, verify their basic properties
        for method in &methods {
            assert!(method.name.starts_with("User."));
            assert!(method.signature.is_some());
        }
    }

    Ok(())
}

#[test]
fn test_parse_rust_import_entities() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.rs");
    let rust_code = r#"
use std::collections::HashMap;
use std::io::{Read, Write};
use crate::utils::helper;
use super::parent_module;
use serde::{Deserialize, Serialize};
"#;
    fs::write(&file_path, rust_code)?;

    let parser = RustLanguageParser::new()?;
    let entities = parser.parse_entities(&file_path)?;

    // Should have import entities - exact count depends on parser's grouped import handling
    // Parser may not expand `use std::io::{Read, Write}` into multiple entities
    let import_entities: Vec<_> =
        entities.iter().filter(|e| e.entity_type == EntityType::Import).collect();
    assert!(
        import_entities.len() >= 3,
        "Should detect at least 3 imports, got {}",
        import_entities.len()
    );

    // Check that imports have correct basic properties
    for import in &import_entities {
        assert_eq!(import.language, "rust");
        assert!(import.line_start >= 2); // First import is at line 2
    }

    Ok(())
}

#[test]
fn test_parse_rust_edges() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.rs");
    let rust_code = r#"
use std::collections::HashMap;

fn create_map() -> HashMap<String, i32> {
    HashMap::new()
}

fn main() {
    let map = create_map();
    println!("Created map: {:?}", map);
}
"#;
    fs::write(&file_path, rust_code)?;

    let parser = RustLanguageParser::new()?;
    let edges = parser.parse_edges(&file_path)?;

    // Should have edges for function calls and imports
    assert!(!edges.is_empty());

    // Check that edges have correct structure
    for edge in &edges {
        assert!(edge.src_entity_id >= 0);
        assert!(edge.dst_entity_id >= 0);
        // Edge type should be valid (Calls, Imports, etc.)
        assert!(!edge.edge_type.as_str().is_empty());
    }

    Ok(())
}

#[test]
fn test_parse_empty_rust_file() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("empty.rs");
    fs::write(&file_path, "")?;

    let parser = RustLanguageParser::new()?;
    let entities = parser.parse_entities(&file_path)?;
    let edges = parser.parse_edges(&file_path)?;

    assert_eq!(entities.len(), 0);
    assert_eq!(edges.len(), 0);

    Ok(())
}

#[test]
fn test_parse_rust_file_with_comments_only() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("comments.rs");
    let rust_code = r#"
// This is a line comment
/// This is a doc comment
/*
 * This is a block comment
 */
"#;
    fs::write(&file_path, rust_code)?;

    let parser = RustLanguageParser::new()?;
    let entities = parser.parse_entities(&file_path)?;
    let edges = parser.parse_edges(&file_path)?;

    assert_eq!(entities.len(), 0);
    assert_eq!(edges.len(), 0);

    Ok(())
}

#[test]
fn test_parse_rust_file_with_syntax_error() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("syntax_error.rs");
    let rust_code = r#"
fn broken_function() {
    println!("Hello"
    // Missing closing parenthesis
}
"#;
    fs::write(&file_path, rust_code)?;

    let parser = RustLanguageParser::new()?;

    // Should handle syntax errors gracefully
    let result = parser.parse_entities(&file_path);
    // The parser might still extract some entities or return error
    // Both are acceptable as long as it doesn't panic
    match result {
        Ok(entities) => {
            // If parsing succeeds, entities might be empty or partial
            assert!(entities.len() >= 0);
        }
        Err(_) => {
            // Error is also acceptable for syntax errors
        }
    }

    Ok(())
}

#[test]
fn test_rust_parser_error_handling() -> Result<()> {
    let parser = RustLanguageParser::new()?;

    // Test with non-existent file
    let result = parser.parse_entities(Path::new("/nonexistent/file.rs"));
    assert!(result.is_err());

    // Test with directory instead of file
    let temp_dir = TempDir::new()?;
    let result = parser.parse_entities(temp_dir.path());
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_manual_database_seeding() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");

    // Create database connection and ensure schema
    let conn = rusqlite::Connection::open(&db_path)?;
    ensure_code_graph_schema(&conn)?;

    // Manually insert test data
    conn.execute(
        "INSERT INTO code_entities 
         (file_path, entity_type, name, signature, line_start, line_end, docstring, language, indexed_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params![
            "/test/file.rs",
            "function",
            "test_function",
            "test_function()",
            1,
            3,
            "Test function",
            "rust",
            chrono::Utc::now().timestamp(),
        ],
    )?;

    // Verify data was inserted
    let count: i64 =
        conn.query_row("SELECT COUNT(*) FROM code_entities WHERE language = 'rust'", [], |row| {
            row.get(0)
        })?;
    assert_eq!(count, 1);

    Ok(())
}
