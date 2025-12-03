//! Simple tests for LLM health and metrics MCP tools
//!
//! Tests the core functionality without complex state setup

use anyhow::Result;
use serde_json::json;
use std::sync::Arc;
use syncore::intellitask::IntelliTask;
use syncore::llm::LanguageModel;
use syncore::models::gguf_engine::GGUFEngine;

#[test]
fn test_llm_health_via_trait() -> Result<()> {
    // Create GGUFEngine and test health via trait
    let engine = Arc::new(GGUFEngine::new_test());

    // Test the trait method directly
    let health_json = engine.get_health()?;
    let health_obj = health_json.as_object().unwrap();

    // Verify required fields
    assert_eq!(health_obj.get("backend_name").unwrap().as_str().unwrap(), "gguf_engine");
    assert!(health_obj.contains_key("status"));
    assert!(health_obj.contains_key("device"));
    assert!(health_obj.contains_key("model_loaded"));
    assert!(health_obj.contains_key("tokenizer_loaded"));

    Ok(())
}

#[test]
fn test_llm_metrics_via_trait() -> Result<()> {
    // Create GGUFEngine and test metrics via trait
    let engine = Arc::new(GGUFEngine::new_test());

    // Test the trait method directly
    let metrics_json = engine.get_metrics()?;
    let metrics_obj = metrics_json.as_object().unwrap();

    // Verify required fields
    assert!(metrics_obj.contains_key("total_requests"));
    assert!(metrics_obj.contains_key("total_tokens_in"));
    assert!(metrics_obj.contains_key("total_tokens_out"));
    assert!(metrics_obj.contains_key("last_latency_ms"));
    assert!(metrics_obj.contains_key("avg_latency_ms"));

    Ok(())
}

#[test]
fn test_intellitask_wraps_llm() -> Result<()> {
    // Test that IntelliTask properly wraps the LLM
    let engine = Arc::new(GGUFEngine::new_test());
    let intellitask = IntelliTask::new(engine.clone());

    // Get LLM back from IntelliTask
    let retrieved_llm = intellitask.get_llm();

    // Test health through retrieved LLM
    let health_json = retrieved_llm.get_health()?;
    let health_obj = health_json.as_object().unwrap();

    assert_eq!(health_obj.get("backend_name").unwrap().as_str().unwrap(), "gguf_engine");

    // Test metrics through retrieved LLM
    let metrics_json = retrieved_llm.get_metrics()?;
    let metrics_obj = metrics_json.as_object().unwrap();

    assert!(metrics_obj.contains_key("total_requests"));

    Ok(())
}

#[test]
fn test_llm_trait_methods_dont_panic() -> Result<()> {
    // Test that trait methods work without panicking
    let engine = Arc::new(GGUFEngine::new_test());

    // These should not panic
    let _health = engine.get_health()?;
    let _metrics = engine.get_metrics()?;

    // Test backend name
    assert_eq!(engine.backend_name(), "gguf_engine");

    Ok(())
}
