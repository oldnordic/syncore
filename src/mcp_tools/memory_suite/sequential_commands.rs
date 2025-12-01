//! Sequential Commands Module
//!
//! Handles execution of sequential reasoning operations.
//! Extracted from memory_suite.rs (lines 194-396).
//!
//! Commands:
//! - sequential_record: Record a thought step in reasoning chain
//! - sequential_get: Get all thought steps for a task
//! - sequential_search: Search thought steps by semantic content
//! - sequential_cycle: Run sequential thinking cycles with Ollama LLM

use super::{MemorySuite, MemorySuiteArgs};
use crate::mcp_tools::SuiteResult;

/// Execute sequential_record command
pub fn cmd_sequential_record(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    let step_number = match args.step_number {
        Some(n) => n,
        None => {
            return SuiteResult::err("sequential_record", "Missing required parameter: step_number")
        }
    };

    let thought = match args.thought {
        Some(ref t) => t,
        None => {
            return SuiteResult::err("sequential_record", "Missing required parameter: thought")
        }
    };

    let reasoning = match args.reasoning {
        Some(ref r) => r,
        None => {
            return SuiteResult::err("sequential_record", "Missing required parameter: reasoning")
        }
    };

    use crate::portfolio::sequential_step::{SequentialStep, ThoughtStep};

    let sequential = SequentialStep::new(suite.state.clone());

    let step = ThoughtStep {
        task_id: args.task_id,
        step_number,
        thought: thought.clone(),
        action: args.action.clone(),
        observation: args.observation.clone(),
        reasoning: reasoning.clone(),
    };

    match sequential.record_step(&step) {
        Ok(step_id) => SuiteResult::ok(
            "sequential_record",
            serde_json::json!({
                "success": true,
                "step_id": step_id,
                "message": "Thought step recorded successfully"
            }),
        ),
        Err(e) => SuiteResult::err("sequential_record", format!("Failed to record step: {}", e)),
    }
}

/// Execute sequential_get command
pub fn cmd_sequential_get(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    let task_id = match args.task_id {
        Some(id) => id,
        None => return SuiteResult::err("sequential_get", "Missing required parameter: task_id"),
    };

    use crate::portfolio::sequential_step::SequentialStep;

    let sequential = SequentialStep::new(suite.state.clone());

    match sequential.get_steps_for_task(task_id) {
        Ok(steps) => SuiteResult::ok(
            "sequential_get",
            serde_json::json!({
                "task_id": task_id,
                "steps": steps,
                "count": steps.len()
            }),
        ),
        Err(e) => SuiteResult::err("sequential_get", format!("Failed to get steps: {}", e)),
    }
}

/// Execute sequential_search command
pub fn cmd_sequential_search(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    let query = match args.query {
        Some(ref q) => q,
        None => return SuiteResult::err("sequential_search", "Missing required parameter: query"),
    };

    use crate::portfolio::sequential_step::SequentialStep;

    let sequential = SequentialStep::new(suite.state.clone());
    let limit = args.limit.unwrap_or(10);

    match sequential.search_steps(query) {
        Ok(mut steps) => {
            // Apply limit manually since API doesn't support it
            steps.truncate(limit);

            SuiteResult::ok(
                "sequential_search",
                serde_json::json!({
                    "query": query,
                    "steps": steps,
                    "count": steps.len()
                }),
            )
        }
        Err(e) => SuiteResult::err("sequential_search", format!("Failed to search steps: {}", e)),
    }
}

/// Execute sequential_cycle command
pub fn cmd_sequential_cycle(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    let max_cycles = args.max_cycles.unwrap_or(1);

    // Create Ollama configuration
    use crate::ollama::OllamaConfig;
    use crate::sequential::{CycleResult, OllamaLanguageModel, SequentialCore};
    use std::sync::{Arc, Mutex};

    let model_name =
        std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5-coder:3B".to_string());

    let config = OllamaConfig {
        model: model_name.clone(),
        temperature: 0.7,
        max_tokens: 2000,
        timeout_secs: 60,
    };

    let llm = match OllamaLanguageModel::new(config) {
        Ok(llm) => Arc::new(Mutex::new(llm)) as Arc<Mutex<dyn crate::sequential::LanguageModel>>,
        Err(e) => {
            return SuiteResult::err(
                "sequential_cycle",
                format!(
                    "Failed to initialize Ollama language model with model '{}': {}. \
                    Ensure Ollama is installed and the model is available (ollama pull {}).",
                    model_name, e, model_name
                ),
            );
        }
    };

    // Create sequential reasoning engine (uses GENERAL domain for reasoning steps)
    let reasoning = SequentialCore::new(
        Arc::clone(&suite.state.tasks),
        Arc::clone(&suite.state.general_store),
        Arc::clone(&suite.state.memory),
        llm,
        suite.state.logger.clone(),
    );

    // Execute the reasoning cycle(s)
    let mut results = Vec::new();
    for _ in 0..max_cycles {
        match reasoning.run_cycle() {
            Ok(result) => match result {
                CycleResult::Completed {
                    task_id,
                    thought,
                    decision,
                    actions,
                    action_results,
                    reflection,
                } => {
                    results.push(serde_json::json!({
                        "success": true,
                        "task_id": task_id,
                        "thought": thought,
                        "decision": decision,
                        "actions": actions,
                        "action_results": action_results,
                        "reflection": reflection,
                    }));
                }
                CycleResult::NoTasks => {
                    break; // No more tasks to process
                }
            },
            Err(e) => {
                return SuiteResult::err(
                    "sequential_cycle",
                    format!("Sequential reasoning cycle failed: {}", e),
                );
            }
        }
    }

    SuiteResult::ok(
        "sequential_cycle",
        serde_json::json!({
            "cycles_completed": results.len(),
            "max_cycles": max_cycles,
            "results": results
        }),
    )
}
