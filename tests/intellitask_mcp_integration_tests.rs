//! Integration Tests for IntelliTask MCP Tools with Translator Layer
//!
//! These tests specifically target the schema validation failures discovered in real MCP usage
//! and verify that the translator layer fixes them.

use anyhow::Result;
use serde_json::{json, Value};
use syncore::intellitask::{Complexity, FileReference, ParentTask, Subtask, TaskBreakdown};
use syncore::mcp_tools::translator::{translate_llm_output, TargetSchema};

/// Test: intellitask_save schema validation failure (14 errors)
/// This reproduces the exact failure pattern seen in MCP logs
#[test]
fn test_intellitask_save_schema_validation_failures() -> Result<()> {
    // This is the problematic LLM output that caused "Schema validation failed with 14 error(s)"
    let problematic_llm_output = r#"
    {
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
    }
    "#;

    // Test without translator (should fail with schema validation)
    let direct_parse_result: Result<TaskBreakdown, serde_json::Error> =
        serde_json::from_str(problematic_llm_output);

    assert!(direct_parse_result.is_err(), "Direct parse should fail: {:?}", direct_parse_result);

    // Test with translator (should fix structural issues)
    let translated = translate_llm_output(problematic_llm_output, TargetSchema::TaskBreakdown)?;

    // Should succeed without errors after translation
    assert!(
        !translated.get("error").is_some(),
        "Translation failed: {:?}",
        translated.get("error")
    );

    // Verify specific fixes:
    let parent_tasks = translated["parent_tasks"].as_array().unwrap();
    assert_eq!(parent_tasks.len(), 2);

    // Check subtasks are objects, not strings
    let subtasks = parent_tasks[0]["subtasks"].as_array().unwrap();
    assert!(subtasks.len() >= 2, "Subtasks should be objects");

    // Check complexity normalization
    assert_eq!(parent_tasks[0]["complexity"], "Complex"); // "High" -> "Complex"
    assert_eq!(parent_tasks[1]["complexity"], "Moderate"); // "Medium" -> "Moderate"
    assert_eq!(translated["estimated_complexity"], "Simple"); // "Low" -> "Simple"

    // Check FileReference structure
    let relevant_files = translated["relevant_files"].as_array().unwrap();
    assert!(relevant_files.len() > 0);

    // Should have 'purpose' field, not 'description'
    let file_ref = &relevant_files[0];
    assert!(file_ref.get("purpose").is_some(), "Should have 'purpose' field");
    assert!(file_ref.get("description").is_none(), "Should not have 'description' field");

    // Verify final TaskBreakdown can be deserialized
    let task_breakdown: TaskBreakdown = serde_json::from_value(translated)?;
    assert_eq!(task_breakdown.prd_title, "User Authentication System");

    Ok(())
}

/// Test: intellitask_prioritize "Invalid tasks JSON" failure
#[test]
fn test_intellitask_prioritize_invalid_json() -> Result<()> {
    // LLM output that can't be parsed into Vec<ParentTask>
    let invalid_tasks_json = r#"
    Based on the analysis, here are the tasks prioritized:

    1. Database Schema (Critical) - Foundation for everything else
    2. User Authentication (High) - Core feature needed
    3. API Integration (Medium) - Connect with external services

    The database schema must be completed first.
    "#;

    // This should fail to parse as Vec<ParentTask>
    let direct_parse_result: Result<Vec<ParentTask>, serde_json::Error> =
        serde_json::from_str(invalid_tasks_json);

    assert!(direct_parse_result.is_err(), "Direct parse should fail: {:?}", direct_parse_result);

    // Test that translator can handle non-JSON input gracefully
    let translated = translate_llm_output(invalid_tasks_json, TargetSchema::TaskBreakdown);

    // Should return an error, not panic
    assert!(translated.is_ok());
    let result = translated.unwrap();
    assert_eq!(result["error"], "SchemaValidationFailed");
}

/// Test: Valid TaskBreakdown should pass through unchanged
#[test]
fn test_valid_taskbreakdown_passthrough() -> Result<()> {
    let valid_input = json!({
        "prd_title": "Feature Implementation",
        "parent_tasks": [{
            "id": "1.0",
            "title": "Core Feature",
            "description": "Implement the main functionality",
            "subtasks": [{
                "id": "1.1",
                "description": "Create data structures",
                "acceptance_criteria": ["Structs defined", "Tests pass"],
                "dependencies": [],
                "files_to_modify": ["src/models.rs"],
                "complexity": "Simple",
                "estimated_hours": 4.0
            }],
            "dependencies": [],
            "complexity": "Moderate",
            "estimated_hours": 8.0
        }],
        "relevant_files": [{
            "path": "src/main.rs",
            "purpose": "Entry point implementation",
            "action": "Modify"
        }],
        "estimated_complexity": "Moderate"
    })
    .to_string();

    let translated = translate_llm_output(&valid_input, TargetSchema::TaskBreakdown)?;

    // Should succeed without errors
    assert!(
        !translated.get("error").is_some(),
        "Valid input should not cause errors: {:?}",
        translated.get("error")
    );

    // Should be deserializable to TaskBreakdown
    let task_breakdown: TaskBreakdown = serde_json::from_value(translated)?;
    assert_eq!(task_breakdown.prd_title, "Feature Implementation");
    assert_eq!(task_breakdown.parent_tasks.len(), 1);
    assert_eq!(task_breakdown.parent_tasks[0].subtasks.len(), 1);
}

/// Test: Sequential tool normalization
#[test]
fn test_sequential_tool_normalization() -> Result<()> {
    let raw_sequential_input = json!({
        "step_number": 1,
        "thought": "I need to understand the current codebase structure",
        "reasoning": "Understanding the codebase is essential before making changes",
        "action": "Read the main files",
        "observation": "Found existing structure"
    })
    .to_string();

    let translated = translate_llm_output(&raw_sequential_input, TargetSchema::SequentialStep)?;

    // Should succeed and auto-generate missing fields
    assert!(
        !translated.get("error").is_some(),
        "Sequential translation failed: {:?}",
        translated.get("error")
    );

    // Should have auto-generated fields
    assert!(translated.get("step_id").is_some());
    assert!(translated.get("timestamp").is_some());
    assert_eq!(translated["status"], "pending"); // Default status

    // Should preserve provided fields
    assert_eq!(translated["step_number"], 1);
    assert!(translated["thought"].as_str().unwrap().contains("codebase structure"));
}

/// Test: Missing required fields should produce clear errors
#[test]
fn test_missing_required_fields_error() -> Result<()> {
    let incomplete_input = json!({
        "prd_title": "Test Feature"
        // Missing parent_tasks and estimated_complexity
    })
    .to_string();

    let translated = translate_llm_output(&incomplete_input, TargetSchema::TaskBreakdown)?;

    // Should produce structured error
    assert_eq!(translated["error"], "SchemaValidationFailed");

    let missing_fields = translated["missing_fields"].as_array().unwrap();
    assert!(missing_fields.iter().any(|f| f.as_str() == Some("parent_tasks")));
    assert!(missing_fields.iter().any(|f| f.as_str() == Some("estimated_complexity")));
}

/// Test: Type coercion edge cases
#[test]
fn test_type_coercion_edge_cases() -> Result<()> {
    // Test numeric strings being coerced to numbers
    let input_with_numeric_strings = json!({
        "prd_title": "Test",
        "parent_tasks": [{
            "id": "1.0",
            "title": "Test Task",
            "description": "Test description",
            "subtasks": [],
            "dependencies": [],
            "complexity": "Simple",
            "estimated_hours": "8.5"  // String that should become f32
        }],
        "relevant_files": [],
        "estimated_complexity": "Simple"
    })
    .to_string();

    let translated =
        translate_llm_output(&input_with_numeric_strings, TargetSchema::TaskBreakdown)?;

    assert!(
        !translated.get("error").is_some(),
        "Type coercion failed: {:?}",
        translated.get("error")
    );

    let task_breakdown: TaskBreakdown = serde_json::from_value(translated)?;
    assert_eq!(task_breakdown.parent_tasks[0].estimated_hours, 8.5);
}

/// Test: FileAction alias normalization
#[test]
fn test_fileaction_alias_normalization() -> Result<()> {
    let input_with_aliases = json!({
        "prd_title": "Test",
        "parent_tasks": [{
            "id": "1.0",
            "title": "Test",
            "description": "Test",
            "subtasks": [],
            "dependencies": [],
            "complexity": "Simple",
            "estimated_hours": 4.0
        }],
        "relevant_files": [
            {
                "path": "test1.rs",
                "purpose": "Test file 1",
                "action": "Add"  // Should normalize to Modify2
            },
            {
                "path": "test2.rs",
                "purpose": "Test file 2",
                "action": "Implement"  // Should normalize to Modify2
            },
            {
                "path": "test3.rs",
                "purpose": "Test file 3",
                "action": "Create"  // Should stay as Create
            }
        ],
        "estimated_complexity": "Simple"
    })
    .to_string();

    let translated = translate_llm_output(&input_with_aliases, TargetSchema::TaskBreakdown)?;

    let task_breakdown: TaskBreakdown = serde_json::from_value(translated)?;

    // Check FileAction normalization
    assert_eq!(task_breakdown.relevant_files[0].action, syncore::intellitask::FileAction::Modify2);
    assert_eq!(task_breakdown.relevant_files[1].action, syncore::intellitask::FileAction::Modify2);
    assert_eq!(task_breakdown.relevant_files[2].action, syncore::intellitask::FileAction::Create);
}
