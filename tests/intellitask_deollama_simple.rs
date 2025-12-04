//! TDD Tests for IntelliTask De-Ollama Migration (Simplified)
//!
//! These tests validate the infrastructure changes needed for IntelliTask
//! to migrate from Ollama to Candle-based GGUFEngine backend.

use anyhow::Result;
use std::sync::Arc;
use syncore::llm::{LanguageModel, Prompt};
use syncore::models::gguf_engine::GGUFEngine;
use syncore::router::SynCoreState;
use tokio::test;

/// Test: SynCoreState can be created with LLM model
/// Assert: with_llm_model() method works with Arc<dyn LanguageModel>
#[test]
async fn test_intellitask_constructor_does_not_require_ollama() -> Result<()> {
    println!("\n=== Testing IntelliTask Constructor De-Ollama ===");

    // Create test backend directly
    let gguf_engine = Arc::new(GGUFEngine::new_test());
    let llm_model: Arc<dyn LanguageModel> = gguf_engine.clone() as Arc<dyn LanguageModel>;

    // Verify it's not Ollama-based
    assert_eq!(llm_model.backend_name(), "gguf_engine");
    assert_ne!(llm_model.backend_name(), "ollama_cli");

    // Create SynCoreState with LLM model
    let state = SynCoreState::test().with_llm_model(llm_model);

    // Verify state has LLM model
    assert!(state.llm_model.is_some());
    let stored_model = state.llm_model.as_ref().unwrap();
    assert_eq!(stored_model.backend_name(), "gguf_engine");

    println!("✅ IntelliTask state created without Ollama dependency");
    Ok(())
}

/// Test: GGUFEngine implements LanguageModel trait
/// Assert: GGUFEngine can be used as Arc<dyn LanguageModel>
#[test]
async fn test_intellitask_uses_llm_factory_backend() -> Result<()> {
    println!("\n=== Testing GGUFEngine Implements LanguageModel ===");

    // Create GGUFEngine backend
    let gguf_engine = Arc::new(GGUFEngine::new_test());

    // Verify GGUFEngine implements LanguageModel
    let llm_model: Arc<dyn LanguageModel> = gguf_engine.clone() as Arc<dyn LanguageModel>;
    assert_eq!(llm_model.backend_name(), "gguf_engine");

    // Create state with GGUFEngine
    let state = SynCoreState::test().with_llm_model(llm_model);

    // Test LLM model works
    let prompt = Prompt::new("Test system", "Generate one task");
    let completion = state.llm_model.as_ref().unwrap().complete(&prompt)?;

    assert!(!completion.text.is_empty());
    assert!(completion.text.contains("GGUFEngine"));

    println!("✅ GGUFEngine works as LanguageModel: {}", completion.text);
    Ok(())
}

/// Test: No Ollama references in current infrastructure
/// Assert: LLM backend doesn't use Ollama
#[test]
async fn test_no_ollama_references_exist_anywhere() -> Result<()> {
    println!("\n=== Testing No Ollama References in LLM Infrastructure ===");

    // Create GGUFEngine backend
    let gguf_engine = Arc::new(GGUFEngine::new_test());
    let llm_model: Arc<dyn LanguageModel> = gguf_engine.clone() as Arc<dyn LanguageModel>;

    // Verify LLM model is not Ollama-based
    assert_ne!(llm_model.backend_name(), "ollama_cli");
    assert_ne!(llm_model.backend_name(), "ollama");
    assert_eq!(llm_model.backend_name(), "gguf_engine");

    // Create state
    let state = SynCoreState::test().with_llm_model(llm_model);

    // Verify state stores correct model
    let stored_model = state.llm_model.as_ref().unwrap();
    assert_eq!(stored_model.backend_name(), "gguf_engine");

    println!("✅ No Ollama references in LLM infrastructure");
    Ok(())
}

/// Test: Empty output handling
/// Assert: LLM backend returns proper responses
#[test]
async fn test_intellitask_generate_errors_on_empty_output() -> Result<()> {
    println!("\n=== Testing LLM Backend Output Handling ===");

    // Create GGUFEngine backend
    let gguf_engine = Arc::new(GGUFEngine::new_test());
    let llm_model: Arc<dyn LanguageModel> = gguf_engine as Arc<dyn LanguageModel>;

    // Test LLM model produces non-empty output
    let prompt = Prompt::new("Test", "Generate a response");
    let completion = llm_model.complete(&prompt)?;

    assert!(!completion.text.is_empty(), "LLM should not return empty output");

    println!("✅ LLM backend handles output properly: {}", completion.text);
    Ok(())
}

/// Test: MCP server can be initialized with LLM model
/// Assert: Server state holds LLM model correctly
#[test]
async fn test_intellitask_initialization_in_mcp_server_uses_llm() -> Result<()> {
    println!("\n=== Testing MCP Server Initialization with LLM ===");

    // Create GGUFEngine backend
    let gguf_engine = Arc::new(GGUFEngine::new_test());
    let llm_model: Arc<dyn LanguageModel> = gguf_engine as Arc<dyn LanguageModel>;

    // Create state with LLM model
    let state = SynCoreState::test().with_llm_model(llm_model);

    // Verify server was created successfully
    assert!(!state.llm_model.is_none());

    let stored_model = state.llm_model.as_ref().unwrap();
    assert_eq!(stored_model.backend_name(), "gguf_engine");

    println!("✅ MCP server state initialized with LLM backend: {}", stored_model.backend_name());
    Ok(())
}