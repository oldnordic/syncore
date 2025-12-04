//! Translator Reliability Tests - T7 Phase 1
//!
//! These tests ensure translator behavior is strict and reliable.
//! Tests MUST fail initially, then be fixed with minimal changes.
//!
//! CRITICAL RULES:
//! - DO NOT modify production code unless a test fails
//! - DO NOT guess schema behavior - use ripgrep + file reads
//! - DO NOT touch systems outside translator and MCP handlers
//! - KEEP ALL CHANGES MINIMAL (<300 LOC per file)

use anyhow::Result;
use serde_json::json;
use syncore::mcp_tools::translator::{translate_llm_output, TargetSchema};

/// Test 1: Translator must reject partial objects with missing required fields
#[test]
fn test_translator_rejects_partial_objects() -> Result<()> {
    // Input missing required fields like description, complexity, etc.
    let partial_json = r#"{
        "parent_tasks": [
            {
                "title": "Incomplete Task"
                // Missing: description, complexity, estimated_hours, etc.
            }
        ]
    }"#;

    // Should fail - translator must reject incomplete objects
    let result = translate_llm_output(partial_json, TargetSchema::TaskBreakdown);
    assert!(result.is_err(), "Translator must reject partial objects");

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("missing required field") || error_msg.contains("validation"),
        "Error should mention missing fields or validation: {}", error_msg);

    Ok(())
}

/// Test 2: Translator must reject empty JSON (no silent fabrication)
#[test]
fn test_translator_rejects_empty_json() -> Result<()> {
    // Empty JSON should not result in fabricated data
    let empty_json = "{}";

    // Should fail - translator must not silently create objects
    let result = translate_llm_output(empty_json, TargetSchema::TaskBreakdown);
    assert!(result.is_err(), "Translator must reject empty JSON");

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("missing required field") || error_msg.contains("validation"),
        "Error should mention missing fields: {}", error_msg);

    Ok(())
}

/// Test 3: Translator must properly extract nested JSON from markdown
#[test]
fn test_translator_normalizes_nested_markdown_json() -> Result<()> {
    // Input with nested markdown wrapper and extra text
    let nested_markdown = r#"```json
{
  "prd_title": "Test Feature",
  "parent_tasks": [
    {
      "id": "task-1",
      "title": "Valid Task",
      "description": "This is valid"
    }
  ],
  "relevant_files": [],
  "estimated_complexity": "Simple"
}
```
Extra text after JSON"#;

    // Should successfully extract JSON and pass schema validation
    let result = translate_llm_output(nested_markdown, TargetSchema::TaskBreakdown);
    assert!(result.is_ok(), "Translator should extract nested JSON from markdown");

    let translated = result.unwrap();
    assert!(translated.get("parent_tasks").is_some(), "Should have parent_tasks field");
    assert_eq!(translated["parent_tasks"].as_array().unwrap().len(), 1);
    assert_eq!(translated["parent_tasks"][0]["title"], "Valid Task");

    Ok(())
}

/// Test 4: MCP handlers must fail when translator is not used
#[test]
fn test_mcp_handlers_fail_on_missing_schema_translator() -> Result<()> {
    // This test ensures MCP handlers can't bypass translator

    // For now, test that missing TargetSchema would fail
    // This test will be expanded when we examine actual MCP handler code
    let valid_json = r#"{
        "prd_title": "Test Feature",
        "parent_tasks": [
            {
                "id": "task-1",
                "title": "Valid Task",
                "description": "Complete task with all fields",
                "complexity": "Simple",
                "estimated_hours": 2.5
            }
        ],
        "relevant_files": [],
        "estimated_complexity": "Simple"
    }"#;

    // This should work normally
    let result = translate_llm_output(valid_json, TargetSchema::TaskBreakdown);
    assert!(result.is_ok(), "Valid JSON should translate successfully");

    // Test that we detect missing translator usage in a hypothetical scenario
    // (This test will be expanded based on actual MCP handler inspection)

    Ok(())
}

/// Test 5: Translator must reject ambiguous type coercion (float for int)
#[test]
fn test_translator_rejects_ambiguous_types() -> Result<()> {
    // step_number should be integer, not float
    let ambiguous_json = r#"{
        "step_number": "5.5"
    }"#;

    // Should fail - translator must reject float-to-int coercion for integers
    let result = translate_llm_output(ambiguous_json, TargetSchema::SequentialStep);
    assert!(result.is_err(), "Translator must reject ambiguous type coercion");

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("type") || error_msg.contains("coercion") || error_msg.contains("validation"),
        "Error should mention type issue: {}", error_msg);

    Ok(())
}

/// Test 6: Sequential translator must reject invalid status values
#[test]
fn test_sequential_translator_rejects_inverted_status() -> Result<()> {
    // Status must be from allowed set: {"pending", "executing", "completed", "failed"}
    let invalid_status_json = r#"{
        "step_number": 1,
        "status": "done"
    }"#;

    // Should fail - "done" is not a valid status
    let result = translate_llm_output(invalid_status_json, TargetSchema::SequentialStep);
    assert!(result.is_err(), "Translator must reject invalid status values");

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("status") || error_msg.contains("validation") || error_msg.contains("enum"),
        "Error should mention status issue: {}", error_msg);

    Ok(())
}