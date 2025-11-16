use anyhow::Result;
use std::sync::{Arc, Mutex};
use tempfile::NamedTempFile;

use syncore::{
    sequential::{SequentialCore, LanguageModel},
    tasks::{Tasks, Task},
    vector::{VectorStore, RealEmbeddings, Embeddings},
    memory::Memory,
    logger::CogLogger,
    cognitive_db,
};

// Test implementation of LanguageModel that provides realistic responses
struct TestLanguageModel {
    responses: Vec<String>,
    call_count: Arc<Mutex<usize>>,
}

impl TestLanguageModel {
    fn new() -> Self {
        Self {
            responses: vec![
                "I need to analyze the requirements and create a structured approach".to_string(),
                "Based on analysis, I will implement solution step by step".to_string(),
                "Action: Create implementation plan and execute code changes".to_string(),
                "The implementation was successful and meets all requirements".to_string(),
            ],
            call_count: Arc::new(Mutex::new(0)),
        }
    }
}

impl LanguageModel for TestLanguageModel {
    fn think(&self, context: &str) -> Result<String> {
        let mut count = self.call_count.lock().unwrap();
        let response = if *count < self.responses.len() {
            self.responses[*count].clone()
        } else {
            let current_count = *count;
            format!("Thinking step {} about: {}", current_count + 1, context)
        };
        *count += 1;
        Ok(response)
    }

    fn decide(&self, thought: &str) -> Result<String> {
        let mut count = self.call_count.lock().unwrap();
        let response = if *count < self.responses.len() {
            if *count == 1 {
                "Action: Create implementation plan and execute code changes".to_string()
            } else {
                self.responses[*count].clone()
            }
        } else {
            format!("Decision based on: {}", thought)
        };
        *count += 1;
        Ok(response)
    }

    fn reflect(&self, goal: &str) -> Result<String> {
        let mut count = self.call_count.lock().unwrap();
        let response = if *count < self.responses.len() {
            self.responses[*count].clone()
        } else {
            format!("Reflection on goal: {}", goal)
        };
        *count += 1;
        Ok(response)
    }
}

// Test implementation of CogLogger that captures logs
struct TestLogger {
    logs: Arc<Mutex<Vec<String>>>,
}

impl TestLogger {
    fn new() -> Self {
        Self {
            logs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn get_logs(&self) -> Vec<String> {
        self.logs.lock().unwrap().clone()
    }
}

impl CogLogger for TestLogger {
    fn log_step(&self, step: &crate::cognitive_db::Step, task: &Task) -> std::io::Result<()> {
        let mut logs = self.logs.lock().unwrap();
        if step.state == "Think" {
            logs.push(format!("THINK - Task {}: {}", task.id, step.content));
        } else if step.state == "Reflect" {
            logs.push(format!("REFLECT - Task {}: {}", task.id, step.content));
        }
        Ok(())
    }

    fn log_summary(&self, task: &Task, reflection: &str) -> std::io::Result<()> {
        let mut logs = self.logs.lock().unwrap();
        logs.push(format!("REFLECT - Task {}: {}", task.id, reflection));
        Ok(())
    }
}

#[tokio::test]
async fn test_sequential_core_real_implementation() -> Result<()> {
    // Setup temp database
    let temp_db = NamedTempFile::new()?;
    let db_path = temp_db.path().to_str().unwrap();

    // Initialize components with REAL implementations
    let memory = Arc::new(Memory::new(db_path)?);
    let tasks = Arc::new(Tasks::new(&format!("{}_tasks", db_path))?);
    let embeddings = Box::new(RealEmbeddings::new(384)?) as Box<dyn Embeddings>;
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let model = Arc::new(Mutex::new(TestLanguageModel::new()));
    let logger = Arc::new(TestLogger::new());

    // Create SequentialCore with real components
    let core = SequentialCore::new(
        tasks.clone(),
        vector_store,
        memory.clone(),
        model,
        logger.clone(),
    );

    // Create a test task
    let goal = "Implement sequential thinking functionality";
    let task_id = tasks.add_task(goal, "Test sequential thinking", 1, None)?;

    // Run cognitive cycle - this should work with real implementation
    core.run_cycle()?;

    // Verify task was processed
    let completed_task = tasks.get_task(task_id)?;
    assert!(completed_task.is_some(), "Task should exist");
    let completed_task = completed_task.unwrap();
    assert_eq!(completed_task.goal, goal);

    // Verify logs were captured
    let logs = logger.get_logs();
    assert_eq!(logs.len(), 2, "Should have think and reflect logs");
    assert!(logs[0].contains("THINK"));
    assert!(logs[1].contains("REFLECT"));

    // Verify cognitive steps were stored in database
    let db = tasks.get_db();
    let db_guard = db.lock().unwrap();
    let steps = syncore::cognitive_db::recent_steps(&db_guard, completed_task.id, 10)?;
    assert_eq!(steps.len(), 4, "Should have Think, Decide, Act, Reflect steps");

    let step_types: Vec<String> = steps.iter().map(|s| s.state.clone()).collect();
    assert!(step_types.contains(&"Think".to_string()));
    assert!(step_types.contains(&"Decide".to_string()));
    assert!(step_types.contains(&"Act".to_string()));
    assert!(step_types.contains(&"Reflect".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_sequential_core_context_building_with_real_embeddings() -> Result<()> {
    // Setup temp database
    let temp_db = NamedTempFile::new()?;
    let db_path = temp_db.path().to_str().unwrap();

    // Initialize components with REAL implementations
    let memory = Arc::new(Memory::new(db_path)?);
    let tasks = Arc::new(Tasks::new(&format!("{}_tasks", db_path))?);
    let embeddings = Box::new(RealEmbeddings::new(384)?) as Box<dyn Embeddings>;
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let model = Arc::new(Mutex::new(TestLanguageModel::new()));
    let logger = Arc::new(TestLogger::new());

    // Create SequentialCore
    let core = SequentialCore::new(
        tasks.clone(),
        vector_store.clone(),
        memory.clone(),
        model,
        logger.clone(),
    );

    // Create a test task
    let goal = "Test context building with real embeddings";
    let task_id = tasks.add_task(goal, "Testing context with real embeddings", 1, None)?;

    // Get created task
    let task = tasks.get_task(task_id)?;
    assert!(task.is_some(), "Task should exist");
    let task = task.unwrap();

    // Insert some test context into vector store
    {
        let mut vs = vector_store.lock().unwrap();
        vs.insert_text(task.id, Some(task.id), "Previous context information about cats and dogs", "context")?;
        vs.insert_text(task.id, Some(task.id), "Additional context about programming and algorithms", "context")?;
    }

    // Test context building by accessing the private method through the public interface
    // This is tested indirectly through the run_cycle method which uses build_context
    core.run_cycle()?;

    // Verify the cycle completed successfully
    let completed_task = tasks.get_task(task_id)?;
    assert!(completed_task.is_some(), "Task should be completed");

    Ok(())
}

#[tokio::test]
async fn test_sequential_core_no_tasks_with_real_implementation() -> Result<()> {
    // Setup temp database
    let temp_db = NamedTempFile::new()?;
    let db_path = temp_db.path().to_str().unwrap();

    // Initialize components with REAL implementations
    let memory = Arc::new(Memory::new(db_path)?);
    let tasks = Arc::new(Tasks::new(&format!("{}_tasks", db_path))?);
    let embeddings = Box::new(RealEmbeddings::new(384)?) as Box<dyn Embeddings>;
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let model = Arc::new(Mutex::new(TestLanguageModel::new()));
    let logger = Arc::new(TestLogger::new());

    // Create SequentialCore with no tasks
    let core = SequentialCore::new(
        tasks.clone(),
        vector_store.clone(),
        memory,
        model,
        logger,
    );

    // Run cycle - should complete without error even with no tasks
    core.run_cycle()?;

    // Verify no tasks exist by checking next_task
    let next_task = tasks.next_task(None, None)?;
    assert!(next_task.is_none(), "Should have no pending tasks");

    Ok(())
}
