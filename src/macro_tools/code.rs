//! Code Macro Tool
//!
//! Provides intelligent orchestration for code-related operations.
//! Routes to underlying tools based on action:
//! - semantic_search → mapping_search + code_search + vector_search (3-step)
//! - analyze_module → parser_analyze + mapping_deps + code_search (3-step)
//! - index_directory → code_index_directory + mapping_record (2-step)
//! - analyze → parser_analyze (simple routing)
//! - search → code_search (simple routing)
//! - index → code_index (simple routing)

use crate::macro_tools::planner::{CodeMacroPlan, ExecutionRecorder};
use anyhow::Result;
use serde_json::Value;

/// Execute a code macro request with intelligent orchestration
pub fn execute_code_macro<R: ExecutionRecorder>(params: &Value, recorder: &R) -> Result<()> {
    let action = params
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required field: action"))?;

    // Check if this is an intelligent multi-step action
    match action {
        "semantic_search" | "analyze_module" | "index_directory" => {
            // Create and execute multi-step plan
            let plan = CodeMacroPlan::from_request(params)?;
            for (tool_name, tool_params) in plan.get_steps() {
                recorder.record_step(&tool_name, tool_params);
            }
            Ok(())
        }
        // Simple routing actions (single tool calls)
        "analyze" => {
            let file_path = params
                .get("file_path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing required field: file_path"))?;
            recorder.record_step(
                "parser_analyze",
                serde_json::json!({ "file_path": file_path }),
            );
            Ok(())
        }
        "search" => {
            let pattern = params
                .get("pattern")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing required field: pattern"))?;
            recorder.record_step("parser_search", serde_json::json!({ "pattern": pattern }));
            Ok(())
        }
        "index" => {
            let file_path = params
                .get("file_path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing required field: file_path"))?;
            recorder.record_step("code_index", serde_json::json!({ "file_path": file_path }));
            Ok(())
        }
        _ => Err(anyhow::anyhow!(
            "Invalid action for syncore.code: {}",
            action
        )),
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
            self.calls
                .lock()
                .unwrap()
                .push((tool_name.to_string(), params));
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
    fn test_semantic_search_orchestration() {
        let recorder = TestRecorder::new();
        let params = json!({
            "action": "semantic_search",
            "query": "find async message bus",
            "limit": 5
        });

        execute_code_macro(&params, &recorder).unwrap();

        let calls = recorder.get_calls();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].0, "mapping_search");
        assert_eq!(calls[1].0, "code_search");
        assert_eq!(calls[2].0, "vector_search");
    }

    #[test]
    fn test_analyze_module_orchestration() {
        let recorder = TestRecorder::new();
        let params = json!({
            "action": "analyze_module",
            "file_path": "/src/main.rs",
            "focus": "error handling"
        });

        execute_code_macro(&params, &recorder).unwrap();

        let calls = recorder.get_calls();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].0, "parser_analyze");
        assert_eq!(calls[1].0, "mapping_deps");
        assert_eq!(calls[2].0, "code_search");
    }

    #[test]
    fn test_simple_analyze_routing() {
        let recorder = TestRecorder::new();
        let params = json!({
            "action": "analyze",
            "file_path": "/src/main.rs"
        });

        execute_code_macro(&params, &recorder).unwrap();

        let calls = recorder.get_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "parser_analyze");
    }
}
