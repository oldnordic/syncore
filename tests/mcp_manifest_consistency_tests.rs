//! MCP Manifest Consistency Tests
//!
//! Tests to ensure the MCP tool manifest matches the runtime implementation
//! and that no forbidden tools are exposed.

use serde_json::Value;
use std::collections::HashSet;

/// Get expected list of tools that should be exposed via MCP
fn get_expected_tools() -> Vec<&'static str> {
    vec![
        // Individual tools (14 tools)
        "document_search",
        "document_index",
        "vector_insert",
        "vector_search",
        "code_search",
        "code_index",
        "code_index_directory",
        "graph_query",
        "graph_insert",
        "memory_query",
        "memory_store",
        "parser_search",
        "parser_analyze",
        "task_create",
        // Suite tools (2 tools)
        "debug_suite",
        "mapping_suite",
    ]
}

/// Get list of forbidden tool name patterns
fn get_forbidden_patterns() -> Vec<&'static str> {
    vec!["sequential", "ollama", "reasoning", "tree_of_thoughts"]
}

#[test]
fn test_expected_tool_count() {
    let expected_tools = get_expected_tools();
    assert_eq!(expected_tools.len(), 16, "Should have exactly 16 tools");
}

#[test]
fn test_no_forbidden_tool_names() {
    let expected_tools = get_expected_tools();
    let forbidden_patterns = get_forbidden_patterns();

    for tool_name in expected_tools {
        for pattern in &forbidden_patterns {
            assert!(
                !tool_name.contains(pattern),
                "Tool '{}' contains forbidden pattern '{}'",
                tool_name,
                pattern
            );
        }
    }
}

#[test]
fn test_expected_tools_are_unique() {
    let expected_tools = get_expected_tools();
    let unique_tools: HashSet<_> = expected_tools.iter().collect();

    assert_eq!(
        unique_tools.len(),
        expected_tools.len(),
        "Expected tools list contains duplicates: {:?}",
        expected_tools
            .iter()
            .filter(|&t| expected_tools.iter().filter(|&x| x == t).count() > 1)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_suite_implementations_exist() {
    // Check that we have implementations for the suites we expose
    let suite_modules = vec![
        ("code_suite", "src/mcp_tools/code_suite.rs"),
        ("graph_suite", "src/mcp_tools/graph_suite.rs"),
        ("memory_suite", "src/mcp_tools/memory_suite/mod.rs"),
        ("debug_suite", "src/mcp_tools/debug_suite.rs"),
        ("mapping_suite", "src/mcp_tools/mapping_suite.rs"),
    ];

    for (suite_name, file_path) in suite_modules {
        assert!(
            std::path::Path::new(file_path).exists(),
            "Suite '{}' implementation file '{}' does not exist",
            suite_name,
            file_path
        );
    }
}

#[test]
fn test_suite_schema_files_exist() {
    let schema_files = vec![
        "schemas/code_suite.json",
        "schemas/graph_suite.json",
        "schemas/memory_suite.json",
        "schemas/debug_suite.json",
        "schemas/mapping_suite.json",
        "schemas/suite_result.json",
    ];

    for schema_file in schema_files {
        assert!(
            std::path::Path::new(schema_file).exists(),
            "Schema file '{}' does not exist",
            schema_file
        );
    }
}

#[test]
fn test_no_reasoning_tools_exposed() {
    let expected_tools = get_expected_tools();

    for tool_name in expected_tools {
        assert!(
            !tool_name.starts_with("reasoning.") && tool_name != "reasoning_suite",
            "Tool '{}' exposes reasoning functionality which should stay internal",
            tool_name
        );
    }
}

#[test]
fn test_tool_categories_are_valid() {
    // The 16 tools should fall into logical categories
    let expected_tools = get_expected_tools();

    // Count by expected patterns
    let mut code_tools = 0;
    let mut memory_tools = 0;
    let mut graph_tools = 0;
    let mut debug_tools = 0;
    let mut mapping_tools = 0;

    for tool_name in expected_tools {
        if tool_name.starts_with("code_") || tool_name.starts_with("parser_") {
            code_tools += 1;
        } else if tool_name.starts_with("memory_")
            || tool_name.starts_with("vector_")
            || tool_name.starts_with("task_")
        {
            memory_tools += 1;
        } else if tool_name.starts_with("graph_") {
            graph_tools += 1;
        } else if tool_name.starts_with("document_") {
            memory_tools += 1; // Document tools use memory suite
        } else if tool_name == "debug_suite" {
            debug_tools = 1;
        } else if tool_name == "mapping_suite" {
            mapping_tools = 1;
        }
    }

    assert_eq!(code_tools, 5, "Should have 5 code/parser tools");
    assert_eq!(memory_tools, 7, "Should have 7 memory/vector/task/document tools");
    assert_eq!(graph_tools, 2, "Should have 2 graph tools");
    assert_eq!(debug_tools, 1, "Should have 1 debug suite tool");
    assert_eq!(mapping_tools, 1, "Should have 1 mapping suite tool");

    // Total should be 16
    assert_eq!(code_tools + memory_tools + graph_tools + debug_tools + mapping_tools, 16);
}

#[cfg(feature = "integration-tests")]
mod integration_tests {
    use super::*;
    use crate::mcp::protocol;

    #[tokio::test]
    async fn test_protocol_list_tools_matches_expected() {
        let protocol_tools = protocol::list_tools().await;
        let expected_tools = get_expected_tools();

        assert_eq!(
            protocol_tools.len(),
            expected_tools.len(),
            "Protocol returns {} tools, expected {}",
            protocol_tools.len(),
            expected_tools.len()
        );

        let protocol_tool_names: HashSet<String> =
            protocol_tools.into_iter().map(|t| t.name).collect();

        let expected_tool_names: HashSet<&str> = expected_tools.into_iter().collect();

        assert_eq!(
            protocol_tool_names, expected_tool_names,
            "Protocol tools don't match expected tools. Protocol: {:?}, Expected: {:?}",
            protocol_tool_names, expected_tool_names
        );
    }

    #[tokio::test]
    async fn test_protocol_tools_have_valid_schemas() {
        let protocol_tools = protocol::list_tools().await;

        for tool in protocol_tools {
            assert!(!tool.input_schema.is_empty(), "Tool '{}' has empty input_schema", tool.name);
            assert!(!tool.output_schema.is_empty(), "Tool '{}' has empty output_schema", tool.name);
            assert!(
                tool.input_schema.starts_with("schemas/"),
                "Tool '{}' input schema '{}' should start with 'schemas/'",
                tool.name,
                tool.input_schema
            );
            assert!(
                tool.output_schema.starts_with("schemas/"),
                "Tool '{}' output schema '{}' should start with 'schemas/'",
                tool.name,
                tool.output_schema
            );
        }
    }
}
