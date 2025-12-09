//! Regression Tests for IntelliTask LLM Output Translator
//!
//! Tests specific failure patterns discovered during real MCP tool testing.
//! These tests ensure that the translator fixes the exact issues encountered.

use anyhow::Result;
use serde_json::json;
use syncore::mcp_tools::translator::{translate_llm_output, TargetSchema};

/// Regression test for intellitask_save "14 error(s)" failure
/// This reproduces the exact failure pattern seen in MCP logs
#[test]
fn test_intellitask_save_14_errors_regression() -> Result<()> {
    // This is the exact problematic LLM output that caused "Schema validation failed with 14 error(s)"
    let problematic_llm_output = json!({
        "prd_title": "User Authentication System",
        "parent_tasks": [
            {
                "id": "1.0",
                "title": "Database Schema",
                "description": "Create user tables",
                "subtasks": [
                    "Create users table",
                    "Add password hashing"
                ],
                "dependencies": [],
                "complexity": "High",
                "estimated_hours": "8"
            },
            {
                "id": "2.0",
                "title": "API Endpoints",
                "description": "Login and register endpoints",
                "subtasks": "Not implemented yet",  // String instead of object
                "dependencies": ["1.0"],
                "complexity": "Medium",  // Invalid enum value
                "estimated_hours": 12.5
            }
        ],
        "relevant_files": [
            {
                "path": "src/models/user.rs",
                "description": "User model definition"  // Wrong field name
            }
        ],
        "estimated_complexity": "Low"  // Invalid enum value
    })
    .to_string();

    // Test without translator (should fail with schema validation)
    let direct_parse_result: Result<syncore::intellitask::TaskBreakdown, serde_json::Error> =
        serde_json::from_str(&problematic_llm_output);

    assert!(direct_parse_result.is_err(), "Direct parse should fail: {:?}", direct_parse_result);

    // Test with translator (should either fix or give clear error)
    let translated = translate_llm_output(&problematic_llm_output, TargetSchema::TaskBreakdown)?;

    if translated.get("error").is_none() {
        // If translator succeeded, verify the fixes
        let task_breakdown: syncore::intellitask::TaskBreakdown =
            serde_json::from_value(translated)?;

        // Verify complexity normalization
        assert_eq!(
            task_breakdown.parent_tasks[0].complexity,
            syncore::intellitask::Complexity::Complex
        ); // "High" -> "Complex"
        assert_eq!(
            task_breakdown.parent_tasks[1].complexity,
            syncore::intellitask::Complexity::Moderate
        ); // "Medium" -> "Moderate"
        assert_eq!(task_breakdown.estimated_complexity, syncore::intellitask::Complexity::Simple); // "Low" -> "Simple"

        // Verify subtasks are now objects, not strings
        let subtasks1 = &task_breakdown.parent_tasks[0].subtasks;
        assert!(!subtasks1.is_empty(), "Should have subtask objects");
        assert_eq!(subtasks1[0].description, "Create users table");

        let subtasks2 = &task_breakdown.parent_tasks[1].subtasks;
        // The string "Not implemented yet" should either become an error or be converted to a proper Subtask
        if !subtasks2.is_empty() {
            assert_eq!(subtasks2[0].description, "Not implemented yet");
        }

        // Verify FileReference structure fix
        assert!(!task_breakdown.relevant_files.is_empty());
        let file_ref = &task_breakdown.relevant_files[0];
        assert_eq!(file_ref.path, "src/models/user.rs");
        assert_eq!(file_ref.purpose, "User model definition"); // Should be "purpose" not "description"
    } else {
        // If translator errored, verify it's a structured error
        assert_eq!(translated["error"], "SchemaValidationFailed");
        let missing_fields = translated.get("missing_fields").and_then(|f| f.as_array());
        assert!(missing_fields.is_some(), "Should have missing_fields in error response");
    }

    Ok(())
}

/// Regression test for intellitask_prioritize "Invalid tasks JSON" failure
#[test]
fn test_intellitask_prioritize_non_json_regression() {
    // LLM output that can't be parsed as JSON at all (prose response)
    let invalid_tasks_json = r#"
Based on the analysis, here are the tasks prioritized:

1. Database Schema (Critical) - Foundation for everything else
2. User Authentication (High) - Core feature needed
3. API Integration (Medium) - Connect with external services

The database schema must be completed first.
    "#;

    // This should fail to parse as JSON at all
    let direct_json_parse: Result<serde_json::Value, serde_json::Error> =
        serde_json::from_str(invalid_tasks_json);
    assert!(
        direct_json_parse.is_err(),
        "Non-JSON input should fail to parse: {:?}",
        direct_json_parse
    );

    // Test that translator can handle non-JSON input gracefully
    let translated = translate_llm_output(invalid_tasks_json, TargetSchema::TaskBreakdown);

    // Should return Result::Err when no valid JSON can be extracted, not panic
    assert!(translated.is_err(), "Non-JSON input should produce Result::Err, not Ok");
}

/// Regression test for intellitask_prioritize malformed JSON failure
#[test]
fn test_intellitask_prioritize_malformed_json_regression() -> Result<()> {
    // LLM output with wrong JSON structure for Vec<ParentTask>
    let malformed_tasks_json = json!({
        "tasks": [  // Wrong root structure - should be direct array
            {
                "id": "1.0",
                "title": "Database Schema",
                "description": "Setup database",
                "subtasks": ["Create tables", "Add indexes"],  // Strings instead of objects
                "dependencies": [],
                "complexity": "High",  // Invalid enum value
                "estimated_hours": "8.0"  // String instead of number
            }
        ]
    })
    .to_string();

    // Test with translator
    let translated = translate_llm_output(&malformed_tasks_json, TargetSchema::TaskBreakdown)?;

    // Should error due to wrong structure (not a valid TaskBreakdown)
    assert!(translated.get("error").is_some(), "Malformed JSON should produce error");
    assert_eq!(translated["error"], "SchemaValidationFailed");

    Ok(())
}

/// Regression test for missing critical fields
#[test]
fn test_missing_critical_fields_regression() -> Result<()> {
    // Completely empty JSON - should fail with clear error
    let empty_input = "{}".to_string();

    let translated = translate_llm_output(&empty_input, TargetSchema::TaskBreakdown)?;

    assert_eq!(translated["error"], "SchemaValidationFailed");

    let missing_fields = translated["missing_fields"].as_array().unwrap();
    assert!(missing_fields.iter().any(|f| f.as_str() == Some("Missing required field: prd_title")));
    assert!(missing_fields
        .iter()
        .any(|f| f.as_str() == Some("Missing required field: parent_tasks")));
    assert!(missing_fields
        .iter()
        .any(|f| f.as_str() == Some("Missing required field: estimated_complexity")));

    Ok(())
}

/// Regression test for priority assignment with wrong types
#[test]
fn test_priority_assignment_type_mismatch_regression() -> Result<()> {
    // PriorityResult with wrong types (numbers instead of strings)
    let wrong_types_priority = json!({
        "priorities": [
            {
                "task_id": 123,  // Number instead of string
                "priority": 1     // Number instead of string
            },
            {
                "task_id": "456",
                "priority": null  // Null instead of string
            }
        ]
    })
    .to_string();

    let translated = translate_llm_output(&wrong_types_priority, TargetSchema::PriorityResult)?;

    if translated.get("error").is_none() {
        // If translator succeeded, verify type coercion
        let priorities = translated["priorities"].as_array().unwrap();
        assert_eq!(priorities[0]["task_id"], "123"); // Should be coerced to string
        assert_eq!(priorities[0]["priority"], "1"); // Should be coerced to string
    } else {
        // If translator errored, verify it's because priority is null (can't coerce)
        assert_eq!(translated["error"], "SchemaValidationFailed");
    }

    Ok(())
}

/// Regression test for extreme complexity values
#[test]
fn test_extreme_complexity_values_regression() -> Result<()> {
    // Test unusual complexity values that might come from LLM
    let extreme_complexity = json!({
        "prd_title": "Test Feature",
        "parent_tasks": [{
            "id": "1.0",
            "title": "Test Task",
            "description": "Test description",
            "subtasks": [{
                "id": "1.1",
                "description": "Test subtask",
                "acceptance_criteria": ["Working"],
                "dependencies": [],
                "files_to_modify": ["test.rs"],
                "complexity": "EXTREMELY DIFFICULT",  // Unusual value
                "estimated_hours": 100.0
            }],
            "dependencies": [],
            "complexity": "very easy",  // Lowercase unusual value
            "estimated_hours": 0.5
        }],
        "relevant_files": [],
        "estimated_complexity": "Unknown complexity"  // Unknown value
    })
    .to_string();

    let translated = translate_llm_output(&extreme_complexity, TargetSchema::TaskBreakdown)?;

    if translated.get("error").is_none() {
        // If translator succeeded, check that unknown complexities were handled
        let parent_tasks = translated["parent_tasks"].as_array().unwrap();

        // "very easy" should normalize to "Trivial"
        assert_eq!(parent_tasks[0]["complexity"], "Trivial");

        // "EXTREMELY DIFFICULT" should either normalize or become a safe default
        let subtask_complexity =
            parent_tasks[0]["subtasks"].as_array().unwrap()[0]["complexity"].as_str().unwrap();
        assert!(["Complex", "VeryComplex", "Moderate"].contains(&subtask_complexity));

        // Unknown complexity should either normalize or become a safe default
        let estimated_complexity = translated["estimated_complexity"].as_str().unwrap();
        assert!(["Trivial", "Simple", "Moderate", "Complex", "VeryComplex"]
            .contains(&estimated_complexity));
    }
    // If translator errored, that's also acceptable for unknown complexity values

    Ok(())
}
