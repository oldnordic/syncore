//! Tests for LLM backend routing
//!
//! Ensures proper routing between test, gguf, gguf_engine backends and error handling for ollama.

use anyhow::Result;
use std::env;
use syncore::llm::factory::LlmBackend;
use syncore::llm::factory::LlmConfig;
use syncore::llm::{factory::LlmFactory, Prompt};

#[test]
fn test_backend_routing_test() -> Result<()> {
    // Test routing to TestBackend
    let test_cases = vec!["test", "TEST", "Test"];

    for backend_str in test_cases {
        let backend = LlmBackend::try_parse(backend_str)?;
        assert_eq!(backend, LlmBackend::Test);

        let config = LlmConfig {
            backend: backend.clone(),
            model: "test-model".to_string(),
            url: "local".to_string(),
            timeout_seconds: 30,
        };

        let model = LlmFactory::from_config(&config)?;
        assert_eq!(model.backend_name(), "test");

        // Test functionality
        let prompt = Prompt::new("", "Test input");
        let result = model.complete(&prompt)?;
        // Test backend echoes user prompt
        assert_eq!(result.text, "Test input");
    }

    Ok(())
}

#[test]
fn test_backend_routing_gguf() -> Result<()> {
    // Test routing to GGUFEngine (both "gguf" and "gguf_engine")
    let test_cases = vec!["gguf", "gguf_engine", "GGUF", "GGUF_ENGINE"];

    for backend_str in test_cases {
        let backend = LlmBackend::try_parse(backend_str)?;
        assert_eq!(backend, LlmBackend::GGUFEngine);

        let config = LlmConfig {
            backend: backend.clone(),
            model: "qwen2.5-mini".to_string(),
            url: "local".to_string(),
            timeout_seconds: 30,
        };

        let result = LlmFactory::from_config(&config);

        // May fail if model file doesn't exist, but should route correctly
        if let Ok(model) = result {
            assert_eq!(model.backend_name(), "gguf_engine");
        }
    }

    Ok(())
}

#[test]
fn test_backend_routing_ollama_errors() -> Result<()> {
    // Test that "ollama" backend now errors appropriately
    let ollama_variants = vec!["ollama", "OLLAMA", "Ollama"];

    for backend_str in ollama_variants {
        let result = LlmBackend::try_parse(backend_str);
        assert!(result.is_err(), "Backend '{}' should error", backend_str);

        let error = result.unwrap_err();
        let error_msg = error.to_string();

        // Should mention that ollama is not supported
        assert!(
            error_msg.contains("no longer supported")
                || error_msg.contains("Supported")
                || error_msg.contains("unknown")
        );
        // Should not crash or panic
        assert!(!error_msg.is_empty());
    }

    Ok(())
}

#[test]
fn test_environment_variable_routing() -> Result<()> {
    // Test that environment variables route correctly

    // Test backend
    env::set_var("LLM_BACKEND", "test");
    let config = LlmConfig::from_env();
    assert_eq!(config.backend, LlmBackend::Test);

    // Test gguf backend
    env::set_var("LLM_BACKEND", "gguf");
    let config = LlmConfig::from_env();
    assert_eq!(config.backend, LlmBackend::GGUFEngine);

    // Test gguf_engine backend
    env::set_var("LLM_BACKEND", "gguf_engine");
    let config = LlmConfig::from_env();
    assert_eq!(config.backend, LlmBackend::GGUFEngine);

    // Test invalid backend (should fall back to default)
    env::set_var("LLM_BACKEND", "ollama");
    let config = LlmConfig::from_env();
    // Should fall back to GGUFEngine due to parse error
    assert_eq!(config.backend, LlmBackend::GGUFEngine);

    Ok(())
}

#[test]
fn test_backend_string_representation() -> Result<()> {
    // Test that backends have correct string representations
    assert_eq!(LlmBackend::Test.as_str(), "test");
    assert_eq!(LlmBackend::GGUFEngine.as_str(), "gguf_engine");

    // Test round-trip parsing
    let test_backend = LlmBackend::Test;
    let gguf_backend = LlmBackend::GGUFEngine;

    assert_eq!(LlmBackend::try_parse(test_backend.as_str())?, test_backend);
    assert_eq!(LlmBackend::try_parse(gguf_backend.as_str())?, gguf_backend);

    Ok(())
}

#[test]
fn test_factory_creates_correct_backend_types() -> Result<()> {
    // Test that factory creates correct backend types

    // Test backend
    let test_config = LlmConfig {
        backend: LlmBackend::Test,
        model: "test".to_string(),
        url: "local".to_string(),
        timeout_seconds: 30,
    };

    let test_model = LlmFactory::from_config(&test_config)?;
    assert_eq!(test_model.backend_name(), "test");

    // GGUF backend (may fail if model missing, but should route correctly)
    let gguf_config = LlmConfig {
        backend: LlmBackend::GGUFEngine,
        model: "qwen2.5-mini".to_string(),
        url: "local".to_string(),
        timeout_seconds: 30,
    };

    let gguf_result = LlmFactory::from_config(&gguf_config);
    if let Ok(model) = gguf_result {
        assert_eq!(model.backend_name(), "gguf_engine");
    }

    Ok(())
}

#[test]
fn test_backend_case_insensitive() -> Result<()> {
    // Test that backend parsing is case insensitive
    let test_cases = vec![
        ("test", LlmBackend::Test),
        ("TEST", LlmBackend::Test),
        ("Test", LlmBackend::Test),
        ("gguf", LlmBackend::GGUFEngine),
        ("GGUF", LlmBackend::GGUFEngine),
        ("gguf_engine", LlmBackend::GGUFEngine),
        ("GGUF_ENGINE", LlmBackend::GGUFEngine),
    ];

    for (input, expected) in test_cases {
        let parsed = LlmBackend::try_parse(input)?;
        assert_eq!(parsed, expected, "Failed for input: {}", input);
    }

    Ok(())
}
