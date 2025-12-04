//! MCP Tool Surface Specification Tests for ST-16
//!
//! Tests to verify the final target MCP tool surface matches the ST-16 plan.
//! This test encodes the intended final manifest without breaking on missing implementations.

use syncore::mcp::protocol::list_tools;
use syncore::mcp::tool_metadata::list_all_metadata;

/// Expected final tool categories and counts from ST-16 plan
const EXPECTED_CATEGORIES: &[&str] = &[
    "Document",
    "Code",
    "Parser",
    "Vector",
    "Graph",
    "Memory",
    "Task",
    "Reasoning",   // To be restored
    "IntelliTask", // To be restored
    "Debug Suite",
    "Mapping Suite",
    "Refrag Suite",
];

/// Expected final tool names from ST-16 plan
const EXPECTED_TOOLS: &[&str] = &[
    // Document
    "document_index",
    "document_search",
    // Code
    "code_index",
    "code_search",
    "code_index_directory",
    "code_explain", // Missing - to be added
    // Parser
    "parse_file", // Missing - to be added (rename from parser_analyze)
    "parser_search",
    // Vector
    "vector_insert",
    "vector_search",
    // Graph
    "graph_query",
    "graph_insert",
    "graph_relate", // Missing - to be added
    "graph_suite",  // Missing - to be added
    // Memory
    "memory_store",
    "memory_query",
    // Task
    "task_create",
    "task_list",   // Missing - to be added
    "task_get",    // Missing - to be added
    "task_update", // Missing - to be added
    "task_next",   // Missing - to be added
    // Reasoning (to be restored)
    "reasoning_session_create", // Missing - to be added
    "reasoning_tree_get",       // Missing - to be added
    "reasoning_tree_prune",     // Missing - to be added
    "reasoning_branch_expand",  // Missing - to be added
    // IntelliTask (to be restored)
    "intellitask_generate",      // Missing - to be added
    "intellitask_subtasks",      // Missing - to be added
    "intellitask_prioritize",    // Missing - to be added
    "intellitask_next",          // Missing - to be added
    "intellitask_save",          // Missing - to be added
    "intellitask_get",           // Missing - to be added
    "intellitask_list",          // Missing - to be added
    "intellitask_update_status", // Missing - to be added
    // Debug Suite
    "debug_suite",
    // Mapping Suite
    "mapping_suite",
    // Refrag Suite
    "refrag_suite",
];

#[test]
fn test_current_real_tools_exist() {
    // This test only asserts on CURRENTLY REAL tools (16 tools)
    // to avoid false failures during implementation

    let protocol_tools = tokio::runtime::Runtime::new().unwrap().block_on(list_tools());
    let protocol_names: std::collections::HashSet<String> =
        protocol_tools.iter().map(|t| t.name.clone()).collect();

    // Current real tools that MUST exist
    let current_real_tools = vec![
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
        "debug_suite",
        "mapping_suite",
    ];

    for tool in current_real_tools {
        assert!(
            protocol_names.contains(tool),
            "Current real tool '{}' not found in protocol registration",
            tool
        );
    }
}

#[test]
fn test_no_forbidden_tools_currently() {
    // Verify no sequential or ollama tools exist currently
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
fn test_missing_but_required_tools_documented() {
    // This test documents missing tools as comments without asserting on them
    // The actual assertions will be added incrementally as tools are implemented

    let protocol_tools = tokio::runtime::Runtime::new().unwrap().block_on(list_tools());
    let protocol_names: std::collections::HashSet<String> =
        protocol_tools.iter().map(|t| t.name.clone()).collect();

    // Document missing tools (these will be asserted in later phases)
    let missing_tools = vec![
        "code_explain",
        "parse_file",
        "graph_relate",
        "graph_suite",
        "task_list",
        "task_get",
        "task_update",
        "task_next",
        // Reasoning tools (to be restored)
        "reasoning_session_create",
        "reasoning_tree_get",
        "reasoning_tree_prune",
        "reasoning_branch_expand",
        // IntelliTask tools (to be restored)
        "intellitask_generate",
        "intellitask_subtasks",
        "intellitask_prioritize",
        "intellitask_next",
        "intellitask_save",
        "intellitask_get",
        "intellitask_list",
        "intellitask_update_status",
    ];

    // Currently, we expect these to be missing - this will change as we implement
    for missing_tool in &missing_tools {
        // TODO: Change this to assert!(protocol_names.contains(missing_tool)) once implemented
        if protocol_names.contains(*missing_tool) {
            println!("Tool '{}' is already implemented", missing_tool);
        } else {
            println!("Tool '{}' is missing (expected during implementation)", missing_tool);
        }
    }
}

#[test]
fn test_suite_tools_currently_available() {
    // Verify current suite tools are available
    let protocol_tools = tokio::runtime::Runtime::new().unwrap().block_on(list_tools());
    let protocol_names: std::collections::HashSet<String> =
        protocol_tools.iter().map(|t| t.name.clone()).collect();

    // Current suite tools that should be available
    let current_suite_tools = vec![
        "debug_suite",
        "mapping_suite",
        // refrag_suite may be missing currently
    ];

    for suite_tool in current_suite_tools {
        assert!(
            protocol_names.contains(suite_tool),
            "Suite tool '{}' not found in protocol registration",
            suite_tool
        );
    }
}

#[test]
fn test_cost_classifications_currently_correct() {
    // Verify cost classifications for current tools are reasonable
    let runtime_tools = list_all_metadata();

    for tool in runtime_tools {
        match tool.name {
            // High cost operations
            "code_index_directory" | "document_index" => {
                assert_eq!(
                    tool.cost,
                    syncore::mcp::tool_metadata::ToolCost::VeryHigh,
                    "Tool '{}' should be VeryHigh cost",
                    tool.name
                );
            }
            "code_index" | "graph_query" | "graph_insert" => {
                assert_eq!(
                    tool.cost,
                    syncore::mcp::tool_metadata::ToolCost::High,
                    "Tool '{}' should be High cost",
                    tool.name
                );
            }
            // Low cost operations
            "memory_store" | "memory_query" | "task_create" => {
                assert_eq!(
                    tool.cost,
                    syncore::mcp::tool_metadata::ToolCost::Low,
                    "Tool '{}' should be Low cost",
                    tool.name
                );
            }
            // CPU heavy operations
            "reasoning_branch_expand"
            | "intellitask_generate"
            | "intellitask_subtasks"
            | "intellitask_prioritize" => {
                assert_eq!(
                    tool.cost,
                    syncore::mcp::tool_metadata::ToolCost::CpuHeavy,
                    "Tool '{}' should be CpuHeavy cost",
                    tool.name
                );
            }
            // Medium cost operations (most others)
            _ => {
                // Most tools should be Medium cost by default
                if ![
                    "code_index_directory",
                    "document_index",
                    "code_index",
                    "graph_query",
                    "graph_insert",
                    "memory_store",
                    "memory_query",
                    "task_create",
                    "reasoning_branch_expand",
                    "intellitask_generate",
                    "intellitask_subtasks",
                    "intellitask_prioritize",
                ]
                .contains(&tool.name)
                {
                    assert_eq!(
                        tool.cost,
                        syncore::mcp::tool_metadata::ToolCost::Medium,
                        "Tool '{}' should be Medium cost",
                        tool.name
                    );
                }
            }
        }
    }
}

#[test]
fn test_side_effects_currently_correct() {
    // Verify side effects for current tools are accurate
    let runtime_tools = list_all_metadata();

    for tool in runtime_tools {
        match tool.name {
            // Database write operations
            "memory_store" | "task_create" | "mapping_suite" => {
                assert!(
                    tool.side_effects.modifies_database,
                    "Tool '{}' should modify database",
                    tool.name
                );
            }
            // Vector store operations
            "vector_insert" | "code_index" | "code_index_directory" | "document_index" => {
                assert!(
                    tool.side_effects.modifies_vector_store,
                    "Tool '{}' should modify vector store",
                    tool.name
                );
            }
            // Network operations (Neo4j)
            "graph_query" | "graph_insert" => {
                assert!(
                    tool.side_effects.network_call,
                    "Tool '{}' should make network calls",
                    tool.name
                );
            }
            // Read-only operations
            "memory_query" | "vector_search" | "code_search" | "document_search"
            | "parser_search" | "parser_analyze" | "debug_suite" => {
                assert!(
                    !tool.side_effects.modifies_database
                        && !tool.side_effects.modifies_vector_store
                        && !tool.side_effects.modifies_graph
                        && !tool.side_effects.network_call,
                    "Read-only tool '{}' should not have side effects",
                    tool.name
                );
            }
            _ => {} // No specific checks for other tools
        }
    }
}

// TODO: Add these tests as tools are implemented:
// - test_reasoning_tools_restored()
// - test_intellitask_tools_restored()
// - test_full_task_surface_available()
// - test_graph_operations_complete()
// - test_final_tool_count_matches_target()
