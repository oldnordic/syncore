//! End-to-end integration test for GGUFEngine with external GGUF model
//!
//! This test verifies:
//! - External GGUF model loading from config
//! - External tokenizer loading from config  
//! - Real inference using mistral.rs
//! - Deterministic output with fixed seed
//!
//! Run with: cargo test candle_integration_e2e -- --ignored

use anyhow::Result;
use std::path::Path;
use syncore::config::SyncoreConfig;
use syncore::llm::{LanguageModel, Prompt};
use syncore::models::gguf_engine::GGUFEngine;
use tempfile::TempDir;
use tokio;

#[test]
#[ignore] // Requires external model file, not for CI
fn candle_integration_e2e() -> Result<()> {
    // Create temporary config
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("syncore.toml");

    // Create test config pointing to external model
    let config_content = r#"
[llm]
backend = "gguf"
model = "qwen2.5-0.5b"
model_path = "models/qwen2.5-0.5b.gguf"
tokenizer_path = "models/qwen2.5/tokenizer.json"
timeout_seconds = 30
"#;

    std::fs::write(&config_path, config_content)?;

    // Load and initialize config
    let config = SyncoreConfig::load(&config_path.to_string_lossy())?;
    SyncoreConfig::init_global(config);

    // Verify model file exists
    let model_path = Path::new("models/qwen2.5-0.5b.gguf");
    if !model_path.exists() {
        println!("Skipping test - model file not found at: {:?}", model_path);
        return Ok(());
    }

    // Create Candle backend
    let rt = tokio::runtime::Runtime::new()?;
    let backend = rt.block_on(async { GGUFEngine::new("qwen2.5-0.5b").await })?;

    // Health check
    assert!(backend.health_check()?, "Backend health check failed");

    // Test deterministic inference
    let prompt = Prompt::new("System: You are helpful.", "Say 'test response'");
    let completion = backend.complete(&prompt)?;

    // Verify response
    assert!(!completion.text.is_empty(), "Generated text should not be empty");
    assert!(completion.metadata.is_some(), "Should have metadata");

    let metadata = completion.metadata.unwrap();
    assert_eq!(metadata["backend"], "gguf_engine");
    assert_eq!(metadata["model_loaded"], true);
    assert_eq!(metadata["inference_type"], "real_gguf");

    println!("Generated text: {}", completion.text);
    println!("Model path: {}", metadata["model_path"]);

    // Test deterministic output - same prompt should give same result
    let completion2 = backend.complete(&prompt)?;
    assert_eq!(completion.text, completion2.text, "Output should be deterministic");

    println!("✅ Candle integration test passed!");
    Ok(())
}

#[test]
#[ignore] // Requires external model file, not for CI
fn candle_integration_config_precedence() -> Result<()> {
    // Test config → env → defaults precedence

    // Set environment variable
    std::env::set_var("SYNC_LLM_MODEL_PATH", "models/qwen2.5-0.5b.gguf");

    // Create config without model_path
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("syncore.toml");

    let config_content = r#"
[llm]
backend = "gguf"
model = "qwen2.5-0.5b"
"#;

    std::fs::write(&config_path, config_content)?;

    // Load config with env overrides
    let config = SyncoreConfig::load_with_env(&config_path.to_string_lossy())?;
    let model_path = config.llm.model_path.clone(); // Clone before move
    SyncoreConfig::init_global(config);

    // Verify env var was used
    assert_eq!(model_path, "models/qwen2.5-0.5b.gguf");

    // Clean up
    std::env::remove_var("SYNC_LLM_MODEL_PATH");

    println!("✅ Config precedence test passed!");
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn candle_integration_fallback_behavior() -> Result<()> {
    // Test fallback when model file doesn't exist

    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("syncore.toml");

    // Config pointing to non-existent model
    let config_content = r#"
[llm]
backend = "gguf"
model = "nonexistent-model"
model_path = "models/nonexistent.gguf"
tokenizer_path = "models/nonexistent/tokenizer.json"
"#;

    std::fs::write(&config_path, config_content)?;

    let config = SyncoreConfig::load(&config_path.to_string_lossy())?;
    SyncoreConfig::init_global(config);

    // Clear environment variable to test fallback behavior
    std::env::remove_var("SYNC_LLM_MODEL_PATH");

    // Set fallback to the actual test model file
    std::env::set_var("SYNC_LLM_MODEL_PATH", "models/qwen2.5-0.5b.gguf");

    // Backend creation should succeed but health check should pass due to fallback
    let backend = GGUFEngine::new("nonexistent-model").await?;

    // Health check should pass due to fallback to existing model
    assert!(backend.health_check()?, "Health check should pass due to fallback to existing model");

    // Completion should work since model loads successfully via fallback
    let prompt = Prompt::new("System", "Test");
    let completion = backend.complete(&prompt)?;

    assert!(!completion.text.is_empty());
    assert_eq!(completion.metadata.unwrap()["model_loaded"], true);

    println!("✅ Fallback behavior test passed!");
    Ok(())
}
