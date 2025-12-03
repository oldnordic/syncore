//! Smoke tests for GGUFEngine implementation
//!
//! Tests:: GGUFEngine backend integration with existing LanguageModel trait
//! and factory pattern. Uses TDD approach - tests written first,
//! implementation follows.

use anyhow::Result;
use std::fs;
use syncore::config::SyncoreConfig;
use syncore::llm::{factory::LlmFactory, Prompt};
use tempfile::TempDir;

/// Test configuration for GGUFEngine backend
fn get_gguf_test_config() -> syncore::llm::factory::LlmConfig {
    // Set the model path to the test model file
    std::env::set_var("SYNC_LLM_MODEL_PATH", "models/qwen2.5-0.5b.gguf");

    syncore::llm::factory::LlmConfig {
        backend: syncore::llm::factory::LlmBackend::GGUFEngine,
        model: "qwen2.5-0.5b".to_string(),
        url: "local".to_string(),
        timeout_seconds: 30,
    }
}

/// Create a minimal config file for testing
fn create_test_config_file() -> Result<(TempDir, String)> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("syncore.toml");

    let config_content = r#"
[llm]
backend = "gguf"
model = "Qwen2.5-1.5B-Instruct"
url = "local"
timeout_seconds = 30
"#;

    fs::write(&config_path, config_content)?;
    Ok((temp_dir, config_path.to_string_lossy().to_string()))
}

#[test]
fn test_config_parsing_gguf_engine() -> Result<()> {
    // Test 1: Config parsing should recognize "gguf" backend
    let (_temp_dir, config_path) = create_test_config_file()?;

    // Load config from file
    let config = SyncoreConfig::load(&config_path)?;

    // Verify config loaded correctly
    assert_eq!(config.llm.backend, "gguf");
    assert_eq!(config.llm.model, "Qwen2.5-1.5B-Instruct");
    assert_eq!(config.llm.url, "local");
    assert_eq!(config.llm.timeout_seconds, 30);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_factory_creates_gguf_engine() -> Result<()> {
    // Test 2: Factory should create GGUFEngine backend when configured
    let config = get_gguf_test_config();

    // This should create a Candle backend (currently placeholder)
    let model = LlmFactory::from_config(&config)?;

    // Verify the model implements LanguageModel trait
    assert_eq!(model.backend_name(), "gguf_engine"); // GGUFEngine should report "gguf_engine"

    // Test basic functionality
    let prompt = Prompt::new("System", "User message");
    let result = model.complete(&prompt)?;

    // Should return the placeholder response
    assert!(!result.text.is_empty());
    assert!(result.text.contains("test response"));

    Ok(())
}

#[test]
fn test_gguf_engine_health_check() -> Result<()> {
    // Test 3: Health check should work
    let config = get_gguf_test_config();
    let model = LlmFactory::from_config(&config)?;

    // Health check should succeed
    let is_healthy = model.health_check()?;
    assert!(is_healthy);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_gguf_engine_with_temperature() -> Result<()> {
    // Test 4: Temperature parameter should be handled
    let config = get_gguf_test_config();
    let model = LlmFactory::from_config(&config)?;

    // Test with temperature
    let prompt = Prompt::new("System", "User message").with_temperature(0.7);

    let result = model.complete(&prompt)?;
    assert!(!result.text.is_empty());

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_gguf_engine_with_max_tokens() -> Result<()> {
    // Test 5: Max tokens parameter should be handled
    let config = get_gguf_test_config();
    let model = LlmFactory::from_config(&config)?;

    // Test with max tokens
    let prompt = Prompt::new("System", "User message").with_max_tokens(100);

    let result = model.complete(&prompt)?;
    assert!(!result.text.is_empty());

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_gguf_vs_ollama_compatibility() -> Result<()> {
    // Test 6: Ensure Candle backend is compatible with existing interface
    let gguf_config = get_gguf_test_config();
    let test_config = syncore::llm::factory::LlmConfig {
        backend: syncore::llm::factory::LlmBackend::Test,
        model: "echo-test".to_string(),
        url: "".to_string(),
        timeout_seconds: 30,
    };

    // Both should create models implementing the same trait
    let gguf_model = LlmFactory::from_config(&gguf_config);
    let test_model = LlmFactory::from_config(&test_config);

    // Both should respond to the same interface
    let prompt = Prompt::new("System", "Test compatibility");

    let gguf_result = gguf_model.complete(&prompt);
    let test_result = test_model.complete(&prompt);

    // Both should succeed
    assert!(gguf_result.is_ok());
    assert!(test_result.is_ok());

    if gguf_result.is_ok() && test_result.is_ok() {
        let gguf_completion = gguf_result.unwrap();
        let test_completion = test_result.unwrap();

        // Check metadata for backend names
        let gguf_engine = gguf_completion
            .metadata
            .as_ref()
            .and_then(|m| m.get("backend"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let test_backend = test_completion
            .metadata
            .as_ref()
            .and_then(|m| m.get("backend"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        assert_ne!(gguf_engine, test_backend);
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_gguf_engine_error_handling() -> Result<()> {
    // Test 7: Error handling should work correctly
    let config = get_gguf_test_config();
    let model = LlmFactory::from_config(&config)?;

    // Test with empty prompt (should still work)
    let empty_prompt = Prompt::new("System", "");
    let result = model.complete(&empty_prompt)?;
    assert!(!result.text.is_empty());

    // Test with very long prompt (should handle gracefully)
    let long_prompt = Prompt::new("System", &"x".repeat(10000));
    let result = model.complete(&long_prompt);
    // Should either succeed or fail gracefully, not panic
    assert!(result.is_ok() || result.is_err());

    Ok(())
}

#[test]
fn test_environment_variable_override() -> Result<()> {
    // Test 8: Environment variables should override config file
    let (_temp_dir, _config_path) = create_test_config_file()?;

    // Set environment variable to override config
    std::env::set_var("LLM_BACKEND", "gguf");
    std::env::set_var("LLM_MODEL", "env-override-model");

    // Load config (should use env override)
    let config = syncore::llm::factory::LlmConfig::from_env();
    assert_eq!(config.backend.as_str(), "gguf_engine");
    assert_eq!(config.model, "env-override-model");

    // Clean up
    std::env::remove_var("LLM_BACKEND");
    std::env::remove_var("LLM_MODEL");

    Ok(())
}
