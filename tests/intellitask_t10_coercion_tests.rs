//! IntelliTask T10 Coercion Mode Tests
//!
//! Tests for Phase T10 Hybrid Coercion Mode that adds a coercion pre-processor
//! for intellitask_next and intellitask_save commands without changing core
//! translator validation logic.

use syncore::mcp_tools::memory_suite::intellitask_commands::coerce_intellitask_payload;
use serde_json::json;

/// Test that intellitask_next accepts lightweight task arrays
#[test]
fn test_next_accepts_lightweight_tasks() {
    // Test array coercion (intellitask_next use case)
    let lightweight_tasks = json!([
        {"id": "1", "goal": "Implement user authentication", "priority": 1},
        {"id": "2", "goal": "Build user profile page", "priority": 2},
        {"id": "3", "goal": "Add password reset functionality"}  // missing priority
    ]);

    let coerced = coerce_intellitask_payload(lightweight_tasks);

    // Verify coercion produces valid TaskBreakdown structure
    assert!(coerced.is_object());
    assert_eq!(coerced.get("prd_title").unwrap().as_str().unwrap(), "Unknown PRD");
    assert_eq!(coerced.get("estimated_complexity").unwrap().as_str().unwrap(), "Moderate");

    let parent_tasks = coerced.get("parent_tasks").unwrap().as_array().unwrap();
    assert_eq!(parent_tasks.len(), 3);

    // Check that missing fields were added for each task
    for task in parent_tasks {
        assert!(task.get("id").is_some());
        assert!(task.get("title").is_some());
        assert!(task.get("description").is_some());
        assert!(task.get("complexity").is_some());
        assert!(task.get("estimated_hours").is_some());
        assert!(task.get("subtasks").is_some());
        assert!(task.get("dependencies").is_some());
    }
}

/// Test that intellitask_save accepts missing subtask fields
#[test]
fn test_save_accepts_missing_subtask_fields() {
    // Test object coercion (intellitask_save use case)
    let minimal_breakdown = json!({
        "prd_title": "User Authentication System",
        "parent_tasks": [
            {
                "id": "1.0",
                "title": "Implement Authentication",
                "description": "Add user login functionality",
                "subtasks": [
                    {
                        "id": "1.1",
                        "description": "Create login form"
                        // Missing: acceptance_criteria, files_to_modify, status
                    },
                    {
                        "id": "1.2",
                        "description": "Implement password hashing"
                        // Missing: acceptance_criteria, files_to_modify, status
                    }
                ]
            }
        ]
        // Missing: estimated_complexity
    });

    let coerced = coerce_intellitask_payload(minimal_breakdown);

    // Verify structure is preserved and missing fields added
    assert!(coerced.is_object());
    assert_eq!(coerced.get("prd_title").unwrap().as_str().unwrap(), "User Authentication System");
    assert_eq!(coerced.get("estimated_complexity").unwrap().as_str().unwrap(), "Moderate");

    let parent_tasks = coerced.get("parent_tasks").unwrap().as_array().unwrap();
    assert_eq!(parent_tasks.len(), 1);

    let subtasks = parent_tasks[0].get("subtasks").unwrap().as_array().unwrap();
    assert_eq!(subtasks.len(), 2);

    // Check that missing subtask fields were added
    for subtask in subtasks {
        assert!(subtask.get("acceptance_criteria").is_some());
        assert!(subtask.get("files_to_modify").is_some());
        assert!(subtask.get("description").is_some());
        assert!(subtask.get("status").is_some());
    }
}

/// Test that coercion still respects strict validation
#[test]
fn test_coercion_still_respects_strict_validation() {
    // Coercion should not fix invalid data types - only add missing fields
    let valid_structure_wrong_types = json!([
        {
            "id": "1",
            "goal": "Test task",
            "priority": "high"  // Wrong type, should be number - coercion won't fix this
        }
    ]);

    let coerced = coerce_intellitask_payload(valid_structure_wrong_types);

    // The structure should be corrected but types remain invalid
    assert!(coerced.is_object());
    let parent_tasks = coerced.get("parent_tasks").unwrap().as_array().unwrap();
    let task = &parent_tasks[0];

    // The priority should still be a string (invalid), which translator will reject
    assert_eq!(task.get("priority").unwrap().as_str().unwrap(), "high");
}

/// Test that valid canonical payloads remain unchanged after coercion
#[test]
fn test_does_not_modify_valid_payloads() {
    // Full canonical payload that should already be valid
    let valid_breakdown = json!({
        "prd_title": "Complete Project Setup",
        "estimated_complexity": "Complex",
        "parent_tasks": [
            {
                "id": "1.0",
                "title": "Database Setup",
                "description": "Configure database connection and schema",
                "complexity": "Moderate",
                "estimated_hours": 4.0,
                "subtasks": [
                    {
                        "id": "1.1",
                        "description": "Create database migration files",
                        "acceptance_criteria": ["Migration runs successfully"],
                        "files_to_modify": ["src/db/migrations/001_initial.sql"],
                        "status": "pending"
                    }
                ],
                "dependencies": []
            }
        ]
    });

    let coerced = coerce_intellitask_payload(valid_breakdown);

    // Valid payload should remain essentially unchanged
    assert!(coerced.is_object());
    assert_eq!(coerced.get("prd_title").unwrap().as_str().unwrap(), "Complete Project Setup");
    assert_eq!(coerced.get("estimated_complexity").unwrap().as_str().unwrap(), "Complex");

    let parent_tasks = coerced.get("parent_tasks").unwrap().as_array().unwrap();
    assert_eq!(parent_tasks.len(), 1);

    let subtasks = parent_tasks[0].get("subtasks").unwrap().as_array().unwrap();
    assert_eq!(subtasks.len(), 1);

    // All original fields should be preserved
    assert_eq!(subtasks[0].get("description").unwrap().as_str().unwrap(), "Create database migration files");
    assert_eq!(subtasks[0].get("status").unwrap().as_str().unwrap(), "pending");
}

/// Test coercion function behavior with various edge cases
#[test]
fn test_coercion_function_isolation() {
    // Test array coercion
    let input_array = json!([
        {"id": "1", "goal": "Task 1", "priority": 1},
        {"id": "2", "goal": "Task 2"}  // missing priority
    ]);

    let coerced = coerce_intellitask_payload(input_array);

    assert!(coerced.is_object());
    assert_eq!(coerced.get("prd_title").unwrap().as_str().unwrap(), "Unknown PRD");
    assert_eq!(coerced.get("estimated_complexity").unwrap().as_str().unwrap(), "Moderate");

    let parent_tasks = coerced.get("parent_tasks").unwrap().as_array().unwrap();
    assert_eq!(parent_tasks.len(), 2);

    // Check that missing fields were added
    let task1 = &parent_tasks[0];
    assert!(task1.get("title").is_some());
    assert!(task1.get("description").is_some());
    assert!(task1.get("complexity").is_some());
    assert!(task1.get("estimated_hours").is_some());
    assert!(task1.get("subtasks").is_some());
    assert!(task1.get("dependencies").is_some());

    // Test object coercion
    let input_object = json!({
        "parent_tasks": [
            {
                "id": "1.0",
                "title": "Test Task",
                "subtasks": [
                    {"id": "1.1", "description": "Subtask 1"}  // missing acceptance_criteria
                ]
            }
        ]
        // missing estimated_complexity
    });

    let coerced = coerce_intellitask_payload(input_object);

    assert!(coerced.is_object());
    assert_eq!(coerced.get("estimated_complexity").unwrap().as_str().unwrap(), "Moderate");

    let parent_tasks = coerced.get("parent_tasks").unwrap().as_array().unwrap();
    let subtasks = parent_tasks[0].get("subtasks").unwrap().as_array().unwrap();
    let subtask = &subtasks[0];

    // Check that missing subtask fields were added
    assert!(subtask.get("acceptance_criteria").is_some());
    assert!(subtask.get("files_to_modify").is_some());
    assert!(subtask.get("description").is_some());
    assert!(subtask.get("status").is_some());

    // Test that other value types are returned as-is
    let other_value = json!("string");
    let coerced = coerce_intellitask_payload(other_value);
    assert_eq!(coerced, json!("string"));
}