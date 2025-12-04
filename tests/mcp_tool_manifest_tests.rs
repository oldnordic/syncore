//! MCP Tool Manifest Consistency Tests for ST-13
//!
//! Tests to ensure runtime metadata matches protocol registration
//! and no zombie tools exist.

use serde_json::json;
use syncore::mcp::protocol::{describe_server, list_tools};
use syncore::mcp::tool_metadata::{get_tool_metadata, list_all_metadata};

#[test]
fn test_runtime_metadata_matches_protocol_registration() {
    // Get runtime metadata
    let runtime_tools = list_all_metadata();
    let runtime_names: std::collections::HashSet<&str> =
        runtime_tools.iter().map(|t| t.name).collect();

    // Get protocol registration
    let protocol_tools = tokio::runtime::Runtime::new().unwrap().block_on(list_tools());
    let protocol_names: std::collections::HashSet<String> =
        protocol_tools.iter().map(|t| t.name.clone()).collect();

    // Convert protocol names to &str for comparison
    let protocol_names_str: std::collections::HashSet<&str> =
        protocol_names.iter().map(|s| s.as_str()).collect();

    // All runtime tools should be in protocol
    for runtime_name in &runtime_names {
        assert!(
            protocol_names_str.contains(runtime_name),
            "Runtime tool '{}' not found in protocol registration",
            runtime_name
        );
    }

    // All protocol tools should be in runtime (no orphaned protocol tools)
    for protocol_name in &protocol_names_str {
        assert!(
            runtime_names.contains(protocol_name),
            "Protocol tool '{}' not found in runtime metadata",
            protocol_name
        );
    }

    // Counts should match
    assert_eq!(
        runtime_tools.len(),
        protocol_tools.len(),
        "Runtime metadata has {} tools but protocol registers {} tools",
        runtime_tools.len(),
        protocol_tools.len()
    );
}

#[test]
fn test_no_sequential_or_ollama_tools() {
    let runtime_tools = list_all_metadata();
    let protocol_tools = tokio::runtime::Runtime::new().unwrap().block_on(list_tools());

    // Check runtime metadata for forbidden tools
    for tool in runtime_tools {
        assert!(
            !tool.name.contains("sequential"),
            "Found sequential tool in runtime metadata: {}",
            tool.name
        );
        assert!(
            !tool.name.contains("ollama"),
            "Found ollama tool in runtime metadata: {}",
            tool.name
        );
    }

    // Check protocol registration for forbidden tools
    for tool in protocol_tools {
        assert!(
            !tool.name.contains("sequential"),
            "Found sequential tool in protocol registration: {}",
            tool.name
        );
        assert!(
            !tool.name.contains("ollama"),
            "Found ollama tool in protocol registration: {}",
            tool.name
        );
    }
}

#[test]
fn test_reasoning_tools_not_present() {
    let runtime_tools = list_all_metadata();
    let protocol_tools = tokio::runtime::Runtime::new().unwrap().block_on(list_tools());

    // Check that reasoning tools are NOT available (ST-13 requirement)
    let reasoning_tools: Vec<&str> =
        runtime_tools.iter().filter(|t| t.name.contains("reasoning")).map(|t| t.name).collect();

    let protocol_reasoning_tools: Vec<String> = protocol_tools
        .iter()
        .filter(|t| t.name.contains("reasoning"))
        .map(|t| t.name.clone())
        .collect();

    // Should NOT have reasoning tools (ST-13 rule)
    assert!(
        reasoning_tools.is_empty(),
        "Found reasoning tools in runtime metadata: {:?}",
        reasoning_tools
    );
    assert!(
        protocol_reasoning_tools.is_empty(),
        "Found reasoning tools in protocol registration: {:?}",
        protocol_reasoning_tools
    );
}

#[test]
fn test_suite_based_tools_available() {
    let protocol_tools = tokio::runtime::Runtime::new().unwrap().block_on(list_tools());
    let protocol_names: std::collections::HashSet<String> =
        protocol_tools.iter().map(|t| t.name.clone()).collect();

    // Check that suite-based INDIVIDUAL tools are available (not the suites themselves)
    let expected_tools = vec![
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
    ];

    for tool in expected_tools {
        assert!(
            protocol_names.contains(tool),
            "Tool '{}' not found in protocol registration",
            tool
        );
    }

    // Should NOT have suite tools themselves (only individual tools)
    let unexpected_suite_tools = vec![
        "code_suite",
        "memory_suite",
        "graph_suite",
        "debug_suite",
        "mapping_suite",
        "refrag_suite",
    ];

    for suite_tool in unexpected_suite_tools {
        assert!(
            !protocol_names.contains(suite_tool),
            "Suite tool '{}' should not be in protocol registration (only individual tools)",
            suite_tool
        );
    }
}

#[test]
fn test_tool_metadata_consistency() {
    let runtime_tools = list_all_metadata();
    let protocol_tools = tokio::runtime::Runtime::new().unwrap().block_on(list_tools());

    // Build mapping from protocol to runtime metadata
    let protocol_to_runtime: std::collections::HashMap<
        String,
        &&syncore::mcp::tool_metadata::ToolMetadata,
    > = protocol_tools
        .iter()
        .filter_map(|pt| {
            let runtime_name = pt.name.replace('.', "_"); // Convert dots to underscores
            runtime_tools.iter().find(|rt| rt.name == runtime_name).map(|rt| (pt.name.clone(), rt))
        })
        .collect();

    // Check that each protocol tool has matching runtime metadata
    for protocol_tool in &protocol_tools {
        let runtime_name = protocol_tool.name.replace('.', "_");
        if let Some(runtime_meta) = protocol_to_runtime.get(&protocol_tool.name) {
            // Check category consistency (rough mapping)
            assert!(
                !runtime_meta.description.is_empty(),
                "Runtime metadata for '{}' has empty description",
                protocol_tool.name
            );

            // Check cost consistency
            match runtime_meta.cost {
                syncore::mcp::tool_metadata::ToolCost::Low => {
                    // Low cost tools should be fast operations
                }
                syncore::mcp::tool_metadata::ToolCost::VeryHigh => {
                    // Very high cost tools should be expensive operations
                    assert!(
                        protocol_tool.description.to_lowercase().contains("index")
                            || protocol_tool.description.to_lowercase().contains("directory"),
                        "Very high cost tool '{}' should mention expensive operation",
                        protocol_tool.name
                    );
                }
                _ => {} // Medium/High cost - no specific checks
            }
        }
    }
}

#[test]
fn test_describe_server_accuracy() {
    let server_info = tokio::runtime::Runtime::new().unwrap().block_on(describe_server());

    // Check that tools_count matches actual
    let protocol_tools = tokio::runtime::Runtime::new().unwrap().block_on(list_tools());
    let actual_count = protocol_tools.len();

    if let Some(count) = server_info.get("tools_count").and_then(|v| v.as_u64()) {
        assert_eq!(
            count as usize, actual_count,
            "Server info reports {} tools but actually has {} tools",
            count, actual_count
        );
    }

    // Check capabilities
    if let Some(capabilities) = server_info.get("capabilities").and_then(|v| v.as_object()) {
        // Should have basic capabilities
        assert!(capabilities.contains_key("memory"), "Missing memory capability");
        assert!(capabilities.contains_key("vector_search"), "Missing vector_search capability");
        assert!(capabilities.contains_key("mcp_compliant"), "Missing mcp_compliant capability");
    }
}
