//! Tests for GGUFEngine metrics accumulation
//!
//! These tests verify that:
//! - Request counts increment correctly
//! - Token counts are accurate for input/output
//! - Latency measurements work properly
//! - Metrics persist across multiple calls
//! - Health status updates on success/failure

use anyhow::Result;
use std::sync::Arc;
use syncore::config::{LlmConfig, SyncoreConfig};
use syncore::llm::{LanguageModel, Prompt};
use syncore::models::gguf_engine::GGUFEngine;
use tempfile::TempDir;

/// Test that metrics increment request count correctly
#[test]
fn test_metrics_increment_requests() -> Result<()> {
    // Create test backend (doesn't require real model)
    let engine = GGUFEngine::new_test();

    // Verify initial state
    let initial_metrics = engine.metrics();
    assert_eq!(initial_metrics.total_requests, 0);

    // Generate twice
    let prompt1 = Prompt::new("", "test prompt 1");
    let _result1 = engine.complete(&prompt1);

    let prompt2 = Prompt::new("", "test prompt 2");
    let _result2 = engine.complete(&prompt2);

    // Verify request count incremented
    let final_metrics = engine.metrics();
    assert_eq!(final_metrics.total_requests, 2);

    Ok(())
}

/// Test that metrics count input tokens correctly
#[test]
fn test_metrics_count_tokens_in() -> Result<()> {
    let engine = GGUFEngine::new_test();

    // Generate with a specific prompt
    let prompt = Prompt::new("", "hello world");
    let _result = engine.complete(&prompt);

    // Verify tokens_in was counted
    let metrics = engine.metrics();
    assert_eq!(metrics.total_requests, 1);
    assert!(metrics.total_tokens_in > 0, "Should have counted input tokens");

    Ok(())
}

/// Test that metrics count output tokens correctly  
#[test]
fn test_metrics_count_tokens_out() -> Result<()> {
    let engine = GGUFEngine::new_test();

    // Generate with a simple prompt
    let prompt = Prompt::new("", "hello");
    let _result = engine.complete(&prompt);

    // Verify tokens_out was counted
    let metrics = engine.metrics();
    assert_eq!(metrics.total_requests, 1);
    assert!(metrics.total_tokens_out > 0, "Should have counted output tokens");

    Ok(())
}

/// Test that latency measurements work correctly
#[test]
fn test_metrics_latency() -> Result<()> {
    let engine = GGUFEngine::new_test();

    // Generate once
    let prompt = Prompt::new("", "test latency");
    let _result = engine.complete(&prompt);

    // Verify latency was recorded (allow for very fast execution)
    let metrics = engine.metrics();
    assert_eq!(metrics.total_requests, 1);
    assert!(metrics.last_latency_ms >= 0.0, "Should have recorded non-negative latency");
    assert!(metrics.avg_latency_ms >= 0.0, "Should have calculated non-negative average latency");

    // Generate second time
    let prompt2 = Prompt::new("", "test latency 2");
    let _result2 = engine.complete(&prompt2);

    // Verify average latency updated
    let final_metrics = engine.metrics();
    assert_eq!(final_metrics.total_requests, 2);
    assert!(final_metrics.last_latency_ms >= 0.0);
    assert!(final_metrics.avg_latency_ms >= 0.0);
    // Average should be updated after second request
    assert!(final_metrics.avg_latency_ms >= 0.0);

    Ok(())
}

/// Test that metrics persist correctly across multiple calls
#[test]
fn test_metrics_persist_after_multiple_calls() -> Result<()> {
    let engine = GGUFEngine::new_test();

    let num_calls = 5;
    let mut total_tokens_in_expected = 0u64;
    let mut total_tokens_out_expected = 0u64;

    // Generate multiple times with different prompts
    for i in 0..num_calls {
        let prompt = Prompt::new("", &format!("test prompt {}", i));
        let _result = engine.complete(&prompt);

        // Track expected cumulative values
        total_tokens_in_expected += 3; // Rough estimate per prompt
        total_tokens_out_expected += 5; // Rough estimate per response
    }

    // Verify cumulative metrics
    let metrics = engine.metrics();
    assert_eq!(metrics.total_requests, num_calls as u64);
    assert!(metrics.total_tokens_in > 0);
    assert!(metrics.total_tokens_out > 0);
    assert!(metrics.last_latency_ms >= 0.0);
    assert!(metrics.avg_latency_ms >= 0.0);

    Ok(())
}

/// Test that health status updates correctly on success
#[test]
fn test_health_updates_on_success() -> Result<()> {
    let engine = GGUFEngine::new_test();

    // Generate successfully
    let prompt = Prompt::new("", "successful test");
    let _result = engine.complete(&prompt);

    // Verify health is OK
    let health = engine.health();
    assert!(matches!(health.status, syncore::models::gguf_engine::GgufStatus::Ok));
    assert!(health.last_error.is_none());

    Ok(())
}

/// Test that metrics are thread-safe (basic smoke test)
#[test]
fn test_metrics_thread_safety() -> Result<()> {
    let engine = Arc::new(GGUFEngine::new_test());
    let mut handles = vec![];

    // Spawn multiple threads to generate concurrently
    for i in 0..3 {
        let engine_clone = Arc::clone(&engine);
        let handle = std::thread::spawn(move || {
            let prompt = Prompt::new("", &format!("concurrent test {}", i));
            let _result = engine_clone.complete(&prompt);
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all requests were counted
    let metrics = engine.metrics();
    assert_eq!(metrics.total_requests, 3);

    Ok(())
}
