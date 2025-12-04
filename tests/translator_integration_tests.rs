//! Integration Tests for LLM Output Translator
//!
//! Tests translator integration with real MCP tool handlers and end-to-end workflows

use anyhow::Result;
use serde_json::json;
use std::sync::Arc;
use syncore::mcp_server::{create_tool_router, SynCoreState};
use syncore::mcp_tools::translator::{translate_llm_output, TargetSchema};
use syncore::llm::{LanguageModel, Prompt};
use syncore::memory::MemoryManager;
use syncore::tasks::TaskManager;
use syncore::vector::VectorStore;

/// Mock LLM for testing with controlled responses
struct MockLlm {
    response: String,
}

impl MockLlm {
    fn new(response: &str) -> Self {
        Self {
            response: response.to_string(),
        }
    }
}

impl LanguageModel for MockLlm {
    fn complete(&self, _prompt: &Prompt) -> Result<String> {
        Ok(self.response.clone())
    }

    fn complete_with_options(&self, _prompt: &Prompt, _options: &crate::llm::CompletionOptions) -> Result<String> {
        Ok(self.response.clone())
    }

    fn is_available(&self) -> bool {
        true
    }

    fn model_info(&self) -> Result<serde_json::Value> {
        Ok(json!({
            "name": "mock-llm",
            "available": true
        }))
    }
}

/// Integration test setup
struct TestSetup {
    state: Arc<SynCoreState>,
    mock_llm: Arc<MockLlm>,
}

impl TestSetup {
    fn new() -> Result<Self> {
        // Initialize components
        let memory = MemoryManager::new_memory(":memory:")?;
        let tasks = TaskManager::new(":memory:")?;
        let vector = VectorStore::new_memory()?;

        // Create mock LLM
        let mock_llm = Arc::new(MockLlm::new(""));

        // Create state
        let state = Arc::new(SynCoreState {
            memory: Arc::new(memory),
            tasks: Arc::new(tasks),
            vector: Arc::new(vector),
            llm: mock_llm.clone(),
            intellitask: None, // Will be tested separately
            router: Default::default(),
        });

        Ok(Self { state, mock_llm })
    }

    fn set_llm_response(&mut self, response: &str) {
        // Note: In real implementation, we'd need a way to update the mock response
        // For now, we'll create new mock instances as needed
    }
}

/// Test: End-to-end IntelliTask generate with translator
#[test]
fn test_intellitask_generate_with_translator() -> Result<()> {
    let mut setup = TestSetup::new()?;

    // Mock LLM response that needs translation
    let raw_llm_response = r#"
    Based on the PRD analysis, here's the task breakdown:

    {
      "prd_title": "User Authentication System",
      "parent_tasks": [
        {
          "id": "1.0",
          "title": "Database Schema Design",
          "description": "Design and implement user database schema",
          "subtasks": [],
          "dependencies": [],
          "complexity": "Moderate",
          "estimated_hours": 12.0
        },
        {
          "id": "2.0",
          "title": "Authentication API",
          "description": "Implement REST endpoints for login/register",
          "subtasks": [],
          "dependencies": ["1.0"],
          "complexity": "High",
          "estimated_hours": 16.0
        }
      ],
      "relevant_files": [
        {
          "path": "src/models/user.rs",
          "purpose": "Define User struct",
          "action": "Create"
        }
      ],
      "estimated_complexity": "Complex"
    }

    This breakdown covers the core requirements.
    "#;

    // Translate the LLM output
    let translated = translate_llm_output(raw_llm_response, TargetSchema::TaskBreakdown)?;

    // Verify translation success
    assert!(!translated.get("error").is_some(),
        "Translation failed: {:?}", translated.get("error"));

    // Verify specific fields
    assert_eq!(translated["prd_title"], "User Authentication System");
    assert_eq!(translated["estimated_complexity"], "Complex");

    let parent_tasks = translated["parent_tasks"].as_array().unwrap();
    assert_eq!(parent_tasks.len(), 2);

    // Verify complexity normalization ("High" -> "Complex")
    assert_eq!(parent_tasks[1]["complexity"], "Complex");

    // Verify FileAction alias normalization
    let relevant_files = translated["relevant_files"].as_array().unwrap();
    assert_eq!(relevant_files[0]["action"], "Create");

    Ok(())
}

/// Test: Priority validation with translator (CRITICAL from Phase 1 discovery)
#[test]
fn test_intellitask_prioritize_with_translator() -> Result<()> {
    // Raw LLM response that might have incorrect types
    let raw_llm_response = r#"
    After analyzing the task dependencies and business context:

    {
      "priorities": [
        {
          "task_id": "1.0",
          "priority": "Critical"
        },
        {
          "task_id": "2.0",
          "priority": "High"
        },
        {
          "task_id": "3.0",
          "priority": "Medium"
        }
      ]
    }

    The database schema must be completed first as it blocks all other tasks.
    "#;

    // Translate the LLM output
    let translated = translate_llm_output(raw_llm_response, TargetSchema::PriorityResult)?;

    // Verify translation success
    assert!(!translated.get("error").is_some(),
        "Priority translation failed: {:?}", translated.get("error"));

    let priorities = translated["priorities"].as_array().unwrap();
    assert_eq!(priorities.len(), 3);

    // CRITICAL: Verify priority remains as STRING, not enum
    for priority_item in priorities {
        assert!(priority_item["priority"].is_string(),
            "Priority must be string, got: {:?}", priority_item["priority"]);

        let priority_str = priority_item["priority"].as_str().unwrap();
        assert!(["Critical", "High", "Medium", "Low", "Optional"].contains(&priority_str),
            "Invalid priority string: {}", priority_str);
    }

    // Verify task_id is also string
    for priority_item in priorities {
        assert!(priority_item["task_id"].is_string(),
            "Task ID must be string, got: {:?}", priority_item["task_id"]);
    }

    Ok(())
}

/// Test: Subtask generation with real Subtask structure
#[test]
fn test_intellitask_subtasks_with_translator() -> Result<()> {
    let raw_llm_response = r#"
    Breaking down the database schema design task:

    {
      "subtasks": [
        {
          "id": "1.1",
          "description": "Design user table schema",
          "acceptance_criteria": [
            "User table includes id, email, password_hash fields",
            "Proper indexes defined",
            "Migration file created"
          ],
          "dependencies": [],
          "files_to_modify": [
            "migrations/001_create_users.sql",
            "src/models/user.rs"
          ],
          "complexity": "Simple",
          "estimated_hours": "4.0"
        },
        {
          "id": "1.2",
          "description": "Implement password hashing",
          "acceptance_criteria": [
            "Bcrypt integration",
            "Password validation rules",
            "Unit tests added"
          ],
          "dependencies": ["1.1"],
          "files_to_modify": [
            "src/auth/password.rs",
            "tests/test_password.rs"
          ],
          "complexity": "Moderate",
          "estimated_hours": 6
        }
      ]
    }
    "#;

    let translated = translate_llm_output(raw_llm_response, TargetSchema::SubtaskBreakdown)?;

    assert!(!translated.get("error").is_some(),
        "Subtask translation failed: {:?}", translated.get("error"));

    let subtasks = translated["subtasks"].as_array().unwrap();
    assert_eq!(subtasks.len(), 2);

    // Verify required fields
    for subtask in subtasks {
        // Required fields
        assert!(subtask.get("id").and_then(Value::as_str).is_some());
        assert!(subtask.get("description").and_then(Value::as_str).is_some());
        assert!(subtask.get("complexity").and_then(Value::as_str).is_some());
        assert!(subtask.get("estimated_hours").is_some());

        // Auto-fixed arrays should be present
        assert!(subtask.get("acceptance_criteria").and_then(Value::as_array).is_some());
        assert!(subtask.get("dependencies").and_then(Value::as_array).is_some());
        assert!(subtask.get("files_to_modify").and_then(Value::as_array).is_some());

        // Verify estimated_hours coercion (string -> f32)
        let hours = subtask["estimated_hours"].as_f64().unwrap();
        assert!(hours > 0.0);
    }

    // Verify first subtask details
    let first_subtask = &subtasks[0];
    assert_eq!(first_subtask["id"], "1.1");
    assert_eq!(first_subtask["complexity"], "Simple");
    assert_eq!(first_subtask["estimated_hours"], 4.0); // Should be f32, not string

    let acceptance_criteria = first_subtask["acceptance_criteria"].as_array().unwrap();
    assert_eq!(acceptance_criteria.len(), 3);

    Ok(())
}

/// Test: Sequential reasoning with translator
#[test]
fn test_sequential_reasoning_with_translator() -> Result<()> {
    let raw_llm_response = r#"
    {
      "step_number": 1,
      "thought": "I need to understand the current codebase structure before making changes",
      "reasoning": "Analyzing the existing code helps identify potential conflicts and ensures consistency with established patterns",
      "action": "Explore the src/ directory and read key files",
      "observation": "Found existing authentication module in src/auth/ with OAuth2 implementation"
    }
    "#;

    let translated = translate_llm_output(raw_llm_response, TargetSchema::SequentialStep)?;

    assert!(!translated.get("error").is_some(),
        "Sequential step translation failed: {:?}", translated.get("error"));

    // Verify auto-generated fields
    assert!(translated.get("step_id").and_then(Value::as_str).is_some());
    assert!(translated.get("timestamp").is_some());
    assert_eq!(translated["status"], "pending");

    // Verify required fields
    assert_eq!(translated["step_number"], 1);
    assert!(translated["thought"].as_str().unwrap().len() > 0);
    assert!(translated["reasoning"].as_str().unwrap().len() > 0);

    // Verify optional fields
    assert!(translated["action"].as_str().unwrap().len() > 0);
    assert!(translated["observation"].as_str().unwrap().len() > 0);

    Ok(())
}

/// Test: Error handling with malformed LLM output
#[test]
fn test_translator_error_handling() -> Result<()> {
    // Test completely malformed input
    let malformed_input = "This is not JSON at all!";

    let result = translate_llm_output(malformed_input, TargetSchema::TaskBreakdown)?;

    assert_eq!(result["error"], "SchemaValidationFailed");
    assert!(result.get("missing_fields").is_some());

    // Test JSON with missing critical fields
    let incomplete_json = r#"
    {
      "prd_title": "Test Feature"
      // Missing parent_tasks and estimated_complexity
    }
    "#;

    let result = translate_llm_output(incomplete_json, TargetSchema::TaskBreakdown)?;

    assert_eq!(result["error"], "SchemaValidationFailed");

    let missing_fields = result["missing_fields"].as_array().unwrap();
    assert!(missing_fields.iter().any(|f| f.as_str() == Some("parent_tasks")));

    Ok(())
}

/// Test: Complex nested structure with multiple translation needs
#[test]
fn test_complex_nested_translation() -> Result<()> {
    let complex_input = r#"
    {
      "prd_title": "E-commerce Platform Integration",
      "parent_tasks": [
        {
          "id": "1.0",
          "title": "Payment Gateway Integration",
          "description": "Integrate Stripe payment processing",
          "subtasks": [],
          "dependencies": [],
          "complexity": "VeryHigh", // Should be normalized
          "estimated_hours": "24.5" // Should be coerced to f32
        }
      ],
      "relevant_files": [
        {
          "path": "src/payments/stripe.rs",
          "purpose": "Implement Stripe client",
          "action": "Add" // Should be normalized to alias
        },
        {
          "path": "src/config.rs",
          "purpose": "Add payment configuration",
          "action": "Implement" // Another alias
        }
      ],
      "estimated_complexity": "hard" // Should be normalized
    }
    "#;

    let translated = translate_llm_output(complex_input, TargetSchema::TaskBreakdown)?;

    assert!(!translated.get("error").is_some(),
        "Complex translation failed: {:?}", translated.get("error"));

    // Verify nested normalizations
    let parent_tasks = translated["parent_tasks"].as_array().unwrap();
    assert_eq!(parent_tasks[0]["complexity"], "VeryComplex"); // "VeryHigh" -> "VeryComplex"
    assert_eq!(parent_tasks[0]["estimated_hours"], 24.5); // string -> f32

    assert_eq!(translated["estimated_complexity"], "Complex"); // "hard" -> "Complex"

    // Verify FileAction alias normalization
    let relevant_files = translated["relevant_files"].as_array().unwrap();
    assert_eq!(relevant_files[0]["action"], "Modify2"); // "Add" -> "Modify2"
    assert_eq!(relevant_files[1]["action"], "Modify2"); // "Implement" -> "Modify2"

    Ok(())
}

/// Test: Regression tests for Phase 1 discovered issues
#[test]
fn test_phase_1_regression_issues() -> Result<()> {
    // Issue 1: PriorityResult priority field must be STRING, not enum
    let priority_input = r#"
    {
      "priorities": [
        {
          "task_id": "123",
          "priority": "High"
        }
      ]
    }
    "#;

    let result = translate_llm_output(priority_input, TargetSchema::PriorityResult)?;
    assert!(result["priorities"][0]["priority"].is_string());

    // Issue 2: Complexity enum validation (not "High", "Low", "Medium")
    let complexity_input = r#"
    {
      "prd_title": "Test",
      "parent_tasks": [],
      "relevant_files": [],
      "estimated_complexity": "Medium"
    }
    "#;

    let result = translate_llm_output(complexity_input, TargetSchema::TaskBreakdown)?;
    assert_eq!(result["estimated_complexity"], "Moderate"); // "Medium" -> "Moderate" (valid)

    // Issue 3: FileReference structure differences
    let fileref_input = r#"
    {
      "prd_title": "Test",
      "parent_tasks": [],
      "relevant_files": [{
        "path": "test.rs",
        "purpose": "Test file",
        "action": "Update"
      }],
      "estimated_complexity": "Simple"
    }
    "#;

    let result = translate_llm_output(fileref_input, TargetSchema::TaskBreakdown)?;
    assert_eq!(result["relevant_files"][0]["action"], "Modify2"); // "Update" -> "Modify2"

    Ok(())
}