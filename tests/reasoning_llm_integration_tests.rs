//! Reasoning LLM Integration Tests - ST-4
//!
//! Test suite for LLM integration into Tree-of-Thought reasoning.
//! Uses mock LLM backends, no real inference.

use anyhow::Result;
use std::sync::Arc;
use syncore::databases::cognition_graph::ThoughtNodeProperties;
use syncore::llm::{LanguageModel, Prompt};
use syncore::models::gguf_engine::{GGUFEngine, ThoughtNodeDraft};
use syncore::reasoning::{ReasoningNodeContext, ToTEngine};

// Mock Neo4j client for testing
struct MockNeo4jClient;

impl MockNeo4jClient {
    fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

// Mock session manager for testing
struct MockSessionManager {
    nodes: Arc<std::sync::Mutex<Vec<ThoughtNodeProperties>>>,
}

impl MockSessionManager {
    fn new() -> Self {
        Self {
            nodes: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    fn add_nodes(&self, nodes: Vec<ThoughtNodeProperties>) {
        if let Ok(mut stored_nodes) = self.nodes.lock() {
            stored_nodes.extend(nodes);
        }
    }

    fn get_nodes(&self) -> Vec<ThoughtNodeProperties> {
        self.nodes.lock().unwrap().clone()
    }
}

fn create_test_node_context() -> ReasoningNodeContext {
    let node = ThoughtNodeProperties {
        id: "test_node".to_string(),
        session_id: "test_session".to_string(),
        parent_id: Some("parent_node".to_string()),
        step_index: 2,
        content: "Current approach: Use recursive algorithm".to_string(),
        score: Some(0.8),
    };

    let parent = ThoughtNodeProperties {
        id: "parent_node".to_string(),
        session_id: "test_session".to_string(),
        parent_id: None,
        step_index: 1,
        content: "Problem: Need to process tree structure".to_string(),
        score: Some(0.9),
    };

    ReasoningNodeContext::from_properties(
        node,
        "test_session".to_string(),
        vec![parent],
        Vec::new(),
    )
}

fn create_mock_llm() -> Box<dyn LanguageModel> {
    Box::new(GGUFEngine::new_test())
}

#[tokio::test]
async fn test_llm_expand_creates_children() -> Result<()> {
    let client = MockNeo4jClient::new();
    let llm = create_mock_llm();
    let engine = ToTEngine::with_language_model(client, llm);

    let context = create_test_node_context();

    // Test LLM expansion
    let drafts = engine.llm_expand(&context, &*engine.language_model.as_ref().unwrap()).await?;

    // Should create 3 child thoughts from mock response
    assert_eq!(drafts.len(), 3);

    // Verify draft structure
    for draft in &drafts {
        assert!(!draft.text.is_empty());
        assert!(draft.confidence >= 0.0 && draft.confidence <= 1.0);
    }

    // Verify deterministic mock content
    assert!(drafts[0].text.contains("Continue with current approach"));
    assert_eq!(drafts[0].confidence, 0.92);

    Ok(())
}

#[tokio::test]
async fn test_llm_expand_propagates_errors() -> Result<()> {
    let client = MockNeo4jClient::new();

    // Create a faulty LLM that always returns errors
    struct FaultyLLM;
    impl LanguageModel for FaultyLLM {
        fn complete(&self, _prompt: &Prompt) -> Result<syncore::llm::Completion> {
            Err(anyhow::anyhow!("LLM error"))
        }

        fn backend_name(&self) -> &str {
            "faulty"
        }
    }

    let llm: Box<dyn LanguageModel> = Box::new(FaultyLLM);
    let engine = ToTEngine::with_language_model(client, llm);

    let context = create_test_node_context();

    // Should propagate LLM errors
    let result = engine.llm_expand(&context, &*engine.language_model.as_ref().unwrap()).await;
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("LLM error"));

    Ok(())
}

#[tokio::test]
async fn test_llm_prompt_is_correct() -> Result<()> {
    let context = create_test_node_context();

    // Test prompt building
    let prompt = syncore::reasoning::llm_adapter::PromptBuilder::create_prompt(&context);

    // Verify prompt contains required elements
    assert!(prompt.system.contains("reasoning assistant"));
    assert!(prompt.user.contains("Session ID: test_session"));
    assert!(prompt.user.contains("Current Thought: Current approach: Use recursive algorithm"));
    assert!(prompt.user.contains("Reasoning History:"));
    assert!(prompt.user.contains("Problem: Need to process tree structure"));
    assert!(prompt.user.contains("JSON format"));
    assert!(prompt.user.contains("\"thoughts\""));

    // Verify prompt parameters
    assert_eq!(prompt.temperature, Some(0.7));
    assert_eq!(prompt.max_tokens, Some(1024));

    Ok(())
}

#[tokio::test]
async fn test_llm_multiple_sessions_isolated() -> Result<()> {
    let client = MockNeo4jClient::new();
    let llm = create_mock_llm();
    let engine = ToTEngine::with_language_model(client, llm);

    // Create contexts for different sessions
    let context1 = ReasoningNodeContext::from_properties(
        ThoughtNodeProperties {
            id: "node1".to_string(),
            session_id: "session1".to_string(),
            parent_id: None,
            step_index: 0,
            content: "Session 1 problem".to_string(),
            score: None,
        },
        "session1".to_string(),
        Vec::new(),
        Vec::new(),
    );

    let context2 = ReasoningNodeContext::from_properties(
        ThoughtNodeProperties {
            id: "node2".to_string(),
            session_id: "session2".to_string(),
            parent_id: None,
            step_index: 0,
            content: "Session 2 problem".to_string(),
            score: None,
        },
        "session2".to_string(),
        Vec::new(),
        Vec::new(),
    );

    // Expand both contexts
    let drafts1 = engine.llm_expand(&context1, &*engine.language_model.as_ref().unwrap()).await?;
    let drafts2 = engine.llm_expand(&context2, &*engine.language_model.as_ref().unwrap()).await?;

    // Both should produce valid results
    assert_eq!(drafts1.len(), 3);
    assert_eq!(drafts2.len(), 3);

    // Content should be different based on context
    assert!(drafts1[0].text.contains("Session 1 problem"));
    assert!(drafts2[0].text.contains("Session 2 problem"));

    Ok(())
}

#[tokio::test]
async fn test_llm_metrics_tracked() -> Result<()> {
    let llm = GGUFEngine::new_test();

    // Initial metrics
    let initial_metrics = llm.metrics();
    assert_eq!(initial_metrics.total_calls, 0);
    assert_eq!(initial_metrics.successful_calls, 0);

    // Make some calls
    let prompt = Prompt::new("", "tot_expand");
    for _ in 0..3 {
        let _ = llm.complete(&prompt);
    }

    // Check updated metrics
    let updated_metrics = llm.metrics();
    assert_eq!(updated_metrics.total_calls, 3);
    assert_eq!(updated_metrics.successful_calls, 3);
    assert_eq!(updated_metrics.failed_calls, 0);
    assert!(updated_metrics.average_response_time_ms > 0.0);

    Ok(())
}

#[tokio::test]
async fn test_llm_health_called() -> Result<()> {
    let llm = GGUFEngine::new_test();

    // Test health check
    let is_healthy = llm.health_check()?;
    assert!(is_healthy);

    // Check health details
    let health = llm.health();
    assert_eq!(health.backend_name, "gguf_engine_test");
    assert_eq!(health.status, "healthy");
    assert!(health.initialized);
    assert_eq!(health.call_count, 0); // No calls yet
    assert!(health.last_error.is_none());

    // Make a call and check health again
    let prompt = Prompt::new("", "test");
    let _ = llm.complete(&prompt);

    let health_after_call = llm.health();
    assert_eq!(health_after_call.call_count, 1);
    assert_eq!(health_after_call.status, "healthy");

    Ok(())
}

#[tokio::test]
async fn test_llm_output_parsing() -> Result<()> {
    let llm = GGUFEngine::new_test();

    // Test parsing valid ToT response
    let valid_response = r#"{"thoughts": [
        {"text": "Continue with recursion", "confidence": 0.9},
        {"text": "Try iterative approach", "confidence": 0.7},
        {"text": "Consider hybrid solution", "confidence": 0.5}
    ]}"#;

    let drafts = llm.parse_tot_response(valid_response)?;
    assert_eq!(drafts.len(), 3);
    assert_eq!(drafts[0].text, "Continue with recursion");
    assert_eq!(drafts[0].confidence, 0.9);
    assert_eq!(drafts[1].text, "Try iterative approach");
    assert_eq!(drafts[1].confidence, 0.7);
    assert_eq!(drafts[2].text, "Consider hybrid solution");
    assert_eq!(drafts[2].confidence, 0.5);

    // Test parsing invalid response
    let invalid_response = r#"{"not_thoughts": []}"#;
    assert!(llm.parse_tot_response(invalid_response).is_err());

    Ok(())
}

#[tokio::test]
async fn test_llm_fallback_to_stub() -> Result<()> {
    let client = MockNeo4jClient::new();

    // Create engine without LLM (should fallback to stub)
    let engine = ToTEngine::new(client);
    assert!(engine.language_model.is_none());

    let context = create_test_node_context();

    // Should use stub expansion
    let drafts = engine.llm_expand(&context, &*engine.language_model.as_ref().unwrap()).await;

    // Since no LLM, this should fail gracefully
    assert!(drafts.is_err());

    Ok(())
}

#[test]
fn test_thought_node_draft_validation() {
    // Valid draft
    let valid_draft = ThoughtNodeDraft {
        text: "Valid thought content".to_string(),
        confidence: 0.8,
    };
    assert!(syncore::reasoning::llm_adapter::LLMOutputParser::validate_draft(&valid_draft).is_ok());

    // Invalid drafts
    let empty_draft = ThoughtNodeDraft {
        text: "".to_string(),
        confidence: 0.8,
    };
    assert!(syncore::reasoning::llm_adapter::LLMOutputParser::validate_draft(&empty_draft).is_err());

    let invalid_confidence = ThoughtNodeDraft {
        text: "Valid text".to_string(),
        confidence: 1.5,
    };
    assert!(syncore::reasoning::llm_adapter::LLMOutputParser::validate_draft(&invalid_confidence)
        .is_err());

    let too_long = ThoughtNodeDraft {
        text: "a".repeat(1001),
        confidence: 0.8,
    };
    assert!(syncore::reasoning::llm_adapter::LLMOutputParser::validate_draft(&too_long).is_err());
}
