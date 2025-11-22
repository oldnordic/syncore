//! Intelligent Macro Tools Tests
//!
//! This file tests the intelligent orchestration behavior of macro tools.
//! Unlike macro_tools_tests.rs which validates simple routing, these tests
//! verify that macro tools can plan and execute multi-step operations.
//!
//! Focus areas in Phase 3:
//! 1. syncore.code - Smart code search and analysis
//! 2. syncore.task - Smart task management and prioritization

use anyhow::Result;
use serde_json::json;
use std::sync::{Arc, Mutex};
use syncore::macro_tools::code::execute_code_macro;
use syncore::macro_tools::planner::ExecutionRecorder;
use syncore::macro_tools::task::execute_task_macro;

// ============================================================================
// MOCK EXECUTION TRACKER - Captures multi-step operation sequences
// ============================================================================

/// Tracks the execution order and parameters of underlying tool calls
#[derive(Debug, Default, Clone)]
pub struct ExecutionTracker {
    steps: Arc<Mutex<Vec<ExecutionStep>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionStep {
    pub tool_name: String,
    pub params: serde_json::Value,
    pub step_number: usize,
}

impl ExecutionTracker {
    pub fn new() -> Self {
        Self {
            steps: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn record_step(&self, tool_name: &str, params: serde_json::Value) {
        let mut steps = self.steps.lock().unwrap();
        let step_number = steps.len() + 1;
        steps.push(ExecutionStep {
            tool_name: tool_name.to_string(),
            params,
            step_number,
        });
    }

    pub fn get_steps(&self) -> Vec<ExecutionStep> {
        self.steps.lock().unwrap().clone()
    }

    pub fn step_count(&self) -> usize {
        self.steps.lock().unwrap().len()
    }

    pub fn get_step(&self, index: usize) -> Option<ExecutionStep> {
        self.steps.lock().unwrap().get(index).cloned()
    }

    pub fn verify_step_order(&self, expected_tools: Vec<&str>) -> bool {
        let steps = self.get_steps();
        if steps.len() != expected_tools.len() {
            return false;
        }
        steps
            .iter()
            .zip(expected_tools.iter())
            .all(|(step, expected)| step.tool_name == *expected)
    }

    pub fn clear(&self) {
        self.steps.lock().unwrap().clear();
    }
}

impl ExecutionRecorder for ExecutionTracker {
    fn record_step(&self, tool_name: &str, params: serde_json::Value) {
        self.record_step(tool_name, params);
    }

    fn wrap_success(&self, _tool: &str, data: serde_json::Value) -> serde_json::Value {
        data
    }

    fn wrap_error(&self, _tool: &str, error: &str) -> serde_json::Value {
        serde_json::json!({"error": error})
    }

    fn executor_type(&self) -> &str {
        "test"
    }
}

// ============================================================================
// TEST 1: syncore.code - Semantic Search Intelligence
// ============================================================================

#[test]
fn test_code_semantic_search_executes_multi_step_plan_in_order() {
    // Goal: Verify that syncore.code with action="semantic_search" orchestrates:
    // 1. mapping_search (find relevant files)
    // 2. code_search (semantic search in those files)
    // 3. vector_search (optional refinement)

    let tracker = ExecutionTracker::new();

    // Macro tool request
    let request = json!({
        "action": "semantic_search",
        "query": "find async message bus implementation",
        "limit": 5
    });

    // Expected execution plan:
    // Step 1: mapping_search with query
    // Step 2: code_search with query and discovered files
    // Step 3: vector_search with query for ranking refinement

    // Execute using real implementation
    execute_code_macro(&request, &tracker).unwrap();

    // Verify execution order
    assert_eq!(tracker.step_count(), 3, "Should execute 3 steps");

    // Verify step 1: mapping_search
    let step1 = tracker.get_step(0).unwrap();
    assert_eq!(step1.tool_name, "mapping_search");
    assert_eq!(step1.step_number, 1);
    assert_eq!(
        step1.params["query"],
        "find async message bus implementation"
    );

    // Verify step 2: code_search
    let step2 = tracker.get_step(1).unwrap();
    assert_eq!(step2.tool_name, "code_search");
    assert_eq!(step2.step_number, 2);
    assert_eq!(
        step2.params["query"],
        "find async message bus implementation"
    );

    // Verify step 3: vector_search
    let step3 = tracker.get_step(2).unwrap();
    assert_eq!(step3.tool_name, "vector_search");
    assert_eq!(step3.step_number, 3);
    assert_eq!(
        step3.params["query"],
        "find async message bus implementation"
    );
    assert_eq!(step3.params["limit"], 5);

    // Verify overall order
    assert!(tracker.verify_step_order(vec!["mapping_search", "code_search", "vector_search"]));
}

#[test]
fn test_code_analyze_module_executes_multi_step_plan() {
    // Goal: Verify that syncore.code with action="analyze_module" orchestrates:
    // 1. parser_analyze (extract structure)
    // 2. mapping_deps (find dependencies)
    // 3. code_search (find related code patterns)

    let tracker = ExecutionTracker::new();

    let request = json!({
        "action": "analyze_module",
        "file_path": "/src/message_bus.rs",
        "focus": "agent communication"
    });

    execute_code_macro(&request, &tracker).unwrap();

    assert_eq!(tracker.step_count(), 3, "Should execute 3 steps");

    // Step 1: parser_analyze
    let step1 = tracker.get_step(0).unwrap();
    assert_eq!(step1.tool_name, "parser_analyze");
    assert_eq!(step1.params["file_path"], "/src/message_bus.rs");

    // Step 2: mapping_deps
    let step2 = tracker.get_step(1).unwrap();
    assert_eq!(step2.tool_name, "mapping_deps");
    assert_eq!(step2.params["path"], "/src/message_bus.rs");

    // Step 3: code_search with focus
    let step3 = tracker.get_step(2).unwrap();
    assert_eq!(step3.tool_name, "code_search");
    assert_eq!(step3.params["query"], "agent communication");

    assert!(tracker.verify_step_order(vec!["parser_analyze", "mapping_deps", "code_search"]));
}

#[test]
fn test_code_index_directory_orchestrates_batch_operations() {
    // Goal: Verify that syncore.code with action="index_directory" orchestrates:
    // 1. code_index_directory (bulk indexing)
    // 2. mapping_record (for each file)
    // 3. document_index (for documentation)

    let tracker = ExecutionTracker::new();

    let request = json!({
        "action": "index_directory",
        "directory": "/src/macro_tools",
        "pattern": "**/*.rs"
    });

    execute_code_macro(&request, &tracker).unwrap();

    // Should have at least 2 steps (could be more with multiple files)
    assert!(tracker.step_count() >= 2);

    let step1 = tracker.get_step(0).unwrap();
    assert_eq!(step1.tool_name, "code_index_directory");
    assert_eq!(step1.params["directory"], "/src/macro_tools");
    assert_eq!(step1.params["pattern"], "**/*.rs");
}

// ============================================================================
// TEST 2: syncore.task - Smart Task Management
// ============================================================================

#[test]
fn test_task_next_executes_multi_step_plan_in_order() {
    // Goal: Verify that syncore.task with action="next" orchestrates:
    // 1. intellitask_task_statistics (get overview)
    // 2. intellitask_next_ready (filter by dependencies)
    // 3. intellitask_prioritize (pick best task)

    let tracker = ExecutionTracker::new();

    let request = json!({
        "action": "next",
        "prd_title": "Macro Tools Implementation",
        "strategy": "priority"
    });

    execute_task_macro(&request, &tracker).unwrap();

    assert_eq!(tracker.step_count(), 3, "Should execute 3 steps");

    // Step 1: Get task statistics
    let step1 = tracker.get_step(0).unwrap();
    assert_eq!(step1.tool_name, "intellitask_task_statistics");

    // Step 2: Get ready tasks (dependencies satisfied)
    let step2 = tracker.get_step(1).unwrap();
    assert_eq!(step2.tool_name, "intellitask_next_ready");

    // Step 3: Prioritize
    let step3 = tracker.get_step(2).unwrap();
    assert_eq!(step3.tool_name, "intellitask_prioritize");
    assert_eq!(step3.params["strategy"], "priority");

    assert!(tracker.verify_step_order(vec![
        "intellitask_task_statistics",
        "intellitask_next_ready",
        "intellitask_prioritize"
    ]));
}

#[test]
fn test_task_bootstrap_from_prd_orchestrates_workflow() {
    // Goal: Verify that syncore.task with action="bootstrap_from_prd" orchestrates:
    // 1. intellitask_generate (create task breakdown)
    // 2. intellitask_save (persist to database)
    // 3. intellitask_subtasks (expand high-level tasks)

    let tracker = ExecutionTracker::new();

    let request = json!({
        "action": "bootstrap_from_prd",
        "prd_text": "Implement macro tools layer with intelligent orchestration",
        "auto_expand": true
    });

    execute_task_macro(&request, &tracker).unwrap();

    assert_eq!(tracker.step_count(), 3, "Should execute 3 steps");

    // Step 1: Generate breakdown
    let step1 = tracker.get_step(0).unwrap();
    assert_eq!(step1.tool_name, "intellitask_generate");
    assert!(step1.params["prd_content"]
        .as_str()
        .unwrap()
        .contains("macro tools"));

    // Step 2: Save to database
    let step2 = tracker.get_step(1).unwrap();
    assert_eq!(step2.tool_name, "intellitask_save");

    // Step 3: Expand subtasks
    let step3 = tracker.get_step(2).unwrap();
    assert_eq!(step3.tool_name, "intellitask_subtasks");

    assert!(tracker.verify_step_order(vec![
        "intellitask_generate",
        "intellitask_save",
        "intellitask_subtasks"
    ]));
}

#[test]
fn test_task_complete_updates_and_suggests_next() {
    // Goal: Verify that syncore.task with action="complete" orchestrates:
    // 1. intellitask_update_status (mark task complete)
    // 2. intellitask_subtask_stats (update parent stats)
    // 3. intellitask_next_ready (suggest next task)

    let tracker = ExecutionTracker::new();

    let request = json!({
        "action": "complete",
        "task_id": 123,
        "suggest_next": true
    });

    execute_task_macro(&request, &tracker).unwrap();

    assert_eq!(tracker.step_count(), 3, "Should execute 3 steps");

    let step1 = tracker.get_step(0).unwrap();
    assert_eq!(step1.tool_name, "intellitask_update_status");
    assert_eq!(step1.params["task_id"], 123);
    assert_eq!(step1.params["status"], "completed");

    let step2 = tracker.get_step(1).unwrap();
    assert_eq!(step2.tool_name, "intellitask_subtask_stats");
    assert_eq!(step2.params["parent_id"], 123);

    let step3 = tracker.get_step(2).unwrap();
    assert_eq!(step3.tool_name, "intellitask_next_ready");

    assert!(tracker.verify_step_order(vec![
        "intellitask_update_status",
        "intellitask_subtask_stats",
        "intellitask_next_ready"
    ]));
}
