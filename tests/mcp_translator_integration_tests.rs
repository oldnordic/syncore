//! MCP Integration Tests for Translator Pipeline
//!
//! Tests prove that MCP handlers MUST use translator before deserialization.
//! All tests are EXPECTED TO FAIL initially, proving missing translator wiring.

use anyhow::Result;
use serde_json::json;
use syncore::mcp_tools::translator::{translate_llm_output, TargetSchema};

/// Test 1: Translator should normalize invalid TaskBreakdown JSON
#[test]
fn test_translator_normalizes_invalid_task_breakdown() -> Result<()> {
    // Malformed JSON that needs translator normalization
    let malformed_json = json!({
        "prd_title": "Test Feature",
        "parent_tasks": [{
            "id": "1.0",
            "title": "Test Task",
            "description": "Test description",
            "subtasks": ["Create database schema", "Implement authentication"], // STRINGS - should be objects
            "dependencies": [],
            "complexity": "High",  // INVALID - should be "Complex"
            "estimated_hours": "8.5"  // STRING - should be f32
        }],
        "relevant_files": [{
            "path": "src/models/user.rs",
            "description": "User model definition"  // WRONG FIELD - should be "purpose"
        }],
        "estimated_complexity": "Low"  // INVALID - should be "Simple"
    }).to_string();

    // This should be normalized by translator
    let result = translate_llm_output(&malformed_json, TargetSchema::TaskBreakdown)?;

    // Should succeed without errors (translator normalizes invalid values)
    assert!(!result.get("error").is_some(),
        "Translator should normalize invalid JSON: {:?}", result.get("error"));

    // Verify complex key normalizations
    assert_eq!(result["estimated_complexity"], "Simple"); // "Low" → "Simple"

    let parent_tasks = result["parent_tasks"].as_array().unwrap();
    assert_eq!(parent_tasks[0]["complexity"], "Complex"); // "High" → "Complex"
    assert_eq!(parent_tasks[0]["estimated_hours"], 8.5); // "8.5" string → 8.5 f32

    // Verify field mapping
    let relevant_files = result["relevant_files"].as_array().unwrap();
    assert!(relevant_files[0].get("purpose").is_some(), "Should have 'purpose' field, not 'description'");
    assert!(relevant_files[0].get("description").is_none(), "Should not have 'description' field");

    Ok(())
}

/// Test 2: Translator should normalize PriorityResult with type coercion
#[test]
fn test_translator_normalizes_priority_result() -> Result<()> {
    let malformed_json = json!({
        "priorities": [
            {
                "task_id": 123,  // NUMBER - should be string
                "priority": 5     // NUMBER - should be string
            },
            {
                "task_id": "456", // already string
                "priority": "High" // already string
            }
        ]
    }).to_string();

    let result = translate_llm_output(&malformed_json, TargetSchema::PriorityResult)?;

    assert!(!result.get("error").is_some(),
        "Translator should normalize priority result: {:?}", result.get("error"));

    let priorities = result["priorities"].as_array().unwrap();
    assert_eq!(priorities[0]["task_id"], "123"); // Number coerced to string
    assert_eq!(priorities[0]["priority"], "5");   // Number coerced to string
    assert_eq!(priorities[1]["task_id"], "456"); // String unchanged
    assert_eq!(priorities[1]["priority"], "High"); // String unchanged

    Ok(())
}

/// Test 3: Translator should normalize SequentialStep with type coercion and auto-generation
#[test]
fn test_translator_normalizes_sequential_step() -> Result<()> {
    let malformed_json = json!({
        "step_number": "5",  // STRING - should be int
        "thought": "Need to implement user authentication",
        "reasoning": "Security is critical for this system"
        // Missing: action, observation - should be auto-filled
    }).to_string();

    let result = translate_llm_output(&malformed_json, TargetSchema::SequentialStep)?;

    assert!(!result.get("error").is_some(),
        "Translator should normalize sequential step: {:?}", result.get("error"));

    // Should coerce string to int
    assert_eq!(result["step_number"], 5);

    // Should auto-generate missing fields
    assert!(result.get("step_id").is_some());
    assert!(result.get("timestamp").is_some());
    assert_eq!(result["status"], "pending"); // Default status

    // Should preserve provided fields
    assert_eq!(result["thought"], "Need to implement user authentication");
    assert_eq!(result["reasoning"], "Security is critical for this system");

    Ok(())
}

/// Test 4: Translator should reject completely malformed JSON
#[test]
fn test_translator_rejects_completely_malformed_json() {
    let completely_broken = r#"
{
  "step_number": "not a number",
  "thought": "This is totally broken
  "reasoning": "unclosed string
  "invalid_field": [broken array
    "#;

    let result = translate_llm_output(completely_broken, TargetSchema::SequentialStep);

    // Should return Result::Err for unparseable JSON
    assert!(result.is_err(),
        "Translator should reject completely malformed JSON");
}

/// Test 7: Verify no raw deserialization in MCP handlers
#[test]
fn test_no_raw_deserialize_allowed() -> Result<()> {
    // Read MCP server handler source
    let server_source = include_str!("../src/mcp_server/server.rs");
    let intellitask_source = include_str!("../src/intellitask.rs");
    let sequential_source = include_str!("../src/mcp_tools/memory_suite/sequential_commands.rs");

    // These patterns should NOT exist in MCP handlers if translator is required
    let forbidden_patterns = [
        "serde_json::from_str(",
        ".from_str(",
    ];

    for pattern in &forbidden_patterns {
        // Check MCP server handlers (should use translator instead)
        assert!(
            server_source.matches(pattern).count() == 0,
            "Found forbidden pattern '{}' in MCP server handlers. Translator should be used instead.",
            pattern
        );

        // Check IntelliTask module - EXCLUDE all test context (legitimate use)
        let intellitask_lines: Vec<&str> = intellitask_source.lines().collect();
        let mut intellitask_non_test_count = 0;
        let mut in_test_context = false;
        for line in intellitask_lines {
            // Detect test context start
            if line.contains("#[test]") || line.contains("fn test_") {
                in_test_context = true;
                continue;
            }
            // Detect end of test function (next function or module end)
            if in_test_context && (line.trim().starts_with("fn ") && !line.contains("fn test_") || line.trim().starts_with("mod ") || line.trim().starts_with("impl ")) {
                in_test_context = false;
                continue;
            }
            // Skip if we're in test context
            if in_test_context {
                continue;
            }
            // Count patterns in non-test code
            intellitask_non_test_count += line.matches(pattern).count();
        }
        assert!(
            intellitask_non_test_count == 0,
            "Found forbidden pattern '{}' in IntelliTask non-test code. Translator should be used instead.",
            pattern
        );

        // Check sequential commands - allow the helper function which uses translator first
        let sequential_lines: Vec<&str> = sequential_source.lines().collect();
        let mut sequential_non_helper_count = 0;
        let mut in_helper_function = false;
        for line in sequential_lines {
            // Detect helper function start
            if line.contains("fn translate_sequential_step_array") {
                in_helper_function = true;
                continue;
            }
            // Detect end of helper function (next function)
            if in_helper_function && line.trim().starts_with("fn ") {
                in_helper_function = false;
                continue;
            }
            // Skip if we're in the helper function
            if in_helper_function {
                continue;
            }
            // Count patterns in non-helper code
            sequential_non_helper_count += line.matches(pattern).count();
        }
        assert!(
            sequential_non_helper_count == 0,
            "Found forbidden pattern '{}' in sequential commands outside translator helper. Translator should be used instead.",
            pattern
        );
    }

    Ok(())
}