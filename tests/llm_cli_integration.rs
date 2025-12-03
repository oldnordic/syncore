//! Integration tests for CLI-based LLM commands
//!
//! Ensures CLI-based LLM commands route to GGUFEngine and produce deterministic output.

use anyhow::Result;
use std::env;
use syncore::llm::factory::LlmBackend;
use syncore::llm::factory::LlmConfig;
use syncore::llm::{factory::LlmFactory, Prompt};

#[test]
fn test_cli_llm_routes_to_gguf_engine() -> Result<()> {
    // Set environment to use GGUFEngine
    env::set_var("LLM_BACKEND", "gguf");
    env::set_var("LLM_MODEL", "qwen2.5-mini");

    let config = LlmConfig::from_env();
    assert_eq!(config.backend, LlmBackend::GGUFEngine);

    // Create the model through factory (may fail if model file missing)
    let model_result = LlmFactory::from_config(&config);

    match model_result {
        Ok(model) => {
            assert_eq!(model.backend_name(), "gguf_engine");

            // Test deterministic generation
            let prompt = Prompt::new("You are a helpful assistant.", "Say 'Hello World'");
            let result = model.complete(&prompt)?;

            // Should contain expected response (deterministic)
            assert!(result.text.contains("Hello") || result.text.contains("World"));
        }
        Err(e) => {
            // Expected if model file doesn't exist
            assert!(
                e.to_string().contains("Model file not found") || e.to_string().contains("GGUF")
            );
        }
    }

    Ok(())
}

#[test]
fn test_cli_deterministic_output() -> Result<()> {
    // Set environment for deterministic GGUFEngine
    env::set_var("LLM_BACKEND", "gguf_engine");
    env::set_var("LLM_MODEL", "qwen2.5-mini");

    let config = LlmConfig::from_env();
    let model_result = LlmFactory::from_config(&config);

    match model_result {
        Ok(model) => {
            // Generate same prompt twice
            let prompt = Prompt::new("System: Respond with exactly '42'", "What is the answer?");

            let result1 = model.complete(&prompt)?;
            let result2 = model.complete(&prompt)?;

            // Should be identical (deterministic)
            assert_eq!(result1.text, result2.text);
        }
        Err(e) => {
            // Expected if model file doesn't exist
            assert!(
                e.to_string().contains("Model file not found") || e.to_string().contains("GGUF")
            );
        }
    }

    Ok(())
}

#[test]
fn test_cli_with_test_backend() -> Result<()> {
    // Set environment to use test backend
    env::set_var("LLM_BACKEND", "test");

    let config = LlmConfig::from_env();
    assert_eq!(config.backend, LlmBackend::Test);

    let model = LlmFactory::from_config(&config)?;
    assert_eq!(model.backend_name(), "test");

    // Test deterministic behavior
    let prompt = Prompt::new("", "Echo this text");
    let result = model.complete(&prompt)?;

    // Test backend should echo the user prompt
    assert_eq!(result.text, "Echo this text");

    Ok(())
}

#[test]
fn test_cli_gguf_engine_model_validation() -> Result<()> {
    // Test with different GGUF model names
    let test_cases = vec![
        ("gguf", "qwen2.5-mini"),
        ("gguf_engine", "qwen2.5-mini"),
        ("gguf", "models/qwen2.5-mini.gguf"),
    ];

    for (backend, model) in test_cases {
        env::set_var("LLM_BACKEND", backend);
        env::set_var("LLM_MODEL", model);

        let config = LlmConfig::from_env();
        assert_eq!(config.backend, LlmBackend::GGUFEngine);
        assert_eq!(config.model, model);

        // Verify factory can create the model (may fail if model file doesn't exist)
        let result = LlmFactory::from_config(&config);

        // If model file exists, it should work
        if result.is_ok() {
            let model_instance = result.unwrap();
            assert_eq!(model_instance.backend_name(), "gguf_engine");
        }
        // If model file doesn't exist, that's expected for this test
    }

    Ok(())
}
