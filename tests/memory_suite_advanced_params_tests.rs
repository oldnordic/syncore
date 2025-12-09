//! Tests for MemorySuite advanced parameter propagation
//!
//! This test file verifies that advanced memory parameters are properly
//! defined in MemorySuiteRequest struct.

use syncore::mcp_server::types::MemorySuiteRequest;

#[test]
fn test_memory_suite_struct_has_advanced_fields() {
    // Test that MemorySuiteRequest struct can be instantiated with advanced fields
    let request = MemorySuiteRequest {
        command: "search_semantic".to_string(),
        query: Some("test query".to_string()),
        keywords: Some(vec!["rust".to_string(), "async".to_string()]),
        tags: Some(vec!["important".to_string(), "cache".to_string()]),
        min_importance: Some(0.7),
        unix_timestamp: Some(1640995200),
        seconds: Some(86400),
        threshold: Some(0.8),
        // Set other required fields to default values
        key: None,
        value: None,
        text: None,
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
        dry_run: Some(true),
    };

    // Verify the fields are set correctly
    assert_eq!(request.command, "search_semantic");
    assert_eq!(request.query, Some("test query".to_string()));
    assert_eq!(request.keywords, Some(vec!["rust".to_string(), "async".to_string()]));
    assert_eq!(request.tags, Some(vec!["important".to_string(), "cache".to_string()]));
    assert_eq!(request.min_importance, Some(0.7));
    assert_eq!(request.unix_timestamp, Some(1640995200));
    assert_eq!(request.seconds, Some(86400));
    assert_eq!(request.threshold, Some(0.8));
}

#[test]
fn test_memory_suite_min_importance_and_threshold_fields() {
    // Test that min_importance and threshold parameters are properly defined
    let request = MemorySuiteRequest {
        command: "query_by_importance".to_string(),
        min_importance: Some(0.7),
        threshold: Some(0.8),
        dry_run: Some(true),
        // Set other fields to defaults
        key: None,
        value: None,
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
        keywords: None,
        tags: None,
        unix_timestamp: None,
        seconds: None,
    };

    // Verify the fields are set correctly
    assert_eq!(request.command, "query_by_importance");
    assert_eq!(request.min_importance, Some(0.7));
    assert_eq!(request.threshold, Some(0.8));
}

#[test]
fn test_memory_suite_temporal_fields() {
    // Test that unix_timestamp and seconds parameters are properly defined
    let request = MemorySuiteRequest {
        command: "query_since".to_string(),
        unix_timestamp: Some(1640995200), // 2022-01-01 00:00:00 UTC
        seconds: Some(86400), // 1 day
        dry_run: Some(true),
        // Set other fields to defaults
        key: None,
        value: None,
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
        keywords: None,
        tags: None,
        min_importance: None,
        threshold: None,
    };

    // Verify the fields are set correctly
    assert_eq!(request.command, "query_since");
    assert_eq!(request.unix_timestamp, Some(1640995200));
    assert_eq!(request.seconds, Some(86400));
}

#[test]
fn test_memory_suite_basic_functionality_regression() {
    // Regression test to ensure basic memory functionality still works
    let request = MemorySuiteRequest {
        command: "store".to_string(),
        key: Some("test_key".to_string()),
        value: Some("test_value".to_string()),
        dry_run: Some(true),
        // Set all other fields to None/defaults
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
        keywords: None,
        tags: None,
        min_importance: None,
        unix_timestamp: None,
        seconds: None,
        threshold: None,
    };

    // Verify basic fields are set correctly
    assert_eq!(request.command, "store");
    assert_eq!(request.key, Some("test_key".to_string()));
    assert_eq!(request.value, Some("test_value".to_string()));
    assert_eq!(request.dry_run, Some(true));
}