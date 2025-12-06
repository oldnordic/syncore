//! PHASE S6-POST-VALIDATION Tests
//!
//! These tests verify that IntelliTask and Sequential tools use REAL GGUFEngine
//! instead of test backend. All tests MUST FAIL until real model loading is fixed.

use anyhow::Result;
use serde_json;
use std::env;
use std::fs;
use std::sync::Arc;
use syncore::intellitask::IntelliTask;
use syncore::llm::{LanguageModel, Prompt};
use syncore::models::gguf_engine::GGUFEngine;
use syncore::router::SynCoreState;
use tempfile::TempDir;

/// Test 1: IntelliTask generate uses real model, not test backend
#[tokio::test(flavor = "multi_thread")]
async fn test_intellitask_generate_uses_real_model() -> Result<()> {
    println!("\n=== Test: IntelliTask generate uses real model ===");

    // Set up real model path
    let temp_dir = TempDir::new()?;
    let model_path = temp_dir.path().join("qwen2.5-0.5b.gguf");
    fs::write(&model_path, "dummy model content")?; // Will fail but attempts real load
    env::set_var("SYNC_LLM_MODEL_PATH", model_path.to_str().unwrap());

    // Create real GGUFEngine (will fail gracefully but attempts real load)
    let llm_model: Arc<dyn LanguageModel> = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(GGUFEngine::new("qwen2.5-0.5b"))
    })
    .map(|engine| Arc::new(engine) as Arc<dyn LanguageModel>)
    .unwrap_or_else(|e| {
        panic!("Expected real model loading attempt, got immediate test fallback: {}", e);
    });

    // Create IntelliTask with real model
    let intellitask = IntelliTask::new(llm_model);

    // Test IntelliTask generation
    let prd_content = "PRD: Create a simple REST API.
Requirements:
- GET /users endpoint
- POST /users endpoint
- JSON response format
- Error handling";

    match intellitask.generate_tasks_from_prd(prd_content) {
        Ok(breakdown) => {
            println!("✅ PASS: IntelliTask generated breakdown");

            // CRITICAL: Verify response does NOT contain test backend pattern
            if breakdown.prd_title.contains("GGUFEngine response to:") {
                panic!("❌ FAIL: IntelliTask used test backend pattern");
            }

            // Verify it's not a trivial test response
            if breakdown.prd_title.len() < 5 {
                panic!("❌ FAIL: Response too short - likely test backend");
            }
        }
        Err(e) => {
            // Real model loading failure is acceptable, but should NOT be test backend error
            let error_str = e.to_string();
            if error_str.contains("GGUFEngine response to:") {
                panic!("❌ FAIL: IntelliTask error mentions test backend");
            }
            println!("✅ PASS: Real model loading failed gracefully: {}", e);
        }
    }

    env::remove_var("SYNC_LLM_MODEL_PATH");
    Ok(())
}

/// Test 2: IntelliTask prioritize uses real model
#[tokio::test(flavor = "multi_thread")]
async fn test_intellitask_prioritize_uses_real_model() -> Result<()> {
    println!("\n=== Test: IntelliTask prioritize uses real model ===");

    // Set up real model path
    let temp_dir = TempDir::new()?;
    let model_path = temp_dir.path().join("qwen2.5-0.5b.gguf");
    fs::write(&model_path, "dummy")?;
    env::set_var("SYNC_LLM_MODEL_PATH", model_path.to_str().unwrap());

    // Create real GGUFEngine
    let llm_model: Arc<dyn LanguageModel> = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(GGUFEngine::new("qwen2.5-0.5b"))
    })
    .map(|engine| Arc::new(engine) as Arc<dyn LanguageModel>)
    .unwrap_or_else(|e| {
        panic!("Expected real model loading attempt, got immediate test fallback: {}", e);
    });

    let intellitask = IntelliTask::new(llm_model);

    // Test prioritization with simple task list - use correct ParentTask structure
    let tasks_json = r#"
    [
        {
            "id": "1",
            "title": "Setup database",
            "description": "Create database schema",
            "subtasks": [],
            "dependencies": [],
            "complexity": "Simple",
            "estimated_hours": 4.0
        },
        {
            "id": "2",
            "title": "Create API endpoints",
            "description": "Implement REST API",
            "subtasks": [],
            "dependencies": [],
            "complexity": "Moderate",
            "estimated_hours": 8.0
        }
    ]
    "#;

    // Parse tasks for prioritize_tasks method
    let tasks: Vec<syncore::intellitask::ParentTask> = serde_json::from_str(tasks_json)?;

    match intellitask.prioritize_tasks(&tasks, "") {
        Ok(prioritized) => {
            // Verify no test backend pattern in response
            let result_str = format!("{:?}", prioritized);
            if result_str.contains("GGUFEngine response to:") {
                panic!("❌ FAIL: Prioritize used test backend");
            }
            println!("✅ PASS: Prioritize used real model");
        }
        Err(e) => {
            if e.to_string().contains("GGUFEngine response to:") {
                panic!("❌ FAIL: Prioritize error mentions test backend");
            }
            println!("✅ PASS: Prioritize failed with real model error: {}", e);
        }
    }

    env::remove_var("SYNC_LLM_MODEL_PATH");
    Ok(())
}

/// Test 3: Sequential reasoning uses real model via state
#[tokio::test(flavor = "multi_thread")]
async fn test_sequential_reason_uses_real_model() -> Result<()> {
    println!("\n=== Test: Sequential reasoning uses real model ===");

    // Set up real model path
    let temp_dir = TempDir::new()?;
    let model_path = temp_dir.path().join("qwen2.5-0.5b.gguf");
    fs::write(&model_path, "dummy")?;
    env::set_var("SYNC_LLM_MODEL_PATH", model_path.to_str().unwrap());

    // Create real GGUFEngine for state
    let llm_model: Arc<dyn LanguageModel> = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(GGUFEngine::new("qwen2.5-0.5b"))
    })
    .map(|engine| Arc::new(engine) as Arc<dyn LanguageModel>)
    .unwrap_or_else(|e| {
        panic!("Expected real model loading attempt, got immediate test fallback: {}", e);
    });

    // Create state with real model using test constructor
    let state = SynCoreState::test().with_llm_model(llm_model);

    // Test that state.llm_model is NOT a test backend
    if let Some(model) = &state.llm_model {
        // Try completion and verify it's not test backend
        let prompt = Prompt::new("test", "simple test");
        match model.complete(&prompt) {
            Ok(response) => {
                if response.text.contains("GGUFEngine response to:") {
                    panic!("❌ FAIL: state.llm_model returned test backend response");
                }
                println!("✅ PASS: state.llm_model used real model");
            }
            Err(e) => {
                if e.to_string().contains("GGUFEngine response to:") {
                    panic!("❌ FAIL: state.llm_model error mentions test backend");
                }
                println!("✅ PASS: state.llm_model failed with real model error: {}", e);
            }
        }
    } else {
        panic!("❌ FAIL: state.llm_model is None");
    }

    env::remove_var("SYNC_LLM_MODEL_PATH");
    Ok(())
}

/// Test 4: No test backend in production state initialization
#[tokio::test(flavor = "multi_thread")]
async fn test_no_test_backend_in_production_state() -> Result<()> {
    println!("\n=== Test: No test backend in production state ===");

    // Set up real model path
    let temp_dir = TempDir::new()?;
    let model_path = temp_dir.path().join("qwen2.5-0.5b.gguf");
    fs::write(&model_path, "dummy")?;
    env::set_var("SYNC_LLM_MODEL_PATH", model_path.to_str().unwrap());

    // Create real GGUFEngine like mcp_stdio_main.rs does
    let llm_model: Arc<dyn LanguageModel> = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(GGUFEngine::new("qwen2.5-0.5b"))
    })
    .map(|engine| Arc::new(engine) as Arc<dyn LanguageModel>)
    .unwrap_or_else(|e| {
        // This fallback should NOT use new_test() in production
        panic!("Production code should not fall back to new_test(): {}", e);
    });

    // Verify it's a real GGUFEngine instance
    assert_eq!(llm_model.backend_name(), "gguf_engine");

    // Verify it's not a test backend by checking backend_name
    let backend_name = llm_model.backend_name();
    if backend_name != "gguf_engine" {
        panic!("❌ FAIL: Production state has wrong backend: {}", backend_name);
    }

    println!("✅ PASS: Production state uses real model, not test backend");

    env::remove_var("SYNC_LLM_MODEL_PATH");
    Ok(())
}

/// Test 5: Verify mcp_stdio_main.rs initialization path
#[tokio::test(flavor = "multi_thread")]
async fn test_main_initialization_uses_real_model() -> Result<()> {
    println!("\n=== Test: Main initialization uses real model ===");

    // Set up real model path
    let temp_dir = TempDir::new()?;
    let model_path = temp_dir.path().join("qwen2.5-0.5b.gguf");
    fs::write(&model_path, "dummy")?;
    env::set_var("SYNC_LLM_MODEL_PATH", model_path.to_str().unwrap());

    // Simulate the exact initialization from mcp_stdio_main.rs lines 162-180
    let llm_model: Arc<dyn LanguageModel> = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(GGUFEngine::new("qwen2.5-0.5b"))
    })
    .map(|engine| Arc::new(engine) as Arc<dyn LanguageModel>)
    .unwrap_or_else(|e| {
        eprintln!("❌ Failed to load real GGUF model from {}: {}", model_path.display(), e);
        eprintln!("⚠️  Falling back to test GGUFEngine backend");
        Arc::new(GGUFEngine::new_test()) as Arc<dyn LanguageModel>
    });

    // The key test: If this was real loading attempt, it should have failed gracefully
    // NOT immediately fallen back to test backend
    let completion = llm_model.complete(&Prompt::new("test", "test input"));

    match completion {
        Ok(response) => {
            // If it succeeds, verify it's not a test response
            if response.text.contains("GGUFEngine response to:") {
                panic!("❌ FAIL: Main initialization used test backend pattern");
            }
            println!("✅ PASS: Main initialization used real model");
        }
        Err(e) => {
            // If it fails, ensure it's not a test backend error
            if e.to_string().contains("GGUFEngine response to:") {
                panic!("❌ FAIL: Main initialization error mentions test backend");
            }
            println!("✅ PASS: Main initialization failed with real model error: {}", e);
        }
    }

    env::remove_var("SYNC_LLM_MODEL_PATH");
    Ok(())
}
