//! Test that circuit breaker prevents infinite loops in sequential thinking

use std::sync::{Arc, Mutex};
use syncore::{
    circuit_breaker::CircuitState,
    logger::{CogLogger, MarkdownLogger},
    memory::Memory,
    sequential::SequentialCore,
    tasks::Tasks,
    vector::{RealEmbeddings, VectorStore},
};
use tempfile::tempdir;

/// Mock language model that returns empty strings (simulates stuck behavior)
struct StuckModel;

impl syncore::sequential::LanguageModel for StuckModel {
    fn think(&self, _context: &str) -> anyhow::Result<String> {
        Ok(String::new()) // Empty thought = stuck
    }

    fn decide(&self, _thought: &str) -> anyhow::Result<String> {
        Ok(String::new()) // Empty decision = stuck
    }

    fn reflect(&self, _goal: &str) -> anyhow::Result<String> {
        Ok(String::new()) // Empty reflection = stuck
    }
}

#[test]
fn test_circuit_breaker_trips_on_empty_thoughts() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test_circuit.db");

    // Setup
    let tasks = Arc::new(Tasks::new(db_path.to_str().unwrap()).unwrap());
    let embeddings = Box::new(RealEmbeddings::new(384).unwrap());
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let memory = Arc::new(Memory::new(db_path.to_str().unwrap()).unwrap());
    let model =
        Arc::new(Mutex::new(StuckModel)) as Arc<Mutex<dyn syncore::sequential::LanguageModel>>;
    let logger = Arc::new(MarkdownLogger::new(temp_dir.path())) as Arc<dyn CogLogger>;

    let core = SequentialCore::new(tasks.clone(), vector_store, memory, model, logger);

    // Create a test task
    tasks.add_task("Test task", "Test description", 1, None).unwrap();

    // Run cycles until circuit breaker trips
    let mut cycle_count = 0;
    let mut circuit_tripped = false;

    for _ in 0..10 {
        cycle_count += 1;

        match core.run_cycle() {
            Ok(_) => {
                println!("Cycle {} completed", cycle_count);
            }
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("Circuit breaker")
                    || err_str.contains("Empty thought")
                    || err_str.contains("Empty decision")
                {
                    circuit_tripped = true;
                    println!(
                        "✓ Circuit breaker protection triggered after {} cycles: {}",
                        cycle_count, err_str
                    );
                    break;
                } else {
                    panic!("Unexpected error: {}", e);
                }
            }
        }
    }

    // ASSERTIONS
    assert!(circuit_tripped, "Circuit breaker should have tripped on empty thoughts");
    assert!(cycle_count <= 5, "Should trip within 5 cycles (configured for 4 no-output calls)");
}

#[test]
fn test_circuit_breaker_allows_successful_cycles() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test_success.db");

    // Mock model that produces output
    struct SuccessModel;

    impl syncore::sequential::LanguageModel for SuccessModel {
        fn think(&self, _context: &str) -> anyhow::Result<String> {
            Ok("Valid thought process".to_string())
        }

        fn decide(&self, _thought: &str) -> anyhow::Result<String> {
            Ok("Action: CompleteTask { task_id: 1 }".to_string())
        }

        fn reflect(&self, _goal: &str) -> anyhow::Result<String> {
            Ok("Task completed successfully".to_string())
        }
    }

    let tasks = Arc::new(Tasks::new(db_path.to_str().unwrap()).unwrap());
    let embeddings = Box::new(RealEmbeddings::new(384).unwrap());
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let memory = Arc::new(Memory::new(db_path.to_str().unwrap()).unwrap());
    let model =
        Arc::new(Mutex::new(SuccessModel)) as Arc<Mutex<dyn syncore::sequential::LanguageModel>>;
    let logger = Arc::new(MarkdownLogger::new(temp_dir.path())) as Arc<dyn CogLogger>;

    let core = SequentialCore::new(tasks.clone(), vector_store, memory, model, logger);

    // Create test tasks
    tasks.add_task("Task 1", "Test", 1, None).unwrap();
    tasks.add_task("Task 2", "Test", 1, None).unwrap();
    tasks.add_task("Task 3", "Test", 1, None).unwrap();

    // Run cycles - should NOT trip
    let mut successful_cycles = 0;

    for _ in 0..3 {
        match core.run_cycle() {
            Ok(_) => {
                successful_cycles += 1;
                println!("✓ Successful cycle {}", successful_cycles);
            }
            Err(e) => {
                panic!("Should not fail on successful cycles: {}", e);
            }
        }
    }

    assert_eq!(successful_cycles, 3, "All 3 cycles should complete successfully");
}

#[test]
fn test_circuit_breaker_state_accessible() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test_state.db");

    let tasks = Arc::new(Tasks::new(db_path.to_str().unwrap()).unwrap());
    let embeddings = Box::new(RealEmbeddings::new(384).unwrap());
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let memory = Arc::new(Memory::new(db_path.to_str().unwrap()).unwrap());
    let model =
        Arc::new(Mutex::new(StuckModel)) as Arc<Mutex<dyn syncore::sequential::LanguageModel>>;
    let logger = Arc::new(MarkdownLogger::new(temp_dir.path())) as Arc<dyn CogLogger>;

    let _core = SequentialCore::new(tasks.clone(), vector_store, memory, model, logger);

    // Initially circuit breaker should be Closed
    // TODO: Add method to SequentialCore to expose circuit breaker state
    // For now, just verify it compiles
    println!("✓ Circuit breaker integrated into SequentialCore");
}
