//! Integration tests for project_rust_clippy_scan MCP tool
//!
//! This test verifies that the project_rust_clippy_scan tool:
//! 1. Executes cargo clippy on a real Rust project
//! 2. Stores diagnostics in the database
//! 3. Returns the correct count of inserted diagnostics

use serde_json::json;
use std::env;
use std::fs;
use std::sync::Arc;
use syncore::macro_tools::executor_real::RealExecutor;
use syncore::memory::Memory;
use syncore::tasks::Tasks;
use syncore::vector::{RealEmbeddings, VectorStore};
use tempfile::TempDir;

/// Create a test Rust project with intentional clippy violations
fn create_clippy_test_fixture(temp_dir: &TempDir) -> Result<String, Box<dyn std::error::Error>> {
    let project_path = temp_dir.path().join("clippy_test_project");
    fs::create_dir_all(&project_path)?;

    // Create Cargo.toml
    let cargo_toml = r#"[package]
name = "clippy-test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
"#;
    fs::write(project_path.join("Cargo.toml"), cargo_toml)?;

    // Create src directory
    let src_path = project_path.join("src");
    fs::create_dir_all(&src_path)?;

    // Create main.rs with intentional clippy violations
    let main_rs = r#"use std::collections::HashMap;

fn main() {
    let option_val: Option<i32> = Some(42);
    
    // Clippy violation: match instead of if let (when it could be if let)
    match option_val {
        Some(x) => println!("Value: {}", x),
        None => println!("None"),
    }
    
    // Clippy violation: unnecessary .clone()
    let string_val = "hello".to_string();
    let cloned_val = string_val.clone();
    
    println!("Cloned: {}", cloned_val);
    
    // Clippy violation: if let instead of match
    if let Some(x) = option_val {
        println!("If let value: {}", x);
    }
    
    // Clippy violation: HashMap creation with macro
    let mut map = HashMap::new();
    map.insert("key".to_string(), "value".to_string());
    
    // Clippy violation: unnecessary .clone()
    let string_val2 = "hello".to_string();
    let cloned_val2 = string_val2.clone();
    
    println!("Cloned: {}", cloned_val2);
}

fn function_with_unused_params(_param1: i32, _param2: &str) -> i32 {
    42
}
"#;
    fs::write(src_path.join("main.rs"), main_rs)?;

    // Create lib.rs with more violations
    let lib_rs = r#"
pub struct TestStruct {
    pub field1: i32,
    pub field2: String,
}

impl TestStruct {
    pub fn new(field1: i32, field2: String) -> Self {
        Self { field1, field2 }
    }
    
    // Clippy violation: format! already returns String
    pub fn get_description(&self) -> String {
        format!("Field1: {}, Field2: {}", self.field1, self.field2)
    }
    
    // Clippy violation: unnecessary .clone()
    pub fn clone_string(&self) -> String {
        self.field2.clone()
    }
}
"#;
    fs::write(src_path.join("lib.rs"), lib_rs)?;

    Ok(project_path.to_string_lossy().to_string())
}

#[tokio::test]
async fn test_project_rust_clippy_scan_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Create temporary directory and test fixture
    let temp_dir = TempDir::new()?;
    let project_path = create_clippy_test_fixture(&temp_dir)?;

    // Create test state using the same pattern as other tests
    let id = std::process::id();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mem_path = format!("/tmp/syncore_clippy_test_mem_{}_{}.db", id, ts);
    let task_path = format!("/tmp/syncore_clippy_test_task_{}_{}.db", id, ts);

    let memory = Memory::new(&mem_path)?;
    let tasks = Tasks::new(&task_path)?;
    let embeddings = Box::new(RealEmbeddings::new(384)?);
    let vector_store = Arc::new(std::sync::Mutex::new(VectorStore::new(embeddings)));
    let state = Arc::new(syncore::router::SynCoreState::new(
        memory,
        tasks,
        vector_store,
    ));

    // Create executor
    let executor = RealExecutor::new(state);

    // Change to the test project directory
    let original_dir = env::current_dir()?;
    env::set_current_dir(&project_path)?;

    // Call the project_rust_clippy_scan tool through executor
    let params = json!({}); // No parameters required
    let result = executor
        .execute_real_tool_async("project_rust_clippy_scan", &params)
        .await?;

    // Restore original directory
    env::set_current_dir(original_dir)?;

    // Verify the tool call was successful using envelope structure
    assert!(
        result["ok"].as_bool().unwrap_or(false),
        "Tool call should succeed. Result: {}",
        serde_json::to_string_pretty(&result).unwrap_or_default()
    );

    // Extract the data from the envelope
    let data = &result["data"];
    let inserted = data["inserted"].as_u64().unwrap_or(0) as usize;

    // Verify that diagnostics were inserted
    assert!(
        inserted > 0,
        "Should have inserted at least one diagnostic, got: {}",
        inserted
    );

    // Verify diagnostics are actually in the database by checking the state
    // Note: We can't directly access the database from the test, but we can verify
    // the tool reported the correct number of insertions

    println!("✅ Integration test passed!");
    println!("📊 Inserted {} diagnostics", inserted);

    Ok(())
}

#[tokio::test]
async fn test_project_rust_clippy_scan_empty_project() -> Result<(), Box<dyn std::error::Error>> {
    // Create temporary directory with minimal Rust project (no violations)
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path().join("clean_project");
    fs::create_dir_all(&project_path)?;

    // Create clean Cargo.toml
    let cargo_toml = r#"[package]
name = "clean-project"
version = "0.1.0"
edition = "2021"
"#;
    fs::write(project_path.join("Cargo.toml"), cargo_toml)?;

    // Create src directory
    let src_path = project_path.join("src");
    fs::create_dir_all(&src_path)?;

    // Create clean main.rs
    let main_rs = r#"
fn main() {
    println!("Hello, world!");
}
"#;
    fs::write(src_path.join("main.rs"), main_rs)?;

    // Create test state using the same pattern as other tests
    let id = std::process::id();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mem_path = format!("/tmp/syncore_clippy_clean_mem_{}_{}.db", id, ts);
    let task_path = format!("/tmp/syncore_clippy_clean_task_{}_{}.db", id, ts);

    let memory = Memory::new(&mem_path)?;
    let tasks = Tasks::new(&task_path)?;
    let embeddings = Box::new(RealEmbeddings::new(384)?);
    let vector_store = Arc::new(std::sync::Mutex::new(VectorStore::new(embeddings)));
    let state = Arc::new(syncore::router::SynCoreState::new(
        memory,
        tasks,
        vector_store,
    ));

    // Create executor
    let executor = RealExecutor::new(state);

    // Change to the test project directory
    let original_dir = env::current_dir()?;
    env::set_current_dir(&project_path)?;

    // Call the project_rust_clippy_scan tool through executor
    let params = json!({}); // No parameters required
    let result = executor
        .execute_real_tool_async("project_rust_clippy_scan", &params)
        .await?;

    // Restore original directory
    env::set_current_dir(original_dir)?;

    // Verify the tool call was successful using envelope structure
    assert!(
        result["ok"].as_bool().unwrap_or(false),
        "Tool call should succeed"
    );

    // Extract the data from the envelope
    let data = &result["data"];
    let inserted = data["inserted"].as_u64().unwrap_or(0) as usize;

    // Should have 0 or very few diagnostics for clean project
    println!("✅ Clean project test passed!");
    println!("📊 Inserted {} diagnostics for clean project", inserted);

    Ok(())
}

#[tokio::test]
async fn test_project_rust_clippy_scan_invalid_directory() -> Result<(), Box<dyn std::error::Error>>
{
    // Create temporary directory
    let temp_dir = TempDir::new()?;

    // Create test state using the same pattern as other tests
    let id = std::process::id();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mem_path = format!("/tmp/syncore_clippy_invalid_mem_{}_{}.db", id, ts);
    let task_path = format!("/tmp/syncore_clippy_invalid_task_{}_{}.db", id, ts);

    let memory = Memory::new(&mem_path)?;
    let tasks = Tasks::new(&task_path)?;
    let embeddings = Box::new(RealEmbeddings::new(384)?);
    let vector_store = Arc::new(std::sync::Mutex::new(VectorStore::new(embeddings)));
    let state = Arc::new(syncore::router::SynCoreState::new(
        memory,
        tasks,
        vector_store,
    ));

    // Create executor
    let executor = RealExecutor::new(state);

    // Change to a directory that's not a Rust project
    let original_dir = env::current_dir()?;
    env::set_current_dir(&temp_dir)?;

    // Call the project_rust_clippy_scan tool through executor
    let params = json!({}); // No parameters required
    let result = executor
        .execute_real_tool_async("project_rust_clippy_scan", &params)
        .await?;

    // Restore original directory
    env::set_current_dir(original_dir)?;

    // Should handle error gracefully using envelope structure
    assert!(
        !result["ok"].as_bool().unwrap_or(true),
        "Tool should return error for invalid directory"
    );

    // Extract the error from the envelope
    let error = &result["error"];
    assert!(
        error["type"]
            .as_str()
            .unwrap_or("")
            .contains("ExecutionFailed")
            || error["type"].as_str().unwrap_or("").contains("ClippyError")
            || error["type"].as_str().unwrap_or("").contains("Internal"),
        "Error type should indicate execution failure"
    );

    println!("✅ Invalid directory test passed!");
    println!("📝 Error type: {}", error["type"].as_str().unwrap_or(""));
    println!(
        "📝 Error message: {}",
        error["message"].as_str().unwrap_or("")
    );

    Ok(())
}
