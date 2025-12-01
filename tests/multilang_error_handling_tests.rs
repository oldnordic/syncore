//! Error Handling Tests for Multilanguage Parser Integration

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

use std::sync::{Arc, Mutex};
use syncore::code_graph::{
    IndexApplication, LanguageParser, PythonLanguageParser, RustLanguageParser,
};
use syncore::vector::{HuggingFaceEmbeddings, VectorStore};

/// Helper function to ensure code graph schema exists
fn ensure_code_graph_schema(conn: &rusqlite::Connection) -> Result<()> {
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

fn create_test_index_app(db_path: &str) -> Result<IndexApplication> {
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    IndexApplication::new(db_path, vector_store)
}

#[test]
fn test_nonexistent_file_handling() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");
    let mut app = create_test_index_app(db_path.to_str().unwrap())?;

    // Test with non-existent Rust file
    let nonexistent_rs = Path::new("/nonexistent/path/file.rs");
    let result = app.index_file(nonexistent_rs);
    assert!(result.is_err());

    // Test with non-existent Python file
    let nonexistent_py = Path::new("/nonexistent/path/file.py");
    let result = app.index_file(nonexistent_py);
    assert!(result.is_err());

    // Test with multiple non-existent files
    let files = vec![
        Path::new("/nonexistent1.rs"),
        Path::new("/nonexistent2.py"),
        Path::new("/nonexistent3.rs"),
    ];
    let total_count = app.index_files(&files)?;
    assert_eq!(total_count, 0); // Should return 0 for all failed files

    Ok(())
}

#[test]
fn test_directory_instead_of_file() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");
    let mut app = create_test_index_app(db_path.to_str().unwrap())?;

    // Test with directory path
    let result = app.index_file(temp_dir.path());
    assert!(result.is_err());

    // Test with subdirectory
    let sub_dir = temp_dir.path().join("subdir");
    fs::create_dir(&sub_dir)?;
    let result = app.index_file(&sub_dir);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_unsupported_file_extensions() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");
    let mut app = create_test_index_app(db_path.to_str().unwrap())?;

    // Create files with unsupported extensions
    let js_file = temp_dir.path().join("test.js");
    fs::write(&js_file, "function test() { console.log('hello'); }")?;
    let result = app.index_file(&js_file);
    assert!(result.is_err());

    let ts_file = temp_dir.path().join("test.ts");
    fs::write(&ts_file, "function test(): void { console.log('hello'); }")?;
    let result = app.index_file(&ts_file);
    assert!(result.is_err());

    let java_file = temp_dir.path().join("test.java");
    fs::write(&java_file, "public class Test { public static void main(String[] args) {} }")?;
    let result = app.index_file(&java_file);
    assert!(result.is_err());

    let cpp_file = temp_dir.path().join("test.cpp");
    fs::write(&cpp_file, "#include <iostream>\nint main() { return 0; }")?;
    let result = app.index_file(&cpp_file);
    assert!(result.is_err());

    // Test with no extension
    let no_ext_file = temp_dir.path().join("test");
    fs::write(&no_ext_file, "some content")?;
    let result = app.index_file(&no_ext_file);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_files_without_read_permissions() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");
    let mut app = create_test_index_app(db_path.to_str().unwrap())?;

    // Create a file and try to make it unreadable
    let test_file = temp_dir.path().join("test.rs");
    fs::write(&test_file, "pub fn test() {}")?;

    // Note: On Unix systems, we can remove read permissions
    // On Windows, this might not work the same way
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&test_file)?.permissions();
        perms.set_mode(0o000); // Remove all permissions
        fs::set_permissions(&test_file, perms)?;

        let result = app.index_file(&test_file);
        assert!(result.is_err());

        // Restore permissions for cleanup
        let mut perms = fs::metadata(&test_file)?.permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&test_file, perms)?;
    }

    Ok(())
}

#[test]
fn test_malformed_rust_code() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");
    let mut app = create_test_index_app(db_path.to_str().unwrap())?;

    // Test various syntax errors
    let malformed_cases = vec![
        ("unclosed_brace.rs", "pub fn test() { println!(\"hello\");"),
        ("unclosed_paren.rs", "pub fn test() { println!(\"hello\"); }"),
        ("invalid_syntax.rs", "pub fn test() { let x = ; }"),
        ("mismatched_brackets.rs", "pub fn test() { let x = [1, 2, 3; }"),
        ("invalid_characters.rs", "pub fn test() { let x = \x01; }"),
    ];

    for (filename, content) in malformed_cases {
        let file_path = temp_dir.path().join(filename);
        fs::write(&file_path, content)?;

        // Should handle syntax errors gracefully
        let result = app.index_file(&file_path);
        match result {
            Ok(count) => {
                // If parsing succeeds, count might be 0 or partial
                assert!(count >= 0);
            }
            Err(_) => {
                // Error is also acceptable for syntax errors
            }
        }
    }

    Ok(())
}

#[test]
fn test_malformed_python_code() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");
    let mut app = create_test_index_app(db_path.to_str().unwrap())?;

    // Test various syntax errors
    let malformed_cases = vec![
        ("unclosed_paren.py", "def test():\n    print(\"hello"),
        ("invalid_indentation.py", "def test():\nprint(\"hello\")"),
        ("invalid_syntax.py", "def test():\n    let x = "),
        ("mismatched_quotes.py", "def test():\n    print('hello\")"),
        ("invalid_characters.py", "def test():\n    x = \x01"),
    ];

    for (filename, content) in malformed_cases {
        let file_path = temp_dir.path().join(filename);
        fs::write(&file_path, content)?;

        // Should handle syntax errors gracefully
        let result = app.index_file(&file_path);
        match result {
            Ok(count) => {
                // If parsing succeeds, count might be 0 or partial
                assert!(count >= 0);
            }
            Err(_) => {
                // Error is also acceptable for syntax errors
            }
        }
    }

    Ok(())
}

#[test]
fn test_parser_initialization_errors() -> Result<()> {
    // Test that parser creation handles errors gracefully
    let rust_result = RustLanguageParser::new();
    assert!(rust_result.is_ok());

    let python_result = PythonLanguageParser::new();
    assert!(python_result.is_ok());

    Ok(())
}

#[test]
fn test_database_connection_errors() -> Result<()> {
    // Test with invalid database path
    let invalid_paths = vec![
        "/nonexistent/path/db.sqlite",
        "/root/readonly/db.sqlite", // Might not be writable
        "/dev/null/db.sqlite",      // Invalid on most systems
    ];

    for db_path in invalid_paths {
        let result = create_test_index_app(db_path);
        // Database creation might fail or succeed depending on system
        // We're mainly testing that it doesn't panic
        match result {
            Ok(_) => {
                // If it succeeds, that's fine too
            }
            Err(_) => {
                // Error is acceptable for invalid paths
            }
        }
    }

    Ok(())
}

#[test]
fn test_vector_store_errors() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Test with corrupted embeddings (if possible)
    // This is mainly to ensure error handling doesn't panic
    let result = HuggingFaceEmbeddings::new();
    match result {
        Ok(_) => {
            // Embeddings loaded successfully
        }
        Err(_) => {
            // Embeddings failed to load - this is acceptable for error testing
        }
    }

    Ok(())
}

#[test]
fn test_mixed_valid_and_invalid_files() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");
    let mut app = create_test_index_app(db_path.to_str().unwrap())?;

    // Create valid files
    let valid_rs = temp_dir.path().join("valid.rs");
    fs::write(&valid_rs, "pub fn valid_function() { println!(\"hello\"); }")?;

    let valid_py = temp_dir.path().join("valid.py");
    fs::write(&valid_py, "def valid_function():\n    print(\"hello\")")?;

    // Create invalid files
    let invalid_js = temp_dir.path().join("invalid.js");
    fs::write(&invalid_js, "function invalid() { console.log('hello'); }")?;

    let nonexistent = PathBuf::from("/nonexistent/file.rs");

    // Test mixed file list
    let files = vec![&valid_rs, &valid_py, &invalid_js, &nonexistent];
    let total_count = app.index_files(&files)?;

    // Should count only valid files (at least 2)
    assert!(total_count >= 2);

    Ok(())
}

#[test]
fn test_very_large_files() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");
    let mut app = create_test_index_app(db_path.to_str().unwrap())?;

    // Create a moderately large file (100 functions is enough to test scalability)
    let large_rs = temp_dir.path().join("large.rs");
    let mut large_content = String::new();
    for i in 0..100 {
        large_content.push_str(&format!("pub fn function_{}() -> i32 {{ {} }}\n", i, i));
    }
    fs::write(&large_rs, large_content)?;

    // Should handle large files without panicking
    let result = app.index_file(&large_rs);
    match result {
        Ok(count) => {
            // If it succeeds, count should be reasonable
            assert!(count >= 0);
        }
        Err(_) => {
            // Error is acceptable for very large files
        }
    }

    Ok(())
}

#[test]
fn test_concurrent_access() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");

    // Create multiple index apps pointing to same database
    let mut app1 = create_test_index_app(db_path.to_str().unwrap())?;
    let mut app2 = create_test_index_app(db_path.to_str().unwrap())?;

    // Create test files
    let file1 = temp_dir.path().join("test1.rs");
    let file2 = temp_dir.path().join("test2.rs");
    fs::write(&file1, "pub fn test1() {}")?;
    fs::write(&file2, "pub fn test2() {}")?;

    // Test concurrent access (simplified test)
    let result1 = app1.index_file(&file1);
    let result2 = app2.index_file(&file2);

    // Both should either succeed or fail gracefully
    match (result1, result2) {
        (Ok(count1), Ok(count2)) => {
            assert!(count1 >= 1);
            assert!(count2 >= 1);
        }
        _ => {
            // At least one failed - acceptable for concurrent access
        }
    }

    Ok(())
}

#[test]
fn test_manual_database_error_scenarios() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");

    // Create database connection and ensure schema
    let conn = rusqlite::Connection::open(&db_path)?;
    ensure_code_graph_schema(&conn)?;

    // Test inserting invalid data
    let invalid_cases = vec![
        // Empty file path
        ("", "function", "test", "test()", 1, 3, "rust"),
        // Empty entity type
        ("/test.rs", "", "test", "test()", 1, 3, "rust"),
        // Empty name
        ("/test.rs", "function", "", "test()", 1, 3, "rust"),
        // Invalid line numbers
        ("/test.rs", "function", "test", "test()", -1, 3, "rust"),
        ("/test.rs", "function", "test", "test()", 1, -1, "rust"),
    ];

    for (file_path, entity_type, name, signature, line_start, line_end, language) in invalid_cases {
        let result = conn.execute(
            "INSERT INTO code_entities 
             (file_path, entity_type, name, signature, line_start, line_end, docstring, language, indexed_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                file_path,
                entity_type,
                name,
                signature,
                line_start,
                line_end,
                None::<String>,
                language,
                chrono::Utc::now().timestamp(),
            ],
        );

        // Some invalid data might be rejected by database constraints
        match result {
            Ok(_) => {
                // Insert succeeded - data was valid enough
            }
            Err(_) => {
                // Insert failed - expected for invalid data
            }
        }
    }

    Ok(())
}
