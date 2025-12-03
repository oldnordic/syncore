//! Integration tests for real GGUFEngine inference
//!
//! These tests require to actual Qwen2.5-0.5B GGUF model and perform
//! real text generation. They are marked as ignored by default
//! since they require model loading and are computationally expensive.

use anyhow::Result;
use std::time::Instant;
use syncore::llm::{factory::LlmFactory, Prompt};

/// Test configuration for real GGUFEngine backend
fn get_real_gguf_config() -> syncore::llm::factory::LlmConfig {
    syncore::llm::factory::LlmConfig {
        backend: syncore::llm::factory::LlmBackend::GGUFEngine,
        model: "Qwen2.5-1.5B-Instruct".to_string(), // This maps to our qwen2.5-0.5b.gguf
        url: "local".to_string(),
        timeout_seconds: 60, // Longer timeout for real inference
    }
}

#[test]
#[ignore] // Requires real model loading - run with: cargo test -- --ignored
fn test_gguf_qwen05b_generates_nonempty_reply() -> Result<()> {
    // Test: Real GGUFEngine should load GGUF and generate non-empty response
    let config = get_real_gguf_config();

    // Create real GGUFEngine (not test backend)
    let model = LlmFactory::from_config(&config)?;

    // Verify it's the real GGUFEngine
    assert_eq!(model.backend_name(), "gguf_engine");

    // Test health check - should find the model file
    let is_healthy = model.health_check()?;
    assert!(is_healthy, "GGUFEngine health check failed - model file not found");

    // Test basic generation with simple prompt
    let prompt = Prompt::new("You are a helpful assistant.", "Hello");
    let start_time = Instant::now();
    let result = model.complete(&prompt)?;
    let duration = start_time.elapsed();

    // Verify response characteristics
    assert!(!result.text.is_empty(), "Generated response should not be empty");
    assert!(result.text.len() < 1000, "Response should be reasonably short for test");
    assert!(duration.as_secs() < 30, "Generation should complete within 30 seconds");

    // Check metadata indicates real inference
    if let Some(metadata) = &result.metadata {
        assert_eq!(metadata["backend"], "gguf_engine");
        assert!(metadata
            .get("model_loaded")
            .unwrap_or(&serde_json::Value::Bool(false))
            .as_bool()
            .unwrap_or(false));
    }

    println!("GGUFEngine inference test passed in {:?}", duration);
    println!("Generated response: {}", result.text);

    Ok(())
}

#[test]
#[ignore] // Requires real model loading
fn test_gguf_deterministic_generation() -> Result<()> {
    // Test: Generation should be deterministic with same prompt
    let config = get_real_gguf_config();
    let model = LlmFactory::from_config(&config)?;

    let prompt = Prompt::new("System", "Say 'test response'");

    // Generate twice with same prompt
    let result1 = model.complete(&prompt)?;
    let result2 = model.complete(&prompt)?;

    // Should be identical (deterministic sampling)
    assert_eq!(result1.text, result2.text, "Generation should be deterministic");

    // Response should contain expected content
    assert!(
        result1.text.to_lowercase().contains("test")
            || result1.text.to_lowercase().contains("response")
    );

    Ok(())
}

#[test]
#[ignore] // Requires real model loading
fn test_candle_handles_different_prompts() -> Result<()> {
    // Test: Different prompts should produce different responses
    let config = get_real_gguf_config();
    let model = LlmFactory::from_config(&config)?;

    let prompts = vec![
        Prompt::new("System", "What is 2+2?"),
        Prompt::new("System", "Hello world"),
        Prompt::new("System", "Say something about Rust"),
    ];

    let mut responses = Vec::new();

    for prompt in prompts {
        let result = model.complete(&prompt)?;
        assert!(!result.text.is_empty());
        responses.push(result.text);
    }

    // All responses should be different
    for i in 0..responses.len() {
        for j in (i + 1)..responses.len() {
            assert_ne!(
                responses[i], responses[j],
                "Different prompts should produce different responses"
            );
        }
    }

    Ok(())
}

#[test]
#[ignore] // Requires real model loading
fn test_candle_max_tokens_limit() -> Result<()> {
    // Test: Max tokens parameter should be respected
    let config = get_real_gguf_config();
    let model = LlmFactory::from_config(&config)?;

    // Test with very short max tokens
    let prompt = Prompt::new("System", "Write a long story about programming").with_max_tokens(16);

    let result = model.complete(&prompt)?;

    // Response should be non-empty but reasonably short
    assert!(!result.text.is_empty());
    assert!(result.text.len() < 500, "Response should be short with max_tokens=16");

    Ok(())
}

#[test]
#[ignore] // Requires real model loading
fn test_candle_error_handling() -> Result<()> {
    // Test: Error handling should work gracefully
    let config = get_real_gguf_config();
    let model = LlmFactory::from_config(&config)?;

    // Test with empty prompt
    let empty_prompt = Prompt::new("", "");
    let result = model.complete(&empty_prompt);
    assert!(result.is_ok(), "Should handle empty prompt gracefully");

    // Test with very long prompt (should handle gracefully or fail cleanly)
    let long_text = "x".repeat(10000);
    let long_prompt = Prompt::new("System", &long_text);
    let result = model.complete(&long_prompt);

    // Should either succeed or fail with a proper error, not panic
    match result {
        Ok(_) => println!("Long prompt handled successfully"),
        Err(e) => println!("Long prompt failed gracefully: {}", e),
    }

    Ok(())
}

#[test]
fn test_candle_model_file_exists() -> Result<()> {
    // Quick test to verify the model file exists without loading it
    let model_path = "/home/feanor/Projects/syncore/models/qwen2.5-0.5b.gguf";

    assert!(
        std::path::Path::new(model_path).exists(),
        "GGUF model file should exist at: {}",
        model_path
    );

    // Check file size (should be substantial for a real model)
    let metadata = std::fs::metadata(model_path)?;
    let size_mb = metadata.len() / (1024 * 1024);
    assert!(size_mb > 100, "Model file should be >100MB, found: {}MB", size_mb);

    println!("GGUF model found: {}MB", size_mb);

    Ok(())
}
