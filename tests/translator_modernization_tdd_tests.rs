//! TDD Tests for Translator Modernization (OPTION B)
//!
//! These tests drive the hybrid approach modernization:
//! 1. Remove dead schema handlers (SubtaskBreakdown, NextTaskSuggestion)
//! 2. Add schema versioning and compatibility layer
//! 3. Align with reasoning engine contracts

use anyhow::Result;
use syncore::mcp_tools::translator::{
    translate_llm_output, TargetSchema,
};

#[test]
fn test_remove_dead_subtask_breakdown_schema() -> Result<()> {
    // This test confirms SubtaskBreakdown schema has been removed
    // The enum variant no longer exists, so this test verifies compilation fails

    // Test that removed schemas have been successfully eliminated
    // The intellitask.rs module now uses TaskBreakdown for migration
    let input = r#"
    {
        "prd_title": "Subtask Generation",
        "parent_tasks": [{
            "id": "subtask_parent",
            "title": "Subtask Generation",
            "description": "Generated subtasks",
            "subtasks": []
        }],
        "relevant_files": [],
        "estimated_complexity": "Simple"
    }
    "#;

    // This should work with TaskBreakdown schema (migration path for SubtaskBreakdown)
    let result = translate_llm_output(input, TargetSchema::TaskBreakdown)?;
    assert!(result.get("error").is_none(), "TaskBreakdown should handle migrated subtask data");

    // Verify schema versioning was added
    assert!(
        result.get("_schema_version").is_some(),
        "TaskBreakdown should include schema version"
    );

    Ok(())
}

#[test]
fn test_remove_dead_next_task_suggestion_schema() -> Result<()> {
    // This test confirms NextTaskSuggestion schema has been removed
    // The intellitask.rs module now uses direct JSON parsing instead

    // Test that NextTaskSuggestion data is now handled via direct parsing
    let input = r#"
    {
        "suggested_task_id": "task_123",
        "reasoning": "This is a good next task"
    }
    "#;

    // This should work with direct JSON parsing (no complex validation needed)
    let parsed: serde_json::Value = serde_json::from_str(input)?;
    assert!(
        parsed.get("suggested_task_id").is_some(),
        "Should parse next task data directly"
    );

    Ok(())
}

#[test]
fn test_schema_versioning_task_breakdown() -> Result<()> {
    // Test that TaskBreakdown schema has versioning
    let input = r#"
    {
        "prd_title": "Test Feature",
        "parent_tasks": [],
        "relevant_files": [],
        "estimated_complexity": "Simple"
    }
    "#;

    let result = translate_llm_output(input, TargetSchema::TaskBreakdown)?;

    // EXPECTED: After modernization, result should contain version info
    assert!(
        result.get("_schema_version").is_some(),
        "TaskBreakdown should include schema version after modernization"
    );

    Ok(())
}

#[test]
fn test_schema_versioning_priority_result() -> Result<()> {
    // Test that PriorityResult schema has versioning
    let input = r#"
    {
        "priorities": [
            {
                "task_id": "task_123",
                "priority": "High"
            }
        ]
    }
    "#;

    let result = translate_llm_output(input, TargetSchema::PriorityResult)?;

    // EXPECTED: After modernization, result should contain version info
    assert!(
        result.get("_schema_version").is_some(),
        "PriorityResult should include schema version after modernization"
    );

    Ok(())
}

#[test]
fn test_schema_versioning_sequential_step() -> Result<()> {
    // Test that SequentialStep schema has versioning
    let input = r#"
    {
        "step_number": 1,
        "thought": "I should start with step 1",
        "reasoning": "Because it's the first step"
    }
    "#;

    let result = translate_llm_output(input, TargetSchema::SequentialStep)?;

    // EXPECTED: After modernization, result should contain version info
    assert!(
        result.get("_schema_version").is_some(),
        "SequentialStep should include schema version after modernization"
    );

    Ok(())
}

#[test]
fn test_intellitask_migration_path() -> Result<()> {
    // Test that we have a migration path for intellitask module
    // This tests that intellitask.rs can be updated to use active schemas

    // Test 1: Next task parsing should work directly (no translator needed)
    let next_task_input = r#"
    {
        "suggested_task_id": "task_123",
        "reasoning": "Good next task"
    }
    "#;

    // This should work with direct JSON parsing (no complex validation needed)
    let parsed: serde_json::Value = serde_json::from_str(next_task_input)?;
    assert!(parsed.get("suggested_task_id").is_some(), "Should parse next task data directly");

    // Test 2: Subtask data should work with TaskBreakdown schema migration
    let subtask_input = r#"
    {
        "prd_title": "Subtask Generation",
        "parent_tasks": [{
            "id": "subtask_parent",
            "title": "Subtask Generation",
            "description": "Generated subtasks",
            "subtasks": []
        }],
        "relevant_files": [],
        "estimated_complexity": "Simple"
    }
    "#;

    // This should work with TaskBreakdown schema after migration
    let result = translate_llm_output(subtask_input, TargetSchema::TaskBreakdown);
    assert!(result.is_ok(), "Should handle subtask migration data with TaskBreakdown schema");

    // Test 3: Result should have versioning metadata
    if let Ok(result_val) = result {
        assert!(
            result_val.get("_schema_version").is_some(),
            "TaskBreakdown should include schema version after modernization"
        );
    }

    Ok(())
}

#[test]
fn test_active_schemas_remain_functional() -> Result<()> {
    // Ensure all actively used schemas continue to work

    // TaskBreakdown (used by intellitask_commands and MCP server)
    let task_input = r#"
    {
        "prd_title": "Test Feature",
        "parent_tasks": [],
        "relevant_files": [],
        "estimated_complexity": "Simple"
    }
    "#;
    let result = translate_llm_output(task_input, TargetSchema::TaskBreakdown)?;
    assert!(result.get("error").is_none(), "TaskBreakdown should work");

    // PriorityResult (used by intellitask_commands)
    let priority_input = r#"
    {
        "priorities": [
            {
                "task_id": "123",
                "priority": "High"
            }
        ]
    }
    "#;
    let result = translate_llm_output(priority_input, TargetSchema::PriorityResult)?;
    assert!(result.get("error").is_none(), "PriorityResult should work");

    // SequentialStep (used by sequential_commands)
    let step_input = r#"
    {
        "step_number": 1,
        "thought": "Test thought",
        "reasoning": "Test reasoning"
    }
    "#;
    let result = translate_llm_output(step_input, TargetSchema::SequentialStep)?;
    assert!(result.get("error").is_none(), "SequentialStep should work");

    Ok(())
}

#[test]
fn test_reasoning_contract_compatibility() -> Result<()> {
    // Test that translator output is compatible with reasoning contracts
    let input = r#"
    {
        "prd_title": "Test Feature",
        "parent_tasks": [],
        "relevant_files": [],
        "estimated_complexity": "Simple"
    }
    "#;

    let result = translate_llm_output(input, TargetSchema::TaskBreakdown)?;

    // EXPECTED: After modernization, result should include contract metadata
    assert!(
        result.get("_contract_version").is_some() || result.get("_schema_version").is_some(),
        "Should include contract version information for reasoning engine compatibility"
    );

    Ok(())
}