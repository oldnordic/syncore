/*
// Self-Operating Agent Demonstration Test
// This test demonstrates the complete CREATE → THINK → ACT → REFLECT → MARK DONE loop

use anyhow::Result;
use std::sync::{Arc, Mutex};
use tempfile::NamedTempFile;

use syncore::cognitive_db;
use syncore::logger::CogLogger;
use syncore::memory::Memory;
use syncore::sequential::{LanguageModel, SequentialCore};
use syncore::tasks::Tasks;
use syncore::vector::VectorStore;

// Enhanced GLM client for demonstration
struct DemoGlmClient {
    cycle_count: Mutex<u32>,
}

impl LanguageModel for DemoGlmClient {
    fn think(&self, context: &str) -> Result<String> {
        let mut count = self.cycle_count.lock().unwrap();
        *count += 1;

        Ok(format!(
            "🧠 Cycle {}: Analyzing task context: '{}'\n\
             - Previous cognitive steps considered\n\
             - Vector memory searched for relevant patterns\n\
             - Task goal prioritization completed",
            *count,
            context,
        ))
    }

    fn decide(&self, thought: &str) -> Result<String> {
        Ok(format!(
            "🎯 Decision: Based on thought process, executing action: complete_task\n\
             Reasoning: {}\n\
             Confidence: High\n\
             Action parameters: standard_completion",
            thought.lines().next().unwrap_or("Unknown thought")
        ))
    }

    fn reflect(&self, goal: &str) -> Result<String> {
        Ok(format!(
            "🪞 Reflection: Goal '{}' successfully accomplished\n\
             - Task execution completed without errors\n\
             - Learning: Task completion follows expected pattern\n\
             - Next steps: Mark task as done and archive cognitive steps",
            goal
        ))
    }
}

impl DemoGlmClient {
    pub fn new() -> Self {
        Self {
            cycle_count: Mutex::new(0),
        }
    }

    pub fn get_cycle_count(&self) -> u32 {
        *self.cycle_count.lock().unwrap()
    }
}

// Demo logger that captures cognitive steps
struct DemoLogger {
    log_entries: Mutex<Vec<String>>,
}

impl DemoLogger {
    pub fn new() -> Self {
        Self {
            log_entries: Mutex::new(Vec::new()),
        }
    }

    pub fn get_entries(&self) -> Vec<String> {
        self.log_entries.lock().unwrap().clone()
    }
}

impl CogLogger for DemoLogger {
    fn log_step(
        &self,
        step: &crate::cognitive_db::Step,
        task: &syncore::tasks::Task,
    ) -> std::io::Result<()> {
        let entry = format!("📝 STEP - Task {}: {} - {}", task.id, task.goal, step.state);
        self.log_entries.lock().unwrap().push(entry);
        Ok(())
    }

    fn log_summary(&self, task: &syncore::tasks::Task, reflection: &str) -> std::io::Result<()> {
        let entry = format!(
            "🪞 SUMMARY - Task {}: {} - {}\n",
            task.id,
            task.goal,
            reflection,
        );
        self.log_entries.lock().unwrap().push(entry);
        Ok(())
    }
}

#[test]
fn test_self_operating_agent() -> Result<()> {
    println!("🚀 SYNCore Self-Operating Agent Test");
    println!("   ==================================");
    println!("   Phase 7: Self-Operating Cognition Loop");
    println!("   Pattern: CREATE → THINK → ACT → REFLECT → MARK DONE\n");

    // Setup test environment
    let temp_file = NamedTempFile::new()?;
    let db_path = temp_file.path().to_str().unwrap();

    println!("📁 Initializing agent components...");
    println!("   Database: {}", db_path);

    // Initialize core components
    let memory = Arc::new(Memory::new(db_path)?);
    let tasks = Arc::new(Tasks::new(db_path)?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(Box::new(
        syncore::vector::RealEmbeddings::new(384).unwrap(),
    ))));
    let glm_client = Arc::new(Mutex::new(DemoGlmClient::new())) as Arc<Mutex<dyn LanguageModel>>;
    let logger = Arc::new(DemoLogger::new());
    let logger_ref = logger.clone();

    // Create sequential core (the brain of agent)
    let sequential_core = SequentialCore::new(
        tasks.clone(),
        vector_store.clone(),
        memory.clone(),
        glm_client.clone(),
        logger_ref as Arc<dyn CogLogger>,
    );

    println!("✅ Agent components initialized successfully!");
    println!("   - Tasks: Ready for task management");
    println!("   - VectorStore: Memory system online");
    println!("   - LanguageModel: Cognitive engine ready");
    println!("   - Logger: Cognitive logging active\n");

    // ==================== PHASE 1: CREATE ====================
    println!("🎯 PHASE 1: CREATE - Defining autonomous objectives");
    println!("   ------------------------------------------------");

    let task_goals = vec![
        "Analyze system performance metrics",
        "Optimize database query efficiency",
        "Implement user authentication system",
        "Deploy application to production",
    ];

    let mut task_ids = Vec::new();

    for (i, goal) in task_goals.iter().enumerate() {
        let task_id = tasks.add_task(
            goal,
            &format!("Autonomous task {}", i + 1),
            (i + 1) as i32,
            None,
        )?;
        task_ids.push(task_id);
        println!("   📋 Task {} created: '{}'", task_id, goal);
    }

    println!(
        "   ✅ {} autonomous tasks created successfully!\n",
        task_ids.len()
    );

    // ==================== PHASE 2-5: THINK → ACT → REFLECT → DONE ====================
    println!("🧠 PHASES 2-5: Autonomous Cognition Loop Execution");
    println!("   ===============================================");
    println!("   Starting self-operating cognition cycles...\n");

    let mut completed_tasks = 0;
    let max_cycles = 30; // Increased safety limit
    let mut cycle_count = 0;

    while completed_tasks < task_ids.len() && cycle_count < max_cycles {
        cycle_count += 1;
        println!("🔄 Cognition Cycle {}:", cycle_count);

        // Execute one cognition cycle
        if let Err(e) = sequential_core.run_cycle() {
            println!("   ⚠️  Cycle failed: {}", e);
            continue;
        }

        // Check task completion
        let mut tasks_completed_this_cycle = 0;
        for &task_id in &task_ids {
            if let Ok(Some(task)) = tasks.get_task(task_id) {
                if task.status == "done" {
                    tasks_completed_this_cycle += 1;
                }
            }
        }

        if tasks_completed_this_cycle > 0 {
            completed_tasks += tasks_completed_this_cycle;
            println!(
                "   ✅ {} task(s) completed in this cycle",
                tasks_completed_this_cycle
            );
        } else {
            println!("   ⏳ No tasks completed in this cycle");
        }

        println!();
    }

    // ==================== RESULTS SUMMARY ====================
    println!("📊 TEST RESULTS");
    println!("   ==============");

    // Final task status check
    let mut final_completed = 0;
    for &task_id in &task_ids {
        if let Ok(Some(task)) = tasks.get_task(task_id) {
            println!(
                "   📋 Task {}: {} - Status: {}",
                task_id,
                task.goal,
                task.status,
            );
            if task.status == "done" {
                final_completed += 1;
            }
        }
    }

    // Cognitive steps analysis
    let total_cognitive_steps = task_ids
        .iter()
        .map(|&task_id| {
            let db = tasks.get_db();
            let db_guard = db.lock().unwrap();
            cognitive_db::recent_steps(&db_guard, task_id, 100)
                .unwrap_or_default()
                .len()
        })
        .sum::<usize>();

    // Agent metrics (simplified for test)
    let agent_cycles = cycle_count; // Use actual cycle count as proxy

    println!("\n🎯 PERFORMANCE METRICS:");
    println!("   Tasks Created: {}", task_ids.len());
    println!("   Tasks Completed: {}/{}", final_completed, task_ids.len());
    println!(
        "   Success Rate: {:.1}%",
        (final_completed as f64 / task_ids.len() as f64) * 100.0
    );
    println!("   Total Cognition Cycles: {}", cycle_count);
    println!("   Agent Thinking Cycles: {}", agent_cycles);
    println!("   Total Cognitive Steps: {}", total_cognitive_steps);
    println!(
        "   Average Steps per Task: {:.1}",
        total_cognitive_steps as f64 / task_ids.len() as f64
    );

    // Show cognitive log (simplified)
    println!("\n📝 COGNITIVE LOG:");
    println!(
        "   Cognitive steps recorded in database: {}",
        total_cognitive_steps
    );

    // ==================== VALIDATION ====================
    println!("\n✅ SELF-OPERATING AGENT VALIDATION:");
    println!("   ==================================");

    let mut validation_passed = 0;
    let mut validation_total = 0;

    // Validate CREATE phase
    validation_total += 1;
    if !task_ids.is_empty() {
        println!("   ✅ CREATE: Tasks successfully created");
        validation_passed += 1;
    } else {
        println!("   ❌ CREATE: No tasks created");
    }

    // Validate THINK phase
    validation_total += 1;
    if total_cognitive_steps >= task_ids.len() * 2 {
        // At least Think + Reflect per task
        println!("   ✅ THINK: Cognitive steps recorded");
        validation_passed += 1;
    } else {
        println!("   ❌ THINK: Insufficient cognitive steps");
    }

    // Validate ACT phase
    validation_total += 1;
    if final_completed > 0 {
        println!("   ✅ ACT: Tasks were processed and actions executed");
        validation_passed += 1;
    } else {
        println!("   ❌ ACT: No tasks completed");
    }

    // Validate REFLECT phase
    validation_total += 1;
    if total_cognitive_steps >= task_ids.len() * 3 {
        // Think + Decide + Reflect per task
        println!("   ✅ REFLECT: Reflection steps completed");
        validation_passed += 1;
    } else {
        println!("   ❌ REFLECT: Incomplete cognitive process");
    }

    // Validate DONE phase
    validation_total += 1;
    if final_completed == task_ids.len() {
        println!("   ✅ DONE: All tasks marked as complete");
        validation_passed += 1;
    } else {
        println!(
            "   ⚠️  DONE: Some tasks incomplete ({} of {})",
            final_completed,
            task_ids.len()
        );
    }

    let success_rate = (validation_passed as f64 / validation_total as f64) * 100.0;
    println!(
        "\n🎉 OVERALL SUCCESS RATE: {:.1}% ({}/{})",
        success_rate, validation_passed, validation_total
    );

    // Assert that the demonstration was successful
    assert!(
        success_rate >= 60.0,
        "Self-operating agent success rate too low: {:.1}%",
        success_rate
    );
    assert!(
        final_completed >= task_ids.len() / 2,
        "Too few tasks completed: {}/{}",
        final_completed,
        task_ids.len()
    );
    assert!(
        total_cognitive_steps >= task_ids.len() * 2,
        "Too few cognitive steps: {} for {} tasks",
        total_cognitive_steps,
        task_ids.len()
    );

    if success_rate >= 80.0 {
        println!("🚀 SYNCore Self-Operating Agent: MISSION ACCOMPLISHED!");
        println!("   The agent successfully demonstrated autonomous cognition loops.");
    } else {
        println!("🔧 SYNCore Self-Operating Agent: PARTIAL SUCCESS");
        println!("   Some components need refinement for full autonomy.");
    }

    Ok(())
}
*/
