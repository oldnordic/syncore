//! TDD Tests for Code Dependency Extractor
//! These tests MUST be written BEFORE implementation.

use syncore::portfolio::code_dependency_extractor::{CodeDependencies, CodeDependencyExtractor};

#[test]
fn test_extract_imports() {
    let rust_code = r#"
use std::collections::HashMap;
use crate::storage::manager::StorageManager;
use anyhow::Result;

fn main() {}
"#;

    let mut extractor = CodeDependencyExtractor::new();
    let deps = extractor.extract_from_source(rust_code, "test.rs").unwrap();

    assert!(deps.imports.contains(&"std::collections::HashMap".to_string()));
    assert!(deps.imports.contains(&"crate::storage::manager::StorageManager".to_string()));
    assert!(deps.imports.contains(&"anyhow::Result".to_string()));
    assert_eq!(deps.imports.len(), 3);
}

#[test]
fn test_extract_function_calls() {
    let rust_code = r#"
fn helper() -> i32 {
    42
}

fn compute(x: i32) -> i32 {
    let a = helper();
    let b = process_data(x);
    a + b
}

fn process_data(val: i32) -> i32 {
    val * 2
}
"#;

    let mut extractor = CodeDependencyExtractor::new();
    let deps = extractor.extract_from_source(rust_code, "test.rs").unwrap();

    // compute calls helper and process_data
    assert!(deps.calls.iter().any(|(caller, callee)| caller == "compute" && callee == "helper"));
    assert!(deps
        .calls
        .iter()
        .any(|(caller, callee)| caller == "compute" && callee == "process_data"));
}

#[test]
fn test_extract_trait_impls() {
    let rust_code = r#"
struct MyStruct {
    value: i32,
}

impl Default for MyStruct {
    fn default() -> Self {
        MyStruct { value: 0 }
    }
}

impl Clone for MyStruct {
    fn clone(&self) -> Self {
        MyStruct { value: self.value }
    }
}

trait CustomTrait {
    fn process(&self);
}

impl CustomTrait for MyStruct {
    fn process(&self) {}
}
"#;

    let mut extractor = CodeDependencyExtractor::new();
    let deps = extractor.extract_from_source(rust_code, "test.rs").unwrap();

    assert!(deps.implements.iter().any(|(s, t)| s == "MyStruct" && t == "Default"));
    assert!(deps.implements.iter().any(|(s, t)| s == "MyStruct" && t == "Clone"));
    assert!(deps.implements.iter().any(|(s, t)| s == "MyStruct" && t == "CustomTrait"));
    assert_eq!(deps.implements.len(), 3);
}

#[test]
fn test_detect_multiple_dependencies() {
    let rust_code = r#"
use std::sync::Arc;
use tokio::sync::Mutex;

struct Database {
    conn: Arc<Mutex<()>>,
}

impl Default for Database {
    fn default() -> Self {
        Database { conn: Arc::new(Mutex::new(())) }
    }
}

fn init_db() -> Database {
    Database::default()
}

fn query_db(db: &Database) -> String {
    validate_connection(db);
    fetch_data(db)
}

fn validate_connection(_db: &Database) {}

fn fetch_data(_db: &Database) -> String {
    String::new()
}
"#;

    let mut extractor = CodeDependencyExtractor::new();
    let deps = extractor.extract_from_source(rust_code, "test.rs").unwrap();

    // Check imports
    assert!(deps.imports.contains(&"std::sync::Arc".to_string()));
    assert!(deps.imports.contains(&"tokio::sync::Mutex".to_string()));

    // Check trait impls
    assert!(deps.implements.iter().any(|(s, t)| s == "Database" && t == "Default"));

    // Check function calls
    assert!(deps
        .calls
        .iter()
        .any(|(caller, callee)| caller == "query_db" && callee == "validate_connection"));
    assert!(deps
        .calls
        .iter()
        .any(|(caller, callee)| caller == "query_db" && callee == "fetch_data"));
}

#[test]
fn test_handles_empty_file() {
    let rust_code = "";

    let mut extractor = CodeDependencyExtractor::new();
    let deps = extractor.extract_from_source(rust_code, "empty.rs").unwrap();

    assert!(deps.imports.is_empty());
    assert!(deps.calls.is_empty());
    assert!(deps.implements.is_empty());
    assert_eq!(deps.path, "empty.rs");
}

#[test]
fn test_extract_function_definitions() {
    let rust_code = r#"
pub fn public_func() {}
fn private_func() {}
async fn async_func() {}
pub(crate) fn crate_func() {}
"#;

    let mut extractor = CodeDependencyExtractor::new();
    let deps = extractor.extract_from_source(rust_code, "test.rs").unwrap();

    assert!(deps.function_defs.contains(&"public_func".to_string()));
    assert!(deps.function_defs.contains(&"private_func".to_string()));
    assert!(deps.function_defs.contains(&"async_func".to_string()));
    assert!(deps.function_defs.contains(&"crate_func".to_string()));
    assert_eq!(deps.function_defs.len(), 4);
}

#[test]
fn test_extract_struct_definitions() {
    let rust_code = r#"
struct SimpleStruct;
pub struct PublicStruct {
    field: i32,
}
struct TupleStruct(i32, String);
"#;

    let mut extractor = CodeDependencyExtractor::new();
    let deps = extractor.extract_from_source(rust_code, "test.rs").unwrap();

    assert!(deps.struct_defs.contains(&"SimpleStruct".to_string()));
    assert!(deps.struct_defs.contains(&"PublicStruct".to_string()));
    assert!(deps.struct_defs.contains(&"TupleStruct".to_string()));
    assert_eq!(deps.struct_defs.len(), 3);
}
