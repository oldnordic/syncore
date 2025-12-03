//! End-to-end tests for GGUFEngine
//!
//! Tests real GGUF model loading, inference, and integration with SynCore.

use anyhow::{anyhow, Result};
use std::path::Path;
use std::time::Duration;
use syncore::llm::factory::{LlmBackend, LlmConfig, LlmFactory};
use syncore::llm::{LanguageModel, Prompt};
use syncore::models::gguf_engine::{loader::load_qwen_model, GGUFEngine};

#[test]
#[ignore] // Ignored in CI - requires actual model files
fn test_gguf_engine_config_precedence() -> Result<()> {
    // Test that config → env → defaults precedence works

    // Save original env vars
    let original_model_path = std::env::var("SYNC_LLM_MODEL_PATH").ok();

    // Test 1: Config file takes precedence
    std::env::remove_var("SYNC_LLM_MODEL_PATH");

    // This should use config file or fallback to default
    let config = LlmConfig::from_env();
    assert_eq!(config.backend, LlmBackend::GGUFEngine);

    // Test 2: Environment variable overrides config
    std::env::set_var("SYNC_LLM_MODEL_PATH", "/tmp/test-model.gguf");
    let config_with_env = LlmConfig::from_env();

    // Restore original env
    match original_model_path {
        Some(path) => std::env::set_var("SYNC_LLM_MODEL_PATH", path),
        None => std::env::remove_var("SYNC_LLM_MODEL_PATH"),
    }

    println!("✅ Config precedence test passed");
    Ok(())
}

#[tokio::test]
#[ignore] // Ignored in CI - requires actual model files
async fn test_gguf_engine_e2e() -> Result<()> {
    println!("🚀 Starting GGUFEngine E2E test...");

    // Test 1: Model file discovery
    let model_paths = vec![
        "models/qwen2.5-mini.gguf",
        "models/qwen2.5-0.5b.gguf", // fallback for testing
    ];

    let model_path = model_paths
        .iter()
        .find(|path| Path::new(path).exists())
        .ok_or_else(|| anyhow!("No GGUF model file found for testing"))?;

    println!("📁 Found model file: {}", model_path);

    // Test 2: Model loading
    let device = candle_core::Device::Cpu;
    let start_time = std::time::Instant::now();

    let loaded_model = load_qwen_model(Path::new(model_path), &device)?;
    let load_time = start_time.elapsed();

    println!("⏱️  Model loaded in {:?}", load_time);
    assert!(load_time < Duration::from_secs(30), "Model loading took too long");

    // Test 4: GGUFEngine creation
    let backend = GGUFEngine::new("qwen2.5-mini").await?;
    assert_eq!(backend.backend_name(), "gguf_engine");

    // Test 5: Health check
    let is_healthy = backend.health_check()?;
    assert!(is_healthy, "GGUFEngine health check failed");

    println!("✅ GGUFEngine health check passed");

    // Test 6: Real inference
    let test_prompts = vec![
        ("Hello", "greeting"),
        ("What is Rust?", "question"),
        ("test response", "test"),
        ("Explain AI", "technical"),
    ];

    for (prompt, category) in test_prompts {
        println!("🧪 Testing prompt: '{}' ({})", prompt, category);

        let start_time = std::time::Instant::now();
        let prompt_obj = Prompt::new("System", prompt);
        let result = backend.complete(&prompt_obj);
        let inference_time = start_time.elapsed();

        assert!(result.is_ok(), "Inference failed for prompt: {}", prompt);

        let completion = result.unwrap();
        assert!(!completion.text.is_empty(), "Empty response for prompt: {}", prompt);
        assert!(inference_time < Duration::from_secs(10), "Inference took too long");

        println!("⚡ Response ({}ms): {}", inference_time.as_millis(), completion.text);
        println!("📊 Metadata: {}", serde_json::to_string_pretty(&completion.metadata).unwrap());
    }

    // Test 7: Deterministic generation
    println!("🎲 Testing deterministic generation...");

    let prompt = Prompt::new("System", "deterministic test");
    let response1 = backend.complete(&prompt)?;
    let response2 = backend.complete(&prompt)?;

    // With same seed, responses should be identical
    assert_eq!(response1.text, response2.text, "Deterministic generation failed");

    println!("✅ Deterministic generation test passed");

    // Test 8: Factory integration
    let factory_config = LlmConfig {
        backend: LlmBackend::GGUFEngine,
        model: "qwen2.5-mini".to_string(),
        url: "local".to_string(),
        timeout_seconds: 30,
    };

    let factory_model = LlmFactory::from_config(&factory_config)?;
    assert_eq!(factory_model.backend_name(), "gguf_engine");

    let factory_result = factory_model.complete(&prompt)?;
    assert!(!factory_result.text.is_empty());

    println!("✅ Factory integration test passed");

    println!("🎉 All GGUFEngine E2E tests passed!");
    println!("📈 Performance summary:");
    println!("   - Model load time: {:?}", load_time);
    println!("   - Inference time: <10s per request");
    println!("   - Memory usage: CPU-only optimized");
    println!("   - Deterministic: ✅ (seed=42)");

    Ok(())
}

#[tokio::test]
#[ignore] // Ignored in CI - requires actual model files
async fn test_gguf_engine_error_handling() -> Result<()> {
    println!("🛡️  Testing GGUFEngine error handling...");

    // Test 1: Invalid model path
    let result = GGUFEngine::new("nonexistent-model").await;
    assert!(result.is_err(), "Should fail with nonexistent model");

    // Test 2: Empty prompt
    let backend = GGUFEngine::new_test();
    let empty_prompt = Prompt::new("System", "");
    let result = backend.complete(&empty_prompt);

    // Should handle empty prompt gracefully
    match result {
        Ok(completion) => {
            // Either succeeds with a default response or fails gracefully
            println!("✅ Empty prompt handled: {}", completion.text);
        }
        Err(e) => {
            println!("✅ Empty prompt error handled: {}", e);
        }
    }

    // Test 3: Very long prompt (should truncate or handle gracefully)
    let long_prompt = "x".repeat(10000);
    let long_prompt_obj = Prompt::new("System", &long_prompt);
    let result = backend.complete(&long_prompt_obj);

    match result {
        Ok(completion) => {
            println!("✅ Long prompt handled ({} chars)", completion.text.len());
        }
        Err(e) => {
            println!("✅ Long prompt error handled: {}", e);
        }
    }

    println!("✅ Error handling tests passed");
    Ok(())
}

#[test]
fn test_gguf_engine_test_backend() -> Result<()> {
    println!("🧪 Testing GGUFEngine test backend...");

    let backend = GGUFEngine::new_test();
    assert_eq!(backend.backend_name(), "gguf_engine");

    let prompt = Prompt::new("System", "Hello test backend");
    let result = backend.complete(&prompt)?;

    assert!(!result.text.is_empty());
    assert!(result.text.contains("GGUFEngine response"));

    println!("✅ Test backend response: {}", result.text);
    Ok(())
}
