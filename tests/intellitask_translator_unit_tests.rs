//! Unit Tests for IntelliTask LLM Output Translator
//!
//! Tests individual translator functions with deterministic inputs and outputs.

use anyhow::Result;
use serde_json::{json, Value};
use syncore::mcp_tools::translator::{translate_llm_output, TargetSchema};

#[test]
fn test_valid_task_breakdown_passthrough() -> Result<()> {
    // Valid TaskBreakdown should pass through unchanged
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
    }).to_string();

    let result = translate_llm_output(&valid_input, TargetSchema::TaskBreakdown)?;

    // Should succeed without errors
    assert!(!result.get("error").is_some(), "Valid input should not cause errors: {:?}", result.get("error"));

    // Should preserve all valid fields
    assert_eq!(result["prd_title"], "Feature Implementation");
    assert_eq!(result["estimated_complexity"], "Moderate");

    let parent_tasks = result["parent_tasks"].as_array().unwrap();
    assert_eq!(parent_tasks.len(), 1);
    assert_eq!(parent_tasks[0]["id"], "1.0");
    assert_eq!(parent_tasks[0]["complexity"], "Moderate");

    let subtasks = parent_tasks[0]["subtasks"].as_array().unwrap();
    assert_eq!(subtasks.len(), 1);
    assert_eq!(subtasks[0]["id"], "1.1");
    assert_eq!(subtasks[0]["estimated_hours"], 4.0);
    Ok(())
}

#[test]
fn test_missing_required_fields_error() -> Result<()> {
    // Missing prd_title, parent_tasks, estimated_complexity should error
    let incomplete_input = json!({
        "parent_tasks": [{
            "id": "1.0",
            "title": "Test Task"
            // Missing required fields: description, subtasks, complexity, estimated_hours
        }]
        // Missing prd_title, estimated_complexity, relevant_files
    }).to_string();

    let result = translate_llm_output(&incomplete_input, TargetSchema::TaskBreakdown)?;

    // Should produce structured error
    assert_eq!(result["error"], "SchemaValidationFailed");

    let missing_fields = result["missing_fields"].as_array().unwrap();
    assert!(missing_fields.iter().any(|f| f.as_str() == Some("Missing required field: prd_title")));
    assert!(missing_fields.iter().any(|f| f.as_str() == Some("Missing required field: estimated_complexity")));
    Ok(())
}

#[test]
fn test_complexity_enum_normalization() -> Result<()> {
    // Test that invalid complexity values get normalized to valid enum values
    let invalid_complexity_input = json!({
        "prd_title": "Test Feature",
        "parent_tasks": [{
            "id": "1.0",
            "title": "Test Task",
            "description": "Test description",
            "subtasks": [],
            "dependencies": [],
            "complexity": "High",  // Should normalize to "Complex"
            "estimated_hours": 8.0
        }, {
            "id": "2.0",
            "title": "Another Task",
            "description": "Another description",
            "subtasks": [],
            "dependencies": [],
            "complexity": "Low",   // Should normalize to "Simple"
            "estimated_hours": 2.0
        }, {
            "id": "3.0",
            "title": "Medium Task",
            "description": "Medium description",
            "subtasks": [],
            "dependencies": [],
            "complexity": "Medium", // Should normalize to "Moderate"
            "estimated_hours": 6.0
        }],
        "relevant_files": [],
        "estimated_complexity": "Low" // Should normalize to "Simple"
    }).to_string();

    let result = translate_llm_output(&invalid_complexity_input, TargetSchema::TaskBreakdown)?;

    // Should succeed after normalization
    assert!(!result.get("error").is_some(),
        "Complexity normalization failed: {:?}", result.get("error"));

    let parent_tasks = result["parent_tasks"].as_array().unwrap();
    assert_eq!(parent_tasks[0]["complexity"], "Complex");  // "High" → "Complex"
    assert_eq!(parent_tasks[1]["complexity"], "Simple");    // "Low" → "Simple"
    assert_eq!(parent_tasks[2]["complexity"], "Moderate");  // "Medium" → "Moderate"
    assert_eq!(result["estimated_complexity"], "Simple");    // "Low" → "Simple"
    Ok(())
}

#[test]
fn test_subtasks_as_strings_error() -> Result<()> {
    // Subtasks as strings should cause validation error (not auto-fixed)
    let invalid_subtasks_input = json!({
        "prd_title": "Test Feature",
        "parent_tasks": [{
            "id": "1.0",
            "title": "Test Task",
            "description": "Test description",
            "subtasks": [
                "Create database schema",
                "Implement authentication",
                "Write tests"
            ],
            "dependencies": [],
            "complexity": "Moderate",
            "estimated_hours": 12.0
        }],
        "relevant_files": [],
        "estimated_complexity": "Moderate"
    }).to_string();

    let result = translate_llm_output(&invalid_subtasks_input, TargetSchema::TaskBreakdown)?;

    // Should produce error due to subtasks being strings instead of objects
    assert!(result.get("error").is_some(),
        "Should error when subtasks are strings");
    Ok(())
}

#[test]
fn test_file_reference_field_fix() -> Result<()> {
    // FileReference with "description" field should get corrected to "purpose"
    let wrong_file_field_input = json!({
        "prd_title": "Test Feature",
        "parent_tasks": [{
            "id": "1.0",
            "title": "Test Task",
            "description": "Test description",
            "subtasks": [],
            "dependencies": [],
            "complexity": "Simple",
            "estimated_hours": 2.0
        }],
        "relevant_files": [{
            "path": "src/test.rs",
            "description": "Test file implementation"  // Wrong field name
        }],
        "estimated_complexity": "Simple"
    }).to_string();

    let result = translate_llm_output(&wrong_file_field_input, TargetSchema::TaskBreakdown)?;

    // Should succeed with field normalization
    assert!(!result.get("error").is_some(),
        "FileReference field fix failed: {:?}", result.get("error"));

    let relevant_files = result["relevant_files"].as_array().unwrap();
    let file_ref = &relevant_files[0];

    // Should have "purpose" field, not "description"
    assert!(file_ref.get("purpose").is_some(), "Should have 'purpose' field");
    assert_eq!(file_ref["purpose"], "Test file implementation");

    // Should not have "description" field
    assert!(file_ref.get("description").is_none(), "Should not have 'description' field");
    Ok(())
}

#[test]
fn test_type_coercion_estimated_hours() -> Result<()> {
    // Test that string numbers get coerced to f32 for estimated_hours
    let string_numbers_input = json!({
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
                "complexity": "Simple",
                "estimated_hours": "4.5"  // String that should become f32
            }],
            "dependencies": [],
            "complexity": "Moderate",
            "estimated_hours": "8.5"  // String that should become f32
        }],
        "relevant_files": [],
        "estimated_complexity": "Simple"
    }).to_string();

    let result = translate_llm_output(&string_numbers_input, TargetSchema::TaskBreakdown)?;

    // Should succeed with type coercion
    assert!(!result.get("error").is_some(),
        "Type coercion failed: {:?}", result.get("error"));

    let parent_tasks = result["parent_tasks"].as_array().unwrap();
    assert_eq!(parent_tasks[0]["estimated_hours"], 8.5);  // Should be f32, not string

    let subtasks = parent_tasks[0]["subtasks"].as_array().unwrap();
    assert_eq!(subtasks[0]["estimated_hours"], 4.5);     // Should be f32, not string
    Ok(())
}

#[test]
fn test_empty_arrays_auto_fix() -> Result<()> {
    // Missing array fields should get auto-fixed as empty arrays
    let missing_arrays_input = json!({
        "prd_title": "Test Feature",
        "parent_tasks": [{
            "id": "1.0",
            "title": "Test Task",
            "description": "Test description",
            // Missing subtasks, dependencies arrays
            "complexity": "Simple",
            "estimated_hours": 2.0
        }],
        "estimated_complexity": "Simple"  // Add this required field
        // Missing relevant_files array
    }).to_string();

    let result = translate_llm_output(&missing_arrays_input, TargetSchema::TaskBreakdown)?;

    // Should succeed with auto-generated arrays
    assert!(!result.get("error").is_some(),
        "Empty array auto-fix failed: {:?}", result.get("error"));

    let parent_tasks = result["parent_tasks"].as_array().unwrap();
    assert_eq!(parent_tasks[0]["subtasks"].as_array().unwrap().len(), 0);
    assert_eq!(parent_tasks[0]["dependencies"].as_array().unwrap().len(), 0);
    assert_eq!(result["relevant_files"].as_array().unwrap().len(), 0);
    Ok(())
}

#[test]
fn test_priority_result_validation() -> Result<()> {
    // Valid PriorityResult should pass through
    let valid_priority_input = json!({
        "priorities": [
            {
                "task_id": "1.0",
                "priority": "High"
            },
            {
                "task_id": "2.0",
                "priority": "Low"
            }
        ]
    }).to_string();

    let result = translate_llm_output(&valid_priority_input, TargetSchema::PriorityResult)?;

    assert!(!result.get("error").is_some(),
        "Valid PriorityResult failed: {:?}", result.get("error"));

    let priorities = result["priorities"].as_array().unwrap();
    assert_eq!(priorities.len(), 2);
    assert_eq!(priorities[0]["task_id"], "1.0");
    assert_eq!(priorities[0]["priority"], "High");  // Should remain string
    assert_eq!(priorities[1]["task_id"], "2.0");
    assert_eq!(priorities[1]["priority"], "Low");   // Should remain string
    Ok(())
}

#[test]
fn test_priority_result_missing_priorities_error() -> Result<()> {
    // Missing priorities array should get auto-fixed as empty array (translator is smart!)
    let missing_priorities_input = json!({
        "results": [  // Wrong field name - will be ignored, empty priorities array created
            {
                "task_id": "1.0",
                "priority": "High"
            }
        ]
    }).to_string();

    let result = translate_llm_output(&missing_priorities_input, TargetSchema::PriorityResult)?;

    // Should succeed with auto-generated empty priorities array
    assert!(!result.get("error").is_some(),
        "PriorityResult auto-fix failed: {:?}", result.get("error"));

    let priorities = result["priorities"].as_array().unwrap();
    assert_eq!(priorities.len(), 0); // Should be empty array
    Ok(())
}

// Debug helper function to understand what's happening
fn _debug_missing_fields() -> Result<()> {
    let incomplete_input = json!({
        "parent_tasks": [{
            "id": "1.0",
            "title": "Test Task"
        }]
    }).to_string();

    let result = translate_llm_output(&incomplete_input, TargetSchema::TaskBreakdown)?;
    println!("=== DEBUG Missing fields test ===");
    println!("Error: {:?}", result.get("error"));
    println!("Missing fields: {:?}", result.get("missing_fields"));
    Ok(())
}

#[test]
fn test_priority_result_type_coercion() -> Result<()> {
    // Test that numeric task_id and priority get coerced to strings
    let numeric_priority_input = json!({
        "priorities": [
            {
                "task_id": 123,  // Number that should become string
                "priority": 1     // Number that should become string
            },
            {
                "task_id": "456", // Already string
                "priority": "High" // Already string
            }
        ]
    }).to_string();

    let result = translate_llm_output(&numeric_priority_input, TargetSchema::PriorityResult)?;

    assert!(!result.get("error").is_some(),
        "Priority type coercion failed: {:?}", result.get("error"));

    let priorities = result["priorities"].as_array().unwrap();
    assert_eq!(priorities[0]["task_id"], "123");  // Should be string "123"
    assert_eq!(priorities[0]["priority"], "1");   // Should be string "1"
    assert_eq!(priorities[1]["task_id"], "456");  // Should remain string "456"
    assert_eq!(priorities[1]["priority"], "High"); // Should remain string "High"
    Ok(())
}