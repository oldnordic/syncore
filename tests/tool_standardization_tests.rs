//! Tool Standardization Tests
//!
//! Tests for Phase 5 Tool Consolidation features:
//! 1. Unified error types (SynCoreError)
//! 2. Standardized request/response schemas
//! 3. Tool metadata layer
//! 4. dry_run support
//! 5. RealExecutor async path

use serde_json::json;
use syncore::macro_tools::executor_real::RealExecutor;
use syncore::mcp::tool_metadata::{self, ToolCategory, ToolCost};
use syncore::mcp::types::{SynCoreError, SynCoreResult, ToolRequest};

#[test]
fn test_tool_metadata_registry() {
    // Verify metadata registry is populated
    let all_metadata = tool_metadata::list_all_metadata();
    assert!(!all_metadata.is_empty(), "Tool registry should not be empty");

    // Check specific tools exist
    let memory_store = tool_metadata::get_tool_metadata("memory_store");
    assert!(memory_store.is_some(), "memory_store metadata should exist");

    let metadata = memory_store.unwrap();
    assert_eq!(metadata.name, "memory_store");
    assert_eq!(metadata.category, ToolCategory::Memory);
    assert_eq!(metadata.cost, ToolCost::Low);
    assert!(metadata.side_effects.modifies_database);
}

#[test]
fn test_tool_metadata_by_category() {
    // Test filtering by category
    let memory_tools = tool_metadata::list_by_category(ToolCategory::Memory);
    assert!(!memory_tools.is_empty(), "Should have memory tools");

    let vector_tools = tool_metadata::list_by_category(ToolCategory::Vector);
    assert!(!vector_tools.is_empty(), "Should have vector tools");

    // Verify categories are correct
    for tool in memory_tools {
        assert_eq!(tool.category, ToolCategory::Memory);
    }
}

#[test]
fn test_syncore_error_variants() {
    // Test different error variants
    let invalid_input = SynCoreError::invalid_input("Bad input");
    assert!(matches!(invalid_input, SynCoreError::InvalidInput { .. }));

    let not_found = SynCoreError::not_found("Task", "123");
    assert!(matches!(not_found, SynCoreError::NotFound { .. }));

    let internal = SynCoreError::internal("Something went wrong");
    assert!(matches!(internal, SynCoreError::Internal { .. }));
}

#[test]
fn test_syncore_error_to_json() {
    let error = SynCoreError::invalid_field("email", "Invalid email format");
    let json_value = error.to_json();

    assert!(json_value.is_object());
    assert_eq!(json_value["type"], "InvalidInput");
}

#[test]
fn test_syncore_result_type() {
    // Test SynCoreResult success case
    let success: SynCoreResult<i32> = Ok(42);
    assert!(success.is_ok());
    assert_eq!(success.unwrap(), 42);

    // Test SynCoreResult error case
    let failure: SynCoreResult<i32> = Err(SynCoreError::internal("Test error"));
    assert!(failure.is_err());
}

#[test]
fn test_real_executor_async() {
    // Test async executor with memory_store
    use std::sync::{Arc, Mutex};
    use syncore::memory::Memory;
    use syncore::router::SynCoreState;
    use syncore::tasks::Tasks;
    use syncore::vector::{RealEmbeddings, VectorStore};

    // Create executor with unique paths to avoid Sled lock conflicts
    let memory = Memory::new(":memory:_test_async").expect("Failed to create memory");
    let tasks = Tasks::new(":memory:").expect("Failed to create tasks");
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState::new(memory, tasks, vector_store);
    let executor = RealExecutor::new(Arc::new(state));

    let params = json!({
        "key": "test_key",
        "value": "test_value"
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("memory_store", &params).await });

    // RealExecutor returns Ok(Value) with envelope
    assert!(result.is_ok(), "Async execution should succeed");
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_eq!(envelope["ok"], true, "Envelope should have ok=true");
    assert!(envelope.get("data").is_some(), "Envelope should have data field");

    // Check data contents
    let data = &envelope["data"];
    assert_eq!(data["stored"], true);
}

#[test]
fn test_real_executor_async_fallback() {
    // Test async executor fallback to synthetic results
    use std::sync::{Arc, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};
    use syncore::memory::Memory;
    use syncore::router::SynCoreState;
    use syncore::tasks::Tasks;
    use syncore::vector::{RealEmbeddings, VectorStore};

    // Create executor with unique database path
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let db_path = format!(":memory:_test_fallback_{}", timestamp);

    let memory = Memory::new(&db_path).expect("Failed to create memory");
    let tasks = Tasks::new(":memory:").expect("Failed to create tasks");
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState::new(memory, tasks, vector_store);
    let executor = RealExecutor::new(Arc::new(state));

    let params = json!({
        "query": "test query",
        "limit": 5
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("vector_search", &params).await });

    // RealExecutor returns Ok(Value) with envelope
    assert!(result.is_ok(), "Async fallback should succeed");
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_eq!(envelope["ok"], true, "Envelope should have ok=true");
    assert!(envelope.get("data").is_some(), "Envelope should have data field");

    // Check data contents
    let data = &envelope["data"];
    assert!(data["results"].is_array());
}

#[test]
fn test_tool_metadata_side_effects() {
    // Test side effects tracking
    let memory_store = tool_metadata::get_tool_metadata("memory_store").unwrap();
    assert!(memory_store.side_effects.modifies_database);
    assert!(!memory_store.side_effects.modifies_filesystem);
    assert!(!memory_store.side_effects.network_call);

    let vector_insert = tool_metadata::get_tool_metadata("vector_insert").unwrap();
    assert!(vector_insert.side_effects.modifies_vector_store);
    assert!(!vector_insert.side_effects.modifies_database);
}

#[test]
fn test_tool_metadata_cost_levels() {
    // Test cost estimation
    let memory_query = tool_metadata::get_tool_metadata("memory_query").unwrap();
    assert_eq!(memory_query.cost, ToolCost::Low);

    let vector_insert = tool_metadata::get_tool_metadata("vector_insert").unwrap();
    assert_eq!(vector_insert.cost, ToolCost::Medium);

    let document_index = tool_metadata::get_tool_metadata("document_index").unwrap();
    assert_eq!(document_index.cost, ToolCost::VeryHigh);
}

#[test]
fn test_real_executor_async_error_handling() {
    // Test async executor with missing parameters
    use std::sync::{Arc, Mutex};
    use syncore::memory::Memory;
    use syncore::router::SynCoreState;
    use syncore::tasks::Tasks;
    use syncore::vector::{RealEmbeddings, VectorStore};

    // Create executor with unique paths to avoid Sled lock conflicts
    let memory = Memory::new(":memory:_test_error").expect("Failed to create memory");
    let tasks = Tasks::new(":memory:").expect("Failed to create tasks");
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState::new(memory, tasks, vector_store);
    let executor = RealExecutor::new(Arc::new(state));

    let params = json!({
        "key": "test_key"
        // Missing 'value' parameter
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("memory_store", &params).await });

    // RealExecutor returns Ok(Value) with error envelope, NOT Err
    assert!(result.is_ok(), "RealExecutor should return Ok(Value) even for errors");
    let envelope = result.unwrap();

    // Validate error envelope structure
    assert_eq!(envelope["ok"], false, "Envelope should have ok=false for errors");
    assert!(envelope.get("error").is_some(), "Error envelope should have error field");
    assert!(envelope.get("data").is_none(), "Error envelope should not have data field");

    // Validate error fields
    let error = &envelope["error"];
    assert!(error.get("type").is_some(), "Error should have type field");
    assert!(error.get("message").is_some(), "Error should have message field");
}
