use std::sync::Arc;
use syncore::intellitask::IntelliTask;
use syncore::llm::factory::{LlmBackend, LlmConfig, LlmFactory};
use syncore::llm::{LanguageModel, Prompt};
use syncore::models::gguf_engine::GGUFEngine;
use syncore::router::SynCoreState;

/// Test that LlmFactory::from_config() does NOT return test backend by default
#[tokio::test]
async fn test_factory_does_not_return_test_backend_by_default() {
    let config = LlmConfig {
        backend: LlmBackend::GGUFEngine,
        model: "qwen2.5-0.5b".to_string(),
        url: "".to_string(),
        timeout_seconds: 30,
    };

    let result = LlmFactory::from_config(&config).await;

    // This should succeed
    assert!(result.is_ok(), "LlmFactory::from_config should succeed");

    let model = result.unwrap();

    // The model should NOT be a test backend
    let response = model.complete(&Prompt::new("test", "test input"));
    assert!(response.is_ok(), "Model completion should succeed");

    let response_text = response.unwrap().text;
    assert!(
        !response_text.starts_with("GGUFEngine response to:"),
        "Factory should not return test backend response. Got: {}",
        response_text
    );
}

/// Test that main initialization code uses the factory model, not test model
#[tokio::test]
async fn test_main_initialization_uses_factory_model() {
    // This simulates the initialization pattern from mcp_stdio_main.rs
    let config = LlmConfig {
        backend: LlmBackend::GGUFEngine,
        model: "qwen2.5-0.5b".to_string(),
        url: "".to_string(),
        timeout_seconds: 30,
    };

    // Simulate getting model from factory (as main should do)
    let factory_model: Arc<dyn LanguageModel> = {
        let model_box = LlmFactory::from_config(&config).await.unwrap();
        Arc::from(model_box)
    };

    // Create state as main does
    let state = SynCoreState::test().with_llm_model(factory_model.clone());

    // Verify state contains the factory model, not test model
    assert!(state.llm_model.is_some(), "State should have LLM model");

    let stored_model = state.llm_model.as_ref().unwrap();
    let response = stored_model.complete(&Prompt::new("test", "validation"));

    assert!(response.is_ok(), "Stored model should work");
    let response_text = response.unwrap().text;
    assert!(
        !response_text.starts_with("GGUFEngine response to:"),
        "State should contain real model, not test backend. Got: {}",
        response_text
    );
}

/// Test that no code path in main overwrites a valid model with new_test()
#[tokio::test]
async fn test_no_overwrite_of_real_model_in_main() {
    // Create a real model first
    let config = LlmConfig {
        backend: LlmBackend::GGUFEngine,
        model: "qwen2.5-0.5b".to_string(),
        url: "".to_string(),
        timeout_seconds: 30,
    };

    let real_model: Arc<dyn LanguageModel> = {
        let model_box = LlmFactory::from_config(&config).await.unwrap();
        Arc::from(model_box)
    };

    // Verify it's a real model
    let response = real_model.complete(&Prompt::new("test", "check"));
    assert!(response.is_ok());
    assert!(!response.unwrap().text.starts_with("GGUFEngine response to:"));

    // This test ensures we don't have the pattern:
    // let model_arc: Arc<dyn LanguageModel> = Arc::new(GGUFEngine::new_test());
    // which would overwrite the real model

    // The real fix should be: let model_arc: Arc<dyn LanguageModel> = real_model.into();
    let model_arc: Arc<dyn LanguageModel> = real_model.into();

    let final_response = model_arc.complete(&Prompt::new("test", "final"));
    assert!(final_response.is_ok());
    assert!(
        !final_response.unwrap().text.starts_with("GGUFEngine response to:"),
        "Model should still be real after conversion"
    );
}

/// Test that IntelliTask uses real LLM, not test backend
#[tokio::test]
async fn test_intellitask_uses_real_llm() {
    // Create real LLM model
    let config = LlmConfig {
        backend: LlmBackend::GGUFEngine,
        model: "qwen2.5-0.5b".to_string(),
        url: "".to_string(),
        timeout_seconds: 30,
    };

    let llm_model: Arc<dyn LanguageModel> = {
        let model_box = LlmFactory::from_config(&config).await.unwrap();
        Arc::from(model_box)
    };

    // Create IntelliTask with real model
    let intellitask = IntelliTask::new(llm_model.clone());

    // Test that IntelliTask doesn't use test backend
    let simple_prd = "Create a simple login feature";
    let result = intellitask.generate_tasks_from_prd(simple_prd);

    // Check if the response contains test backend indicator
    match result {
        Ok(task_breakdown) => {
            // Convert to JSON to check for test backend response
            let tasks_json = format!("{:?}", task_breakdown);
            assert!(
                !tasks_json.contains("GGUFEngine response to:"),
                "IntelliTask should not return test backend response. Got: {}",
                tasks_json
            );
        }
        Err(e) => {
            // Error is acceptable, but it should not be about test backend
            assert!(
                !e.to_string().contains("GGUFEngine response to:"),
                "Error should not mention test backend: {}",
                e
            );
        }
    }
}

/// Test that state.llm_model is Arc of real model, not test backend
#[tokio::test]
async fn test_state_llm_model_is_arc_of_real_model() {
    let config = LlmConfig {
        backend: LlmBackend::GGUFEngine,
        model: "qwen2.5-0.5b".to_string(),
        url: "".to_string(),
        timeout_seconds: 30,
    };

    // Get real model from factory
    let factory_model: Arc<dyn LanguageModel> = {
        let model_box = LlmFactory::from_config(&config).await.unwrap();
        Arc::from(model_box)
    };

    // Verify it's not a test backend
    let test_response = factory_model.complete(&Prompt::new("test", "identity"));
    assert!(test_response.is_ok());
    assert!(!test_response.unwrap().text.starts_with("GGUFEngine response to:"));

    // Create state with real model
    let state = SynCoreState::test().with_llm_model(factory_model);

    // Verify state.llm_model is the real model
    assert!(state.llm_model.is_some(), "State should have llm_model");

    let stored_model = state.llm_model.as_ref().unwrap();

    // Verify backend type
    assert_eq!(
        stored_model.backend_name(),
        "gguf_engine",
        "Should be gguf_engine backend, not test"
    );

    // Verify it's not a test response
    let final_check = stored_model.complete(&Prompt::new("final", "check"));
    assert!(final_check.is_ok());
    assert!(
        !final_check.unwrap().text.starts_with("GGUFEngine response to:"),
        "Stored model should be real, not test backend"
    );
}
