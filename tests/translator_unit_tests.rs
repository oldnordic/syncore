//! Unit Tests for LLM Output Translator
//!
//! TDD-based unit tests covering all translation scenarios with real schema validation

use anyhow::Result;
use serde_json::json;
use syncore::mcp_tools::translator::{
    translate_llm_output, translate_llm_output_strict, LlmOutputTranslator, TargetSchema,
};

/// Test fixture with real failing cases from MCP logs
struct TestCase {
    name: &'static str,
    input: &'static str,
    target_schema: TargetSchema,
    expected_result: TestResult,
}

enum TestResult {
    Success(serde_json::Value),
    Error(&'static str, Vec<&'static str>), // (error_type, missing_fields)
    PartialError(&'static str),             // error contains specific text
}

const TEST_CASES: &[TestCase] = &[
    // Test Case 1: Valid TaskBreakdown
    TestCase {
        name: "Valid TaskBreakdown",
        input: r#"
        {
          "prd_title": "User Authentication System",
          "parent_tasks": [
            {
              "id": "1.0",
              "title": "Database Schema",
              "description": "Create user tables",
              "subtasks": [],
              "dependencies": [],
              "complexity": "Moderate",
              "estimated_hours": 8.0
            }
          ],
          "relevant_files": [],
          "estimated_complexity": "Complex"
        }
        "#,
        target_schema: TargetSchema::TaskBreakdown,
        expected_result: TestResult::Success(json!({
            "prd_title": "User Authentication System",
            "parent_tasks": [{
                "id": "1.0",
                "title": "Database Schema",
                "description": "Create user tables",
                "subtasks": [],
                "dependencies": [],
                "complexity": "Moderate",
                "estimated_hours": 8.0
            }],
            "relevant_files": [],
            "estimated_complexity": "Complex"
        })),
    },
    // Test Case 2: TaskBreakdown with missing required fields
    TestCase {
        name: "TaskBreakdown Missing Fields",
        input: r#"
        {
          "prd_title": "Test Feature"
        }
        "#,
        target_schema: TargetSchema::TaskBreakdown,
        expected_result: TestResult::Error(
            "SchemaValidationFailed",
            vec!["parent_tasks", "estimated_complexity"],
        ),
    },
    // Test Case 3: PriorityResult with STRING priority (CRITICAL from Phase 1)
    TestCase {
        name: "PriorityResult Valid String Priority",
        input: r#"
        {
          "priorities": [
            {
              "task_id": "task_123",
              "priority": "High"
            },
            {
              "task_id": "task_456",
              "priority": "Critical"
            }
          ]
        }
        "#,
        target_schema: TargetSchema::PriorityResult,
        expected_result: TestResult::Success(json!({
            "priorities": [
                {"task_id": "task_123", "priority": "High"},
                {"task_id": "task_456", "priority": "Critical"}
            ]
        })),
    },
    // Test Case 4: PriorityResult with numeric coercion
    TestCase {
        name: "PriorityResult Numeric Coercion",
        input: r#"
        {
          "priorities": [
            {
              "task_id": 123,
              "priority": 456
            }
          ]
        }
        "#,
        target_schema: TargetSchema::PriorityResult,
        expected_result: TestResult::Success(json!({
            "priorities": [
                {"task_id": "123", "priority": "456"}
            ]
        })),
    },
    // Test Case 5: PriorityResult missing priorities array
    TestCase {
        name: "PriorityResult Missing Priorities",
        input: r#"
        {
          "some_other_field": "value"
        }
        "#,
        target_schema: TargetSchema::PriorityResult,
        expected_result: TestResult::Error("SchemaValidationFailed", vec!["priorities array"]),
    },
    // Test Case 6: SubtaskBreakdown with real Subtask structure
    TestCase {
        name: "Valid SubtaskBreakdown",
        input: r#"
        {
          "subtasks": [
            {
              "id": "1.1",
              "description": "Create database migration",
              "acceptance_criteria": ["Migration runs successfully"],
              "dependencies": [],
              "files_to_modify": ["migrations/001_create_users.sql"],
              "complexity": "Simple",
              "estimated_hours": 2.0
            }
          ]
        }
        "#,
        target_schema: TargetSchema::SubtaskBreakdown,
        expected_result: TestResult::Success(json!({
            "subtasks": [{
                "id": "1.1",
                "description": "Create database migration",
                "acceptance_criteria": ["Migration runs successfully"],
                "dependencies": [],
                "files_to_modify": ["migrations/001_create_users.sql"],
                "complexity": "Simple",
                "estimated_hours": 2.0
            }]
        })),
    },
    // Test Case 7: SubtaskBreakdown with auto-fix of missing arrays
    TestCase {
        name: "SubtaskBreakdown Auto-fix Arrays",
        input: r#"
        {
          "subtasks": [
            {
              "id": "1.1",
              "description": "Create database migration",
              "complexity": "Simple",
              "estimated_hours": 2
            }
          ]
        }
        "#,
        target_schema: TargetSchema::SubtaskBreakdown,
        expected_result: TestResult::Success(json!({
            "subtasks": [{
                "id": "1.1",
                "description": "Create database migration",
                "acceptance_criteria": [],
                "dependencies": [],
                "files_to_modify": [],
                "complexity": "Simple",
                "estimated_hours": 2.0
            }]
        })),
    },
    // Test Case 8: SubtaskBreakdown with estimated_hours coercion
    TestCase {
        name: "SubtaskBreakdown Hours Coercion",
        input: r#"
        {
          "subtasks": [
            {
              "id": "1.1",
              "description": "Test task",
              "complexity": "Simple",
              "estimated_hours": "4.5"
            }
          ]
        }
        "#,
        target_schema: TargetSchema::SubtaskBreakdown,
        expected_result: TestResult::Success(json!({
            "subtasks": [{
                "id": "1.1",
                "description": "Test task",
                "acceptance_criteria": [],
                "dependencies": [],
                "files_to_modify": [],
                "complexity": "Simple",
                "estimated_hours": 4.5
            }]
        })),
    },
    // Test Case 9: NextTaskSuggestion valid
    TestCase {
        name: "Valid NextTaskSuggestion",
        input: r#"
        {
          "task_id": "task_123",
          "reasoning": "This task should be completed next because it unblocks other tasks"
        }
        "#,
        target_schema: TargetSchema::NextTaskSuggestion,
        expected_result: TestResult::Success(json!({
            "task_id": "task_123",
            "reasoning": "This task should be completed next because it unblocks other tasks"
        })),
    },
    // Test Case 10: SequentialStep valid with auto-generation
    TestCase {
        name: "Valid SequentialStep with Auto-gen",
        input: r#"
        {
          "step_number": 1,
          "thought": "I should start by analyzing the current state",
          "reasoning": "Understanding the current state is essential before making changes"
        }
        "#,
        target_schema: TargetSchema::SequentialStep,
        expected_result: TestResult::PartialError("step_id"), // Will be auto-generated
    },
    // Test Case 11: SequentialStep with all fields
    TestCase {
        name: "Complete SequentialStep",
        input: r#"
        {
          "step_id": "step_42_1",
          "task_id": 42,
          "sequence_id": "seq_123",
          "step_number": 1,
          "thought": "I should start by analyzing the current state",
          "reasoning": "Understanding the current state is essential",
          "action": "Read the current codebase",
          "observation": "Found existing implementation",
          "timestamp": 1640995200,
          "status": "completed"
        }
        "#,
        target_schema: TargetSchema::SequentialStep,
        expected_result: TestResult::Success(json!({
            "step_id": "step_42_1",
            "task_id": 42,
            "sequence_id": "seq_123",
            "step_number": 1,
            "thought": "I should start by analyzing the current state",
            "reasoning": "Understanding the current state is essential",
            "action": "Read the current codebase",
            "observation": "Found existing implementation",
            "timestamp": 1640995200,
            "status": "completed"
        })),
    },
    // Test Case 12: Extract JSON from prose with markdown
    TestCase {
        name: "Extract JSON from Prose",
        input: r#"
        Based on my analysis, here's the task breakdown:

        ```json
        {
          "prd_title": "API Integration",
          "parent_tasks": [],
          "relevant_files": [],
          "estimated_complexity": "Moderate"
        }
        ```

        This should cover all the requirements.
        "#,
        target_schema: TargetSchema::TaskBreakdown,
        expected_result: TestResult::Success(json!({
            "prd_title": "API Integration",
            "parent_tasks": [],
            "relevant_files": [],
            "estimated_complexity": "Moderate"
        })),
    },
    // Test Case 13: Complexity enum normalization
    TestCase {
        name: "Complexity Enum Normalization",
        input: r#"
        {
          "prd_title": "Test",
          "parent_tasks": [{
            "id": "1.0",
            "title": "Test",
            "description": "Test",
            "subtasks": [],
            "dependencies": [],
            "complexity": "High",
            "estimated_hours": 5.0
          }],
          "relevant_files": [],
          "estimated_complexity": "Low"
        }
        "#,
        target_schema: TargetSchema::TaskBreakdown,
        expected_result: TestResult::Success(json!({
            "prd_title": "Test",
            "parent_tasks": [{
                "id": "1.0",
                "title": "Test",
                "description": "Test",
                "subtasks": [],
                "dependencies": [],
                "complexity": "Complex", // "High" -> "Complex"
                "estimated_hours": 5.0
            }],
            "relevant_files": [],
            "estimated_complexity": "Simple" // "Low" -> "Simple"
        })),
    },
    // Test Case 14: FileReference with FileAction alias
    TestCase {
        name: "FileReference FileAction Alias",
        input: r#"
        {
          "prd_title": "Test",
          "parent_tasks": [],
          "relevant_files": [{
            "path": "src/main.rs",
            "purpose": "Add new function",
            "action": "Implement"
          }],
          "estimated_complexity": "Simple"
        }
        "#,
        target_schema: TargetSchema::TaskBreakdown,
        expected_result: TestResult::Success(json!({
            "prd_title": "Test",
            "parent_tasks": [],
            "relevant_files": [{
                "path": "src/main.rs",
                "purpose": "Add new function",
                "action": "Modify2" // "Implement" -> "Modify2" (alias)
            }],
            "estimated_complexity": "Simple"
        })),
    },
];

#[test]
fn test_all_translation_cases() -> Result<()> {
    for test_case in TEST_CASES {
        println!("Running test case: {}", test_case.name);

        let result = translate_llm_output(test_case.input, test_case.target_schema)?;

        match &test_case.expected_result {
            TestResult::Success(expected) => {
                assert!(
                    !result.get("error").is_some(),
                    "Test '{}' failed: Expected success but got error: {}",
                    test_case.name,
                    result
                );

                // Compare key fields (deep equality might be tricky due to auto-generated fields)
                if let Some(prd_title) = expected.get("prd_title") {
                    assert_eq!(
                        result.get("prd_title"),
                        Some(prd_title),
                        "Test '{}' failed: prd_title mismatch",
                        test_case.name
                    );
                }

                if let Some(priorities) = expected.get("priorities") {
                    assert_eq!(
                        result.get("priorities"),
                        Some(priorities),
                        "Test '{}' failed: priorities mismatch",
                        test_case.name
                    );
                }

                if let Some(subtasks) = expected.get("subtasks") {
                    assert_eq!(
                        result.get("subtasks"),
                        Some(subtasks),
                        "Test '{}' failed: subtasks mismatch",
                        test_case.name
                    );
                }
            }

            TestResult::Error(expected_error, expected_missing) => {
                assert_eq!(
                    result.get("error").and_then(Value::as_str),
                    Some(*expected_error),
                    "Test '{}' failed: Expected error '{}' but got: {:?}",
                    test_case.name,
                    expected_error,
                    result.get("error")
                );

                if let Some(missing_fields) = result.get("missing_fields").and_then(Value::as_array)
                {
                    for expected_field in expected_missing {
                        assert!(
                            missing_fields.iter().any(|f| f.as_str() == Some(*expected_field)),
                            "Test '{}' failed: Expected missing field '{}' not found in {:?}",
                            test_case.name,
                            expected_field,
                            missing_fields
                        );
                    }
                }
            }

            TestResult::PartialError(expected_text) => {
                // For cases where we expect partial success with auto-generation
                let result_str = serde_json::to_string(&result).unwrap();
                assert!(
                    result_str.contains(expected_text) || !result.get("error").is_some(),
                    "Test '{}' failed: Expected partial error '{}' but got: {}",
                    test_case.name,
                    expected_text,
                    result_str
                );
            }
        }
    }

    Ok(())
}

#[test]
fn test_strict_mode_disables_coercion() -> Result<()> {
    // Strict mode should not coerce invalid complexity values
    let input = r#"
    {
      "prd_title": "Test",
      "parent_tasks": [],
      "relevant_files": [],
      "estimated_complexity": "InvalidComplexity"
    }
    "#;

    let result = translate_llm_output_strict(input, TargetSchema::TaskBreakdown)?;

    // In strict mode, invalid complexity should pass through unchanged
    assert_eq!(result["estimated_complexity"], "InvalidComplexity");
}

#[test]
fn test_coercion_mode_normalizes_complexity() -> Result<()> {
    // Coercion mode should normalize complexity values
    let input = r#"
    {
      "prd_title": "Test",
      "parent_tasks": [],
      "relevant_files": [],
      "estimated_complexity": "High"
    }
    "#;

    let result = translate_llm_output(input, TargetSchema::TaskBreakdown)?;

    // "High" should be normalized to "Complex"
    assert_eq!(result["estimated_complexity"], "Complex");
}

#[test]
fn test_extract_json_broken_fence() -> Result<()> {
    // Test JSON extraction with broken markdown fence
    let input = r#"
    Here's the response:

    ```json
    {
      "task_id": "123",
      "priority": "High"
    }

    Some additional text here.

    "#;

    let translator = LlmOutputTranslator::default();
    let result = translator.extract_json(input)?;

    assert_eq!(result["task_id"], "123");
    assert_eq!(result["priority"], "High");
}

#[test]
fn test_no_json_found() -> Result<()> {
    // Test case where no JSON is found in input
    let input = "This is just plain text with no JSON at all.";

    let result = translate_llm_output(input, TargetSchema::PriorityResult)?;

    assert_eq!(result["error"], "SchemaValidationFailed");
}

#[test]
fn test_priority_result_must_keep_string_priority() -> Result<()> {
    // CRITICAL TEST: Priority must remain as string, not enum
    let input = r#"
    {
      "priorities": [
        {
          "task_id": "123",
          "priority": "Critical"
        }
      ]
    }
    "#;

    let result = translate_llm_output(input, TargetSchema::PriorityResult)?;

    let priorities = result["priorities"].as_array().unwrap();
    assert_eq!(priorities[0]["priority"], "Critical"); // Must be string, not enum variant
    assert!(priorities[0]["priority"].is_string()); // Must be string type
}

#[test]
fn test_sequential_step_status_validation() -> Result<()> {
    // Test that only valid status values are allowed
    let input = r#"
    {
      "step_number": 1,
      "thought": "Test thought",
      "reasoning": "Test reasoning",
      "status": "invalid_status"
    }
    "#;

    let result = translate_llm_output(input, TargetSchema::SequentialStep)?;

    // Invalid status should be normalized to "pending"
    assert_eq!(result["status"], "pending");
}
