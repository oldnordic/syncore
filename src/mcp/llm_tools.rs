//! MCP Tools for LLM Health and Metrics
//!
//! Provides "llm.health" and "llm.metrics" tools for monitoring
//! GGUFEngine status and performance. Extended to include reasoning
//! metrics from ToTEngine for PHASE ST-9.

use crate::models::gguf_engine::GGUFEngine;
use crate::reasoning::engine::ToTEngine;
use anyhow::Result;
use serde_json::Value;

/// Handle "llm.health" MCP tool
/// Returns GGUFEngine health snapshot as JSON
pub async fn handle_llm_health(_params: Value, engine: &GGUFEngine) -> Result<Value> {
    let health = engine.health();

    // Convert health struct to JSON value
    let health_json = serde_json::to_value(health)?;

    Ok(health_json)
}

/// Handle "llm.health" MCP tool with reasoning metrics
/// Returns GGUFEngine health snapshot plus reasoning metrics as JSON
pub async fn handle_llm_health_with_reasoning(
    _params: Value,
    engine: &GGUFEngine,
    tot_engine: Option<&ToTEngine>,
) -> Result<Value> {
    let health = engine.health();
    let mut health_json = serde_json::to_value(health)?;

    // Add reasoning metrics if ToTEngine is available
    if let Some(tot) = tot_engine {
        let reasoning_snapshot = tot.get_metrics_snapshot();
        let reasoning_json = serde_json::to_value(reasoning_snapshot)?;
        health_json["reasoning"] = reasoning_json;
    }

    Ok(health_json)
}

/// Handle "llm.metrics" MCP tool  
/// Returns GGUFEngine metrics snapshot as JSON
pub async fn handle_llm_metrics(_params: Value, engine: &GGUFEngine) -> Result<Value> {
    let metrics = engine.metrics();

    // Convert metrics struct to JSON value
    let metrics_json = serde_json::to_value(metrics)?;

    Ok(metrics_json)
}

/// Handle "llm.metrics" MCP tool with reasoning metrics
/// Returns GGUFEngine metrics snapshot plus reasoning metrics as JSON
pub async fn handle_llm_metrics_with_reasoning(
    _params: Value,
    engine: &GGUFEngine,
    tot_engine: Option<&ToTEngine>,
) -> Result<Value> {
    let metrics = engine.metrics();
    let mut metrics_json = serde_json::to_value(metrics)?;

    // Add reasoning metrics if ToTEngine is available
    if let Some(tot) = tot_engine {
        let reasoning_snapshot = tot.get_metrics_snapshot();
        let reasoning_json = serde_json::to_value(reasoning_snapshot)?;
        metrics_json["reasoning"] = reasoning_json;
    }

    Ok(metrics_json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::gguf_engine::GGUFEngine;

    #[test]
    fn test_handle_llm_health() {
        let engine = GGUFEngine::new_test();
        let params = serde_json::json!({});

        let result =
            tokio::runtime::Runtime::new().unwrap().block_on(handle_llm_health(params, &engine));

        assert!(result.is_ok());
        let result_value = result.unwrap();
        let health_obj = result_value.as_object().unwrap();

        assert_eq!(health_obj.get("backend_name").unwrap().as_str().unwrap(), "gguf_engine");
        assert!(health_obj.contains_key("status"));
        assert!(health_obj.contains_key("device"));
        assert!(health_obj.contains_key("model_loaded"));
        assert!(health_obj.contains_key("tokenizer_loaded"));
    }

    #[test]
    fn test_handle_llm_metrics() {
        let engine = GGUFEngine::new_test();
        let params = serde_json::json!({});

        let result =
            tokio::runtime::Runtime::new().unwrap().block_on(handle_llm_metrics(params, &engine));

        assert!(result.is_ok());
        let result_value = result.unwrap();
        let metrics_obj = result_value.as_object().unwrap();

        assert!(metrics_obj.contains_key("total_requests"));
        assert!(metrics_obj.contains_key("total_tokens_in"));
        assert!(metrics_obj.contains_key("total_tokens_out"));
        assert!(metrics_obj.contains_key("last_latency_ms"));
        assert!(metrics_obj.contains_key("avg_latency_ms"));
    }

    #[test]
    fn test_handle_llm_health_with_reasoning_no_tot() {
        let engine = GGUFEngine::new_test();
        let params = serde_json::json!({});

        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(handle_llm_health_with_reasoning(params, &engine, None));

        assert!(result.is_ok());
        let result_value = result.unwrap();
        let health_obj = result_value.as_object().unwrap();

        // Should contain GGUF health fields
        assert_eq!(health_obj.get("backend_name").unwrap().as_str().unwrap(), "gguf_engine");
        assert!(health_obj.contains_key("status"));
        assert!(health_obj.contains_key("device"));
        assert!(health_obj.contains_key("model_loaded"));
        assert!(health_obj.contains_key("tokenizer_loaded"));

        // Should NOT contain reasoning block when no ToTEngine provided
        assert!(!health_obj.contains_key("reasoning"));
    }

    #[test]
    fn test_handle_llm_metrics_with_reasoning_no_tot() {
        let engine = GGUFEngine::new_test();
        let params = serde_json::json!({});

        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(handle_llm_metrics_with_reasoning(params, &engine, None));

        assert!(result.is_ok());
        let result_value = result.unwrap();
        let metrics_obj = result_value.as_object().unwrap();

        // Should contain GGUF metrics fields
        assert!(metrics_obj.contains_key("total_requests"));
        assert!(metrics_obj.contains_key("total_tokens_in"));
        assert!(metrics_obj.contains_key("total_tokens_out"));
        assert!(metrics_obj.contains_key("last_latency_ms"));
        assert!(metrics_obj.contains_key("avg_latency_ms"));

        // Should NOT contain reasoning block when no ToTEngine provided
        assert!(!metrics_obj.contains_key("reasoning"));
    }
}
