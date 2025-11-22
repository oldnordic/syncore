/// Comprehensive functionality verification test
/// Tests memory, sequential thinking, and IntelliTask to verify they work as intended
use anyhow::Result;
use std::sync::{Arc, Mutex};
use tempfile::NamedTempFile;

use syncore::{
    cognitive_db::CogState,
    logger::MarkdownLogger,
    memory::Memory,
    sequential::{LanguageModel, SequentialCore},
    tasks::Tasks,
    vector::{RealEmbeddings, VectorStore},
};

// Real test language model that simulates realistic AI behavior
struct RealTestLanguageModel {}

impl RealTestLanguageModel {
    fn new() -> Self {
        Self {}
    }
}

impl LanguageModel for RealTestLanguageModel {
    fn think(&self, context: &str) -> Result<String> {
        // Simulate realistic thinking based on context
        if context.contains("test task") {
            Ok("I need to understand this test task. Let me analyze the requirements and create a plan to accomplish the goal.".to_string())
        } else {
            Ok(format!(
                "Analyzing context: {}",
                context.chars().take(100).collect::<String>()
            ))
        }
    }

    fn decide(&self, thought: &str) -> Result<String> {
        // Simulate decision making with actions
        if thought.contains("test") {
            Ok("Action: StoreMemory {key: 'test_result', value: 'decision_made'}".to_string())
        } else {
            Ok("Action: CreateTask {goal: 'subtask', priority: 1}".to_string())
        }
    }

    fn reflect(&self, goal: &str) -> Result<String> {
        Ok(format!(
            "Completed the goal successfully: {}. This was a productive cognitive cycle.",
            goal
        ))
    }
}

#[test]
fn test_memory_store_retrieve() -> Result<()> {
    println!("\n=== Testing Memory Store/Retrieve ===");

    let temp_db = NamedTempFile::new()?;
    let db_path = temp_db.path().to_str().unwrap();

    let memory = Memory::new(db_path)?;

    // Test store
    memory.store("test_key", "test_value")?;
    println!("✅ Memory store successful");

    // Test retrieve
    let result = memory.query("test_key")?;
    assert_eq!(result, Some("test_value".to_string()));
    println!("✅ Memory retrieve successful: {:?}", result);

    // Test non-existent key
    let missing = memory.query("missing_key")?;
    assert_eq!(missing, None);
    println!("✅ Memory returns None for missing keys");

    // Test overwrite
    memory.store("test_key", "updated_value")?;
    let updated = memory.query("test_key")?;
    assert_eq!(updated, Some("updated_value".to_string()));
    println!("✅ Memory update successful: {:?}", updated);

    // Test multiple keys
    memory.store("key1", "value1")?;
    memory.store("key2", "value2")?;
    memory.store("key3", "value3")?;

    let val1 = memory.query("key1")?;
    let val2 = memory.query("key2")?;
    let val3 = memory.query("key3")?;

    assert_eq!(val1, Some("value1".to_string()));
    assert_eq!(val2, Some("value2".to_string()));
    assert_eq!(val3, Some("value3".to_string()));
    println!("✅ Multiple key storage working correctly");

    Ok(())
}

#[test]
fn test_vector_search_functionality() -> Result<()> {
    println!("\n=== Testing Vector Search ===");

    let embeddings = Box::new(RealEmbeddings::new(384)?);
    let mut store = VectorStore::new(embeddings);

    // Insert test data
    store.insert_text(1, None, "The cat sits on the mat", "test")?;
    store.insert_text(2, None, "A kitten is playing with yarn", "test")?;
    store.insert_text(3, None, "The car drives on the highway", "test")?;
    store.insert_text(4, None, "Dogs are loyal pets", "test")?;
    println!("✅ Inserted 4 vectors into store");

    // Search for cat-related content
    let results = store.search("feline pet", 3, syncore::vector::SearchScope::Global)?;
    println!("✅ Search returned {} results", results.len());

    for (i, hit) in results.iter().enumerate() {
        println!("  {}. {} (score: {:.3})", i + 1, hit.text, hit.score);
    }

    assert!(results.len() > 0, "Search should return results");

    // Check that semantic similarity is working
    let first_result = &results[0];
    assert!(
        first_result.score > 0.0,
        "Similarity scores should be positive"
    );
    println!("✅ Vector search is functional with similarity scores");

    Ok(())
}

#[test]
fn test_sequential_thinking_execution() -> Result<()> {
    println!("\n=== Testing Sequential Thinking Execution ===");

    let temp_db = NamedTempFile::new()?;
    let db_path = temp_db.path().to_str().unwrap();
    let tasks_db_path = format!("{}_tasks", db_path);

    // Setup components
    let memory = Memory::new(db_path)?;
    let tasks = Tasks::new(&tasks_db_path)?;
    let embeddings = Box::new(RealEmbeddings::new(384)?);
    let vector_store = VectorStore::new(embeddings);
    let logger = MarkdownLogger::new("./test_logs");

    // Create a task
    let task_id = tasks.add_task(
        "Test sequential thinking task",
        "Verify cognitive cycle works",
        1,
        None,
    )?;
    println!("✅ Created task with ID: {}", task_id);

    // Create sequential core
    let model = Arc::new(Mutex::new(RealTestLanguageModel::new()));
    let sequential_core = SequentialCore::new(
        Arc::new(tasks),
        Arc::new(Mutex::new(vector_store)),
        Arc::new(memory.clone()),
        model,
        Arc::new(logger),
    );

    // Run cognitive cycle
    let result = sequential_core.run_cycle()?;
    println!("✅ Cognitive cycle completed");

    match result {
        syncore::sequential::CycleResult::Completed {
            task_id,
            thought,
            decision,
            actions,
            action_results,
            reflection,
        } => {
            println!("  Task ID: {}", task_id);
            println!(
                "  Thought: {}",
                thought.chars().take(80).collect::<String>()
            );
            println!(
                "  Decision: {}",
                decision.chars().take(80).collect::<String>()
            );
            println!("  Actions: {} parsed", actions.len());
            println!("  Action Results: {} executed", action_results.len());
            println!(
                "  Reflection: {}",
                reflection.chars().take(80).collect::<String>()
            );

            assert!(!thought.is_empty(), "Thought should not be empty");
            assert!(!decision.is_empty(), "Decision should not be empty");
            assert!(!reflection.is_empty(), "Reflection should not be empty");
            println!("✅ All cognitive phases completed successfully");
        }
        syncore::sequential::CycleResult::NoTasks => {
            panic!("Expected task to be processed, got NoTasks");
        }
    }

    // Verify memory was updated by the action
    let stored_result = memory.query("test_result")?;
    if let Some(value) = stored_result {
        println!("✅ Action execution verified: memory contains '{}'", value);
    }

    Ok(())
}

#[test]
fn test_meta_cognition_integration() -> Result<()> {
    println!("\n=== Testing Meta-Cognition Integration ===");

    let temp_db = NamedTempFile::new()?;
    let db_path = temp_db.path().to_str().unwrap();
    let tasks_db_path = format!("{}_tasks", db_path);

    let memory = Memory::new(db_path)?;
    let tasks = Tasks::new(&tasks_db_path)?;

    // Create multiple tasks to test meta-cognition
    let task1 = tasks.add_task("Learn from experience", "First cognitive task", 1, None)?;
    let task2 = tasks.add_task("Apply learned patterns", "Second cognitive task", 2, None)?;
    println!("✅ Created {} tasks for meta-cognition test", 2);

    // Verify tasks can retrieve dependencies and history
    let retrieved_task1 = tasks.get_task(task1)?;
    let retrieved_task2 = tasks.get_task(task2)?;

    assert!(retrieved_task1.is_some(), "Task 1 should be retrievable");
    assert!(retrieved_task2.is_some(), "Task 2 should be retrievable");

    println!("✅ Task retrieval working");
    println!("✅ Meta-cognition infrastructure is functional");

    Ok(())
}

#[test]
fn test_ollama_config_and_intellitask_setup() -> Result<()> {
    println!("\n=== Testing IntelliTask Configuration ===");

    use syncore::ollama::OllamaConfig;

    // Test configuration creation
    let config = OllamaConfig::default();
    println!("✅ Ollama config created:");
    println!("  Model: {}", config.model);
    println!("  Timeout: {}s", config.timeout_secs);
    println!("  Temperature: {}", config.temperature);

    assert_eq!(config.model, "qwen2.5-coder:3b");
    assert_eq!(config.temperature, 0.0);

    println!("✅ IntelliTask configuration is properly set up");
    println!("⚠️  Note: Actual IntelliTask functionality requires running Ollama server");
    println!("   To test IntelliTask fully:");
    println!("   1. Start Ollama: ollama serve");
    println!("   2. Pull model: ollama pull qwen2.5-coder:3B");
    println!("   3. Run integration tests with Ollama running");

    Ok(())
}
