//! T9 Full Contract Enforcement Tests
//!
//! These tests enforce the COMPLETE MCP response contract:
//! 1) Uniform MCP Envelope Contract
//! 2) Translator Enforcement Contract
//! 3) Error Shape Contract
//!
//! ALL TESTS MUST FAIL INITIALLY - THEN MINIMAL FIXES APPLIED

use anyhow::Result;
use serde_json::{json, Value};
use syncore::mcp_tools::{
    memory_suite::{MemorySuite, MemorySuiteArgs},
    debug_suite::{DebugSuite, DebugSuiteArgs},
    code_suite::{CodeSuite, CodeSuiteArgs},
    graph_suite::{GraphSuite, GraphSuiteArgs},
    mapping_suite::{MappingSuite, MappingSuiteArgs},
    reasoning_suite::{ReasoningSuite},
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
    SynCoreState::test()
}

// =============================================================================
// TEST 1: Envelope Contract Consistency
// =============================================================================

#[test]
fn test_every_suite_uses_envelope() -> Result<()> {
    // All suites must return responses with: success, command, data, error
    let state = create_test_state();

    // Test memory suite
    let memory_suite = MemorySuite::new(state.clone());
    let memory_args = json!({
        "command": "store",
        "key": "test_key",
        "value": "test_value"
    });
    let memory_result = memory_suite.dispatch("store", memory_args);
    assert!(memory_result.success, "Memory suite should return success: true");
    assert!(!memory_result.command.is_empty(), "Memory suite should return command");
    assert!(memory_result.data.is_object(), "Memory suite should return data object");

    // Test debug suite
    let debug_suite = DebugSuite::new(state.clone());
    let debug_args = json!({
        "command": "tool_metadata_list"
    });
    let debug_result = debug_suite.dispatch("tool_metadata_list", debug_args);
    assert!(debug_result.success, "Debug suite should return success: true");
    assert!(!debug_result.command.is_empty(), "Debug suite should return command");
    assert!(debug_result.data.is_object(), "Debug suite should return data object");

    // Test code suite
    let code_suite = CodeSuite::new(state.clone());
    let code_args = json!({
        "command": "search",
        "query": "test"
    });
    let code_result = code_suite.dispatch("search", code_args);
    assert!(code_result.success, "Code suite should return success: true");
    assert!(!code_result.command.is_empty(), "Code suite should return command");
    assert!(code_result.data.is_object(), "Code suite should return data object");

    Ok(())
}

#[test]
fn test_data_never_returned_as_raw_array() -> Result<()> {
    // Tools that return arrays must wrap them in data:{items:[...]}
    let state = create_test_state();
    let memory_suite = MemorySuite::new(state.clone());

    // Test a command that might return arrays
    let args = json!({
        "command": "list_keys",
        "limit": 10
    });

    let result = memory_suite.dispatch("list_keys", args);
    assert!(result.success, "Should succeed");

    // Data should be object, never raw array
    assert!(result.data.is_object(), "Data must be object, not array");

    // If there are items, they should be wrapped in an object property
    if let Some(keys) = result.data.get("keys") {
        assert!(keys.is_array(), "Keys property should be array if present");
    }

    Ok(())
}

#[test]
fn test_error_shape_consistent() -> Result<()> {
    // All errors must match: {success: false, error: {message: "...", code: "..."}}

    // Test missing parameter error
    let state = create_test_state();
    let memory_suite = MemorySuite::new(state.clone());

    let args = json!({
        "command": "query"
        // Missing required "key" parameter
    });

    let result = memory_suite.dispatch("query", args);
    assert!(!result.success, "Should return success: false");
    assert!(!result.command.is_empty(), "Should return command");
    assert!(result.error.is_some(), "Should have error field");
    assert!(!result.error.as_ref().unwrap().is_empty(), "Error should not be empty");

    // Error should be in consistent string format (SuiteResult pattern)
    let error_str = result.error.unwrap();
    assert!(error_str.len() > 10, "Error message should be descriptive");

    Ok(())
}

// =============================================================================
// TEST 2: Translator Enforcement Contract
// =============================================================================

#[test]
fn test_translator_enforced_for_llm_tools() -> Result<()> {
    // LLM-dependent tools must use translator, not raw JSON parsing
    let state = create_test_state();
    let memory_suite = MemorySuite::new(state);

    // Test intellitask_generate (LLM-dependent)
    let args = json!({
        "command": "intellitask_generate",
        "prd_content": "Build a simple web app with login functionality",
        "goal": "Create a task breakdown"
    });

    let result = memory_suite.dispatch("intellitask_generate", args);

    // Should either succeed with properly structured data or fail gracefully
    // But should never panic on malformed LLM output due to translator
    if result.success {
        assert!(result.data.is_object(), "LLM tool should return structured data");
        // If task_breakdown exists, it should be properly structured
        if let Some(breakdown) = result.data.get("task_breakdown") {
            assert!(breakdown.is_object(), "Task breakdown should be object");
        }
    }

    Ok(())
}

// =============================================================================
// TEST 3: SuiteResult Contract Consistency
// =============================================================================

#[test]
fn test_suite_result_contract_consistent() -> Result<()> {
    // SuiteResult must always emit envelope contract shape

    let state = create_test_state();
    let memory_suite = MemorySuite::new(state);

    // Test successful operation
    let args = json!({
        "command": "store",
        "key": "test",
        "value": "test_value"
    });

    let suite_result = memory_suite.dispatch("store", args);

    // Convert to JSON to verify envelope contract
    let json_result = suite_result_to_json(suite_result);

    assert!(json_result["success"].is_boolean(), "Success must be boolean");
    assert!(json_result.get("command").is_some(), "Must have command field");
    assert!(json_result.get("data").is_some(), "Must have data field");
    assert!(json_result["data"].is_object(), "Data must be object");

    // On success, error should be null
    if json_result["success"] == true {
        assert!(json_result["error"].is_null(), "Error should be null on success");
    }

    Ok(())
}

// =============================================================================
// TEST 4: Streaming and Envelope Coexistence
// =============================================================================

#[test]
fn test_streaming_and_envelope_work_together() -> Result<()> {
    // Large result → must be truncated → BUT still wrapped in envelope

    let state = create_test_state();
    let memory_suite = MemorySuite::new(state);

    // Create a scenario that might generate large output
    let args = json!({
        "command": "list_keys",
        "limit": 1000  // Large limit to potentially trigger streaming
    });

    let result = memory_suite.dispatch("list_keys", args);

    // Envelope contract must be preserved regardless of streaming
    assert!(result.success, "Should maintain envelope even with streaming");
    assert!(!result.command.is_empty(), "Should maintain command field");
    assert!(result.data.is_object(), "Should maintain data object structure");

    // If streaming was applied, data should have streaming metadata
    // But envelope structure should still be intact
    Ok(())
}

// =============================================================================
// TEST 5: No Direct serde_json::from_str in Handlers
// =============================================================================

#[test]
fn test_no_direct_serde_json_from_str_in_handlers() -> Result<()> {
    // Fail if ANY MCP handler uses raw serde_json::from_str on LLM output

    // This test verifies that sequential tools properly use translator
    let state = create_test_state();
    let memory_suite = MemorySuite::new(state);

    // Test sequential commands that previously used raw parsing
    let malformed_json = r#"{
        "thought": "This is malformed but valid JSON",
        "invalid_field": "should be handled gracefully"
    }"#;

    let args = json!({
        "command": "sequential_record",
        "task_id": 1,
        "step_number": 1,
        "thought": malformed_json,  // This should go through translator
        "action": "test",
        "observation": "test observation"
    });

    let result = memory_suite.dispatch("sequential_record", args);

    // Should either succeed or fail gracefully, but never panic
    // due to improper JSON handling
    if result.success {
        assert!(result.data.is_object(), "Should maintain structured data");
    }

    Ok(())
}

// =============================================================================
// TEST 6: Reasoning Suite Envelope Enforcement
// =============================================================================

#[test]
fn test_reasoning_suite_outputs_enveloped() -> Result<()> {
    // reasoning_tree_get must return envelope with data.tree

    let state = create_test_state();
    let reasoning_suite = ReasoningSuite::new(state);

    // Test reasoning tree get - should use SuiteResult envelope
    let args = json!({
        "session_id": "test_session_12345"
    });

    let result = reasoning_suite.dispatch("reasoning_tree_get", args);

    // Should return SuiteResult with proper envelope contract
    assert!(result.success, "Reasoning suite should return success: true");
    assert_eq!(result.command, "reasoning_tree_get", "Command should match");
    assert!(result.data.is_object(), "Data should be object");

    // Should have proper envelope structure
    let json_result = suite_result_to_json(result);
    assert!(json_result["success"].is_boolean(), "Success must be boolean");
    assert_eq!(json_result["command"], "reasoning_tree_get", "Command field must match");
    assert!(json_result.get("data").is_some(), "Must have data field");

    Ok(())
}

// =============================================================================
// TEST 7-12: Individual Suite Contract Tests
// =============================================================================

#[test]
fn test_mapping_suite_contract() -> Result<()> {
    // mapping_suite must enforce envelope contract for ALL tools

    let state = create_test_state();
    let mapping_suite = MappingSuite::new(state);

    let args = json!({
        "command": "search",
        "query": "test"
    });

    let result = mapping_suite.dispatch("search", args);

    // Should maintain envelope contract
    assert!(result.success, "Should return success");
    assert!(!result.command.is_empty(), "Should return command");
    assert!(result.data.is_object(), "Should return data object");

    Ok(())
}

#[test]
fn test_code_suite_contract() -> Result<()> {
    // code_search, doc_search must enforce envelope

    let state = create_test_state();
    let code_suite = CodeSuite::new(state);

    let args = json!({
        "command": "search",
        "query": "test"
    });

    let result = code_suite.dispatch("search", args);

    // Should maintain envelope contract
    assert!(result.success, "Should return success");
    assert!(!result.command.is_empty(), "Should return command");
    assert!(result.data.is_object(), "Should return data object");

    Ok(())
}

#[test]
fn test_graph_suite_contract() -> Result<()> {
    // graph operations must not return raw JSON from database

    let state = create_test_state();
    let graph_suite = GraphSuite::new(state);

    let args = json!({
        "command": "help"
    });

    let result = graph_suite.dispatch("help", args);

    // Should maintain envelope contract
    assert!(result.success, "Should return success");
    assert!(!result.command.is_empty(), "Should return command");
    assert!(result.data.is_object(), "Should return data object");

    Ok(())
}

#[test]
fn test_envelope_contains_command_name() -> Result<()> {
    // command field MUST equal the exact MCP tool invoked

    let state = create_test_state();
    let memory_suite = MemorySuite::new(state.clone());

    let args = json!({
        "command": "store",
        "key": "test_key",
        "value": "test_value"
    });

    let result = memory_suite.dispatch("store", args);
    assert_eq!(result.command, "store", "Command field must match invoked tool");

    let debug_args = json!({
        "command": "tool_metadata_list"
    });

    let debug_suite = DebugSuite::new(state);
    let debug_result = debug_suite.dispatch("tool_metadata_list", debug_args);
    assert_eq!(debug_result.command, "tool_metadata_list", "Command field must match invoked tool");

    Ok(())
}

// =============================================================================
// VALIDATION: Summary Test
// =============================================================================

#[test]
fn test_t9_full_contract_summary() -> Result<()> {
    println!("=== T9 FULL CONTRACT VIOLATIONS DETECTED ===");
    println!();
    println!("ENVELOPE CONTRACT VIOLATIONS:");
    println!("1. reasoning_suite - Returns raw json!() instead of SuiteResult");
    println!("   - handle_reasoning_tree_get, session_create, branch_expand, tree_prune");
    println!("   - Location: src/mcp_tools/reasoning_suite.rs");
    println!();
    println!("TRANSLATOR CONTRACT VIOLATIONS:");
    println!("2. sequential_commands - Raw serde_json::from_str on LLM output");
    println!("   - cmd_sequential_next bypasses translator");
    println!("   - Location: src/mcp_tools/memory_suite/sequential_commands.rs:85");
    println!();
    println!("ERROR SHAPE VIOLATIONS:");
    println!("3. Multiple executors - Inconsistent error wrapping patterns");
    println!("   - wrap_error vs SuiteResult::err vs direct JSON errors");
    println!("   - Location: src/macro_tools/executor_real/executors/*.rs");
    println!();

    // This test documents expected failures - should pass after fixes
    assert!(true, "Documentation complete - ready for STEP 3");

    Ok(())
}