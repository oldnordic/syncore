//! Simple integration test for GGUFEngine with factory
//!
//! Tests basic GGUFEngine functionality without complex runtime issues.

use anyhow::Result;
use syncore::llm::factory::{LlmBackend, LlmConfig, LlmFactory};
use syncore::llm::{LanguageModel, Prompt};

#[test]
fn test_gguf_engine_factory_integration() -> Result<()> {
    println!("🧪 Testing GGUFEngine factory integration...");

    // Test 1: Test backend creation
    let backend = syncore::models::gguf_engine::GGUFEngine::new_test();
    assert_eq!(backend.backend_name(), "gguf_engine");

    let prompt = Prompt::new("System", "Hello test backend");
    let result = backend.complete(&prompt)?;
    assert!(!result.text.is_empty());
    assert!(result.text.contains("GGUFEngine response"));

    println!("✅ Test backend working: {}", result.text);

    // Test 2: Factory with test backend
    let test_config = LlmConfig {
        backend: LlmBackend::Test,
        model: "test-model".to_string(),
        url: "local".to_string(),
        timeout_seconds: 30,
    };

    let factory_model = LlmFactory::from_config(&test_config)?;
    assert_eq!(factory_model.backend_name(), "test");

    let factory_result = factory_model.complete(&prompt)?;
    assert!(!factory_result.text.is_empty());

    println!("✅ Factory test backend working: {}", factory_result.text);

    // Test 3: Config parsing
    let config = LlmConfig::from_env();
    assert_eq!(config.backend, LlmBackend::GGUFEngine);
    assert_eq!(config.model, "qwen2.5-mini");

    println!(
        "✅ Config parsing working: backend={}, model={}",
        config.backend.as_str(),
        config.model
    );

    println!("🎉 GGUFEngine factory integration test passed!");
    Ok(())
}

#[test]
fn test_gguf_engine_config_validation() -> Result<()> {
    println!("🔧 Testing GGUFEngine config validation...");

    // Test backend parsing
    assert_eq!(LlmBackend::try_parse("gguf")?, LlmBackend::GGUFEngine);
    assert_eq!(LlmBackend::try_parse("GGUF_ENGINE")?, LlmBackend::GGUFEngine);
    assert_eq!(LlmBackend::try_parse("test")?, LlmBackend::Test);

    // Test default config
    let default_config = LlmConfig::default();
    assert_eq!(default_config.backend, LlmBackend::GGUFEngine);
    assert_eq!(default_config.model, "qwen2.5-mini");
    assert_eq!(default_config.url, "local");
    assert_eq!(default_config.timeout_seconds, 30);

    println!("✅ Config validation passed");
    Ok(())
}

#[test]
fn test_gguf_engine_health_check() -> Result<()> {
    println!("🏥 Testing GGUFEngine health check...");

    let backend = syncore::models::gguf_engine::GGUFEngine::new_test();
    let is_healthy = backend.health_check()?;
    assert!(is_healthy, "Test backend should always be healthy");

    println!("✅ Health check passed");
    Ok(())
}
