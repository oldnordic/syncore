use crate::{
    circuit_breaker::{AgentCircuitBreaker, CircuitBreakerConfig},
    cognitive_db::{self, CogState, Step},
    logger::CogLogger,
    memory::Memory,
    ollama::{OllamaClient, OllamaConfig},
    tasks::{Task, Tasks},
    vector::VectorStore,
};
use anyhow::{anyhow, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// Trait for language model interactions
pub trait LanguageModel: Send + Sync {
    fn think(&self, context: &str) -> Result<String>;
    fn decide(&self, thought: &str) -> Result<String>;
    fn reflect(&self, goal: &str) -> Result<String>;
}

/// Action types that can be parsed and executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    CreateFile {
        path: String,
        content: String,
    },
    ReadFile {
        path: String,
    },
    UpdateFile {
        path: String,
        content: String,
    },
    DeleteFile {
        path: String,
    },
    CreateTask {
        goal: String,
        priority: i32,
    },
    CompleteTask {
        task_id: u64,
    },
    SearchCode {
        query: String,
        path: String,
    },
    AnalyzeCode {
        file_path: String,
    },
    StoreMemory {
        key: String,
        value: String,
    },
    QueryMemory {
        key: String,
    },
    SearchVector {
        query: String,
        limit: usize,
    },
    CustomAction {
        action: String,
        parameters: std::collections::HashMap<String, String>,
    },
}

/// Represents a parsed action with its parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedAction {
    pub action_type: ActionType,
    pub description: String,
    pub confidence: f64,
}

/// Advanced sequential thinking core with real action execution
pub struct SequentialCore {
    tasks: Arc<Tasks>,
    vector_store: Arc<Mutex<VectorStore>>,
    memory: Arc<Memory>,
    model: Arc<Mutex<dyn LanguageModel>>,
    logger: Arc<dyn CogLogger>,
    action_parser: ActionParser,
    circuit_breaker: AgentCircuitBreaker,
    cycle_count: Arc<Mutex<usize>>,
}

impl SequentialCore {
    pub fn new(
        tasks: Arc<Tasks>,
        vector_store: Arc<Mutex<VectorStore>>,
        memory: Arc<Memory>,
        model: Arc<Mutex<dyn LanguageModel>>,
        logger: Arc<dyn CogLogger>,
    ) -> Self {
        // Configure circuit breaker for sequential thinking
        let circuit_config = CircuitBreakerConfig {
            max_identical_calls: 3,   // Same task 3x = stuck
            max_no_output_calls: 4,   // 4 empty thoughts = stuck
            max_calls_per_window: 10, // 10 cycles/30s max
            ..Default::default()
        };

        Self {
            tasks,
            vector_store,
            memory,
            model,
            logger,
            action_parser: ActionParser::new(),
            circuit_breaker: AgentCircuitBreaker::with_config(circuit_config),
            cycle_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Run a complete cognitive cycle for task processing
    pub fn run_cycle(&self) -> Result<CycleResult> {
        // Increment cycle count
        let mut count = self.cycle_count.lock().unwrap();
        *count += 1;
        let cycle_num = *count;
        drop(count);

        // Get next task
        let task = match self.tasks.next_task(None, None)? {
            Some(task) => task,
            None => return Ok(CycleResult::NoTasks),
        };

        // CHECK CIRCUIT BREAKER BEFORE STARTING
        let cycle_id = format!("cycle_{}_task_{}", cycle_num, task.id);
        self.circuit_breaker
            .check_tool_call("cognitive_cycle", &cycle_id)
            .map_err(|e| anyhow!("Circuit breaker tripped: {}", e))?;

        // Phase 1: Build comprehensive context
        let context = self.build_enhanced_context(&task)?;

        // Phase 2: Deep thinking
        let model = self.model.lock().unwrap();
        let thought = model.think(&context)?;
        drop(model);

        // DETECT EMPTY THOUGHT - Circuit breaker protection
        if thought.trim().is_empty() {
            self.circuit_breaker.record_result("cognitive_cycle", &cycle_id, false);
            return Err(anyhow!("Empty thought detected - cognitive cycle failed. This may indicate the model is stuck or unable to reason about the task."));
        }

        // Store think step
        self.store_cognitive_step(&task, CogState::Think, &thought)?;
        self.log_step(&task, CogState::Think, &thought)?;

        // Phase 3: Decision making with action parsing
        let model = self.model.lock().unwrap();
        let decision = model.decide(&thought)?;
        drop(model);

        // DETECT EMPTY DECISION - Circuit breaker protection
        if decision.trim().is_empty() {
            self.circuit_breaker.record_result("cognitive_cycle", &cycle_id, false);
            return Err(anyhow!("Empty decision detected - cognitive cycle failed. The model was unable to decide on actions."));
        }

        // Store decide step
        self.store_cognitive_step(&task, CogState::Decide, &decision)?;
        self.log_step(&task, CogState::Decide, &decision)?;

        // Phase 4: Parse and execute actions
        let actions = self.action_parser.parse_actions(&decision)?;
        let mut action_results = Vec::new();

        for action in &actions {
            let result = self.execute_action(&task, action)?;
            action_results.push(result.clone());

            // Store each action step
            self.store_cognitive_step(&task, CogState::Act, &result)?;
            self.log_step(&task, CogState::Act, &result)?;
        }

        // Phase 5: Reflection on outcomes
        let reflection_context = self.build_reflection_context(&task, &action_results)?;
        let model = self.model.lock().unwrap();
        let reflection = model.reflect(&reflection_context)?;
        drop(model);

        // Store reflect step
        self.store_cognitive_step(&task, CogState::Reflect, &reflection)?;
        self.log_step(&task, CogState::Reflect, &reflection)?;

        // Store comprehensive results in memory
        self.store_cycle_results(&task, &thought, &decision, &action_results, &reflection)?;

        // Complete the task
        self.tasks.complete_task(task.id)?;

        // RECORD CIRCUIT BREAKER SUCCESS
        let had_output = !thought.is_empty() && !decision.is_empty();
        self.circuit_breaker.record_result("cognitive_cycle", &cycle_id, had_output);

        Ok(CycleResult::Completed {
            task_id: task.id as u64,
            thought,
            decision,
            actions,
            action_results,
            reflection,
        })
    }

    /// Build enhanced context with multi-dimensional information
    fn build_enhanced_context(&self, task: &Task) -> Result<String> {
        let mut context = format!(
            "=== TASK ANALYSIS ===\nID: {}\nGoal: {}\nDescription: {}\nPriority: {}\nCreated: {}\n\n",
            task.id,
            task.goal,
            task.description,
            task.priority,
            task.created_at
        );

        // 1. Task-specific memories
        if let Ok(Some(memories)) = self.memory.query(&format!("task_context_{}", task.id)) {
            context.push_str(&format!("=== TASK MEMORIES ===\n{}\n\n", memories));
        }

        // 2. Relevant vector memories
        let vector_store = self.vector_store.lock().unwrap();
        let search_results =
            vector_store.search(&task.goal, 10, crate::vector::SearchScope::Global)?;
        drop(vector_store);

        if !search_results.is_empty() {
            context.push_str("=== RELEVANT VECTOR MEMORIES ===\n");
            for (i, result) in search_results.iter().enumerate() {
                context.push_str(&format!(
                    "{}. {} (score: {:.3})\n",
                    i + 1,
                    result.text,
                    result.score
                ));
            }
            context.push_str("\n");
        }

        // 3. Similar tasks from history
        if let Ok(historical_context) = self.get_similar_tasks(task) {
            context
                .push_str(&format!("=== SIMILAR HISTORICAL TASKS ===\n{}\n\n", historical_context));
        }

        // 4. Available actions and capabilities
        context.push_str("=== AVAILABLE ACTIONS ===\n");
        context.push_str("- CreateFile: Create new files with content\n");
        context.push_str("- ReadFile: Read and analyze file contents\n");
        context.push_str("- UpdateFile: Modify existing files\n");
        context.push_str("- DeleteFile: Remove files\n");
        context.push_str("- CreateTask: Create subtasks with priorities\n");
        context.push_str("- CompleteTask: Mark tasks as completed\n");
        context.push_str("- SearchCode: Search code patterns with ripgrep\n");
        context.push_str("- AnalyzeCode: Deep code analysis using tree-sitter\n");
        context.push_str("- StoreMemory: Store key-value memories\n");
        context.push_str("- QueryMemory: Retrieve stored memories\n");
        context.push_str("- SearchVector: Semantic search in vector store\n\n");

        context.push_str("=== COGNITIVE INSTRUCTIONS ===\n");
        context.push_str("1. Analyze the task requirements thoroughly\n");
        context.push_str("2. Consider similar past experiences\n");
        context.push_str("3. Break down complex tasks into actionable steps\n");
        context.push_str("4. Use Action: prefix for executable actions\n");
        context.push_str("5. Reflect on outcomes and learn from them\n");

        Ok(context)
    }

    /// Get similar tasks from history
    fn get_similar_tasks(&self, current_task: &Task) -> Result<String> {
        // Search for similar tasks in vector store
        let vector_store = self.vector_store.lock().unwrap();
        let similar_tasks =
            vector_store.search(&current_task.goal, 5, crate::vector::SearchScope::Global)?;
        drop(vector_store);

        let mut result = String::new();
        for task_hit in similar_tasks {
            if task_hit.text.contains("task_goal:") {
                result.push_str(&format!("- {}\n", task_hit.text));
            }
        }

        Ok(result)
    }

    /// Execute parsed actions with real implementations
    fn execute_action(&self, task: &Task, action: &ParsedAction) -> Result<String> {
        match &action.action_type {
            ActionType::CreateFile {
                path,
                content,
            } => {
                std::fs::write(path, content)?;
                let result = format!("Created file: {} with {} bytes", path, content.len());
                self.memory.store(&format!("file_created_{}", path), &result)?;
                Ok(result)
            }
            ActionType::ReadFile {
                path,
            } => {
                let content = std::fs::read_to_string(path)?;
                let result = format!("Read file: {} ({} bytes)\n{}", path, content.len(), content);
                self.memory.store(&format!("file_read_{}", path), &content)?;
                Ok(result)
            }
            ActionType::UpdateFile {
                path,
                content,
            } => {
                let old_content = std::fs::read_to_string(path).unwrap_or_default();
                std::fs::write(path, content)?;
                let result = format!(
                    "Updated file: {} ({} -> {} bytes)",
                    path,
                    old_content.len(),
                    content.len()
                );
                self.memory.store(&format!("file_updated_{}", path), &result)?;
                Ok(result)
            }
            ActionType::DeleteFile {
                path,
            } => {
                std::fs::remove_file(path)?;
                let result = format!("Deleted file: {}", path);
                self.memory.store(&format!("file_deleted_{}", path), &result)?;
                Ok(result)
            }
            ActionType::CreateTask {
                goal,
                priority,
            } => {
                let task_id = self.tasks.add_task(goal, "", *priority, Some(task.id))?;
                let result = format!(
                    "Created subtask {} with goal: {} (priority: {})",
                    task_id, goal, priority
                );
                self.memory.store(&format!("task_created_{}", task_id), &result)?;
                Ok(result)
            }
            ActionType::CompleteTask {
                task_id,
            } => {
                self.tasks.complete_task((*task_id) as i64)?;
                let result = format!("Completed task: {}", task_id);
                self.memory.store(&format!("task_completed_{}", task_id), &result)?;
                Ok(result)
            }
            ActionType::SearchCode {
                query,
                path,
            } => {
                use std::process::Command;
                let output = Command::new("rg").args(["--json", query, path]).output()?;

                if output.status.success() {
                    let results = String::from_utf8_lossy(&output.stdout);
                    let result = format!("Code search for '{}' in {}:\n{}", query, path, results);
                    self.memory.store(
                        &format!("code_search_{}_{}", query, path.replace("/", "_")),
                        &result,
                    )?;
                    Ok(result)
                } else {
                    Err(anyhow!("Code search failed: {}", String::from_utf8_lossy(&output.stderr)))
                }
            }
            ActionType::AnalyzeCode {
                file_path,
            } => {
                // Use parser for deep code analysis
                let parser = crate::parser::Parser::new()?;
                let structure = parser.parse_file(std::path::Path::new(file_path))?;
                let analysis = format!(
                    "Code analysis for {}:\n{}",
                    file_path,
                    serde_json::to_string_pretty(&structure)?
                );
                self.memory
                    .store(&format!("code_analysis_{}", file_path.replace("/", "_")), &analysis)?;
                Ok(analysis)
            }
            ActionType::StoreMemory {
                key,
                value,
            } => {
                self.memory.store(key, value)?;
                Ok(format!("Stored memory: {} -> {}", key, value))
            }
            ActionType::QueryMemory {
                key,
            } => match self.memory.query(key)? {
                Some(value) => Ok(format!("Retrieved memory: {} -> {}", key, value)),
                None => Ok(format!("Memory key not found: {}", key)),
            },
            ActionType::SearchVector {
                query,
                limit,
            } => {
                let vector_store = self.vector_store.lock().unwrap();
                let results =
                    vector_store.search(query, *limit, crate::vector::SearchScope::Global)?;
                drop(vector_store);

                let result_text = results
                    .into_iter()
                    .map(|hit| format!("{} (score: {:.3})", hit.text, hit.score))
                    .collect::<Vec<_>>()
                    .join("\n");

                Ok(format!("Vector search for '{}':\n{}", query, result_text))
            }
            ActionType::CustomAction {
                action,
                parameters,
            } => {
                let result =
                    format!("Executed custom action: {} with parameters: {:?}", action, parameters);
                self.memory
                    .store(&format!("custom_action_{}", action.replace(" ", "_")), &result)?;
                Ok(result)
            }
        }
    }

    /// Build context for reflection phase
    fn build_reflection_context(&self, task: &Task, action_results: &[String]) -> Result<String> {
        let mut context = format!(
            "=== REFLECTION CONTEXT ===\nOriginal Task: {}\nGoal: {}\n\n",
            task.goal, task.description
        );

        context.push_str("=== ACTION RESULTS ===\n");
        for (i, result) in action_results.iter().enumerate() {
            context.push_str(&format!("{}. {}\n", i + 1, result));
        }

        context.push_str("\n=== REFLECTION QUESTIONS ===\n");
        context.push_str("1. Did the actions successfully accomplish the task goal?\n");
        context.push_str("2. What worked well and what could be improved?\n");
        context.push_str("3. What patterns or insights can be learned?\n");
        context.push_str("4. How can this experience help with similar future tasks?\n");

        Ok(context)
    }

    /// Store comprehensive cycle results
    fn store_cycle_results(
        &self,
        task: &Task,
        thought: &str,
        decision: &str,
        action_results: &[String],
        reflection: &str,
    ) -> Result<()> {
        let cycle_data = serde_json::json!({
            "task_id": task.id,
            "goal": task.goal,
            "thought": thought,
            "decision": decision,
            "action_results": action_results,
            "reflection": reflection,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        let cycle_key = format!("cycle_complete_{}", task.id);
        self.memory.store(&cycle_key, &cycle_data.to_string())?;

        // Store in vector for future semantic search
        let vector_text = format!(
            "task_goal:{} thought:{} decision:{} reflection:{}",
            task.goal, thought, decision, reflection
        );

        if let Ok(mut store) = self.vector_store.try_lock() {
            let _ = store.insert_text(0, Some(task.id), &vector_text, "sequential_cycle");
        }

        Ok(())
    }

    /// Store cognitive step in database
    fn store_cognitive_step(&self, task: &Task, state: CogState, content: &str) -> Result<()> {
        let db = self.tasks.get_db();
        let db_guard = db.lock().unwrap();

        let meta_json =
            format!("{{\"task_id\": {}, \"state\": \"{}\"}}", task.id, state.to_string());
        cognitive_db::store_step(
            &db_guard,
            Some(task.id),
            &state.to_string(),
            content,
            &meta_json,
        )?;
        Ok(())
    }

    /// Log step to logger
    fn log_step(&self, task: &Task, state: CogState, content: &str) -> Result<()> {
        let step = Step {
            id: 0,
            task_id: Some(task.id),
            state: state.to_string(),
            content: content.to_string(),
            meta_json: "{}".to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        };
        self.logger.log_step(&step, task)?;
        Ok(())
    }

    /// Run multiple cycles for batch processing
    pub fn run_batch_cycles(&self, max_cycles: usize) -> Result<Vec<CycleResult>> {
        let mut results = Vec::new();

        for _ in 0..max_cycles {
            match self.run_cycle() {
                Ok(CycleResult::NoTasks) => break,
                Ok(result) => results.push(result),
                Err(e) => {
                    eprintln!("Cycle failed: {}", e);
                    break;
                }
            }
        }

        Ok(results)
    }
}

/// Results of a cognitive cycle
#[derive(Debug, Clone, serde::Serialize)]
pub enum CycleResult {
    NoTasks,
    Completed {
        task_id: u64,
        thought: String,
        decision: String,
        actions: Vec<ParsedAction>,
        action_results: Vec<String>,
        reflection: String,
    },
}

/// Action parser for extracting executable actions from decisions
pub struct ActionParser {
    create_file_regex: Regex,
    read_file_regex: Regex,
    update_file_regex: Regex,
    delete_file_regex: Regex,
    create_task_regex: Regex,
    complete_task_regex: Regex,
    search_code_regex: Regex,
    analyze_code_regex: Regex,
    store_memory_regex: Regex,
    query_memory_regex: Regex,
    search_vector_regex: Regex,
    custom_action_regex: Regex,
}

impl ActionParser {
    pub fn new() -> Self {
        Self {
            create_file_regex: Regex::new(r#"Action:\s*CreateFile\s*\{?path:\s*['"]([^'"]+)['"],?\s*content:\s*['"]([^'"]*)['"]\}?"#).unwrap(),
            read_file_regex: Regex::new(r#"Action:\s*ReadFile\s*\{?path:\s*['"]([^'"]+)['"]\}?"#).unwrap(),
            update_file_regex: Regex::new(r#"Action:\s*UpdateFile\s*\{?path:\s*['"]([^'"]+)['"],?\s*content:\s*['"]([^'"]*)['"]\}?"#).unwrap(),
            delete_file_regex: Regex::new(r#"Action:\s*DeleteFile\s*\{?path:\s*['"]([^'"]+)['"]\}?"#).unwrap(),
            create_task_regex: Regex::new(r#"Action:\s*CreateTask\s*\{?goal:\s*['"]([^'"]+)['"],?\s*priority:\s*(\d+)\}?"#).unwrap(),
            complete_task_regex: Regex::new(r#"Action:\s*CompleteTask\s*\{?task_id:\s*(\d+)\}?"#).unwrap(),
            search_code_regex: Regex::new(r#"Action:\s*SearchCode\s*\{?query:\s*['"]([^'"]+)['"],?\s*path:\s*['"]([^'"]+)['"]\}?"#).unwrap(),
            analyze_code_regex: Regex::new(r#"Action:\s*AnalyzeCode\s*\{?file_path:\s*['"]([^'"]+)['"]\}?"#).unwrap(),
            store_memory_regex: Regex::new(r#"Action:\s*StoreMemory\s*\{?key:\s*['"]([^'"]+)['"],?\s*value:\s*['"]([^'"]+)['"]\}?"#).unwrap(),
            query_memory_regex: Regex::new(r#"Action:\s*QueryMemory\s*\{?key:\s*['"]([^'"]+)['"]\}?"#).unwrap(),
            search_vector_regex: Regex::new(r#"Action:\s*SearchVector\s*\{?query:\s*['"]([^'"]+)['"],?\s*limit:\s*(\d+)\}?"#).unwrap(),
            custom_action_regex: Regex::new(r#"Action:\s*([A-Za-z][A-Za-z0-9_]*)\s*(?:\{([^}]*)\})?"#).unwrap(),
        }
    }

    /// Parse actions from decision text
    pub fn parse_actions(&self, decision: &str) -> Result<Vec<ParsedAction>> {
        let mut actions = Vec::new();

        // Try each action type pattern
        if let Some(caps) = self.create_file_regex.captures(decision) {
            actions.push(ParsedAction {
                action_type: ActionType::CreateFile {
                    path: caps[1].to_string(),
                    content: caps.get(2).map(|m| m.as_str()).unwrap_or("").to_string(),
                },
                description: format!("Create file: {}", &caps[1]),
                confidence: 0.9,
            });
        }

        if let Some(caps) = self.read_file_regex.captures(decision) {
            actions.push(ParsedAction {
                action_type: ActionType::ReadFile {
                    path: caps[1].to_string(),
                },
                description: format!("Read file: {}", &caps[1]),
                confidence: 0.9,
            });
        }

        if let Some(caps) = self.update_file_regex.captures(decision) {
            actions.push(ParsedAction {
                action_type: ActionType::UpdateFile {
                    path: caps[1].to_string(),
                    content: caps.get(2).map(|m| m.as_str()).unwrap_or("").to_string(),
                },
                description: format!("Update file: {}", &caps[1]),
                confidence: 0.9,
            });
        }

        if let Some(caps) = self.delete_file_regex.captures(decision) {
            actions.push(ParsedAction {
                action_type: ActionType::DeleteFile {
                    path: caps[1].to_string(),
                },
                description: format!("Delete file: {}", &caps[1]),
                confidence: 0.8,
            });
        }

        if let Some(caps) = self.create_task_regex.captures(decision) {
            actions.push(ParsedAction {
                action_type: ActionType::CreateTask {
                    goal: caps[1].to_string(),
                    priority: caps[2].parse().unwrap_or(1),
                },
                description: format!("Create task: {}", &caps[1]),
                confidence: 0.9,
            });
        }

        if let Some(caps) = self.complete_task_regex.captures(decision) {
            actions.push(ParsedAction {
                action_type: ActionType::CompleteTask {
                    task_id: caps[1].parse().unwrap_or(0),
                },
                description: format!("Complete task: {}", &caps[1]),
                confidence: 0.9,
            });
        }

        if let Some(caps) = self.search_code_regex.captures(decision) {
            actions.push(ParsedAction {
                action_type: ActionType::SearchCode {
                    query: caps[1].to_string(),
                    path: caps[2].to_string(),
                },
                description: format!("Search code: {} in {}", &caps[1], &caps[2]),
                confidence: 0.8,
            });
        }

        if let Some(caps) = self.analyze_code_regex.captures(decision) {
            actions.push(ParsedAction {
                action_type: ActionType::AnalyzeCode {
                    file_path: caps[1].to_string(),
                },
                description: format!("Analyze code: {}", &caps[1]),
                confidence: 0.8,
            });
        }

        if let Some(caps) = self.store_memory_regex.captures(decision) {
            actions.push(ParsedAction {
                action_type: ActionType::StoreMemory {
                    key: caps[1].to_string(),
                    value: caps[2].to_string(),
                },
                description: format!("Store memory: {} -> {}", &caps[1], &caps[2]),
                confidence: 0.9,
            });
        }

        if let Some(caps) = self.query_memory_regex.captures(decision) {
            actions.push(ParsedAction {
                action_type: ActionType::QueryMemory {
                    key: caps[1].to_string(),
                },
                description: format!("Query memory: {}", &caps[1]),
                confidence: 0.9,
            });
        }

        if let Some(caps) = self.search_vector_regex.captures(decision) {
            actions.push(ParsedAction {
                action_type: ActionType::SearchVector {
                    query: caps[1].to_string(),
                    limit: caps[2].parse().unwrap_or(5),
                },
                description: format!("Vector search: {} (limit: {})", &caps[1], &caps[2]),
                confidence: 0.8,
            });
        }

        // Catch-all for custom actions
        for cap in self.custom_action_regex.captures_iter(decision) {
            let action_name = &cap[1];
            let params = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            let parameters = if params.is_empty() {
                std::collections::HashMap::new()
            } else {
                // Simple parameter parsing
                params
                    .split(',')
                    .filter_map(|p| {
                        let parts: Vec<&str> = p.split(':').collect();
                        if parts.len() == 2 {
                            Some((parts[0].trim().to_string(), parts[1].trim().to_string()))
                        } else {
                            None
                        }
                    })
                    .collect()
            };

            actions.push(ParsedAction {
                action_type: ActionType::CustomAction {
                    action: action_name.to_string(),
                    parameters,
                },
                description: format!("Custom action: {}", action_name),
                confidence: 0.7,
            });
        }

        Ok(actions)
    }
}

/// Production language model using Ollama with phi3-mini for real reasoning
pub struct OllamaLanguageModel {
    client: Arc<std::sync::Mutex<OllamaClient>>,
}

impl OllamaLanguageModel {
    /// Create a new Ollama-based language model with custom configuration
    pub fn new(config: OllamaConfig) -> Result<Self> {
        let client = OllamaClient::new(config)?;

        // Verify Ollama is available
        client.health_check()?;

        Ok(Self {
            client: Arc::new(std::sync::Mutex::new(client)),
        })
    }

    /// Create a new Ollama-based language model with default configuration (phi3-mini)
    pub fn new_default() -> Result<Self> {
        Self::new(OllamaConfig::default())
    }

    /// Generate text using the Ollama model
    fn generate(&self, prompt: &str) -> Result<String> {
        let client =
            self.client.lock().map_err(|e| anyhow!("Failed to lock Ollama client: {}", e))?;

        client.generate(prompt)
    }
}

impl LanguageModel for OllamaLanguageModel {
    fn think(&self, context: &str) -> Result<String> {
        let prompt = format!(
            "You are a reasoning assistant analyzing a task. Carefully analyze the context and think through the task requirements.\n\n\
            CONTEXT:\n{}\n\n\
            INSTRUCTIONS:\n\
            1. Identify the core goal and requirements\n\
            2. Consider relevant past experiences from the context\n\
            3. Think about what information is available\n\
            4. Identify what actions might be needed\n\
            5. Consider potential challenges or obstacles\n\
            6. Formulate a strategic approach\n\n\
            Respond with your analysis in a clear, structured way. Focus on understanding, not yet deciding on actions.",
            context
        );

        let response = self.generate(&prompt)?;
        Ok(format!("🧠 Analysis:\n{}", response))
    }

    fn decide(&self, thought: &str) -> Result<String> {
        let prompt = format!(
            "Based on your previous analysis, now make concrete decisions about what actions to take.\n\n\
            YOUR ANALYSIS:\n{}\n\n\
            AVAILABLE ACTIONS:\n\
            - CreateFile {{path: \"...\", content: \"...\"}}\n\
            - ReadFile {{path: \"...\"}} \n\
            - UpdateFile {{path: \"...\", content: \"...\"}}\n\
            - DeleteFile {{path: \"...\"}}\n\
            - CreateTask {{goal: \"...\", priority: 1-10}}\n\
            - CompleteTask {{task_id: N}}\n\
            - SearchCode {{query: \"...\", path: \"...\"}}\n\
            - AnalyzeCode {{file_path: \"...\"}}\n\
            - StoreMemory {{key: \"...\", value: \"...\"}}\n\
            - QueryMemory {{key: \"...\"}}\n\
            - SearchVector {{query: \"...\", limit: N}}\n\n\
            INSTRUCTIONS:\n\
            1. Choose the most appropriate actions to accomplish the task\n\
            2. Format each action EXACTLY as shown above with 'Action:' prefix\n\
            3. Be specific with parameters (real file paths, concrete goals, etc.)\n\
            4. Prioritize actions that gather information before making changes\n\
            5. Explain your reasoning briefly before listing actions\n\n\
            Respond with your decision and action plan.",
            thought
        );

        let response = self.generate(&prompt)?;
        Ok(format!("🎯 Decision:\n{}", response))
    }

    fn reflect(&self, goal: &str) -> Result<String> {
        let prompt = format!(
            "Reflect on the actions that were just executed and their outcomes.\n\n\
            REFLECTION CONTEXT:\n{}\n\n\
            INSTRUCTIONS:\n\
            1. Did the actions successfully accomplish the goal?\n\
            2. What worked well?\n\
            3. What could have been done differently?\n\
            4. What patterns or insights can be learned?\n\
            5. How can this experience help with similar future tasks?\n\
            6. What should be remembered for next time?\n\n\
            Provide thoughtful reflection that captures lessons learned.",
            goal
        );

        let response = self.generate(&prompt)?;
        Ok(format!("🪞 Reflection:\n{}", response))
    }
}

/// Demo language model for sequential processing (fallback when Ollama unavailable)
pub struct DemoLanguageModel {
    cycle_count: std::sync::atomic::AtomicU32,
}

impl DemoLanguageModel {
    pub fn new() -> Self {
        Self {
            cycle_count: std::sync::atomic::AtomicU32::new(0),
        }
    }
}

impl LanguageModel for DemoLanguageModel {
    fn think(&self, context: &str) -> Result<String> {
        let count = self.cycle_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(format!(
            "🧠 Sequential Thinking Cycle {}:\n\
             Analyzing context and task requirements...\n\
             Context summary: {}\n\
             Considering previous experiences and patterns...\n\
             Formulating strategic approach...\n\
             Ready for decision phase.",
            count,
            context.chars().take(200).collect::<String>()
        ))
    }

    fn decide(&self, thought: &str) -> Result<String> {
        Ok(format!(
            "🎯 Decision based on analysis:\n\
             {}\n\
             \n\
             ACTION PLAN:\n\
             Action: StoreMemory {{key: \"decision_analysis\", value: \"Sequential thinking applied successfully\"}}\n\
             Action: SearchVector {{query: \"sequential processing patterns\", limit: 5}}\n\
             \n\
             This represents a structured approach to problem-solving using sequential cognitive cycles.",
            thought.lines().take(3).collect::<Vec<_>>().join(" ")
        ))
    }

    fn reflect(&self, goal: &str) -> Result<String> {
        Ok(format!(
            "🪞 Reflection on goal '{}':\n\
             - Sequential thinking process completed successfully\n\
             - Actions were parsed and executed systematically\n\
             - Multi-dimensional context was considered\n\
             - Learning: Cognitive cycles provide structured problem decomposition\n\
             - Outcome: Goal accomplished with methodical approach\n\
             - Future improvement: Enhanced action recognition patterns",
            goal
        ))
    }
}
