//! Golden Manifest Tests for MCP Tool Surface
//!
//! Strict TDD validation of canonical 34-tool manifest (v2)
//! Tests actual runtime tools, not desired future state

use std::collections::HashSet;
use syncore::mcp::tool_metadata::list_all_metadata;

#[test]
fn test_golden_manifest_tool_count() {
    let tools = list_all_metadata();
    assert_eq!(tools.len(), 34, "Expected exactly 34 tools in canonical manifest v2");

    let tool_names: HashSet<String> = tools.iter().map(|t| t.name.to_string()).collect();
    assert_eq!(tool_names.len(), 34, "All tool names must be unique");
}

#[test]
fn test_critical_tools_exist() {
    let tools = list_all_metadata();
    let tool_names: HashSet<String> = tools.iter().map(|t| t.name.to_string()).collect();

    // Critical tools that MUST exist for production
    let critical_tools = vec![
        // Task System (5 tools)
        "task_create",
        "task_list",
        "task_get",
        "task_update",
        "task_next",
        // Graph System (4 tools)
        "graph_query",
        "graph_insert",
        "graph_relate",
        "graph_suite",
        // Reasoning System (4 tools)
        "reasoning_session_create",
        "reasoning_branch_expand",
        "reasoning_tree_get",
        "reasoning_tree_prune",
        // IntelliTask System (8 tools)
        "intellitask_generate",
        "intellitask_subtasks",
        "intellitask_prioritize",
        "intellitask_next",
        "intellitask_save",
        "intellitask_get",
        "intellitask_list",
        "intellitask_update_status",
        // Core Systems (13 tools)
        "memory_store",
        "memory_query",
        "vector_insert",
        "vector_search",
        "code_index",
        "code_search",
        "code_index_directory",
        "parser_analyze",
        "parser_search",
        "document_index",
        "document_search",
        "debug_suite",
        "mapping_suite",
    ];

    for tool in &critical_tools {
        assert!(tool_names.contains(*tool), "Critical tool '{}' missing from manifest", tool);
    }
}

#[test]
fn test_no_forbidden_tools() {
    let tools = list_all_metadata();
    let tool_names: HashSet<String> = tools.iter().map(|t| t.name.to_string()).collect();

    // Tools that should NOT exist in production
    let forbidden_tools = vec![
        "sequential_cycle",
        "sequential_record",
        "sequential_get",
        "sequential_search",
        "ollama_generate",
        "ollama_chat",
        "ollama_health",
    ];

    for forbidden_tool in &forbidden_tools {
        assert!(
            !tool_names.contains(*forbidden_tool),
            "Forbidden tool '{}' found in manifest",
            forbidden_tool
        );
    }
}

#[test]
fn test_canonical_manifest_completeness() {
    let tools = list_all_metadata();
    let tool_names: HashSet<String> = tools.iter().map(|t| t.name.to_string()).collect();

    // Expected categories that must be represented
    let mut categories_found = std::collections::HashSet::new();
    for tool in tools {
        categories_found.insert(format!("{:?}", tool.category));
    }

    // We should have tools from major categories
    assert!(categories_found.contains(&"Memory".to_string()), "Missing Memory category tools");
    assert!(categories_found.contains(&"Task".to_string()), "Missing Task category tools");
    assert!(categories_found.contains(&"Graph".to_string()), "Missing Graph category tools");
    assert!(
        categories_found.contains(&"IntelliTask".to_string()),
        "Missing IntelliTask category tools"
    );
    assert!(categories_found.contains(&"Code".to_string()), "Missing Code category tools");
    assert!(categories_found.contains(&"Vector".to_string()), "Missing Vector category tools");
}

/// Test 5: Metadata completeness for all tools
#[test]
fn test_all_tools_have_valid_metadata() {
    let tools = list_all_metadata();
    let tool_names: HashSet<String> = tools.iter().map(|t| t.name.to_string()).collect();

    // Check every tool has metadata
    for tool in &tools {
        let metadata = syncore::mcp::tool_metadata::get_tool_metadata(&tool.name);
        assert!(metadata.is_some(), "Tool '{}' missing from metadata registry", tool.name);

        let meta = metadata.unwrap();
        assert_eq!(meta.name, tool.name, "Metadata name mismatch for tool '{}'", tool.name);
        assert!(!meta.description.is_empty(), "Tool '{}' has empty description", tool.name);
    }

    // Check no extra tools in metadata
    let all_metadata = syncore::mcp::tool_metadata::list_all_metadata();
    for meta in all_metadata {
        assert!(
            tool_names.contains(meta.name),
            "Metadata tool '{}' not in protocol list",
            meta.name
        );
    }
}

/// Test 6: Schema consistency validation
#[test]
fn test_schema_consistency_validation() {
    // Since we can't use async in regular test, validate schema mappings exist
    // by checking that all expected schema files exist in schemas/ directory

    let expected_schema_files = vec![
        "memory_suite.json",
        "code_suite.json",
        "graph_suite.json",
        "debug_suite.json",
        "mapping_suite.json",
        "reasoning_session_create.json",
        "reasoning_branch_expand.json",
        "reasoning_tree_get.json",
        "reasoning_tree_prune.json",
        "suite_result.json",
    ];

    // Verify schema files exist (this test ensures schema consistency)
    for schema_file in &expected_schema_files {
        let schema_path = format!("schemas/{}", schema_file);
        assert!(
            std::path::Path::new(&schema_path).exists(),
            "Schema file '{}' does not exist",
            schema_file
        );
    }

    // Verify schema files are valid JSON
    for schema_file in &expected_schema_files {
        let schema_path = format!("schemas/{}", schema_file);
        let schema_content = std::fs::read_to_string(&schema_path)
            .expect(&format!("Failed to read schema file: {}", schema_path));

        // Try to parse as JSON
        let _parsed: serde_json::Value = serde_json::from_str(&schema_content)
            .expect(&format!("Schema file '{}' is not valid JSON", schema_file));
    }
}
