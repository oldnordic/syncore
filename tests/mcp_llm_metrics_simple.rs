//! Simple tests for LLM metrics MCP tool
//!
//! Tests the core functionality without complex state setup

use anyhow::Result;
use std::sync::Arc;
use syncore::intellitask::IntelliTask;
use syncore::llm::LanguageModel;
use syncore::models::gguf_engine::GGUFEngine;

#[test]
fn test_llm_metrics_fields() -> Result<()> {
    // Create GGUFEngine and test metrics
    let engine = Arc::new(GGUFEngine::new_test());

    // Get metrics via trait
    let metrics_json = engine.get_metrics()?;
    let metrics_obj = metrics_json.as_object().unwrap();

    // Verify all required fields exist
    assert!(metrics_obj.contains_key("total_requests"), "Missing total_requests");
    assert!(metrics_obj.contains_key("total_tokens_in"), "Missing total_tokens_in");
    assert!(metrics_obj.contains_key("total_tokens_out"), "Missing total_tokens_out");
    assert!(metrics_obj.contains_key("last_latency_ms"), "Missing last_latency_ms");
    assert!(metrics_obj.contains_key("avg_latency_ms"), "Missing avg_latency_ms");

    // Verify field types
    assert!(metrics_obj.get("total_requests").unwrap().is_number());
    assert!(metrics_obj.get("total_tokens_in").unwrap().is_number());
    assert!(metrics_obj.get("total_tokens_out").unwrap().is_number());
    assert!(metrics_obj.get("last_latency_ms").unwrap().is_number());
    assert!(metrics_obj.get("avg_latency_ms").unwrap().is_number());

    Ok(())
}

#[test]
fn test_llm_metrics_initial_values() -> Result<()> {
    // Test that metrics start at zero
    let engine = Arc::new(GGUFEngine::new_test());

    let metrics_json = engine.get_metrics()?;
    let metrics_obj = metrics_json.as_object().unwrap();

    // Initial values should be zero
    assert_eq!(metrics_obj.get("total_requests").unwrap().as_u64().unwrap(), 0);
    assert_eq!(metrics_obj.get("total_tokens_in").unwrap().as_u64().unwrap(), 0);
    assert_eq!(metrics_obj.get("total_tokens_out").unwrap().as_u64().unwrap(), 0);

    Ok(())
}

#[test]
fn test_llm_metrics_through_intellitask() -> Result<()> {
    // Test metrics access through IntelliTask
    let engine = Arc::new(GGUFEngine::new_test());
    let intellitask = IntelliTask::new(engine.clone());

    // Get metrics through IntelliTask
    let retrieved_llm = intellitask.get_llm();
    let metrics_json = retrieved_llm.get_metrics()?;
    let metrics_obj = metrics_json.as_object().unwrap();

    // Should have all required fields
    assert!(metrics_obj.contains_key("total_requests"));
    assert!(metrics_obj.contains_key("avg_latency_ms"));

    Ok(())
}

#[test]
fn test_llm_metrics_consistency() -> Result<()> {
    // Test that multiple calls return consistent data
    let engine = Arc::new(GGUFEngine::new_test());

    // Get metrics twice
    let metrics1 = engine.get_metrics()?;
    let metrics2 = engine.get_metrics()?;

    // Should be identical for test engine
    assert_eq!(metrics1, metrics2);

    Ok(())
}
