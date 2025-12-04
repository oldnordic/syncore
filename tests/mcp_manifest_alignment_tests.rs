//! MCP Manifest Alignment Tests
//!
//! Tests to verify that runtime tools match canonical expectations
//! and that all tools are properly categorized.

use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashSet;
use syncore::mcp::protocol::list_tools;

#[tokio::test]
async fn test_runtime_tool_count_matches_expectation() {
    let tools = list_tools().await;
    assert_eq!(tools.len(), 34, "Expected exactly 34 runtime tools");
}

#[tokio::test]
async fn test_all_runtime_tools_have_valid_schemas() {
    let tools = list_tools().await;

    for tool in &tools {
        // Verify input schema exists and is valid JSON
        let input_schema = std::fs::read_to_string(&tool.input_schema);
        assert!(input_schema.is_ok(), "Input schema file not found: {}", tool.input_schema);

        let input_json: Value = serde_json::from_str(&input_schema.unwrap()).unwrap();
        assert!(input_json.get("type").is_some(), "Invalid JSON schema for {}", tool.name);

        // Verify output schema exists and is valid JSON
        let output_schema = std::fs::read_to_string(&tool.output_schema);
        assert!(output_schema.is_ok(), "Output schema file not found: {}", tool.output_schema);

        let output_json: Value = serde_json::from_str(&output_schema.unwrap()).unwrap();
        assert!(output_json.get("type").is_some(), "Invalid output schema for {}", tool.name);
    }
}

#[tokio::test]
async fn test_tool_categorization() {
    let tools = list_tools().await;
    let tool_names: HashSet<String> = tools.iter().map(|t| t.name.clone()).collect();

    // Expected categories and their tools
    let expected_categories = json!({
        "document_suite": ["document_search", "document_index"],
        "vector_suite": ["vector_insert", "vector_search"],
        "code_suite": ["code_search", "code_index", "code_index_directory"],
        "graph_suite": ["graph_query", "graph_insert", "graph_relate", "graph_suite"],
        "memory_suite": ["memory_query", "memory_store"],
        "parser_suite": ["parser_search", "parser_analyze"],
        "task_suite": ["task_create", "task_list", "task_get", "task_update", "task_next"],
        "debug_suite": ["debug_suite"],
        "mapping_suite": ["mapping_suite"],
        "reasoning_suite": ["reasoning_session_create", "reasoning_branch_expand", "reasoning_tree_get", "reasoning_tree_prune"],
        "intellitask_suite": ["intellitask_generate", "intellitask_subtasks", "intellitask_prioritize", "intellitask_next", "intellitask_save", "intellitask_get", "intellitask_list", "intellitask_update_status"]
    });

    // Verify all expected tools exist
    for (category, tools_array) in expected_categories.as_object().unwrap() {
        for tool_name in tools_array.as_array().unwrap() {
            let tool_str = tool_name.as_str().unwrap();
            assert!(
                tool_names.contains(tool_str),
                "Tool '{}' from category '{}' not found in runtime",
                tool_str,
                category
            );
        }
    }

    // Verify no extra tools exist
    let mut expected_tools: HashSet<String> = HashSet::new();
    for tools_array in expected_categories.as_object().unwrap().values() {
        for tool_name in tools_array.as_array().unwrap() {
            expected_tools.insert(tool_name.as_str().unwrap().to_string());
        }
    }

    assert_eq!(tool_names, expected_tools, "Runtime tools don't match expected tool set");
}

#[tokio::test]
async fn test_suite_tools_exist_alongside_individual_tools() {
    let tools = list_tools().await;
    let tool_names: HashSet<String> = tools.iter().map(|t| t.name.clone()).collect();

    // These should exist as both individual tools AND suite tools
    assert!(tool_names.contains("graph_suite"), "graph_suite tool missing");
    assert!(tool_names.contains("debug_suite"), "debug_suite tool missing");
    assert!(tool_names.contains("mapping_suite"), "mapping_suite tool missing");

    // These should exist as individual tools but NOT as suites
    assert!(tool_names.contains("code_search"), "code_search tool missing");
    assert!(tool_names.contains("memory_query"), "memory_query tool missing");
    assert!(tool_names.contains("task_create"), "task_create tool missing");
}

#[tokio::test]
async fn test_no_sequential_tools_per_apex_spec() {
    let tools = list_tools().await;

    for tool in &tools {
        assert!(
            !tool.name.to_lowercase().contains("sequential"),
            "Tool '{}' contains 'sequential' which violates APEX SPEC rule 1",
            tool.name
        );
        assert!(
            !tool.description.to_lowercase().contains("sequential"),
            "Tool '{}' description contains 'sequential' which violates APEX SPEC rule 1",
            tool.name
        );
    }
}

#[tokio::test]
async fn test_tool_descriptions_are_meaningful() {
    let tools = list_tools().await;

    for tool in &tools {
        assert!(!tool.description.is_empty(), "Tool '{}' has empty description", tool.name);
        assert!(tool.description.len() > 10, "Tool '{}' description too short", tool.name);
        assert!(
            !tool.description.to_lowercase().starts_with("todo"),
            "Tool '{}' description starts with TODO",
            tool.name
        );
    }
}
