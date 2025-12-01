//! Task Macro Tool
//!
//! Provides intelligent orchestration for task-related operations.
//! Routes to underlying tools based on action:
//! - next → task_statistics + next_ready + prioritize (3-step)
//! - bootstrap_from_prd → generate + save + subtasks (3-step)
//! - complete → update_status + subtask_stats + next_ready (3-step)
//! - create → task_create (simple routing)
//! - list → intellitask_list (simple routing)
//! - get → intellitask_get (simple routing)
//! - update_status → intellitask_update_status (simple routing)
//! - next_ready → intellitask_next_ready (simple routing)
//! - get_subtasks → intellitask_get_subtasks (simple routing)
//! - subtask_stats → intellitask_subtask_stats (simple routing)
//! - statistics → intellitask_task_statistics (simple routing)
//! - prd_statistics → intellitask_prd_statistics (simple routing)

use crate::macro_tools::planner::{ExecutionRecorder, TaskMacroPlan};
use anyhow::Result;
use serde_json::Value;

/// Execute a task macro request with intelligent orchestration
pub fn execute_task_macro<R: ExecutionRecorder>(params: &Value, recorder: &R) -> Result<()> {
    let action = params
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required field: action"))?;

    // Check if this is an intelligent multi-step action
    match action {
        "next" | "bootstrap_from_prd" | "complete" => {
            // Create and execute multi-step plan
            let plan = TaskMacroPlan::from_request(params)?;
            for (tool_name, tool_params) in plan.get_steps() {
                recorder.record_step(&tool_name, tool_params);
            }
            Ok(())
        }
        // Simple routing actions (single tool calls)
        "create" => {
            let goal = params
                .get("goal")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing required field: goal"))?;
            recorder.record_step("task_create", serde_json::json!({ "goal": goal }));
            Ok(())
        }
        "list" => {
            recorder.record_step("intellitask_list", params.clone());
            Ok(())
        }
        "get" => {
            let task_id = params
                .get("task_id")
                .ok_or_else(|| anyhow::anyhow!("Missing required field: task_id"))?;
            recorder.record_step("intellitask_get", serde_json::json!({ "task_id": task_id }));
            Ok(())
        }
        "update_status" => {
            recorder.record_step("intellitask_update_status", params.clone());
            Ok(())
        }
        "next_ready" => {
            recorder.record_step("intellitask_next_ready", serde_json::json!({}));
            Ok(())
        }
        "get_subtasks" => {
            recorder.record_step("intellitask_get_subtasks", params.clone());
            Ok(())
        }
        "subtask_stats" => {
            recorder.record_step("intellitask_subtask_stats", params.clone());
            Ok(())
        }
        "statistics" => {
            recorder.record_step("intellitask_task_statistics", serde_json::json!({}));
            Ok(())
        }
        "prd_statistics" => {
            recorder.record_step("intellitask_prd_statistics", params.clone());
            Ok(())
        }
        _ => Err(anyhow::anyhow!("Invalid action for syncore.task: {}", action)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    struct TestRecorder {
        calls: Arc<Mutex<Vec<(String, Value)>>>,
    }

    impl TestRecorder {
        fn new() -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn get_calls(&self) -> Vec<(String, Value)> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl ExecutionRecorder for TestRecorder {
        fn record_step(&self, tool_name: &str, params: Value) {
            self.calls.lock().unwrap().push((tool_name.to_string(), params));
        }

        fn wrap_success(&self, _tool: &str, data: Value) -> Value {
            data
        }

        fn wrap_error(&self, _tool: &str, error: &str) -> Value {
            serde_json::json!({"error": error})
        }

        fn executor_type(&self) -> &str {
            "test"
        }
    }

    #[test]
    fn test_next_task_orchestration() {
        let recorder = TestRecorder::new();
        let params = json!({
            "action": "next",
            "prd_title": "Test PRD",
            "strategy": "priority"
        });

        execute_task_macro(&params, &recorder).unwrap();

        let calls = recorder.get_calls();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].0, "intellitask_task_statistics");
        assert_eq!(calls[1].0, "intellitask_next_ready");
        assert_eq!(calls[2].0, "intellitask_prioritize");
    }

    #[test]
    fn test_bootstrap_from_prd_orchestration() {
        let recorder = TestRecorder::new();
        let params = json!({
            "action": "bootstrap_from_prd",
            "prd_text": "Build a feature",
            "auto_expand": true
        });

        execute_task_macro(&params, &recorder).unwrap();

        let calls = recorder.get_calls();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].0, "intellitask_generate");
        assert_eq!(calls[1].0, "intellitask_save");
        assert_eq!(calls[2].0, "intellitask_subtasks");
    }

    #[test]
    fn test_complete_task_orchestration() {
        let recorder = TestRecorder::new();
        let params = json!({
            "action": "complete",
            "task_id": 42,
            "suggest_next": true
        });

        execute_task_macro(&params, &recorder).unwrap();

        let calls = recorder.get_calls();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].0, "intellitask_update_status");
        assert_eq!(calls[1].0, "intellitask_subtask_stats");
        assert_eq!(calls[2].0, "intellitask_next_ready");
    }

    #[test]
    fn test_simple_create_routing() {
        let recorder = TestRecorder::new();
        let params = json!({
            "action": "create",
            "goal": "Test task"
        });

        execute_task_macro(&params, &recorder).unwrap();

        let calls = recorder.get_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "task_create");
    }
}
