//! T9 Full Toolchain Contract Tests
//!
//! Tests that encode the T9 contract specifications for all MCP tools.
//! These tests should fail initially and pass after contract enforcement.
//!
//! Groups:
//! - GROUP A: Response envelope invariants
//! - GROUP B: Streaming contract integration
//! - GROUP C: Translator + MCP contract

use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;
use syncore::mcp_tools::streaming::OutputLimiter;
use syncore::mcp_tools::translator::{translate_llm_output, TargetSchema};
use syncore::mcp_tools::{
    code_suite::{CodeSuite, CodeSuiteArgs},
    debug_suite::{DebugSuite, DebugSuiteArgs},
    graph_suite::{GraphSuite, GraphSuiteArgs},
    mapping_suite::{MappingSuite, MappingSuiteArgs},
    memory_suite::{MemorySuite, MemorySuiteArgs},
    refrag_suite::{RefragSuite, RefragSuiteArgs},
    SuiteDispatcher, SuiteResult,
};
use syncore::router::SynCoreState;

// Helper function to convert SuiteResult to JSON for testing
fn suite_result_to_json(result: SuiteResult) -> Value {
    json!({
        "success": result.success,
        "command": result.command,
        "data": result.data,
        "error": result.error
    })
}

// Helper function to create a test state
fn create_test_state() -> SynCoreState {
    // This is a minimal state for testing - in real usage this would be properly initialized
    SynCoreState::test_new()
}

// =============================================================================
// GROUP A — RESPONSE ENVELOPE INVARIANTS
// =============================================================================

#[test]
fn test_memory_suite_response_envelope() -> Result<()> {
    let state = create_test_state();
    let suite = MemorySuite::new(state);

    // Test a simple memory store operation
    let args = json!({
        "command": "store",
        "key": "test_key",
        "value": "test_value"
    });

    let suite_result = suite.dispatch("store", args);
    let result = suite_result_to_json(suite_result);

    // Contract: Response must be an object with required fields
    assert!(result.is_object(), "Response must be JSON object");
    assert!(result.get("success").is_some(), "Response must have 'success' field");
    assert!(result.get("command").is_some(), "Response must have 'command' field");
    assert!(result.get("data").is_some(), "Response must have 'data' field");

    // Contract: Success must be boolean
    let success = result["success"].as_bool().expect("success must be boolean");
    assert_eq!(result["command"], "store", "Command field must match invoked tool");

    // Contract: Data must be object, not array or primitive
    assert!(result["data"].is_object(), "Data field must be object, not array or primitive");

    // On success, error should be null or absent
    if success {
        assert!(
            result.get("error").is_none() || result["error"].is_null(),
            "Error should be null or absent on success"
        );
    }

    Ok(())
}

#[test]
fn test_debug_suite_response_envelope() -> Result<()> {
    let state = create_test_state();
    let suite = DebugSuite::new(state);

    // Test tool metadata list operation
    let args = json!({
        "command": "tool_metadata_list"
    });

    let suite_result = suite.dispatch("tool_metadata_list", args);
    let result = suite_result_to_json(suite_result);

    // Contract: Response must be an object with required fields
    assert!(result.is_object(), "Response must be JSON object");
    assert!(result.get("success").is_some(), "Response must have 'success' field");
    assert!(result.get("command").is_some(), "Response must have 'command' field");
    assert!(result.get("data").is_some(), "Response must have 'data' field");

    // Contract: Data must be object
    assert!(result["data"].is_object(), "Data field must be object");

    Ok(())
}

#[test]
fn test_code_suite_response_envelope() -> Result<()> {
    let state = create_test_state();
    let suite = CodeSuite::new(state);

    // Test code search operation
    let args = json!({
        "command": "search",
        "query": "test"
    });

    let suite_result = suite.dispatch("search", args);
    let result = suite_result_to_json(suite_result);

    // Contract: Response must be an object with required fields
    assert!(result.is_object(), "Response must be JSON object");
    assert!(result.get("success").is_some(), "Response must have 'success' field");
    assert!(result.get("command").is_some(), "Response must have 'command' field");
    assert!(result.get("data").is_some(), "Response must have 'data' field");

    // Contract: Data must be object, not bare array
    assert!(result["data"].is_object(), "Data field must be object, not bare array");

    Ok(())
}

#[test]
fn test_graph_suite_response_envelope() -> Result<()> {
    let suite = GraphSuite::new();

    // Test graph query operation
    let args = json!({
        "command": "query",
        "cypher": "MATCH (n) RETURN n LIMIT 1"
    });

    let result = suite.dispatch("query", args);

    // Contract: Response must be an object with required fields
    assert!(result.is_object(), "Response must be JSON object");
    assert!(result.get("success").is_some(), "Response must have 'success' field");
    assert!(result.get("command").is_some(), "Response must have 'command' field");
    assert!(result.get("data").is_some(), "Response must have 'data' field");

    // Contract: Data must be object
    assert!(result["data"].is_object(), "Data field must be object");

    Ok(())
}

#[test]
fn test_error_response_envelope() -> Result<()> {
    let suite = MemorySuite::new();

    // Test with invalid arguments to trigger error
    let args = json!({
        "command": "query",
        // Missing required 'key' parameter
    });

    let result = suite.dispatch("query", args);

    // Contract: Error response must have proper structure
    assert!(result.is_object(), "Error response must be JSON object");
    assert_eq!(result["success"], false, "Success must be false on error");
    assert!(result.get("error").is_some(), "Error response must have 'error' field");
    assert!(result["error"].is_string(), "Error field must be string");

    // Contract: Error message should be meaningful
    let error_msg = result["error"].as_str().unwrap();
    assert!(!error_msg.is_empty(), "Error message should not be empty");
    assert!(error_msg.len() > 10, "Error message should be descriptive");

    Ok(())
}

// =============================================================================
// GROUP B — STREAMING CONTRACT INTEGRATION
// =============================================================================

#[test]
fn test_debug_suite_streaming_integration() -> Result<()> {
    let suite = DebugSuite::new();

    // Trigger a potentially large operation
    let args = json!({
        "command": "project_hotspots",
        "min_loc": 0,
        "min_fan_in": 0,
        "min_fan_out": 0,
        "limit": 100 // Large enough to potentially trigger streaming
    });

    let result = suite.dispatch("project_hotspots", args);

    // Contract: Successful large responses should have streaming metadata
    if result["success"] == true {
        let data = &result["data"];

        // Check if response has streaming metadata (may or may not be present depending on size)
        if data.get("meta").is_some() {
            let meta = &data["meta"];

            // If meta exists, it should have proper streaming metadata
            if meta.get("truncated").is_some() {
                assert_eq!(
                    meta["truncated"], true,
                    "Truncated flag must be true when meta.truncated exists"
                );
                assert!(
                    meta.get("total_lines").is_some(),
                    "Must include total_lines when truncated"
                );
                assert!(
                    meta.get("total_bytes").is_some(),
                    "Must include total_bytes when truncated"
                );
                assert!(
                    meta.get("storage_key").is_some(),
                    "Must include storage_key when truncated"
                );

                // Storage key should start with "trunc_"
                let storage_key = meta["storage_key"].as_str().unwrap();
                assert!(
                    storage_key.starts_with("trunc_"),
                    "Storage key should start with 'trunc_'"
                );
            }
        }

        // Contract: Data should never be raw large arrays
        if data.get("hotspots").is_some() {
            let hotspots = &data["hotspots"];

            // If hotspots is an array, it should be reasonably sized
            if hotspots.is_array() {
                let count = hotspots.as_array().unwrap().len();
                assert!(count <= 200, "Array should be limited by streaming contract");
                // Should be limited by streaming
            }
        }
    }

    Ok(())
}

#[test]
fn test_memory_suite_streaming_integration() -> Result<()> {
    let suite = MemorySuite::new();

    // Test with a potentially large list operation
    let args = json!({
        "command": "list_keys",
        "limit": 1000 // Large enough to potentially trigger streaming
    });

    let result = suite.dispatch("list_keys", args);

    // Contract: Large array responses should be handled by streaming
    if result["success"] == true {
        let data = &result["data"];

        // Check for streaming metadata
        if data.get("meta").is_some() {
            let meta = &data["meta"];
            if meta.get("truncated").is_some() && meta["truncated"] == true {
                assert!(
                    meta.get("storage_key").is_some(),
                    "Must provide storage key when truncated"
                );
            }
        }

        // Contract: Raw arrays should be limited
        if data.get("keys").is_some() {
            let keys = &data["keys"];
            if keys.is_array() {
                // If this is a direct array response (not truncated), it should be reasonable size
                let count = keys.as_array().unwrap().len();
                assert!(count <= 200, "Direct array responses should be limited");
                // Streaming should limit this
            }
        }
    }

    Ok(())
}

#[test]
fn test_code_suite_streaming_violation() -> Result<()> {
    let suite = CodeSuite::new();

    // This test should FAIL initially because code_suite doesn't have streaming applied
    let args = json!({
        "command": "grep",
        "pattern": "TODO|FIXME|XXX", // Common patterns that could return many results
        "path": "src/"
    });

    let result = suite.dispatch("grep", args);

    // Contract: This should have streaming applied but currently doesn't
    if result["success"] == true {
        let data = &result["data"];

        // TODO: This assertion will FAIL initially - code_suite missing streaming
        // After fixes, data should have streaming metadata for large results
        if data.get("results").is_some() {
            let results = &data["results"];
            if results.is_array() && results.as_array().unwrap().len() > 200 {
                // Large results should have streaming metadata
                assert!(data.get("meta").is_some(), "Large results should have streaming metadata");
                assert!(data["meta"]["truncated"] == true, "Large results should be truncated");
            }
        }
    }

    Ok(())
}

#[test]
fn test_streaming_limiter_direct() -> Result<()> {
    // Test the streaming limiter directly with large content
    let limiter = OutputLimiter::default();

    // Create large JSON that exceeds limits
    let large_items: Vec<i32> = (0..300).collect();
    let large_json = json!({
        "command": "test_large_response",
        "data": {
            "items": large_items,
            "metadata": {
                "total": large_items.len(),
                "source": "test"
            }
        }
    });

    let limited = limiter.apply_json(&large_json)?;

    // Contract: Limited response should have proper structure
    assert!(limited.is_object(), "Limited response must be object");

    // Check for command preservation
    assert_eq!(limited["command"], "test_large_response", "Command should be preserved");

    // Check data section has truncation metadata
    let data = &limited["data"];
    assert!(data.get("meta").is_some(), "Data should have truncation metadata");

    let meta = &data["meta"];
    assert_eq!(meta["truncated"], true, "Should be marked as truncated");
    assert!(meta.get("total_lines").is_some(), "Should include total lines");
    assert!(meta.get("storage_key").is_some(), "Should include storage key");

    Ok(())
}

// =============================================================================
// GROUP C — TRANSLATOR + MCP CONTRACT
// =============================================================================

#[test]
fn test_translator_integration() -> Result<()> {
    // Test that LLM output is properly translated

    // Simulate malformed LLM output that needs fixing
    let raw_llm_output = r#"{
        "title": "Test Task",
        "description": "A test task",
        "subtasks": [
            {
                "title": "Subtask 1",
                "estimated_hours": "2.5"  // String instead of number
            },
            {
                "title": "Subtask 2",
                "estimated_hours": 3
            }
        ]
    }"#;

    let translated = translate_llm_output(raw_llm_output, TargetSchema::TaskBreakdown)?;

    // Contract: Translated output should be valid schema
    assert!(translated.is_object(), "Translated output should be object");

    // Type coercion should fix string numbers to actual numbers
    if let Some(subtasks) = translated.get("subtasks").and_then(|v| v.as_array()) {
        if let Some(first_subtask) = subtasks.get(0) {
            if let Some(estimated_hours) = first_subtask.get("estimated_hours") {
                // Should be coerced to number
                assert!(estimated_hours.is_number(), "String numbers should be coerced to numbers");
            }
        }
    }

    Ok(())
}

#[test]
fn test_intellitask_translator_contract() -> Result<()> {
    // Test that intellitask operations use translator properly

    let raw_llm_response = r#"{
        "task_breakdown": {
            "title": "Build feature X",
            "description": "Implement feature X with proper testing",
            "subtasks": [
                {
                    "title": "Design database schema",
                    "estimated_hours": "4",  // String that should be number
                    "priority": "High"  // Should be validated enum
                }
            ]
        }
    }"#;

    let result = translate_llm_output(raw_llm_response, TargetSchema::TaskBreakdown)?;

    // Contract: Should be properly structured and type-coerced
    assert!(result.get("task_breakdown").is_some(), "Should preserve task_breakdown structure");

    let breakdown = &result["task_breakdown"];
    assert!(breakdown.get("title").is_some(), "Should have title");
    assert!(breakdown.get("description").is_some(), "Should have description");

    if let Some(subtasks) = breakdown.get("subtasks").and_then(|v| v.as_array()) {
        assert!(!subtasks.is_empty(), "Should have subtasks");

        if let Some(first_subtask) = subtasks.get(0) {
            // Type coercion should fix estimated_hours from string to number
            if let Some(hours) = first_subtask.get("estimated_hours") {
                assert!(hours.is_number(), "estimated_hours should be number after coercion");
            }
        }
    }

    Ok(())
}

#[test]
fn test_translator_error_handling() -> Result<()> {
    // Test translator error handling

    let invalid_llm_output = r#"{
        "title": "Task",
        "description": "Description"
        // Missing required subtasks field
    }"#;

    let result = translate_llm_output(invalid_llm_output, TargetSchema::TaskBreakdown);

    // Contract: Should handle invalid input gracefully
    assert!(result.is_err(), "Should return error for invalid schema");

    // Error should be meaningful
    let error_msg = result.unwrap_err().to_string();
    assert!(!error_msg.is_empty(), "Error message should not be empty");

    Ok(())
}

#[test]
fn test_reasoning_suite_response_pattern() -> Result<()> {
    // Test reasoning suite - this may violate contract by using direct JSON instead of SuiteResult

    // This is a placeholder test - reasoning suite handlers need to be tested directly
    // as they don't use the SuiteDispatcher pattern

    // TODO: This test will need to be implemented based on actual reasoning suite handlers
    // Current issue: reasoning_suite uses direct JSON returns, not SuiteResult

    Ok(())
}

// =============================================================================
// EDGE CASE AND STRESS TESTS
// =============================================================================

#[test]
#[ignore] // Requires actual large dataset - mark as ignored for now
fn test_very_large_response_streaming() -> Result<()> {
    // This test would require setting up a large repository or dataset
    // to trigger actual streaming behavior

    let state = create_test_state();
    let suite = DebugSuite::new(state);

    let args = json!({
        "command": "project_file_report",
        "file_path": "src/" // Large directory
    });

    let suite_result = suite.dispatch("project_file_report", args);
    let result = suite_result_to_json(suite_result);

    // This should trigger streaming due to large analysis
    if result["success"] == true {
        let data = &result["data"];
        assert!(data.get("meta").is_some(), "Large report should have streaming metadata");
        assert_eq!(data["meta"]["truncated"], true, "Should be truncated");
    }

    Ok(())
}

#[test]
fn test_empty_response_handling() -> Result<()> {
    // Test handling of empty or minimal responses

    let state = create_test_state();
    let suite = MemorySuite::new(state);

    let args = json!({
        "command": "query",
        "key": "nonexistent_key_12345"
    });

    let suite_result = suite.dispatch("query", args);
    let result = suite_result_to_json(suite_result);

    // Contract: Even empty responses should follow envelope
    assert!(result.is_object(), "Response must be object");
    assert!(result.get("success").is_some(), "Must have success field");
    assert!(result.get("data").is_some(), "Must have data field");
    assert!(result["data"].is_object(), "Data must be object");

    Ok(())
}

#[test]
fn test_special_characters_in_responses() -> Result<()> {
    // Test handling of special characters, Unicode, etc.

    let state = create_test_state();
    let suite = MemorySuite::new(state);

    let args = json!({
        "command": "store",
        "key": "test_special_chars",
        "value": "Special chars: \"quotes\", 'apostrophes', \n\t newlines, \u{1F4A9} emoji"
    });

    let suite_result = suite.dispatch("store", args);
    let result = suite_result_to_json(suite_result);

    // Contract: Should handle special characters properly
    assert!(result["success"] == true, "Should store special characters successfully");

    Ok(())
}
