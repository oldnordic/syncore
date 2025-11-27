//! Application/Code Change Tracking Tools Executor
//!
//! DEPRECATED: These tools are legacy wrappers that route through mapping_suite.
//! Extracted from executor_real.rs giant match statement (lines 341-470).
//!
//! Tools:
//! - application_record: Record a code change in the application
//! - application_get: Get all code changes for a task
//! - application_history: Get change history for a specific file
//! - application_search: Search code changes by semantic content
//!
//! All tools delegate to MappingSuite with app_* commands.

use crate::mcp_tools::mapping_suite::{MappingSuite, MappingSuiteArgs};
use crate::mcp_tools::SuiteResult;
use crate::router::SynCoreState;
use serde_json::Value;
use std::sync::Arc;

/// Execute application_record tool
/// DEPRECATED: Routes through mapping_suite with command="app_record"
pub async fn execute_application_record(
    state: &Arc<SynCoreState>,
    params: &Value,
) -> SuiteResult {
    let suite_args = MappingSuiteArgs {
        command: "app_record".to_string(),
        path: None,
        kind: None,
        language: None,
        imports: None,
        exports: None,
        dependencies: None,
        query: None,
        file_path: params
            .get("file_path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        change_type: params
            .get("change_type")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        old_content: params
            .get("old_content")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        new_content: params
            .get("new_content")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        line_start: params
            .get("line_start")
            .and_then(|v| v.as_i64())
            .map(|i| i as i32),
        line_end: params
            .get("line_end")
            .and_then(|v| v.as_i64())
            .map(|i| i as i32),
        description: params
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        task_id: params.get("task_id").and_then(|v| v.as_i64()),
    };

    let suite = MappingSuite::new((**state).clone());
    suite.execute(suite_args)
}

/// Execute application_get tool
/// DEPRECATED: Routes through mapping_suite with command="app_get"
pub async fn execute_application_get(
    state: &Arc<SynCoreState>,
    params: &Value,
) -> SuiteResult {
    let suite_args = MappingSuiteArgs {
        command: "app_get".to_string(),
        path: None,
        kind: None,
        language: None,
        imports: None,
        exports: None,
        dependencies: None,
        query: None,
        file_path: None,
        change_type: None,
        old_content: None,
        new_content: None,
        line_start: None,
        line_end: None,
        description: None,
        task_id: params.get("task_id").and_then(|v| v.as_i64()),
    };

    let suite = MappingSuite::new((**state).clone());
    suite.execute(suite_args)
}

/// Execute application_history tool
/// DEPRECATED: Routes through mapping_suite with command="app_history"
pub async fn execute_application_history(
    state: &Arc<SynCoreState>,
    params: &Value,
) -> SuiteResult {
    let suite_args = MappingSuiteArgs {
        command: "app_history".to_string(),
        path: None,
        kind: None,
        language: None,
        imports: None,
        exports: None,
        dependencies: None,
        query: None,
        file_path: params
            .get("file_path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        change_type: None,
        old_content: None,
        new_content: None,
        line_start: None,
        line_end: None,
        description: None,
        task_id: None,
    };

    let suite = MappingSuite::new((**state).clone());
    suite.execute(suite_args)
}

/// Execute application_search tool
/// DEPRECATED: Routes through mapping_suite with command="app_search"
pub async fn execute_application_search(
    state: &Arc<SynCoreState>,
    params: &Value,
) -> SuiteResult {
    let suite_args = MappingSuiteArgs {
        command: "app_search".to_string(),
        path: None,
        kind: None,
        language: None,
        imports: None,
        exports: None,
        dependencies: None,
        query: params
            .get("query")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        file_path: None,
        change_type: None,
        old_content: None,
        new_content: None,
        line_start: None,
        line_end: None,
        description: None,
        task_id: None,
    };

    let suite = MappingSuite::new((**state).clone());
    suite.execute(suite_args)
}
