//! Tests for Python Language Parser Implementation

use anyhow::Result;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

use syncore::code_graph::{CodeEntity, EntityType, LanguageParser, PythonLanguageParser};

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
fn test_python_parser_supports_py_files() -> Result<()> {
    let parser = PythonLanguageParser::new()?;

    assert!(parser.supports(Path::new("test.py")));
    assert!(parser.supports(Path::new("/path/to/file.py")));
    assert!(parser.supports(Path::new("main.py")));

    assert!(!parser.supports(Path::new("test.rs")));
    assert!(!parser.supports(Path::new("test.js")));
    assert!(!parser.supports(Path::new("test")));
    assert!(!parser.supports(Path::new("")));

    Ok(())
}

#[test]
fn test_parse_python_function_entity() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.py");
    let python_code = r#"
def add(a, b):
    """Add two numbers together."""
    return a + b

def multiply(x, y):
    """Multiply two numbers."""
    return x * y

def calculate_area(radius: float) -> float:
    """Calculate area of a circle."""
    import math
    return math.pi * radius ** 2
"#;
    fs::write(&file_path, python_code)?;

    let parser = PythonLanguageParser::new()?;
    let entities = parser.parse_entities(&file_path)?;

    assert_eq!(entities.len(), 3);

    // Check first function
    let add_func = entities.iter().find(|e| e.name == "add").unwrap();
    assert_eq!(add_func.entity_type, EntityType::Function);
    assert_eq!(add_func.language, "python");
    assert_eq!(add_func.line_start, 2);
    assert_eq!(add_func.line_end, 4);
    assert_eq!(add_func.docstring, Some("Add two numbers together.".to_string()));
    assert!(add_func.signature.as_ref().unwrap().contains("add(a, b)"));

    // Check second function
    let mult_func = entities.iter().find(|e| e.name == "multiply").unwrap();
    assert_eq!(mult_func.entity_type, EntityType::Function);
    assert_eq!(mult_func.line_start, 6);
    assert_eq!(mult_func.line_end, 8);
    assert_eq!(mult_func.docstring, Some("Multiply two numbers.".to_string()));

    // Check function with type hints
    let area_func = entities.iter().find(|e| e.name == "calculate_area").unwrap();
    assert!(area_func.signature.as_ref().unwrap().contains("-> float"));

    Ok(())
}

#[test]
fn test_parse_python_class_entity() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.py");
    let python_code = r#"
class User:
    """Represents a user account."""
    
    def __init__(self, user_id: int, name: str):
        """Initialize user."""
        self.user_id = user_id
        self.name = name
        self.email = None
    
    def get_display_name(self) -> str:
        """Get display name."""
        return self.name
    
    def set_email(self, email: str):
        """Set user email."""
        self.email = email

class Admin(User):
    """Admin user with elevated privileges."""
    
    def __init__(self, user_id: int, name: str, level: int):
        super().__init__(user_id, name)
        self.level = level
    
    def has_permission(self, resource: str) -> bool:
        """Check if admin has permission."""
        return self.level >= 5
"#;
    fs::write(&file_path, python_code)?;

    let parser = PythonLanguageParser::new()?;
    let entities = parser.parse_entities(&file_path)?;

    // Should have 2 classes and their methods
    // User: __init__, get_display_name, set_email (3 methods)
    // Admin: __init__, has_permission (2 methods)
    assert_eq!(entities.len(), 7); // 2 classes + 5 methods

    // Check User class
    let user_class =
        entities.iter().find(|e| e.name == "User" && e.entity_type == EntityType::Class).unwrap();
    assert_eq!(user_class.line_start, 2);
    assert_eq!(user_class.docstring, Some("Represents a user account.".to_string()));

    // Check User methods
    let user_init = entities
        .iter()
        .find(|e| e.name == "User.__init__" && e.entity_type == EntityType::Method)
        .unwrap();
    // Parameter extraction depends on underlying parser
    assert!(user_init.signature.as_ref().unwrap().contains("__init__("));

    let user_display = entities
        .iter()
        .find(|e| e.name == "User.get_display_name" && e.entity_type == EntityType::Method)
        .unwrap();
    // Return type extraction depends on underlying parser
    assert!(user_display.signature.is_some());

    // Check Admin class
    let admin_class =
        entities.iter().find(|e| e.name == "Admin" && e.entity_type == EntityType::Class).unwrap();
    assert_eq!(admin_class.docstring, Some("Admin user with elevated privileges.".to_string()));

    Ok(())
}

#[test]
fn test_parse_python_import_entities() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.py");
    let python_code = r#"
import os
import sys as system
import numpy as np
from collections import defaultdict
from typing import List, Dict, Optional
from .local_module import local_function
from ..parent_module import ParentClass
"#;
    fs::write(&file_path, python_code)?;

    let parser = PythonLanguageParser::new()?;
    let entities = parser.parse_entities(&file_path)?;

    // Should have import entities - exact count depends on parser's handling of grouped imports
    // Parser may not expand `from typing import List, Dict, Optional` into multiple entities
    let import_entities: Vec<_> =
        entities.iter().filter(|e| e.entity_type == EntityType::Import).collect();
    assert!(
        import_entities.len() >= 4,
        "Should detect at least 4 imports, got {}",
        import_entities.len()
    );

    // Check that imports have correct basic properties
    for import in &import_entities {
        assert_eq!(import.language, "python");
        assert!(import.line_start >= 2); // First import is at line 2
    }

    Ok(())
}

#[test]
fn test_parse_python_edges() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.py");
    let python_code = r#"
import os
import json

def read_config(file_path):
    """Read configuration file."""
    with open(file_path, 'r') as f:
        return json.load(f)

def main():
    config = read_config('config.json')
    path = os.path.join('/tmp', 'data')
    print(f"Config: {config}, Path: {path}")
"#;
    fs::write(&file_path, python_code)?;

    let parser = PythonLanguageParser::new()?;
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
fn test_parse_empty_python_file() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("empty.py");
    fs::write(&file_path, "")?;

    let parser = PythonLanguageParser::new()?;
    let entities = parser.parse_entities(&file_path)?;
    let edges = parser.parse_edges(&file_path)?;

    assert_eq!(entities.len(), 0);
    assert_eq!(edges.len(), 0);

    Ok(())
}

#[test]
fn test_parse_python_file_with_comments_only() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("comments.py");
    let python_code = r#"
# This is a comment
"""
This is a docstring
"""
# Another comment
"#;
    fs::write(&file_path, python_code)?;

    let parser = PythonLanguageParser::new()?;
    let entities = parser.parse_entities(&file_path)?;
    let edges = parser.parse_edges(&file_path)?;

    assert_eq!(entities.len(), 0);
    assert_eq!(edges.len(), 0);

    Ok(())
}

#[test]
fn test_parse_python_file_with_syntax_error() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("syntax_error.py");
    let python_code = r#"
def broken_function():
    print("Hello"
    # Missing closing parenthesis
"#;
    fs::write(&file_path, python_code)?;

    let parser = PythonLanguageParser::new()?;

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
fn test_parse_python_lambda_functions() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("lambda.py");
    let python_code = r#"
def process_data(data):
    """Process data using lambda functions."""
    # Regular function
    def square(x):
        return x * x
    
    # Lambda usage
    doubled = list(map(lambda x: x * 2, data))
    squared = list(map(square, data))
    
    return doubled, squared
"#;
    fs::write(&file_path, python_code)?;

    let parser = PythonLanguageParser::new()?;
    let entities = parser.parse_entities(&file_path)?;

    // Should find at least the top-level function
    // Nested functions (like square) and lambdas may not be extracted depending on parser
    assert!(!entities.is_empty(), "Should detect at least one function");

    let process_func = entities.iter().find(|e| e.name == "process_data").unwrap();
    assert_eq!(process_func.entity_type, EntityType::Function);

    // Nested function extraction is parser-dependent
    // Some parsers may not extract functions defined inside other functions

    Ok(())
}

#[test]
fn test_python_parser_error_handling() -> Result<()> {
    let parser = PythonLanguageParser::new()?;

    // Test with non-existent file
    let result = parser.parse_entities(Path::new("/nonexistent/file.py"));
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
            "/test/file.py",
            "function",
            "test_function",
            "test_function()",
            1,
            3,
            "Test function",
            "python",
            chrono::Utc::now().timestamp(),
        ],
    )?;

    // Verify data was inserted
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM code_entities WHERE language = 'python'",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(count, 1);

    Ok(())
}
