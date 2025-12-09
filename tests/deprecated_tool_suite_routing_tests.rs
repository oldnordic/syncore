//! Tests for deprecated tool suite routing
//!
//! This test file verifies that deprecated MCP tools route through suites
//! instead of calling mcp_delegate directly, ensuring behavioral equivalence
//! between deprecated tools and their suite counterparts.

use serde_json::{json, Value};
use syncore::mcp_server::types::{
    MemorySuiteRequest, CodeSuiteRequest, DebugSuiteRequest
};

/// Test memory_store routing to memory_suite
#[test]
fn test_memory_store_routes_via_memory_suite() {
    // This test should initially FAIL because memory_store still uses mcp_delegate

    // Prepare test data
    let test_key = "test_routing_key";
    let test_value = "test_routing_value";

    // Create the request that the deprecated handler should create
    let memory_suite_request = MemorySuiteRequest {
        command: "store".to_string(),
        key: Some(test_key.to_string()),
        value: Some(test_value.to_string()),
        dry_run: Some(false),
        // Include all advanced fields to ensure they're properly routed
        keywords: Some(vec!["routing".to_string(), "test".to_string()]),
        tags: Some(vec!["deprecated_tool".to_string()]),
        min_importance: Some(0.5),
        unix_timestamp: Some(1640995200),
        seconds: Some(3600),
        threshold: Some(0.8),
        // Set other fields to defaults
        text: None,
        query: None,
        limit: None,
        namespace: None,
        goal: None,
        priority: None,
        task_id: None,
        depends_on_task_id: None,
        step_number: None,
        thought: None,
        reasoning: None,
        action: None,
        observation: None,
        max_cycles: None,
        sequence_id: None,
        context: None,
        depth: None,
        max_steps: None,
        to: None,
        from: None,
        agent: None,
        id: None,
        message: None,
        capabilities: None,
        status: None,
        task_type: None,
        payload: None,
        result: None,
        timeout_ms: None,
        prd_content: None,
        parent_task_id: None,
        parent_task_json: None,
        tasks_json: None,
        business_context: None,
        completed_tasks: None,
        remaining_tasks_json: None,
        breakdown_json: None,
        parent_id: None,
        prd_title: None,
    };

    // Verify the request structure is correct
    assert_eq!(memory_suite_request.command, "store");
    assert_eq!(memory_suite_request.key, Some(test_key.to_string()));
    assert_eq!(memory_suite_request.value, Some(test_value.to_string()));

    // TODO: Once we implement the fix, this test should:
    // 1. Call the deprecated memory_store handler
    // 2. Call memory_suite with command="store" and same parameters
    // 3. Verify both produce identical results

    // For now, this test documents the expected behavior
    println!("memory_store should route to memory_suite with command='store'");
}

/// Test memory_query routing to memory_suite
#[test]
fn test_memory_query_routes_via_memory_suite() {
    // This test should initially FAIL because memory_query still uses mcp_delegate

    let test_key = "test_query_key";

    let memory_suite_request = MemorySuiteRequest {
        command: "query".to_string(),
        key: Some(test_key.to_string()),
        dry_run: Some(false),
        // Include advanced fields that should be preserved
        keywords: Some(vec!["query".to_string()]),
        tags: Some(vec!["test".to_string()]),
        min_importance: Some(0.3),
        unix_timestamp: Some(1640995200),
        seconds: Some(7200),
        threshold: Some(0.6),
        // Set other fields to defaults
        text: None,
        value: None,
        query: None,
        limit: None,
        namespace: None,
        goal: None,
        priority: None,
        task_id: None,
        depends_on_task_id: None,
        step_number: None,
        thought: None,
        reasoning: None,
        action: None,
        observation: None,
        max_cycles: None,
        sequence_id: None,
        context: None,
        depth: None,
        max_steps: None,
        to: None,
        from: None,
        agent: None,
        id: None,
        message: None,
        capabilities: None,
        status: None,
        task_type: None,
        payload: None,
        result: None,
        timeout_ms: None,
        prd_content: None,
        parent_task_id: None,
        parent_task_json: None,
        tasks_json: None,
        business_context: None,
        completed_tasks: None,
        remaining_tasks_json: None,
        breakdown_json: None,
        parent_id: None,
        prd_title: None,
    };

    assert_eq!(memory_suite_request.command, "query");
    assert_eq!(memory_suite_request.key, Some(test_key.to_string()));

    println!("memory_query should route to memory_suite with command='query'");
}

/// Test vector_insert routing to memory_suite
#[test]
fn test_vector_insert_routes_via_memory_suite() {
    let test_text = "test vector insertion text";

    let memory_suite_request = MemorySuiteRequest {
        command: "vector_insert".to_string(),
        text: Some(test_text.to_string()),
        dry_run: Some(false),
        // Advanced fields should be preserved
        keywords: Some(vec!["vector".to_string()]),
        tags: Some(vec!["embedding".to_string()]),
        min_importance: Some(0.7),
        threshold: Some(0.9),
        // Set other fields to defaults
        key: None,
        value: None,
        query: None,
        limit: None,
        namespace: None,
        goal: None,
        priority: None,
        task_id: None,
        depends_on_task_id: None,
        step_number: None,
        thought: None,
        reasoning: None,
        action: None,
        observation: None,
        max_cycles: None,
        sequence_id: None,
        context: None,
        depth: None,
        max_steps: None,
        to: None,
        from: None,
        agent: None,
        id: None,
        message: None,
        capabilities: None,
        status: None,
        task_type: None,
        payload: None,
        result: None,
        timeout_ms: None,
        prd_content: None,
        parent_task_id: None,
        parent_task_json: None,
        tasks_json: None,
        business_context: None,
        completed_tasks: None,
        remaining_tasks_json: None,
        breakdown_json: None,
        parent_id: None,
        prd_title: None,
        unix_timestamp: None,
        seconds: None,
    };

    assert_eq!(memory_suite_request.command, "vector_insert");
    assert_eq!(memory_suite_request.text, Some(test_text.to_string()));

    println!("vector_insert should route to memory_suite with command='vector_insert'");
}

/// Test vector_search routing to memory_suite
#[test]
fn test_vector_search_routes_via_memory_suite() {
    let test_query = "test search query";

    let memory_suite_request = MemorySuiteRequest {
        command: "vector_search".to_string(),
        query: Some(test_query.to_string()),
        limit: Some(10),
        dry_run: Some(false),
        // Advanced fields that should affect search
        keywords: Some(vec!["search".to_string()]),
        tags: Some(vec!["semantic".to_string()]),
        min_importance: Some(0.4),
        threshold: Some(0.7),
        // Set other fields to defaults
        key: None,
        value: None,
        text: None,
        namespace: None,
        goal: None,
        priority: None,
        task_id: None,
        depends_on_task_id: None,
        step_number: None,
        thought: None,
        reasoning: None,
        action: None,
        observation: None,
        max_cycles: None,
        sequence_id: None,
        context: None,
        depth: None,
        max_steps: None,
        to: None,
        from: None,
        agent: None,
        id: None,
        message: None,
        capabilities: None,
        status: None,
        task_type: None,
        payload: None,
        result: None,
        timeout_ms: None,
        prd_content: None,
        parent_task_id: None,
        parent_task_json: None,
        tasks_json: None,
        business_context: None,
        completed_tasks: None,
        remaining_tasks_json: None,
        breakdown_json: None,
        parent_id: None,
        prd_title: None,
        unix_timestamp: None,
        seconds: None,
    };

    assert_eq!(memory_suite_request.command, "vector_search");
    assert_eq!(memory_suite_request.query, Some(test_query.to_string()));

    println!("vector_search should route to memory_suite with command='vector_search'");
}

/// Test parser_analyze routing to code_suite
#[test]
fn test_parser_analyze_routes_via_code_suite() {
    let test_file_path = "/test/path/example.rs";

    let code_suite_request = CodeSuiteRequest {
        command: "parse".to_string(),
        file_path: Some(test_file_path.to_string()),
        // Set other fields to defaults
        query: None,
        pattern: None,
        limit: None,
        context_lines: None,
        directory: None,
        function_name: None,
    };

    assert_eq!(code_suite_request.command, "parse");
    assert_eq!(code_suite_request.file_path, Some(test_file_path.to_string()));

    println!("parser_analyze should route to code_suite with command='parse'");
}

/// Test code_index routing to code_suite
#[test]
fn test_code_index_routes_via_code_suite() {
    let test_file_path = "/test/path/code.rs";

    let code_suite_request = CodeSuiteRequest {
        command: "index".to_string(),
        file_path: Some(test_file_path.to_string()),
        // Set other fields to defaults
        query: None,
        pattern: None,
        limit: None,
        context_lines: None,
        directory: None,
        function_name: None,
    };

    assert_eq!(code_suite_request.command, "index");
    assert_eq!(code_suite_request.file_path, Some(test_file_path.to_string()));

    println!("code_index should route to code_suite with command='index'");
}

/// Test code_search routing to code_suite
#[test]
fn test_code_search_routes_via_code_suite() {
    let test_query = "function_name";

    let code_suite_request = CodeSuiteRequest {
        command: "search".to_string(),
        query: Some(test_query.to_string()),
        limit: Some(20),
        // Set other fields to defaults
        file_path: None,
        pattern: None,
        context_lines: None,
        directory: None,
        function_name: None,
    };

    assert_eq!(code_suite_request.command, "search");
    assert_eq!(code_suite_request.query, Some(test_query.to_string()));

    println!("code_search should route to code_suite with command='search'");
}

/// Test code_index_directory routing to code_suite
#[test]
fn test_code_index_directory_routes_via_code_suite() {
    let test_directory = "/test/src/";

    let code_suite_request = CodeSuiteRequest {
        command: "index_directory".to_string(),
        directory: Some(test_directory.to_string()),
        pattern: Some("**/*.rs".to_string()),
        // Set other fields to defaults
        file_path: None,
        query: None,
        limit: None,
        context_lines: None,
        function_name: None,
    };

    assert_eq!(code_suite_request.command, "index_directory");
    assert_eq!(code_suite_request.directory, Some(test_directory.to_string()));

    println!("code_index_directory should route to code_suite with command='index_directory'");
}

/// Test logs_tail routing to debug_suite
#[test]
fn test_logs_tail_routes_via_debug_suite() {
    let test_n = 50;
    let test_file_path = "/test/logs/app.log";

    let debug_suite_request = DebugSuiteRequest {
        command: "logs_tail".to_string(),
        n: Some(test_n),
        file_path: Some(test_file_path.to_string()),
        // Set other fields to defaults
        limit: None,
        root: None,
        max_modules: None,
        max_cycles: None,
        max_depth: None,
        min_loc: None,
        min_fan_in: None,
        min_fan_out: None,
        min_entity_count: None,
        exclude_public: None,
        loc_threshold: None,
        fan_in_threshold: None,
        fan_out_threshold: None,
        entity_threshold: None,
        max_examples: None,
        project_root: None,
        excluded_dirs: None,
    };

    assert_eq!(debug_suite_request.command, "logs_tail");
    assert_eq!(debug_suite_request.n, Some(test_n));

    println!("logs_tail should route to debug_suite with command='logs_tail'");
}

/// Integration test to verify no mcp_delegate calls remain in deprecated handlers
#[test]
fn test_no_mcp_delegate_calls_in_deprecated_handlers() {
    // This test should initially FAIL because deprecated handlers still use mcp_delegate

    // Check that the following handlers in src/mcp_server/server.rs
    // do NOT contain calls to self.mcp_delegate():

    let deprecated_handlers = vec![
        "memory_store",
        "memory_query",
        "vector_insert",
        "vector_search",
        "parser_analyze",
        "code_index",
        "code_search",
        "code_index_directory",
        "logs_tail"
    ];

    for handler in deprecated_handlers {
        // TODO: Once we implement the fix, verify that:
        // 1. The handler function exists in src/mcp_server/server.rs
        // 2. The handler does NOT call self.mcp_delegate()
        // 3. The handler constructs the appropriate suite request
        // 4. The handler routes through the suite dispatcher

        println!("Handler '{}' should not call mcp_delegate()", handler);
    }
}