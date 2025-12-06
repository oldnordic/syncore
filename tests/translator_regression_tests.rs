//! Regression Tests for LLM Output Translator
//!
//! Tests specific failure cases discovered during real MCP tool testing in Phase 1.
//! These tests ensure that the translator fixes the exact issues encountered.

use anyhow::Result;
use serde_json::json;
use syncore::mcp_tools::translator::{translate_llm_output, TargetSchema};

/// Regression test for intellitask_prioritize "missing field 'priorities'" error
#[test]
fn test_regression_prioritize_missing_priorities_field() -> Result<()> {
    // This input caused the "missing field 'priorities'" error in Phase 1 testing
    let problematic_input = r#"
    Based on the task analysis, here are the priority assignments:

    Task 1.0: Critical - Database schema blocks all other tasks
    Task 2.0: High - Authentication API is the next most important
    Task 3.0: Medium - User interface can be done in parallel

    The dependencies are clear: 1.0 -> 2.0 -> 3.0
    "#;

    let result = translate_llm_output(problematic_input, TargetSchema::PriorityResult)?;

    // Should detect that this is not valid JSON and produce a proper error
    assert_eq!(result["error"], "SchemaValidationFailed");
    assert!(result["missing_fields"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f.as_str() == Some("priorities array")));
}

/// Regression test for correct PriorityResult structure
#[test]
fn test_regression_priority_result_structure() -> Result<()> {
    // This is the CORRECT structure that intellitask_prioritize expects
    let correct_input = r#"
    {
      "priorities": [
        {
          "task_id": "task_1",
          "priority": "Critical"
        },
        {
          "task_id": "task_2",
          "priority": "High"
        },
        {
          "task_id": "task_3",
          "priority": "Medium"
        }
      ]
    }
    "#;

    let result = translate_llm_output(correct_input, TargetSchema::PriorityResult)?;

    // Should succeed without errors
    assert!(!result.get("error").is_some(), "PriorityResult translation failed: {:?}", result);

    let priorities = result["priorities"].as_array().unwrap();
    assert_eq!(priorities.len(), 3);

    // Verify CRITICAL requirements: priority must be STRING
    for priority_item in priorities {
        assert!(
            priority_item["priority"].is_string(),
            "Priority must be string, got: {:?}",
            priority_item["priority"]
        );
        assert!(
            priority_item["task_id"].is_string(),
            "Task ID must be string, got: {:?}",
            priority_item["task_id"]
        );
    }
}

/// Regression test for intellitask_save complexity enum validation error
#[test]
fn test_regression_complexity_enum_validation() -> Result<()> {
    // These inputs caused enum validation errors in Phase 1
    let invalid_complexity_cases = vec![
        ("High", "Complex"),         // Common incorrect value
        ("Low", "Simple"),           // Common incorrect value
        ("Medium", "Moderate"),      // Sometimes valid, sometimes not
        ("VeryHigh", "VeryComplex"), // Extended value
        ("hard", "Complex"),         // Lowercase
        ("easy", "Simple"),          // Lowercase
    ];

    for (invalid_input, expected_normalized) in invalid_complexity_cases {
        let input = json!({
            "prd_title": "Test Feature",
            "parent_tasks": [{
                "id": "1.0",
                "title": "Test Task",
                "description": "Test description",
                "subtasks": [],
                "dependencies": [],
                "complexity": invalid_input,
                "estimated_hours": 8.0
            }],
            "relevant_files": [],
            "estimated_complexity": invalid_input
        })
        .to_string();

        let result = translate_llm_output(&input, TargetSchema::TaskBreakdown)?;

        assert!(
            !result.get("error").is_some(),
            "Failed to normalize complexity '{}' to '{}': {:?}",
            invalid_input,
            expected_normalized,
            result.get("error")
        );

        // Verify normalization
        let parent_tasks = result["parent_tasks"].as_array().unwrap();
        assert_eq!(parent_tasks[0]["complexity"], expected_normalized);
        assert_eq!(result["estimated_complexity"], expected_normalized);
    }
}

/// Regression test for subtask structure mismatches
#[test]
fn test_regression_subtask_structure_mismatch() -> Result<()> {
    // The real Subtask structure has different fields than assumed in Phase 1
    let correct_subtask_input = r#"
    {
      "subtasks": [
        {
          "id": "1.1",
          "description": "Create user authentication service",
          "acceptance_criteria": [
            "Users can register with email/password",
            "Login validates credentials",
            "JWT tokens are issued correctly"
          ],
          "dependencies": [],
          "files_to_modify": [
            "src/auth/service.rs",
            "src/models/user.rs"
          ],
          "complexity": "Moderate",
          "estimated_hours": 12.0
        }
      ]
    }
    "#;

    let result = translate_llm_output(correct_subtask_input, TargetSchema::SubtaskBreakdown)?;

    assert!(!result.get("error").is_some(), "SubtaskBreakdown translation failed: {:?}", result);

    let subtasks = result["subtasks"].as_array().unwrap();
    assert_eq!(subtasks.len(), 1);

    let subtask = &subtasks[0];
    assert_eq!(subtask["id"], "1.1");
    assert!(subtask["description"].as_str().unwrap().len() > 0);
    assert!(subtask["acceptance_criteria"].as_array().unwrap().len() > 0);
    assert!(subtask["files_to_modify"].as_array().unwrap().len() > 0);
    assert_eq!(subtask["estimated_hours"], 12.0);
}

/// Regression test for FileReference structure differences
#[test]
fn test_regression_file_reference_structure() -> Result<()> {
    // Real FileReference has 'purpose' and 'action', not 'description' and 'confidence'
    let correct_fileref_input = r#"
    {
      "prd_title": "API Feature",
      "parent_tasks": [{
        "id": "1.0",
        "title": "API Implementation",
        "description": "Implement REST API endpoints",
        "subtasks": [],
        "dependencies": [],
        "complexity": "Complex",
        "estimated_hours": 16.0
      }],
      "relevant_files": [
        {
          "path": "src/api/routes.rs",
          "purpose": "Define HTTP routes and handlers",
          "action": "Create"
        },
        {
          "path": "src/api/models.rs",
          "purpose": "Define request/response data structures",
          "action": "Add"
        }
      ],
      "estimated_complexity": "Complex"
    }
    "#;

    let result = translate_llm_output(correct_fileref_input, TargetSchema::TaskBreakdown)?;

    assert!(!result.get("error").is_some(), "FileReference translation failed: {:?}", result);

    let relevant_files = result["relevant_files"].as_array().unwrap();
    assert_eq!(relevant_files.len(), 2);

    // Verify FileAction alias normalization
    for file_ref in relevant_files {
        assert!(file_ref.get("path").and_then(Value::as_str).is_some());
        assert!(file_ref.get("purpose").and_then(Value::as_str).is_some());
        assert!(file_ref.get("action").and_then(Value::as_str).is_some());
    }

    // "Add" should be normalized to "Modify2" alias
    assert_eq!(relevant_files[1]["action"], "Modify2");
}

/// Regression test for sequential step status validation
#[test]
fn test_regression_sequential_step_status_validation() -> Result<()> {
    let invalid_status_cases = vec!["invalid_status", "processing", "waiting", "unknown", ""];

    for invalid_status in invalid_status_cases {
        let input = json!({
            "step_number": 1,
            "thought": "Test thought process",
            "reasoning": "Test reasoning",
            "status": invalid_status
        })
        .to_string();

        let result = translate_llm_output(&input, TargetSchema::SequentialStep)?;

        assert!(
            !result.get("error").is_some(),
            "SequentialStep translation failed for status '{}': {:?}",
            invalid_status,
            result.get("error")
        );

        // Invalid status should be normalized to "pending"
        assert_eq!(result["status"], "pending");
    }
}

/// Regression test for numeric coercion edge cases
#[test]
fn test_regression_numeric_coercion_edge_cases() -> Result<()> {
    // Test edge cases for numeric string coercion
    let edge_cases = vec![
        ("estimated_hours", "4.5", 4.5),
        ("estimated_hours", "8", 8.0),
        ("estimated_hours", "0", 0.0),
        ("task_id", 123, "123"),
        ("task_id", 0, "0"),
        ("step_number", "5", 5),
    ];

    for (field_name, input_value, expected_output) in edge_cases {
        let input = if field_name == "task_id" {
            json!({
                "priorities": [{
                    "task_id": if input_value.is_number() {
                        serde_json::Value::Number(input_value.parse::<i64>().unwrap().into())
                    } else {
                        serde_json::Value::String(input_value.to_string())
                    },
                    "priority": "High"
                }]
            })
            .to_string()
        } else if field_name == "step_number" {
            json!({
                "step_number": if input_value.is_number() {
                    serde_json::Value::Number(input_value.parse::<i64>().unwrap().into())
                } else {
                    serde_json::Value::String(input_value.to_string())
                },
                "thought": "Test",
                "reasoning": "Test"
            })
            .to_string()
        } else {
            json!({
                "subtasks": [{
                    "id": "1.1",
                    "description": "Test",
                    "complexity": "Simple",
                    "estimated_hours": if input_value.is_number() {
                        serde_json::Value::Number(input_value.parse::<f64>().unwrap().into())
                    } else {
                        serde_json::Value::String(input_value.to_string())
                    }
                }]
            })
            .to_string()
        };

        let target_schema = match field_name {
            "task_id" => TargetSchema::PriorityResult,
            "step_number" => TargetSchema::SequentialStep,
            _ => TargetSchema::SubtaskBreakdown,
        };

        let result = translate_llm_output(&input, target_schema)?;

        assert!(
            !result.get("error").is_some(),
            "Numeric coercion failed for {}='{}': {:?}",
            field_name,
            input_value,
            result.get("error")
        );

        // Verify coercion result
        match target_schema {
            TargetSchema::PriorityResult => {
                let priorities = result["priorities"].as_array().unwrap();
                if let Some(expected_str) = expected_output.as_str() {
                    assert_eq!(priorities[0]["task_id"], expected_str);
                }
            }
            TargetSchema::SequentialStep => {
                if let Some(expected_num) = expected_output.as_i64() {
                    assert_eq!(result[field_name], expected_num);
                }
            }
            TargetSchema::SubtaskBreakdown => {
                let subtasks = result["subtasks"].as_array().unwrap();
                if let Some(expected_num) = expected_output.as_f64() {
                    assert_eq!(subtasks[0][field_name].as_f64().unwrap(), expected_num);
                }
            }
            _ => {}
        }
    }
}

/// Regression test for JSON extraction from messy LLM output
#[test]
fn test_regression_messy_llm_output_extraction() -> Result<()> {
    // Real LLM output often contains prose, formatting issues, etc.
    let messy_inputs = vec![
        // Markdown with extra text
        r#"
        Here's my analysis of the requirements:

        ```json
        {
          "prd_title": "User Management System",
          "parent_tasks": [],
          "relevant_files": [],
          "estimated_complexity": "Moderate"
        }
        ```

        This should cover all the essential requirements mentioned in the PRD.
        "#,
        // JSON with comments and trailing commas
        r#"
        {
          // Product requirements breakdown
          "prd_title": "Mobile App Backend",
          "parent_tasks": [
            {
              "id": "1.0",
              "title": "API Design",
              "description": "Design REST API endpoints",
              "subtasks": [],
              "dependencies": [],
              "complexity": "Moderate",
              "estimated_hours": 12.0,
            }
          ],
          "relevant_files": [],
          "estimated_complexity": "Complex",
        }
        "#,
        // JSON embedded in prose
        r#"
        The task breakdown is as follows: {"prd_title": "Database Migration", "parent_tasks": [], "relevant_files": [], "estimated_complexity": "Simple"}.
        This migration will handle the user table updates.
        "#,
    ];

    for (i, messy_input) in messy_inputs.iter().enumerate() {
        let result = translate_llm_output(messy_input, TargetSchema::TaskBreakdown)?;

        assert!(
            !result.get("error").is_some(),
            "Failed to extract JSON from messy input {}: {:?}",
            i + 1,
            result.get("error")
        );

        assert_eq!(result.get("prd_title").and_then(Value::as_str), Some("User Management System"));
    }
}
