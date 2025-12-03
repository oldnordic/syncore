//! Tests for global LLM determinism
//!
//! Ensures the same text generation from MCP tools and CLI commands produces byte-for-byte identical output.

use anyhow::Result;
use std::env;
use std::sync::{Arc, Mutex};
use syncore::llm::factory::LlmBackend;
use syncore::llm::factory::LlmConfig;
use syncore::llm::{factory::LlmFactory, Prompt};

#[test]
fn test_deterministic_output_across_instances() -> Result<()> {
    // Test that multiple instances produce identical output
    env::set_var("LLM_BACKEND", "test");

    let config = LlmConfig::from_env();

    // Create multiple model instances
    let model1 = LlmFactory::from_config(&config)?;
    let model2 = LlmFactory::from_config(&config)?;

    let prompt = Prompt::new("System: You are a test bot.", "Respond with exactly 'DETERMINISTIC'");

    let result1 = model1.complete(&prompt)?;
    let result2 = model2.complete(&prompt)?;

    // Should be byte-for-byte identical
    assert_eq!(result1.text, result2.text);
    assert_eq!(result1.text, "Respond with exactly 'DETERMINISTIC'");

    Ok(())
}

#[test]
fn test_deterministic_output_across_calls() -> Result<()> {
    // Test that multiple calls to same instance produce identical output
    env::set_var("LLM_BACKEND", "test");

    let config = LlmConfig::from_env();
    let model = LlmFactory::from_config(&config)?;

    let prompt = Prompt::new("System", "Echo this exactly: TEST123");

    let result1 = model.complete(&prompt)?;
    let result2 = model.complete(&prompt)?;

    // Should be identical across calls
    assert_eq!(result1.text, result2.text);
    assert_eq!(result1.text, "Echo this exactly: TEST123");

    Ok(())
}

#[test]
fn test_deterministic_output_with_different_system_prompts() -> Result<()> {
    // Test determinism with different system prompts
    env::set_var("LLM_BACKEND", "test");

    let config = LlmConfig::from_env();
    let model = LlmFactory::from_config(&config)?;

    // Same user prompt, different system prompts
    let prompt1 = Prompt::new("System A", "User message");
    let prompt2 = Prompt::new("System B", "User message");

    let result1 = model.complete(&prompt1)?;
    let result2 = model.complete(&prompt2)?;

    // Test backend should echo user message regardless of system prompt
    assert_eq!(result1.text, "User message");
    assert_eq!(result2.text, "User message");
    assert_eq!(result1.text, result2.text);

    Ok(())
}

#[test]
fn test_deterministic_output_temperature_override() -> Result<()> {
    // Test that temperature settings are handled deterministically
    env::set_var("LLM_BACKEND", "test");

    let config = LlmConfig::from_env();
    let model = LlmFactory::from_config(&config)?;

    // Same prompt with different temperature settings
    let prompt1 = Prompt::new("System", "Test message").with_temperature(0.0);
    let prompt2 = Prompt::new("System", "Test message").with_temperature(1.0);

    let result1 = model.complete(&prompt1)?;
    let result2 = model.complete(&prompt2)?;

    // Test backend should ignore temperature and be deterministic
    assert_eq!(result1.text, result2.text);

    Ok(())
}

#[test]
fn test_deterministic_output_max_tokens_override() -> Result<()> {
    // Test that max_tokens settings are handled deterministically
    env::set_var("LLM_BACKEND", "test");

    let config = LlmConfig::from_env();
    let model = LlmFactory::from_config(&config)?;

    // Same prompt with different max_tokens settings
    let prompt1 = Prompt::new("System", "Test message").with_max_tokens(10);
    let prompt2 = Prompt::new("System", "Test message").with_max_tokens(100);

    let result1 = model.complete(&prompt1)?;
    let result2 = model.complete(&prompt2)?;

    // Test backend should ignore max_tokens and be deterministic
    assert_eq!(result1.text, result2.text);

    Ok(())
}

#[test]
fn test_concurrent_deterministic_output() -> Result<()> {
    // Test deterministic behavior across concurrent calls
    env::set_var("LLM_BACKEND", "test");

    let config = LlmConfig::from_env();
    let model = Arc::new(LlmFactory::from_config(&config)?);

    let prompt = Prompt::new("System", "Concurrent test");
    let results = Arc::new(Mutex::new(Vec::new()));

    let mut handles = vec![];

    // Spawn multiple concurrent calls
    for _ in 0..5 {
        let model_clone = Arc::clone(&model);
        let prompt_clone = prompt.clone();
        let results_clone = Arc::clone(&results);

        let handle = std::thread::spawn(move || -> Result<()> {
            let result = model_clone.complete(&prompt_clone)?;
            let mut results_vec = results_clone.lock().unwrap();
            results_vec.push(result.text);
            Ok(())
        });

        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap()?;
    }

    // All results should be identical
    let results_vec = results.lock().unwrap();
    for i in 1..results_vec.len() {
        assert_eq!(results_vec[0], results_vec[i], "Results differ at index {}", i);
    }

    Ok(())
}

#[test]
fn test_deterministic_output_complex_prompt() -> Result<()> {
    // Test determinism with complex multi-line prompts
    env::set_var("LLM_BACKEND", "test");

    let config = LlmConfig::from_env();
    let model = LlmFactory::from_config(&config)?;

    let complex_user = "Line 1\nLine 2\nLine 3\nSpecial chars: !@#$%^&*()\nUnicode: 🚀🔥💻";
    let prompt = Prompt::new("Complex system prompt\nWith multiple lines", complex_user);

    let result1 = model.complete(&prompt)?;
    let result2 = model.complete(&prompt)?;

    // Should be identical even with complex content
    assert_eq!(result1.text, result2.text);
    assert_eq!(result1.text, complex_user);

    Ok(())
}

#[test]
fn test_deterministic_output_empty_prompts() -> Result<()> {
    // Test determinism with edge case prompts
    env::set_var("LLM_BACKEND", "test");

    let config = LlmConfig::from_env();
    let model = LlmFactory::from_config(&config)?;

    // Empty system and user prompts
    let empty_prompt = Prompt::new("", "");

    let result1 = model.complete(&empty_prompt)?;
    let result2 = model.complete(&empty_prompt)?;

    // Should be identical (empty string)
    assert_eq!(result1.text, result2.text);
    assert_eq!(result1.text, "");

    Ok(())
}
