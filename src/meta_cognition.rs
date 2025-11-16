//! Meta-Cognition Module - AI self-reflection using Ollama
//!
//! This module implements meta-cognitive reasoning for AI agents using
//! qwen2.5-coder:3b via Ollama. It helps agents:
//! - Detect when they're stuck in unproductive patterns
//! - Suggest alternative approaches
//! - Self-reflect on tool usage effectiveness
//! - Break down complex problems step-by-step
//!
//! Inspired by IntelliTask's success with structured LLM reasoning.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;
use tracing::{info, warn};

/// Meta-cognitive analysis of agent behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaCognitiveAnalysis {
    /// Is the agent stuck in a loop?
    pub is_stuck: bool,

    /// Confidence score (0.0-1.0)
    pub confidence: f32,

    /// Reason for the assessment
    pub reason: String,

    /// Suggested alternative action
    pub suggestion: Option<String>,

    /// Should the agent stop and ask user?
    pub should_ask_user: bool,
}

/// Meta-cognition engine using Ollama
pub struct MetaCognitionEngine {
    model: String,
    temperature: f32,
}

impl Default for MetaCognitionEngine {
    fn default() -> Self {
        Self {
            model: "qwen2.5-coder:3b".to_string(),
            temperature: 0.0, // Deterministic reasoning
        }
    }
}

impl MetaCognitionEngine {
    /// Create a new meta-cognition engine
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with custom model
    pub fn with_model(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            temperature: 0.0,
        }
    }

    /// Analyze agent behavior and detect loops
    pub fn analyze_behavior(&self, tool_history: &[ToolCall]) -> Result<MetaCognitiveAnalysis> {
        // Build context for the LLM
        let history_summary = self.summarize_tool_history(tool_history);

        let prompt = format!(
            r#"You are a meta-cognitive analyzer for an AI coding assistant.

TOOL CALL HISTORY:
{}

TASK: Analyze if the agent is stuck in an unproductive loop.

A loop is unproductive if:
1. Same tool called 3+ times with similar parameters
2. Multiple no-output tool calls in sequence
3. Repeated searches without narrowing scope
4. Calling tools without using their results

Respond in JSON format:
{{
  "is_stuck": true/false,
  "confidence": 0.0-1.0,
  "reason": "brief explanation",
  "suggestion": "what to do instead (or null)",
  "should_ask_user": true/false
}}

IMPORTANT: Be strict - only mark as stuck if clearly unproductive."#,
            history_summary
        );

        let response = self.call_ollama(&prompt)?;

        // Parse JSON response
        let analysis: MetaCognitiveAnalysis = serde_json::from_str(&response)
            .context("Failed to parse meta-cognitive analysis")?;

        if analysis.is_stuck && analysis.confidence > 0.7 {
            warn!(
                "Meta-cognition detected stuck loop: {} (confidence: {:.2})",
                analysis.reason, analysis.confidence
            );
        }

        Ok(analysis)
    }

    /// Break down a complex problem into steps
    pub fn decompose_problem(&self, problem: &str, context: &str) -> Result<Vec<String>> {
        let prompt = format!(
            r#"You are a problem decomposition expert for GeoGraphDB development.

PROBLEM:
{}

CONTEXT:
{}

TASK: Break this down into 3-5 concrete, actionable steps.

Rules:
1. Each step should be independently testable
2. Steps should be ordered by dependencies
3. Be specific - no vague tasks like "research" or "investigate"
4. Each step should take < 30 minutes

Respond in JSON format:
{{
  "steps": [
    "Step 1: Specific action",
    "Step 2: Specific action",
    ...
  ]
}}

Example good steps:
- "Read src/acceleration/rocm_backend.rs lines 80-100 to understand device initialization"
- "Modify hipSetDevice(0) to accept device_id parameter"
- "Add device_count query using hipGetDeviceCount"

Example bad steps:
- "Understand multi-GPU" (too vague)
- "Research ROCm" (no specific action)
"#,
            problem, context
        );

        let response = self.call_ollama(&prompt)?;

        #[derive(Deserialize)]
        struct StepsResponse {
            steps: Vec<String>,
        }

        let parsed: StepsResponse = serde_json::from_str(&response)
            .context("Failed to parse problem decomposition")?;

        info!("Decomposed problem into {} steps", parsed.steps.len());

        Ok(parsed.steps)
    }

    /// Suggest next action based on current state
    pub fn suggest_next_action(&self,
                                 current_situation: &str,
                                 recent_tools: &[ToolCall]) -> Result<String> {
        let history_summary = self.summarize_tool_history(recent_tools);

        let prompt = format!(
            r#"You are an AI assistant helping another AI decide what to do next.

CURRENT SITUATION:
{}

RECENT TOOL CALLS:
{}

TASK: Suggest ONE specific next action.

Rules:
1. Be concrete - specify exact tool and parameters
2. Don't repeat failed approaches
3. If stuck, suggest asking the user
4. Consider the pattern of recent tool usage

Respond in JSON format:
{{
  "action": "Specific tool call or user question",
  "reasoning": "Why this action makes sense"
}}

Examples:
{{"action": "Read src/acceleration/rocm_backend.rs to understand current GPU device management", "reasoning": "Need to see how device_id is currently set"}}
{{"action": "Ask user: What should I prioritize - multi-GPU orchestration or kernel optimization?", "reasoning": "Multiple valid directions, user input needed"}}
"#,
            current_situation, history_summary
        );

        let response = self.call_ollama(&prompt)?;

        #[derive(Deserialize)]
        struct ActionResponse {
            action: String,
            reasoning: String,
        }

        let parsed: ActionResponse = serde_json::from_str(&response)
            .context("Failed to parse action suggestion")?;

        info!("Meta-cognition suggests: {} ({})", parsed.action, parsed.reasoning);

        Ok(parsed.action)
    }

    /// Call Ollama with a prompt
    fn call_ollama(&self, prompt: &str) -> Result<String> {
        let output = Command::new("ollama")
            .arg("run")
            .arg(&self.model)
            .arg("--temperature")
            .arg(self.temperature.to_string())
            .arg(prompt)
            .output()
            .context("Failed to execute ollama command")?;

        if !output.status.success() {
            anyhow::bail!(
                "Ollama command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let response = String::from_utf8(output.stdout)
            .context("Invalid UTF-8 in Ollama response")?;

        Ok(response.trim().to_string())
    }

    /// Summarize tool call history for LLM context
    fn summarize_tool_history(&self, tool_calls: &[ToolCall]) -> String {
        tool_calls.iter()
            .enumerate()
            .map(|(i, call)| {
                format!(
                    "{}. {} ({}) -> {}",
                    i + 1,
                    call.tool_name,
                    call.short_params(),
                    if call.had_output { "✓ output" } else { "✗ no output" }
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Record of a tool call for meta-cognitive analysis
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub tool_name: String,
    pub params: String,
    pub had_output: bool,
    pub timestamp: std::time::Instant,
}

impl ToolCall {
    /// Get shortened parameters for display
    fn short_params(&self) -> String {
        if self.params.len() > 50 {
            format!("{}...", &self.params[..47])
        } else {
            self.params.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    #[ignore] // Requires Ollama running
    fn test_detect_loop() {
        let engine = MetaCognitionEngine::new();

        // Simulate stuck loop
        let tools = vec![
            ToolCall {
                tool_name: "sequential_cycle".to_string(),
                params: "max_cycles: 5".to_string(),
                had_output: false,
                timestamp: Instant::now(),
            },
            ToolCall {
                tool_name: "sequential_cycle".to_string(),
                params: "max_cycles: 5".to_string(),
                had_output: false,
                timestamp: Instant::now(),
            },
            ToolCall {
                tool_name: "sequential_cycle".to_string(),
                params: "max_cycles: 5".to_string(),
                had_output: false,
                timestamp: Instant::now(),
            },
        ];

        let analysis = engine.analyze_behavior(&tools).unwrap();
        assert!(analysis.is_stuck);
        assert!(analysis.confidence > 0.5);
    }

    #[test]
    #[ignore] // Requires Ollama running
    fn test_problem_decomposition() {
        let engine = MetaCognitionEngine::new();

        let steps = engine.decompose_problem(
            "Implement multi-GPU orchestration for ROCm backend",
            "Current implementation only uses device 0, need to distribute work across 2 GPUs"
        ).unwrap();

        assert!(!steps.is_empty());
        assert!(steps.len() <= 5);

        // Steps should be specific
        for step in &steps {
            assert!(step.len() > 20, "Step too vague: {}", step);
        }
    }
}
