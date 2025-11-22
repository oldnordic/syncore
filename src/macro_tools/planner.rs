//! Macro Tool Planner
//!
//! Orchestrates multi-step execution plans for intelligent macro tools.
//! Converts high-level macro requests into sequences of underlying tool calls.

use anyhow::Result;
use serde_json::{json, Value};

// ============================================================================
// CODE MACRO PLANS - Smart orchestration for syncore.code
// ============================================================================

/// Multi-step execution plan for code-related operations
#[derive(Debug, Clone, PartialEq)]
pub enum CodeMacroPlan {
    /// Semantic search: mapping_search → code_search → vector_search
    SemanticSearch { query: String, limit: i64 },
    /// Module analysis: parser_analyze → mapping_deps → code_search
    AnalyzeModule { file_path: String, focus: String },
    /// Directory indexing: code_index_directory → mapping_record (per file)
    IndexDirectory { directory: String, pattern: String },
}

impl CodeMacroPlan {
    /// Create a plan from macro request parameters
    pub fn from_request(params: &Value) -> Result<Self> {
        let action = params
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required field: action"))?;

        match action {
            "semantic_search" => {
                let query = params
                    .get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required field: query"))?
                    .to_string();
                let limit = params.get("limit").and_then(|v| v.as_i64()).unwrap_or(10);

                Ok(CodeMacroPlan::SemanticSearch { query, limit })
            }
            "analyze_module" => {
                let file_path = params
                    .get("file_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required field: file_path"))?
                    .to_string();
                let focus = params
                    .get("focus")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required field: focus"))?
                    .to_string();

                Ok(CodeMacroPlan::AnalyzeModule { file_path, focus })
            }
            "index_directory" => {
                let directory = params
                    .get("directory")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required field: directory"))?
                    .to_string();
                let pattern = params
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required field: pattern"))?
                    .to_string();

                Ok(CodeMacroPlan::IndexDirectory { directory, pattern })
            }
            _ => Err(anyhow::anyhow!(
                "Invalid action for syncore.code: {}",
                action
            )),
        }
    }

    /// Get the execution steps for this plan
    pub fn get_steps(&self) -> Vec<(String, Value)> {
        match self {
            CodeMacroPlan::SemanticSearch { query, limit } => {
                vec![
                    ("mapping_search".to_string(), json!({ "query": query })),
                    ("code_search".to_string(), json!({ "query": query })),
                    (
                        "vector_search".to_string(),
                        json!({ "query": query, "limit": limit }),
                    ),
                ]
            }
            CodeMacroPlan::AnalyzeModule { file_path, focus } => {
                vec![
                    (
                        "parser_analyze".to_string(),
                        json!({ "file_path": file_path }),
                    ),
                    ("mapping_deps".to_string(), json!({ "path": file_path })),
                    ("code_search".to_string(), json!({ "query": focus })),
                ]
            }
            CodeMacroPlan::IndexDirectory { directory, pattern } => {
                vec![
                    (
                        "code_index_directory".to_string(),
                        json!({
                            "directory": directory,
                            "pattern": pattern
                        }),
                    ),
                    (
                        "mapping_record".to_string(),
                        json!({
                            "path": format!("{}/mod.rs", directory),
                            "kind": "file"
                        }),
                    ),
                ]
            }
        }
    }
}

// ============================================================================
// TASK MACRO PLANS - Smart orchestration for syncore.task
// ============================================================================

/// Multi-step execution plan for task-related operations
#[derive(Debug, Clone, PartialEq)]
pub enum TaskMacroPlan {
    /// Next task: task_statistics → next_ready → prioritize
    Next {
        prd_title: Option<String>,
        strategy: String,
    },
    /// Bootstrap from PRD: generate → save → subtasks
    BootstrapFromPRD { prd_text: String, auto_expand: bool },
    /// Complete task: update_status → subtask_stats → next_ready
    Complete { task_id: i64, suggest_next: bool },
}

impl TaskMacroPlan {
    /// Create a plan from macro request parameters
    pub fn from_request(params: &Value) -> Result<Self> {
        let action = params
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required field: action"))?;

        match action {
            "next" => {
                let prd_title = params
                    .get("prd_title")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let strategy = params
                    .get("strategy")
                    .and_then(|v| v.as_str())
                    .unwrap_or("priority")
                    .to_string();

                Ok(TaskMacroPlan::Next {
                    prd_title,
                    strategy,
                })
            }
            "bootstrap_from_prd" => {
                let prd_text = params
                    .get("prd_text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required field: prd_text"))?
                    .to_string();
                let auto_expand = params
                    .get("auto_expand")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                Ok(TaskMacroPlan::BootstrapFromPRD {
                    prd_text,
                    auto_expand,
                })
            }
            "complete" => {
                let task_id = params
                    .get("task_id")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| anyhow::anyhow!("Missing required field: task_id"))?;
                let suggest_next = params
                    .get("suggest_next")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                Ok(TaskMacroPlan::Complete {
                    task_id,
                    suggest_next,
                })
            }
            _ => Err(anyhow::anyhow!(
                "Invalid action for syncore.task: {}",
                action
            )),
        }
    }

    /// Get the execution steps for this plan
    pub fn get_steps(&self) -> Vec<(String, Value)> {
        match self {
            TaskMacroPlan::Next { strategy, .. } => {
                vec![
                    ("intellitask_task_statistics".to_string(), json!({})),
                    ("intellitask_next_ready".to_string(), json!({})),
                    (
                        "intellitask_prioritize".to_string(),
                        json!({ "strategy": strategy }),
                    ),
                ]
            }
            TaskMacroPlan::BootstrapFromPRD { prd_text, .. } => {
                vec![
                    (
                        "intellitask_generate".to_string(),
                        json!({ "prd_content": prd_text }),
                    ),
                    (
                        "intellitask_save".to_string(),
                        json!({ "breakdown_json": "{}" }),
                    ),
                    (
                        "intellitask_subtasks".to_string(),
                        json!({ "parent_task_id": "1" }),
                    ),
                ]
            }
            TaskMacroPlan::Complete { task_id, .. } => {
                vec![
                    (
                        "intellitask_update_status".to_string(),
                        json!({
                            "task_id": task_id,
                            "status": "completed"
                        }),
                    ),
                    (
                        "intellitask_subtask_stats".to_string(),
                        json!({ "parent_id": task_id }),
                    ),
                    ("intellitask_next_ready".to_string(), json!({})),
                ]
            }
        }
    }
}

// ============================================================================
// PLAN EXECUTOR - Executes multi-step plans with tracking
// ============================================================================

/// Trait for tracking execution steps (for testing)
pub trait ExecutionRecorder: Send + Sync {
    fn record_step(&self, tool_name: &str, params: Value);

    /// Wrap successful tool result in standard envelope
    fn wrap_success(&self, tool: &str, data: Value) -> Value;

    /// Wrap error in standard envelope
    fn wrap_error(&self, tool: &str, error: &str) -> Value;

    /// Get executor type name ("real" or "stub")
    fn executor_type(&self) -> &str;
}

/// Execute a code macro plan with step recording
pub fn execute_code_plan<R: ExecutionRecorder>(plan: &CodeMacroPlan, recorder: &R) -> Result<()> {
    for (tool_name, params) in plan.get_steps() {
        recorder.record_step(&tool_name, params);
    }
    Ok(())
}

/// Execute a task macro plan with step recording
pub fn execute_task_plan<R: ExecutionRecorder>(plan: &TaskMacroPlan, recorder: &R) -> Result<()> {
    for (tool_name, params) in plan.get_steps() {
        recorder.record_step(&tool_name, params);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_semantic_search_plan_creation() {
        let params = json!({
            "action": "semantic_search",
            "query": "find async message bus implementation",
            "limit": 5
        });

        let plan = CodeMacroPlan::from_request(&params).unwrap();

        match plan {
            CodeMacroPlan::SemanticSearch { query, limit } => {
                assert_eq!(query, "find async message bus implementation");
                assert_eq!(limit, 5);
            }
            _ => panic!("Expected SemanticSearch plan"),
        }
    }

    #[test]
    fn test_code_semantic_search_steps() {
        let plan = CodeMacroPlan::SemanticSearch {
            query: "test query".to_string(),
            limit: 10,
        };

        let steps = plan.get_steps();
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0].0, "mapping_search");
        assert_eq!(steps[1].0, "code_search");
        assert_eq!(steps[2].0, "vector_search");
    }

    #[test]
    fn test_task_next_plan_creation() {
        let params = json!({
            "action": "next",
            "prd_title": "Macro Tools Implementation",
            "strategy": "priority"
        });

        let plan = TaskMacroPlan::from_request(&params).unwrap();

        match plan {
            TaskMacroPlan::Next {
                prd_title,
                strategy,
            } => {
                assert_eq!(prd_title, Some("Macro Tools Implementation".to_string()));
                assert_eq!(strategy, "priority");
            }
            _ => panic!("Expected Next plan"),
        }
    }

    #[test]
    fn test_task_next_steps() {
        let plan = TaskMacroPlan::Next {
            prd_title: None,
            strategy: "priority".to_string(),
        };

        let steps = plan.get_steps();
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0].0, "intellitask_task_statistics");
        assert_eq!(steps[1].0, "intellitask_next_ready");
        assert_eq!(steps[2].0, "intellitask_prioritize");
    }
}
