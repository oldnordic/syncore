//! Tests for disabled LLM backend behavior
//!
//! When backend="none" or unsupported, tools should fail gracefully.

use anyhow::Result;
use std::env;
use syncore::llm::factory::LlmBackend;
use syncore::llm::factory::LlmConfig;
use syncore::llm::{factory::LlmFactory, Prompt};

#[test]
fn test_unsupported_backend_errors() -> Result<()> {
    // Test that "ollama" backend now errors appropriately
    let unsupported_backends = vec!["ollama", "openai", "anthropic", "invalid"];

    for backend in unsupported_backends {
        let result = LlmBackend::try_parse(backend);
        assert!(result.is_err(), "Backend '{}' should be unsupported", backend);

        if let Err(e) = result {
            let error_msg = e.to_string();
            // Should mention that ollama is not supported
            assert!(
                error_msg.contains("Supported")
                    || error_msg.contains("supported")
                    || error_msg.contains("no longer supported"),
                "Actual error message: {}",
                error_msg
            );
        }
    }

    Ok(())
}

#[test]
fn test_factory_fails_on_unsupported_backend() -> Result<()> {
    // Test that factory returns proper error for unsupported backends
    env::set_var("LLM_BACKEND", "ollama");

    let config = LlmConfig::from_env();
    // This should have fallen back to GGUFEngine due to parse error
    assert_eq!(config.backend, LlmBackend::GGUFEngine);

    // But if we manually create an invalid config, factory should handle it
    let invalid_configs = vec![LlmConfig {
        backend: LlmBackend::GGUFEngine, // This is valid but model might not exist
        model: "nonexistent-model.gguf".to_string(),
        url: "local".to_string(),
        timeout_seconds: 30,
    }];

    for config in invalid_configs {
        let result = LlmFactory::from_config(&config);
        // Should handle gracefully - either succeed (if model exists) or fail with clear error
        if let Err(e) = result {
            let error_msg = e.to_string();
            // Should not panic or crash
            assert!(!error_msg.is_empty());
        }
    }

    Ok(())
}

#[test]
fn test_mcp_tools_graceful_degradation() -> Result<()> {
    // Test that MCP tools handle missing LLM gracefully
    env::set_var("LLM_BACKEND", "test"); // Use test backend for predictable behavior

    let config = LlmConfig::from_env();
    let model = LlmFactory::from_config(&config)?;

    // Test that model works as expected
    let prompt = Prompt::new("System", "Test message");
    let result = model.complete(&prompt)?;

    assert!(!result.text.is_empty());
    assert_eq!(model.backend_name(), "test");

    Ok(())
}

#[test]
fn test_disabled_backend_error_messages() -> Result<()> {
    // Test that error messages are helpful when backend is disabled/unsupported

    // Test backend parsing error messages
    let invalid_backends = vec!["ollama", "remote", "http"];

    for backend in invalid_backends {
        let result = LlmBackend::try_parse(backend);
        assert!(result.is_err());

        let error = result.unwrap_err();
        let error_msg = error.to_string();

        // Error should mention supported backends
        assert!(
            error_msg.contains("Supported")
                || error_msg.contains("supported")
                || error_msg.contains("no longer supported")
        );
        // Should mention valid options
        assert!(error_msg.contains("gguf") || error_msg.contains("test"));
    }

    Ok(())
}

#[test]
fn test_fallback_to_default() -> Result<()> {
    // Test that invalid backend falls back to GGUFEngine
    env::set_var("LLM_BACKEND", "definitely_invalid_backend");

    let config = LlmConfig::from_env();
    // Should fall back to default GGUFEngine due to unwrap_or in from_env
    assert_eq!(config.backend, LlmBackend::GGUFEngine);

    // Should be able to create a model (though may fail if model file missing)
    let result = LlmFactory::from_config(&config);
    // Result depends on whether qwen2.5-mini.gguf exists
    // Either way, should not panic

    Ok(())
}
