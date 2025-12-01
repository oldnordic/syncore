// End-to-end test for the self-operating cognition loop
// Tests: CREATE → THINK → ACT → REFLECT → MARK DONE

use anyhow::Result;
use std::sync::{Arc, Mutex};
use tempfile::NamedTempFile;

use syncore::cognitive_db;
use syncore::logger::CogLogger;
use syncore::memory::Memory;
use syncore::sequential::{LanguageModel, SequentialCore};
use syncore::tasks::Tasks;
use syncore::vector::VectorStore;

// Test GLM client for testing
struct TestGlmClient {
    base_url: String,
}

impl TestGlmClient {
    pub fn new() -> Self {
        Self {
            base_url: "test://client".to_string(),
        }
    }

    pub fn get_base_url(&self) -> &str {
        &self.base_url
    }
}

impl LanguageModel for TestGlmClient {
    fn think(&self, context: &str) -> Result<String> {
        Ok(format!("Thinking about task: {}", context))
    }

    fn decide(&self, thought: &str) -> Result<String> {
        Ok(format!("Decision based on thought: {} -> action: complete_task", thought))
    }

    fn reflect(&self, goal: &str) -> Result<String> {
        Ok(format!("Successfully completed goal: {}", goal))
    }
}

impl TestGlmClient {
    pub fn connect(_url: &str) -> Result<Self> {
        Ok(Self {
            base_url: "test://client".to_string(),
        })
    }
}

// Test logger that captures cognitive steps
struct TestLogger {
    steps: Mutex<Vec<String>>,
}

impl TestLogger {
    pub fn new() -> Self {
        Self {
            steps: Mutex::new(Vec::new()),
        }
    }

    pub fn get_steps(&self) -> Vec<String> {
        self.steps.lock().unwrap().clone()
    }
}

impl CogLogger for TestLogger {
    fn log_step(
        &self,
        step: &crate::cognitive_db::Step,
        task: &syncore::tasks::Task,
    ) -> std::io::Result<()> {
        let mut steps = self.steps.lock().unwrap();
        steps.push(format!("STEP: {} - {} (Task: {})", step.state, step.content, task.goal));
        Ok(())
    }

    fn log_summary(&self, task: &syncore::tasks::Task, reflection: &str) -> std::io::Result<()> {
        let mut steps = self.steps.lock().unwrap();
        steps.push(format!("SUMMARY: {} (Task ID: {})", reflection, task.id));
        Ok(())
    }
}

#[test]
fn test_e2e_cognition_loop() -> Result<()> {
    // Setup test environment
    let temp_file = NamedTempFile::new()?;
    let db_path = temp_file.path().to_str().unwrap();

    // Initialize components
    let memory = Arc::new(Memory::new(db_path)?);
    let tasks = Arc::new(Tasks::new(db_path)?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(Box::new(
        syncore::vector::RealEmbeddings::new(384).unwrap(),
    ))));
    let glm_client = Arc::new(Mutex::new(TestGlmClient::connect("test://client")?))
        as Arc<Mutex<dyn LanguageModel>>;
    let logger = Arc::new(TestLogger::new());
    let logger_ref = logger.clone();

    // Create sequential core
    let sequential_core = SequentialCore::new(
        tasks.clone(),
        vector_store.clone(),
        memory.clone(),
        glm_client.clone(),
        logger_ref as Arc<dyn CogLogger>,
    );

    // CREATE: Create a test task
    let task_id =
        tasks.add_task("Test autonomous operation", "Verify cognition loop works", 1, None)?;
    assert!(task_id > 0);

    // Verify task is created and open
    let task = tasks.get_task(task_id)?.unwrap();
    assert_eq!(task.status, "open");
    assert_eq!(task.goal, "Test autonomous operation");

    // THINK → ACT → REFLECT → MARK DONE: Run the cognition cycle
    sequential_core.run_cycle()?;

    // Verify task is now done
    let task = tasks.get_task(task_id)?.unwrap();
    assert_eq!(task.status, "done");

    // Verify cognitive steps were logged
    let steps = logger.get_steps();
    assert!(steps.len() >= 2); // At minimum THINK and REFLECT, but may include ACT steps

    // Verify cognitive steps in database
    let db = tasks.get_db();
    let db_guard = db.lock().unwrap();
    let cognitive_steps = cognitive_db::recent_steps(&db_guard, task_id, 10)?;
    assert!(cognitive_steps.len() >= 2); // At least Think and Decide (Act/Reflect may be filtered)

    // Verify the correct sequence of cognitive states
    let states: Vec<String> = cognitive_steps.iter().map(|step| step.state.clone()).collect();

    assert!(states.contains(&"Think".to_string()));
    assert!(states.contains(&"Decide".to_string()));
    assert!(states.contains(&"Act".to_string()));
    assert!(states.contains(&"Reflect".to_string()));

    println!("✅ E2E cognition loop test passed!");
    println!("   Task created: {}", task_id);
    println!("   Cognitive steps: {}", cognitive_steps.len());
    println!("   Final status: {}", task.status);

    Ok(())
}

#[test]
fn test_multiple_tasks_cognition() -> Result<()> {
    // Setup test environment
    let temp_file = NamedTempFile::new()?;
    let db_path = temp_file.path().to_str().unwrap();

    // Initialize components
    let memory = Arc::new(Memory::new(db_path)?);
    let tasks = Arc::new(Tasks::new(db_path)?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(Box::new(
        syncore::vector::RealEmbeddings::new(384).unwrap(),
    ))));
    let glm_client = Arc::new(Mutex::new(TestGlmClient::connect("test://client")?))
        as Arc<Mutex<dyn LanguageModel>>;
    let logger = Arc::new(TestLogger::new()) as Arc<dyn CogLogger>;

    // Create sequential core
    let sequential_core = SequentialCore::new(
        tasks.clone(),
        vector_store.clone(),
        memory.clone(),
        glm_client.clone(),
        logger.clone(),
    );

    // CREATE: Create multiple test tasks with different priorities
    let task1_id = tasks.add_task("High priority task", "Should be processed first", 1, None)?;
    let task2_id = tasks.add_task("Medium priority task", "Should be processed second", 2, None)?;
    let task3_id = tasks.add_task("Low priority task", "Should be processed third", 3, None)?;

    // Verify all tasks are created and open
    for (task_id, expected_goal) in [
        (task1_id, "High priority task"),
        (task2_id, "Medium priority task"),
        (task3_id, "Low priority task"),
    ] {
        let task = tasks.get_task(task_id)?.unwrap();
        assert_eq!(task.status, "open");
        assert_eq!(task.goal, expected_goal);
    }

    // Run cognition cycles until all tasks are done
    let mut cycles = 0;
    let max_cycles = 10; // Prevent infinite loops

    while cycles < max_cycles {
        sequential_core.run_cycle()?;
        cycles += 1;

        // Check if all tasks are done
        let all_done = [task1_id, task2_id, task3_id].iter().all(|&task_id| {
            if let Ok(Some(task)) = tasks.get_task(task_id) {
                task.status == "done"
            } else {
                false
            }
        });

        if all_done {
            break;
        }
    }

    // Verify all tasks are done
    for task_id in [task1_id, task2_id, task3_id] {
        let task = tasks.get_task(task_id)?.unwrap();
        assert_eq!(task.status, "done");
    }

    // Verify cognitive steps for each task
    for task_id in [task1_id, task2_id, task3_id] {
        let db = tasks.get_db();
        let db_guard = db.lock().unwrap();
        let cognitive_steps = cognitive_db::recent_steps(&db_guard, task_id, 10)?;
        assert!(cognitive_steps.len() >= 4); // Think, Decide, Act, Reflect for each task
    }

    println!("✅ Multiple tasks cognition test passed!");
    println!("   Tasks processed: 3");
    println!("   Cognition cycles: {}", cycles);

    Ok(())
}

#[test]
fn test_error_handling_in_cognition_loop() -> Result<()> {
    // Setup test environment
    let temp_file = NamedTempFile::new()?;
    let db_path = temp_file.path().to_str().unwrap();

    // Initialize components
    let memory = Arc::new(Memory::new(db_path)?);
    let tasks = Arc::new(Tasks::new(db_path)?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(Box::new(
        syncore::vector::RealEmbeddings::new(384).unwrap(),
    ))));

    // GLM client that fails during thinking
    struct FailingGlmClient {
        base_url: String,
    }

    impl FailingGlmClient {
        pub fn new() -> Self {
            Self {
                base_url: "failing://test".to_string(),
            }
        }

        pub fn get_base_url(&self) -> &str {
            &self.base_url
        }
    }

    impl LanguageModel for FailingGlmClient {
        fn think(&self, _context: &str) -> Result<String> {
            Err(anyhow::anyhow!("Simulated thinking failure"))
        }

        fn decide(&self, _thought: &str) -> Result<String> {
            Ok("Decision".to_string())
        }

        fn reflect(&self, _goal: &str) -> Result<String> {
            Ok("Reflection".to_string())
        }
    }

    impl FailingGlmClient {
        pub fn connect(_url: &str) -> Result<Self> {
            Ok(Self {
                base_url: "test://client".to_string(),
            })
        }
    }

    let glm_client = Arc::new(Mutex::new(FailingGlmClient::connect("test://client")?))
        as Arc<Mutex<dyn LanguageModel>>;
    let logger = Arc::new(TestLogger::new()) as Arc<dyn CogLogger>;

    // Create sequential core
    let sequential_core = SequentialCore::new(
        tasks.clone(),
        vector_store.clone(),
        memory.clone(),
        glm_client.clone(),
        logger.clone(),
    );

    // Create a test task
    let task_id =
        tasks.add_task("Test error handling", "Should handle failures gracefully", 1, None)?;

    // Run cognition cycle - it should fail gracefully
    let result = sequential_core.run_cycle();

    // The cycle should fail but not panic
    assert!(result.is_err());

    // Task should still exist and not be marked as done
    let task = tasks.get_task(task_id)?.unwrap();
    assert_eq!(task.status, "open"); // Should still be open since the cycle failed

    println!("✅ Error handling test passed!");
    println!("   Cycle failed gracefully: {:?}", result);

    Ok(())
}
