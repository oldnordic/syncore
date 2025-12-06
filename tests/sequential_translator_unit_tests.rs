//! Unit Tests for Sequential Reasoning LLM Output Translator
//!
//! Tests SequentialStep normalization and auto-generation for missing fields.

use anyhow::Result;
use serde_json::{json, Value};
use syncore::mcp_tools::translator::{translate_llm_output, TargetSchema};

#[test]
fn test_valid_sequential_step_passthrough() -> Result<()> {
    // Valid SequentialStep should pass through unchanged
    let valid_input = json!({
        "step_number": 1,
        "thought": "I need to understand the current codebase structure",
        "reasoning": "Understanding the codebase is essential before making changes",
        "action": "Read the main files",
        "observation": "Found existing structure"
    })
    .to_string();

    let result = translate_llm_output(&valid_input, TargetSchema::SequentialStep)?;

    // Should succeed without errors
    assert!(
        !result.get("error").is_some(),
        "Valid input should not cause errors: {:?}",
        result.get("error")
    );

    // Should preserve provided fields
    assert_eq!(result["step_number"], 1);
    assert!(result["thought"].as_str().unwrap().contains("codebase structure"));
    assert!(result["reasoning"].as_str().unwrap().contains("essential"));
    assert_eq!(result["action"], "Read the main files");
    assert_eq!(result["observation"], "Found existing structure");

    // Should have auto-generated fields
    assert!(result.get("step_id").is_some());
    assert!(result.get("timestamp").is_some());
    assert_eq!(result["status"], "pending"); // Default status
    Ok(())
}

#[test]
fn test_sequential_step_auto_generation() -> Result<()> {
    // Minimal SequentialStep with only required fields
    let minimal_input = json!({
        "step_number": 1,
        "thought": "I need to understand the current codebase structure",
        "reasoning": "Understanding the codebase is essential before making changes"
    })
    .to_string();

    let result = translate_llm_output(&minimal_input, TargetSchema::SequentialStep)?;

    // Should succeed and auto-generate missing fields
    assert!(
        !result.get("error").is_some(),
        "Sequential auto-generation failed: {:?}",
        result.get("error")
    );

    // Should have auto-generated fields
    assert!(result.get("step_id").is_some());
    assert!(result.get("timestamp").is_some());
    assert_eq!(result["status"], "pending"); // Default status

    // Should preserve provided fields
    assert_eq!(result["step_number"], 1);
    assert!(result["thought"].as_str().unwrap().contains("codebase structure"));
    assert!(result["reasoning"].as_str().unwrap().contains("essential"));
    Ok(())
}

#[test]
fn test_sequential_step_missing_required_fields_error() -> Result<()> {
    // Missing required fields should produce error
    let incomplete_input = json!({
        "action": "Read files",
        "observation": "Found structure"
        // Missing step_number, thought, reasoning
    })
    .to_string();

    let result = translate_llm_output(&incomplete_input, TargetSchema::SequentialStep)?;

    // Should produce structured error
    assert_eq!(result["error"], "SchemaValidationFailed");

    let missing_fields = result["missing_fields"].as_array().unwrap();
    assert!(missing_fields
        .iter()
        .any(|f| f.as_str() == Some("Missing required field: step_number")));
    assert!(missing_fields.iter().any(|f| f.as_str() == Some("Missing required field: thought")));
    assert!(missing_fields.iter().any(|f| f.as_str() == Some("Missing required field: reasoning")));
    Ok(())
}

#[test]
fn test_sequential_step_invalid_status_correction() -> Result<()> {
    // Invalid status should get corrected to "pending"
    let invalid_status_input = json!({
        "step_number": 1,
        "thought": "Test thought",
        "reasoning": "Test reasoning",
        "status": "invalid_status_value"  // Should be corrected
    })
    .to_string();

    let result = translate_llm_output(&invalid_status_input, TargetSchema::SequentialStep)?;

    // Should succeed and correct status
    assert!(!result.get("error").is_some(), "Status correction failed: {:?}", result.get("error"));

    // Status should be corrected to "pending"
    assert_eq!(result["status"], "pending");
    Ok(())
}

#[test]
fn test_sequential_step_step_id_generation() -> Result<()> {
    // Test step_id auto-generation with task_id
    let input_with_task_id = json!({
        "step_number": 1,
        "thought": "Test thought",
        "reasoning": "Test reasoning",
        "task_id": 42
    })
    .to_string();

    let result = translate_llm_output(&input_with_task_id, TargetSchema::SequentialStep)?;

    // Should succeed and generate step_id
    assert!(!result.get("error").is_some(), "Step ID generation failed: {:?}", result.get("error"));

    // Should have generated step_id based on task_id and step_number
    let step_id = result["step_id"].as_str().unwrap();
    assert_eq!(step_id, "step_42_1");
    Ok(())
}

#[test]
fn test_sequential_step_step_id_generation_no_task_id() -> Result<()> {
    // Test step_id auto-generation without task_id
    let input_no_task_id = json!({
        "step_number": 5,
        "thought": "Test thought",
        "reasoning": "Test reasoning"
    })
    .to_string();

    let result = translate_llm_output(&input_no_task_id, TargetSchema::SequentialStep)?;

    // Should succeed and generate step_id
    assert!(
        !result.get("error").is_some(),
        "Step ID generation without task_id failed: {:?}",
        result.get("error")
    );

    // Should have generated step_id with task_id = 0
    let step_id = result["step_id"].as_str().unwrap();
    assert_eq!(step_id, "step_0_5");
    Ok(())
}

#[test]
fn test_sequential_step_timestamp_generation() -> Result<()> {
    // Test timestamp auto-generation
    let basic_input = json!({
        "step_number": 1,
        "thought": "Test thought",
        "reasoning": "Test reasoning"
    })
    .to_string();

    let result = translate_llm_output(&basic_input, TargetSchema::SequentialStep)?;

    // Should succeed and generate timestamp
    assert!(
        !result.get("error").is_some(),
        "Timestamp generation failed: {:?}",
        result.get("error")
    );

    // Should have timestamp as number
    let timestamp = result["timestamp"].as_u64().unwrap();
    assert!(timestamp > 0, "Timestamp should be positive");

    // Timestamp should be recent (within last minute)
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

    assert!(now - timestamp < 60, "Timestamp should be recent");
    Ok(())
}

#[test]
fn test_sequential_step_valid_statuses_passthrough() -> Result<()> {
    // Test that valid statuses pass through unchanged
    let valid_statuses = ["pending", "executing", "completed", "failed"];

    for status in valid_statuses.iter() {
        let input = json!({
            "step_number": 1,
            "thought": "Test thought",
            "reasoning": "Test reasoning",
            "status": status
        })
        .to_string();

        let result = translate_llm_output(&input, TargetSchema::SequentialStep)?;

        assert!(
            !result.get("error").is_some(),
            "Valid status '{}' should pass through: {:?}",
            status,
            result.get("error")
        );

        assert_eq!(result["status"].as_str().unwrap(), *status);
    }
    Ok(())
}

#[test]
fn test_sequential_step_markdown_extraction() -> Result<()> {
    // Test that translator can extract JSON from markdown
    let markdown_input = r#"
Here's my thinking process:

```json
{
  "step_number": 1,
  "thought": "I need to understand the codebase",
  "reasoning": "First step is always understanding",
  "action": "Read the files"
}
```

This should be the first step.
    "#;

    let result = translate_llm_output(markdown_input, TargetSchema::SequentialStep)?;

    // Should succeed and extract JSON from markdown
    assert!(
        !result.get("error").is_some(),
        "Markdown extraction failed: {:?}",
        result.get("error")
    );

    // Should have extracted the JSON content
    assert_eq!(result["step_number"], 1);
    assert_eq!(result["thought"], "I need to understand the codebase");
    assert_eq!(result["reasoning"], "First step is always understanding");
    assert_eq!(result["action"], "Read the files");

    // Should have auto-generated fields
    assert!(result.get("step_id").is_some());
    assert!(result.get("timestamp").is_some());
    assert_eq!(result["status"], "pending");
    Ok(())
}

#[test]
fn test_sequential_step_malformed_json_error() {
    // Completely malformed JSON should fail gracefully with Result::Err
    let malformed_json = r#"
{
  "step_number": "not a number",
  "thought": "This is totally broken
  "reasoning": "unclosed string
  "invalid_field": [broken array
    "#;

    let result = translate_llm_output(malformed_json, TargetSchema::SequentialStep);

    // Should return Result::Err (parsing or validation error), not panic
    assert!(result.is_err(), "Malformed JSON should produce Result::Err, not Ok");
}
