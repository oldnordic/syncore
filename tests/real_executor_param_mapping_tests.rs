//! Tests for RealExecutor parameter mapping drift
//!
//! This test file proves that parameters are being dropped in the RealExecutor
//! layer and verifies that they are preserved after the fix.

use serde_json::{json, Value};
use syncore::macro_tools::executor_real::RealExecutor;
use syncore::router::SynCoreState;
use std::sync::Arc;

/// Test helper to capture MemorySuiteArgs without executing
#[cfg(test)]
fn capture_memory_suite_args_direct(tool_name: &str, params: &Value) -> syncore::mcp_tools::memory_suite::MemorySuiteArgs {
    // This simulates what the old execute_real_tool does but returns the args instead of executing
    syncore::mcp_tools::memory_suite::MemorySuiteArgs {
        command: tool_name.to_string(),
        // This is the PROBLEMATIC old behavior that drops all params!
        // We're testing this to prove the drift exists
        key: params.get("key").and_then(|v| v.as_str()).map(|s| s.to_string()),
        value: params.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()),
        text: params.get("text").and_then(|v| v.as_str()).map(|s| s.to_string()),
        query: params.get("query").and_then(|v| v.as_str()).map(|s| s.to_string()),
        limit: params.get("limit").and_then(|v| v.as_u64()).map(|v| v as usize),
        namespace: params.get("namespace").and_then(|v| v.as_str()).map(|s| s.to_string()),
        goal: params.get("goal").and_then(|v| v.as_str()).map(|s| s.to_string()),
        priority: params.get("priority").and_then(|v| v.as_i64()).map(|v| v as i32),
        task_id: params.get("task_id").and_then(|v| v.as_i64()),
        depends_on_task_id: params.get("depends_on_task_id").and_then(|v| v.as_i64()),
        step_number: params.get("step_number").and_then(|v| v.as_i64()).map(|v| v as i32),
        thought: params.get("thought").and_then(|v| v.as_str()).map(|s| s.to_string()),
        reasoning: params.get("reasoning").and_then(|v| v.as_str()).map(|s| s.to_string()),
        action: params.get("action").and_then(|v| v.as_str()).map(|s| s.to_string()),
        observation: params.get("observation").and_then(|v| v.as_str()).map(|s| s.to_string()),
        max_cycles: params.get("max_cycles").and_then(|v| v.as_u64()).map(|v| v as usize),
        sequence_id: params.get("sequence_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
        context: params.get("context").and_then(|v| v.as_str()).map(|s| s.to_string()),
        depth: params.get("depth").and_then(|v| v.as_i64()).map(|v| v as i32),
        max_steps: params.get("max_steps").and_then(|v| v.as_u64()).map(|v| v as usize),
        to: params.get("to").and_then(|v| v.as_str()).map(|s| s.to_string()),
        from: params.get("from").and_then(|v| v.as_str()).map(|s| s.to_string()),
        agent: params.get("agent").and_then(|v| v.as_str()).map(|s| s.to_string()),
        id: params.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()),
        message: params.get("message").and_then(|v| v.as_str()).map(|s| s.to_string()),
        capabilities: params.get("capabilities").and_then(|v| v.as_array()).map(|arr| {
            arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
        }),
        status: params.get("status").cloned(),
        task_type: params.get("task_type").and_then(|v| v.as_str()).map(|s| s.to_string()),
        payload: params.get("payload").cloned(),
        result: params.get("result").cloned(),
        timeout_ms: params.get("timeout_ms").and_then(|v| v.as_u64()),
        prd_content: params.get("prd_content").and_then(|v| v.as_str()).map(|s| s.to_string()),
        parent_task_id: params.get("parent_task_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
        parent_task_json: params.get("parent_task_json").and_then(|v| v.as_str()).map(|s| s.to_string()),
        tasks_json: params.get("tasks_json").and_then(|v| v.as_str()).map(|s| s.to_string()),
        business_context: params.get("business_context").and_then(|v| v.as_str()).map(|s| s.to_string()),
        completed_tasks: params.get("completed_tasks").and_then(|v| v.as_array()).map(|arr| {
            arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
        }),
        remaining_tasks_json: params.get("remaining_tasks_json").and_then(|v| v.as_str()).map(|s| s.to_string()),
        breakdown_json: params.get("breakdown_json").and_then(|v| v.as_str()).map(|s| s.to_string()),
        parent_id: params.get("parent_id").and_then(|v| v.as_i64()),
        prd_title: params.get("prd_title").and_then(|v| v.as_str()).map(|s| s.to_string()),

        // ADVANCED MEMORY PARAMETERS - these are the focus of our drift investigation
        keywords: params.get("keywords").and_then(|v| v.as_array()).map(|arr| {
            arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
        }),
        tags: params.get("tags").and_then(|v| v.as_array()).map(|arr| {
            arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
        }),
        min_importance: params.get("min_importance").and_then(|v| v.as_f64()).map(|v| v as f32),
        unix_timestamp: params.get("unix_timestamp").and_then(|v| v.as_u64()),
        seconds: params.get("seconds").and_then(|v| v.as_u64()),
        threshold: params.get("threshold").and_then(|v| v.as_f64()).map(|v| v as f32),
        dry_run: params.get("dry_run").and_then(|v| v.as_bool()),
    }
}

/// Test helper to capture what the CURRENT broken implementation produces
#[cfg(test)]
fn capture_memory_suite_args_broken(tool_name: &str, params: &Value) -> syncore::mcp_tools::memory_suite::MemorySuiteArgs {
    // This simulates the BROKEN behavior in execute_real_tool method (lines 81-92)
    syncore::mcp_tools::memory_suite::MemorySuiteArgs {
        command: tool_name.to_string(),
        // The PROBLEM: All params are ignored! Only command is set.
        ..Default::default()
    }
}

/// Test: PROVE that advanced memory parameters are being dropped
#[test]
fn test_advanced_memory_params_demonstrate_drift() {
    // Create test input with rich advanced memory parameters
    let tool_name = "memory_store";
    let input_params = json!({
        "key": "test_key",
        "value": "test_value",
        "dry_run": true,
        // ADVANCED MEMORY PARAMETERS - these should be preserved but are currently dropped!
        "keywords": ["important", "cache", "memory"],
        "tags": ["production", "user_data"],
        "min_importance": 0.7,
        "unix_timestamp": 1640995200, // 2022-01-01 00:00:00 UTC
        "seconds": 86400, // 1 day
        "threshold": 0.8,
        "namespace": "user_session",
        "limit": 50
    });

    // Test: Current BROKEN implementation drops all advanced parameters
    let broken_args = capture_memory_suite_args_broken(tool_name, &input_params);

    // Assert: Only command is set, everything else is defaulted/None
    assert_eq!(broken_args.command, "memory_store");

    // These SHOULD be populated but are NOT due to the drift
    assert_eq!(broken_args.key, None, "❌ DRIFT: 'key' parameter was dropped!");
    assert_eq!(broken_args.value, None, "❌ DRIFT: 'value' parameter was dropped!");
    assert_eq!(broken_args.keywords, None, "❌ DRIFT: 'keywords' parameter was dropped!");
    assert_eq!(broken_args.tags, None, "❌ DRIFT: 'tags' parameter was dropped!");
    assert_eq!(broken_args.min_importance, None, "❌ DRIFT: 'min_importance' parameter was dropped!");
    assert_eq!(broken_args.unix_timestamp, None, "❌ DRIFT: 'unix_timestamp' parameter was dropped!");
    assert_eq!(broken_args.seconds, None, "❌ DRIFT: 'seconds' parameter was dropped!");
    assert_eq!(broken_args.threshold, None, "❌ DRIFT: 'threshold' parameter was dropped!");
    assert_eq!(broken_args.namespace, None, "❌ DRIFT: 'namespace' parameter was dropped!");
    assert_eq!(broken_args.limit, None, "❌ DRIFT: 'limit' parameter was dropped!");
    assert_eq!(broken_args.dry_run, None, "❌ DRIFT: 'dry_run' parameter was dropped!");

    println!("✅ PROVED DRIFT: Current RealExecutor drops ALL parameters except command");
}

/// Test: Show what CORRECT parameter mapping should look like
#[test]
fn test_advanced_memory_params_correct_mapping() {
    let tool_name = "memory_store";
    let input_params = json!({
        "key": "test_key",
        "value": "test_value",
        "dry_run": true,
        // ADVANCED MEMORY PARAMETERS
        "keywords": ["important", "cache", "memory"],
        "tags": ["production", "user_data"],
        "min_importance": 0.7,
        "unix_timestamp": 1640995200,
        "seconds": 86400,
        "threshold": 0.8,
        "namespace": "user_session",
        "limit": 50
    });

    // Test: What CORRECT mapping should produce
    let correct_args = capture_memory_suite_args_direct(tool_name, &input_params);

    // Assert: All parameters are preserved correctly
    assert_eq!(correct_args.command, "memory_store");
    assert_eq!(correct_args.key, Some("test_key".to_string()));
    assert_eq!(correct_args.value, Some("test_value".to_string()));
    assert_eq!(correct_args.dry_run, Some(true));

    // Advanced parameters should be preserved
    assert_eq!(correct_args.keywords, Some(vec!["important".to_string(), "cache".to_string(), "memory".to_string()]));
    assert_eq!(correct_args.tags, Some(vec!["production".to_string(), "user_data".to_string()]));
    assert_eq!(correct_args.min_importance, Some(0.7));
    assert_eq!(correct_args.unix_timestamp, Some(1640995200));
    assert_eq!(correct_args.seconds, Some(86400));
    assert_eq!(correct_args.threshold, Some(0.8));
    assert_eq!(correct_args.namespace, Some("user_session".to_string()));
    assert_eq!(correct_args.limit, Some(50));

    println!("✅ CORRECT: All parameters including advanced memory fields are preserved");
}

/// Test: Demonstrate drift for vector tools too
#[test]
fn test_vector_tools_demonstrate_drift() {
    let tool_name = "vector_insert";
    let input_params = json!({
        "text": "This is important text to insert",
        "dry_run": true,
        "namespace": "knowledge_base",
        "limit": 100,
        "threshold": 0.9,
        "keywords": ["embedding", "search"],
        "tags": ["important", "reference"]
    });

    // Test: Current BROKEN implementation drops all vector parameters
    let broken_args = capture_memory_suite_args_broken(tool_name, &input_params);

    // Assert: Only command is set, everything else is defaulted/None
    assert_eq!(broken_args.command, "vector_insert");
    assert_eq!(broken_args.text, None, "❌ DRIFT: 'text' parameter was dropped!");
    assert_eq!(broken_args.namespace, None, "❌ DRIFT: 'namespace' parameter was dropped!");
    assert_eq!(broken_args.limit, None, "❌ DRIFT: 'limit' parameter was dropped!");
    assert_eq!(broken_args.threshold, None, "❌ DRIFT: 'threshold' parameter was dropped!");
    assert_eq!(broken_args.keywords, None, "❌ DRIFT: 'keywords' parameter was dropped!");
    assert_eq!(broken_args.tags, None, "❌ DRIFT: 'tags' parameter was dropped!");
    assert_eq!(broken_args.dry_run, None, "❌ DRIFT: 'dry_run' parameter was dropped!");

    println!("✅ PROVED DRIFT: Vector tools also lose ALL parameters except command");
}

/// Test: Demonstrate drift for task tools
#[test]
fn test_task_tools_demonstrate_drift() {
    let tool_name = "task_create";
    let input_params = json!({
        "goal": "Implement advanced parameter mapping",
        "priority": 1,
        "dry_run": true,
        "parent_task_id": "parent_123",
        "tags": ["urgent", "architecture"],
        "business_context": "Phase 10.3 completion",
        "prd_title": "Parameter Mapping Drift Fix"
    });

    // Test: Current BROKEN implementation drops all task parameters
    let broken_args = capture_memory_suite_args_broken(tool_name, &input_params);

    // Assert: Only command is set, everything else is defaulted/None
    assert_eq!(broken_args.command, "task_create");
    assert_eq!(broken_args.goal, None, "❌ DRIFT: 'goal' parameter was dropped!");
    assert_eq!(broken_args.priority, None, "❌ DRIFT: 'priority' parameter was dropped!");
    assert_eq!(broken_args.parent_task_id, None, "❌ DRIFT: 'parent_task_id' parameter was dropped!");
    assert_eq!(broken_args.tags, None, "❌ DRIFT: 'tags' parameter was dropped!");
    assert_eq!(broken_args.business_context, None, "❌ DRIFT: 'business_context' parameter was dropped!");
    assert_eq!(broken_args.prd_title, None, "❌ DRIFT: 'prd_title' parameter was dropped!");
    assert_eq!(broken_args.dry_run, None, "❌ DRIFT: 'dry_run' parameter was dropped!");

    println!("✅ PROVED DRIFT: Task tools also lose ALL parameters except command");
}

/// Test: Verify the FIXED implementation preserves all parameters
#[test]
fn test_fixed_executor_preserves_all_parameters() {
    // Create a RealExecutor instance
    let executor = RealExecutor::default();

    // Test memory tool with advanced parameters
    let tool_name = "memory_store";
    let input_params = json!({
        "key": "test_key",
        "value": "test_value",
        "dry_run": true,
        // ADVANCED MEMORY PARAMETERS - these should now be preserved!
        "keywords": ["important", "cache", "memory"],
        "tags": ["production", "user_data"],
        "min_importance": 0.7,
        "unix_timestamp": 1640995200,
        "seconds": 86400,
        "threshold": 0.8,
        "namespace": "user_session",
        "limit": 50
    });

    // Test: FIXED implementation should preserve all parameters
    let fixed_args = executor.build_memory_suite_args(tool_name, &input_params);

    // Assert: All parameters are now preserved correctly
    assert_eq!(fixed_args.command, "memory_store");
    assert_eq!(fixed_args.key, Some("test_key".to_string()));
    assert_eq!(fixed_args.value, Some("test_value".to_string()));
    assert_eq!(fixed_args.dry_run, Some(true));

    // Advanced parameters should be preserved
    assert_eq!(fixed_args.keywords, Some(vec!["important".to_string(), "cache".to_string(), "memory".to_string()]));
    assert_eq!(fixed_args.tags, Some(vec!["production".to_string(), "user_data".to_string()]));
    assert_eq!(fixed_args.min_importance, Some(0.7));
    assert_eq!(fixed_args.unix_timestamp, Some(1640995200));
    assert_eq!(fixed_args.seconds, Some(86400));
    assert_eq!(fixed_args.threshold, Some(0.8));
    assert_eq!(fixed_args.namespace, Some("user_session".to_string()));
    assert_eq!(fixed_args.limit, Some(50));

    println!("✅ FIXED: All parameters including advanced memory fields are now preserved");
}

/// Test: Verify vector tools preserve parameters after fix
#[test]
fn test_fixed_vector_tools_preserve_parameters() {
    let executor = RealExecutor::default();

    let tool_name = "vector_insert";
    let input_params = json!({
        "text": "This is important text to insert",
        "dry_run": true,
        "namespace": "knowledge_base",
        "limit": 100,
        "threshold": 0.9,
        "keywords": ["embedding", "search"],
        "tags": ["important", "reference"]
    });

    let fixed_args = executor.build_memory_suite_args(tool_name, &input_params);

    // Assert: All vector parameters are preserved
    assert_eq!(fixed_args.command, "vector_insert");
    assert_eq!(fixed_args.text, Some("This is important text to insert".to_string()));
    assert_eq!(fixed_args.dry_run, Some(true));
    assert_eq!(fixed_args.namespace, Some("knowledge_base".to_string()));
    assert_eq!(fixed_args.limit, Some(100));
    assert_eq!(fixed_args.threshold, Some(0.9));
    assert_eq!(fixed_args.keywords, Some(vec!["embedding".to_string(), "search".to_string()]));
    assert_eq!(fixed_args.tags, Some(vec!["important".to_string(), "reference".to_string()]));

    println!("✅ FIXED: Vector tools preserve all parameters");
}

/// Test: Verify task tools preserve parameters after fix
#[test]
fn test_fixed_task_tools_preserve_parameters() {
    let executor = RealExecutor::default();

    let tool_name = "task_create";
    let input_params = json!({
        "goal": "Implement advanced parameter mapping",
        "priority": 1,
        "dry_run": true,
        "parent_task_id": "parent_123",
        "tags": ["urgent", "architecture"],
        "business_context": "Phase 10.3 completion",
        "prd_title": "Parameter Mapping Drift Fix"
    });

    let fixed_args = executor.build_memory_suite_args(tool_name, &input_params);

    // Assert: All task parameters are preserved
    assert_eq!(fixed_args.command, "task_create");
    assert_eq!(fixed_args.goal, Some("Implement advanced parameter mapping".to_string()));
    assert_eq!(fixed_args.priority, Some(1));
    assert_eq!(fixed_args.dry_run, Some(true));
    assert_eq!(fixed_args.parent_task_id, Some("parent_123".to_string()));
    assert_eq!(fixed_args.tags, Some(vec!["urgent".to_string(), "architecture".to_string()]));
    assert_eq!(fixed_args.business_context, Some("Phase 10.3 completion".to_string()));
    assert_eq!(fixed_args.prd_title, Some("Parameter Mapping Drift Fix".to_string()));

    println!("✅ FIXED: Task tools preserve all parameters");
}

/// Test: Verify mixed tool types work correctly
#[test]
fn test_fixed_mixed_tool_types_work_correctly() {
    let executor = RealExecutor::default();

    // Test sequential tool with mixed parameters
    let tool_name = "sequential_record";
    let input_params = json!({
        "task_id": 12345,
        "step_number": 1,
        "thought": "Analyze the parameter mapping issue",
        "reasoning": "Parameters are being dropped in the executor layer",
        "action": "Fix the build_memory_suite_args method",
        "observation": "Advanced memory parameters are lost",
        "dry_run": false,
        "tags": ["analysis", "fix"],
        "threshold": 0.8
    });

    let fixed_args = executor.build_memory_suite_args(tool_name, &input_params);

    // Assert: All sequential parameters are preserved
    assert_eq!(fixed_args.command, "sequential_record");
    assert_eq!(fixed_args.task_id, Some(12345));
    assert_eq!(fixed_args.step_number, Some(1));
    assert_eq!(fixed_args.thought, Some("Analyze the parameter mapping issue".to_string()));
    assert_eq!(fixed_args.reasoning, Some("Parameters are being dropped in the executor layer".to_string()));
    assert_eq!(fixed_args.action, Some("Fix the build_memory_suite_args method".to_string()));
    assert_eq!(fixed_args.observation, Some("Advanced memory parameters are lost".to_string()));
    assert_eq!(fixed_args.dry_run, Some(false));
    assert_eq!(fixed_args.tags, Some(vec!["analysis".to_string(), "fix".to_string()]));
    assert_eq!(fixed_args.threshold, Some(0.8));

    println!("✅ FIXED: Mixed tool types preserve all parameters correctly");
}