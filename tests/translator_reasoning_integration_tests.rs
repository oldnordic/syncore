//! Integration tests for translator and reasoning engine compatibility
//!
//! These tests ensure that the modernized translator integrates correctly
//! with the reasoning engine contracts and maintains backward compatibility.

use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use syncore::mcp_tools::translator::{
    translate_llm_output, TargetSchema,
};
use syncore::mcp_server::reasoning::{parse_unified_request, RequestParameters, RequestType};

#[test]
fn test_task_breakdown_reasoning_compatibility() -> Result<()> {
    // Test that TaskBreakdown output is compatible with reasoning engine

    let input = json!({
        "prd_title": "Test Feature",
        "parent_tasks": [],
        "relevant_files": ["src/lib.rs"],
        "estimated_complexity": "Simple"
    });

    let result = translate_llm_output(
        &serde_json::to_string(&input)?,
        TargetSchema::TaskBreakdown
    )?;

    // Verify schema versioning metadata
    assert!(result.get("_schema_version").is_some(), "Missing schema version");
    assert!(result.get("_contract_version").is_some(), "Missing contract version");

    // Verify reasoning engine can parse the result as valid parameters
    let mut raw_params = result.as_object()
        .ok_or_else(|| anyhow!("Result is not an object"))?
        .clone();

    // Add required query parameter for reasoning engine
    raw_params.insert("query".to_string(), json!("task breakdown analysis"));

    let request = parse_unified_request(raw_params, RequestType::Query, None)?;

    // Should parse successfully for reasoning engine consumption
    match &request.parameters {
        RequestParameters::Query { .. } |
        RequestParameters::Fusion { .. } => {
            // Expected: should be query or fusion type
        }
        _ => panic!("Should be compatible with query/fusion reasoning"),
    }

    Ok(())
}

#[test]
fn test_priority_result_reasoning_compatibility() -> Result<()> {
    // Test that PriorityResult output is compatible with reasoning engine

    let input = json!({
        "priorities": [
            {
                "task_id": "task_123",
                "priority": "High",
                "reasoning": "Critical path dependency"
            }
        ]
    });

    let result = translate_llm_output(
        &serde_json::to_string(&input)?,
        TargetSchema::PriorityResult
    )?;

    // Verify schema versioning metadata
    assert!(result.get("_schema_version").is_some(), "Missing schema version");
    assert!(result.get("_contract_version").is_some(), "Missing contract version");

    // Verify the structure matches reasoning engine expectations
    let priorities = result.get("priorities")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("Missing or invalid priorities array"))?;

    assert!(!priorities.is_empty(), "Should have at least one priority item");

    // Verify each priority item has required fields
    for priority in priorities {
        assert!(priority.get("task_id").is_some(), "Missing task_id in priority item");
        assert!(priority.get("priority").is_some(), "Missing priority in priority item");
    }

    Ok(())
}

#[test]
fn test_sequential_step_reasoning_compatibility() -> Result<()> {
    // Test that SequentialStep output is compatible with reasoning engine

    let input = json!({
        "step_number": 1,
        "thought": "I should analyze the current codebase structure",
        "reasoning": "Understanding the codebase is essential before making changes",
        "confidence": 0.8
    });

    let result = translate_llm_output(
        &serde_json::to_string(&input)?,
        TargetSchema::SequentialStep
    )?;

    // Verify schema versioning metadata
    assert!(result.get("_schema_version").is_some(), "Missing schema version");
    assert!(result.get("_contract_version").is_some(), "Missing contract version");

    // Verify reasoning engine can process the sequential step
    let step_number = result.get("step_number")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("Missing or invalid step_number"))?;

    let thought = result.get("thought")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Missing or invalid thought"))?;

    let reasoning = result.get("reasoning")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Missing or invalid reasoning"))?;

    assert_eq!(step_number, 1, "Step number should be preserved");
    assert!(!thought.is_empty(), "Thought should not be empty");
    assert!(!reasoning.is_empty(), "Reasoning should not be empty");

    Ok(())
}

#[test]
fn test_contract_metadata_propagation() -> Result<()> {
    // Test that contract metadata is properly propagated through all schemas

    let schemas = vec![
        (TargetSchema::TaskBreakdown, json!({
            "prd_title": "Test",
            "parent_tasks": [],
            "relevant_files": [],
            "estimated_complexity": "Simple"
        })),
        (TargetSchema::PriorityResult, json!({
            "priorities": [{"task_id": "123", "priority": "High"}]
        })),
        (TargetSchema::SequentialStep, json!({
            "step_number": 1,
            "thought": "test",
            "reasoning": "test"
        })),
    ];

    for (schema, input) in schemas {
        let result = translate_llm_output(&serde_json::to_string(&input)?, schema)?;

        // All schemas should include contract metadata
        assert!(result.get("_schema_version").is_some(),
                "Schema {:?} missing schema version", schema);
        assert!(result.get("_schema_type").is_some(),
                "Schema {:?} missing schema type", schema);
        assert!(result.get("_contract_version").is_some(),
                "Schema {:?} missing contract version", schema);

        // Schema type should match the enum variant name
        let schema_type = result.get("_schema_type")
            .and_then(Value::as_str)
            .expect("Schema type should be a string");

        assert!(!schema_type.is_empty(), "Schema type should not be empty");
    }

    Ok(())
}

#[test]
fn test_reasoning_engine_request_parsing() -> Result<()> {
    // Test that translated output can be parsed by reasoning engine requests

    let input = json!({
        "prd_title": "Feature Implementation",
        "parent_tasks": [],
        "relevant_files": ["src/main.rs"],
        "estimated_complexity": "Medium"
    });

    let translated = translate_llm_output(
        &serde_json::to_string(&input)?,
        TargetSchema::TaskBreakdown
    )?;

    // Convert to raw parameters for reasoning engine
    let mut raw_params = translated.as_object()
        .ok_or_else(|| anyhow!("Translated result is not an object"))?
        .clone();

    // Add query-specific parameters
    raw_params.insert("query".to_string(), json!("implementation plan"));
    raw_params.insert("top_k".to_string(), json!(5));

    // Try to parse as different request types
    let query_request = parse_unified_request(raw_params.clone(), RequestType::Query, None)?;
    match &query_request.parameters {
        RequestParameters::Query { .. } => {
            // Expected: should be query type
        }
        _ => panic!("Should parse as query request"),
    }

    let fusion_request = parse_unified_request(raw_params, RequestType::Fusion, None)?;
    match &fusion_request.parameters {
        RequestParameters::Fusion { .. } => {
            // Expected: should be fusion type
        }
        _ => panic!("Should parse as fusion request"),
    }

    Ok(())
}

#[test]
fn test_schema_version_consistency() -> Result<()> {
    // Test that all active schemas use consistent versioning

    let schemas = vec![
        TargetSchema::TaskBreakdown,
        TargetSchema::PriorityResult,
        TargetSchema::SequentialStep,
    ];

    let mut versions = Vec::new();

    for schema in schemas {
        let input = match schema {
            TargetSchema::TaskBreakdown => json!({
                "prd_title": "Test",
                "parent_tasks": [],
                "relevant_files": [],
                "estimated_complexity": "Simple"
            }),
            TargetSchema::PriorityResult => json!({
                "priorities": [{"task_id": "123", "priority": "High"}]
            }),
            TargetSchema::SequentialStep => json!({
                "step_number": 1,
                "thought": "test",
                "reasoning": "test"
            }),
        };

        let result = translate_llm_output(&serde_json::to_string(&input)?, schema)?;

        let version = result.get("_schema_version")
            .and_then(Value::as_str)
            .expect("Schema version should be a string")
            .to_string();

        versions.push(version);
    }

    // All schemas should use the same version
    let first_version = &versions[0];
    for version in &versions {
        assert_eq!(version, first_version, "Schema versions should be consistent");
    }

    Ok(())
}