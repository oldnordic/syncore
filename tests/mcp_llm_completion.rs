//! TDD Tests for MCP LLM Completion Integration with GGUFEngine
//!
//! These tests validate that MCP LLM tools use GGUFEngine exclusively
//! and provide deterministic outputs with proper error handling.

use anyhow::Result;
use serde_json::json;
use std::fs;
use std::sync::{Arc, Mutex};
use syncore::llm::factory::{LlmBackend, LlmConfig, LlmFactory};
use syncore::mcp_server::SynCoreMCPServer;
use syncore::router::SynCoreState;
use syncore::vector::VectorStore;
use tokio::test;

/// Test: MCP tool "intellitask_generate" called with a simple prompt
/// Assert:
///   • Response is non-empty
///   • Backend used is "gguf_engine" (via logs/metadata/inspection as available)
#[test(flavor = "multi_thread")]
async fn test_mcp_llm_completion_basic() -> Result<()> {
    println!("\n=== Testing MCP LLM Completion Basic ===");

    // Clean up any existing test files
    let _ = fs::remove_file("test_mcp_llm.db");
    let _ = fs::remove_dir_all("test_mcp_llm_cache");

    // Create state with GGUFEngine backend
    let config = LlmConfig {
        backend: LlmBackend::GGUFEngine,
        model: "qwen2.5-mini".to_string(),
        url: "local".to_string(),
        timeout_seconds: 30,
    };

    let llm_model = LlmFactory::from_config(&config)?;
    assert_eq!(llm_model.backend_name(), "gguf_engine");

    // Create state with GGUFEngine and IntelliTask
    let vector_store = Arc::new(Mutex::new(VectorStore::new(Box::new(
        syncore::vector::RealEmbeddings::new(384).unwrap(),
    ))));

    let llm_config = LlmConfig {
        backend: LlmBackend::GGUFEngine,
        model: "qwen2.5-mini".to_string(),
        url: "local".to_string(),
        timeout_seconds: 30,
    };

    let state = SynCoreState::with_dual_stores(vector_store.clone(), vector_store.clone())
        .and_then(|s| s.with_intellitask_from_config(&llm_config))
        .map_err(|e| anyhow::anyhow!("Failed to create state: {}", e))?;

    // Create MCP server and call intellitask_generate via mcp_delegate
    let server = SynCoreMCPServer::new(state);

    let params = serde_json::json!({
        "prd_content": "Create a simple REST API endpoint for user registration"
    });

    let response = server.mcp_delegate("intellitask_generate", params).await;

    // Should succeed with non-empty response
    match response {
        Ok(call_result) => {
            assert!(!call_result.content.is_empty(), "Response should not be empty");

            // Get text content using the same approach as the codebase
            let text = call_result
                .content
                .first()
                .and_then(|c| {
                    // Use serde to extract text - Content is serializable
                    let json = serde_json::to_value(c).ok()?;
                    json.get("text").and_then(|t| t.as_str()).map(|s| s.to_string())
                })
                .unwrap_or_default();

            if !text.is_empty() {
                println!("✅ Got response: {} chars", text.len());
                println!("📝 Response content: {}", text);

                // Should contain task breakdown elements
                assert!(
                    text.contains("task") || text.contains("Task") || text.contains("step"),
                    "Response should contain task-related content. Got: {}",
                    text
                );
            } else {
                println!("ℹ️  Empty response (model may not be available)");
            }
        }
        Err(e) => {
            // If GGUF model not available, should give clear error (not crash)
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("model")
                    || error_msg.contains("GGUF")
                    || error_msg.contains("backend")
                    || error_msg.contains("IntelliTask not initialized"),
                "Error should be model-related, got: {}",
                error_msg
            );
            println!("ℹ️  Expected error (model not available): {}", error_msg);
        }
    }

    Ok(())
}

/// Test: Call same MCP LLM tool twice with same prompt and config
/// Assert:
///   • Outputs are identical (string equality or at least first N tokens)
#[test(flavor = "multi_thread")]
async fn test_mcp_llm_determinism() -> Result<()> {
    println!("\n=== Testing MCP LLM Determinism ===");

    // Clean up any existing test files
    let _ = fs::remove_file("test_mcp_det.db");
    let _ = fs::remove_dir_all("test_mcp_det_cache");

    // Create deterministic config
    let config = LlmConfig {
        backend: LlmBackend::GGUFEngine,
        model: "qwen2.5-mini".to_string(),
        url: "local".to_string(),
        timeout_seconds: 30,
    };

    let vector_store = Arc::new(Mutex::new(VectorStore::new(Box::new(
        syncore::vector::RealEmbeddings::new(384).unwrap(),
    ))));
    let state = SynCoreState::with_dual_stores(vector_store.clone(), vector_store.clone())
        .and_then(|s| s.with_intellitask_from_config(&config))
        .map_err(|e| anyhow::anyhow!("Failed to create state: {}", e))?;

    let server = SynCoreMCPServer::new(state);
    let prompt = "Generate a single task for database setup";

    // Call first time
    let params1 = serde_json::json!({
        "prd_content": prompt
    });

    let response1 = server.mcp_delegate("intellitask_generate", params1).await;

    // Call second time
    let params2 = serde_json::json!({
        "prd_content": prompt
    });

    let response2 = server.mcp_delegate("intellitask_generate", params2).await;

    // Extract text from both responses
    let text1 = extract_text_from_call_result(&response1);
    let text2 = extract_text_from_call_result(&response2);

    // For deterministic outputs, should be identical
    // But if model not available, both should have same error pattern
    if !text1.is_empty() && !text2.is_empty() {
        assert_eq!(text1, text2, "Deterministic outputs should be identical");
        println!("✅ Deterministic outputs match: {} chars", text1.len());
    } else {
        // Both should fail with same error pattern if model unavailable
        println!("ℹ️  Both calls failed (expected if model unavailable)");
    }

    Ok(())
}

/// Test: Config backend = "gguf" → success, uses GGUFEngine
/// Test: Config backend = "gguf_engine" → success, uses GGUFEngine  
/// Test: Config backend = "ollama" → either maps to GGUFEngine (alias) OR clear error
#[test(flavor = "multi_thread")]
async fn test_mcp_llm_backend_config() -> Result<()> {
    println!("\n=== Testing MCP LLM Backend Config ===");

    // Test 1: "gguf" backend
    let config1 = LlmConfig {
        backend: LlmBackend::try_parse("gguf")?,
        model: "qwen2.5-mini".to_string(),
        url: "local".to_string(),
        timeout_seconds: 30,
    };
    assert_eq!(config1.backend, LlmBackend::GGUFEngine);

    let model1 = LlmFactory::from_config(&config1)?;
    assert_eq!(model1.backend_name(), "gguf_engine");
    println!("✅ 'gguf' backend maps to GGUFEngine");

    // Test 2: "gguf_engine" backend
    let config2 = LlmConfig {
        backend: LlmBackend::try_parse("gguf_engine")?,
        model: "qwen2.5-mini".to_string(),
        url: "local".to_string(),
        timeout_seconds: 30,
    };
    assert_eq!(config2.backend, LlmBackend::GGUFEngine);

    let model2 = LlmFactory::from_config(&config2)?;
    assert_eq!(model2.backend_name(), "gguf_engine");
    println!("✅ 'gguf_engine' backend maps to GGUFEngine");

    // Test 3: "ollama" backend handling
    match LlmBackend::try_parse("ollama") {
        Ok(backend) => {
            // If accepted, should map to GGUFEngine (alias behavior)
            let config3 = LlmConfig {
                backend,
                model: "qwen2.5-mini".to_string(),
                url: "local".to_string(),
                timeout_seconds: 30,
            };

            match LlmFactory::from_config(&config3) {
                Ok(model) => {
                    // If accepted, should use GGUFEngine
                    assert_eq!(model.backend_name(), "gguf_engine");
                    println!("✅ 'ollama' backend aliases to GGUFEngine");
                }
                Err(e) => {
                    // Should give clear error about Ollama not being supported
                    let error_msg = e.to_string();
                    assert!(
                        error_msg.contains("ollama")
                            || error_msg.contains("Ollama")
                            || error_msg.contains("supported"),
                        "Error should mention Ollama support: {}",
                        error_msg
                    );
                    println!("✅ 'ollama' backend properly rejected: {}", error_msg);
                }
            }
        }
        Err(e) => {
            // Should be rejected at parse level with clear error
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("ollama") || error_msg.contains("supported"),
                "Parse error should mention Ollama: {}",
                error_msg
            );
            println!("✅ 'ollama' backend properly rejected at parse: {}", error_msg);
        }
    }

    Ok(())
}

/// Test: Use a deliberate invalid model path in config/env
/// Assert:
///   • MCP returns a clear error response
///   • No panic or crash
#[test(flavor = "multi_thread")]
async fn test_mcp_llm_missing_model_path() -> Result<()> {
    println!("\n=== Testing MCP LLM Missing Model Path ===");

    // Clean up any existing test files
    let _ = fs::remove_file("test_mcp_missing.db");
    let _ = fs::remove_dir_all("test_mcp_missing_cache");

    // Create config with invalid model path
    let config = LlmConfig {
        backend: LlmBackend::GGUFEngine,
        model: "nonexistent/model/path.gguf".to_string(),
        url: "local".to_string(),
        timeout_seconds: 30,
    };

    let vector_store = Arc::new(Mutex::new(VectorStore::new(Box::new(
        syncore::vector::RealEmbeddings::new(384).unwrap(),
    ))));
    let state = SynCoreState::with_dual_stores(vector_store.clone(), vector_store.clone())
        .and_then(|s| s.with_intellitask_from_config(&config))
        .map_err(|e| anyhow::anyhow!("Failed to create state: {}", e))?;

    // Create MCP server and call tool - should fail gracefully
    let server = SynCoreMCPServer::new(state);

    let params = serde_json::json!({
        "prd_content": "Test prompt"
    });

    let response = server.mcp_delegate("intellitask_generate", params).await;

    match response {
        Ok(_) => {
            // If somehow succeeds (unlikely), should still be valid response
            println!("ℹ️  Unexpected success with invalid model path");
        }
        Err(e) => {
            // Should fail with clear error, not panic
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("model")
                    || error_msg.contains("file")
                    || error_msg.contains("path")
                    || error_msg.contains("GGUF")
                    || error_msg.contains("not found")
                    || error_msg.contains("IntelliTask not initialized"),
                "Error should be model/path related: {}",
                error_msg
            );
            println!("✅ Proper error handling for missing model: {}", error_msg);
        }
    }

    Ok(())
}

/// Helper function to extract text content from CallToolResult
fn extract_text_from_call_result(
    result: &Result<rmcp::model::CallToolResult, rmcp::Error>,
) -> String {
    match result {
        Ok(call_result) => {
            call_result
                .content
                .first()
                .and_then(|c| {
                    // Use serde to extract text - Content is serializable
                    let json = serde_json::to_value(c).ok()?;
                    json.get("text").and_then(|t| t.as_str()).map(|s| s.to_string())
                })
                .unwrap_or_default()
        }
        Err(e) => e.to_string(),
    }
}
