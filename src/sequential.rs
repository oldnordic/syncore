use crate::memory::Memory;
use crate::taskmaster::TaskMaster;
use crate::cognition::{CogState, CogStep, Task};
use crate::logger::{CogLogger, MarkdownLogger};
use crate::vector::{VectorStore, MockEmbeddings, SearchScope};
use anyhow::Result;

pub struct GlmClient {
    // Mock GLM client for now
}

impl GlmClient {
    pub fn new() -> Self {
        Self {}
    }
    
    pub fn think(&self, context: &str) -> Result<String> {
        // Mock implementation - in real version this would call GLM
        Ok(format!("Thinking about: {}", context))
    }
    
    pub fn decide(&self, thought: &str) -> Result<String> {
        // Mock implementation
        Ok(format!("Decision based on: {}", thought))
    }
    
    pub fn reflect(&self, task: &Task, result: &str) -> Result<String> {
        // Mock implementation - mark task as complete
        Ok(format!("Task {} is complete. Reflection on: {}", task.id, result))
    }
}

pub struct SequentialCore {
    pub memory: Memory,
    taskmaster: TaskMaster,
    model: GlmClient,
    logger: Box<dyn CogLogger>,
    vector_store: VectorStore,
}

impl SequentialCore {
    pub fn new(memory: Memory, taskmaster: TaskMaster) -> Self {
        let logger = Box::new(MarkdownLogger::new("logs"));
        let embeddings = Box::new(MockEmbeddings::new(384));
        let vector_store = VectorStore::new(embeddings);
        
        Self {
            memory,
            taskmaster,
            model: GlmClient::new(),
            logger,
            vector_store,
        }
    }
    
    pub fn with_components(
        memory: Memory, 
        taskmaster: TaskMaster, 
        logger: Box<dyn CogLogger>,
        vector_store: VectorStore
    ) -> Self {
        Self {
            memory,
            taskmaster,
            model: GlmClient::new(),
            logger,
            vector_store,
        }
    }
    
    pub fn store_step(&self, state: CogState, content: &str, task_id: Option<u64>) -> Result<()> {
        let step = CogStep::new(state, content.to_string(), task_id);
        
        // Store in memory
        let step_key = format!("step:{}", step.timestamp);
        let step_json = serde_json::to_string(&step)?;
        self.memory.store(&step_key, &step_json);
        
        Ok(())
    }
    
    pub fn store_and_log_step(&mut self, state: CogState, content: &str, task: &Task) -> Result<()> {
        let step = CogStep::new(state.clone(), content.to_string(), Some(task.id));
        
        // Store in memory
        let step_key = format!("step:{}", step.timestamp);
        let step_json = serde_json::to_string(&step)?;
        self.memory.store(&step_key, &step_json);
        
        // Log to markdown
        self.logger.log_step(&step, task)?;
        
        // Index in vector store for Think and Reflect states (richest content)
        match state {
            CogState::Think | CogState::Reflect => {
                let _point_id = self.vector_store.insert(step.timestamp as u64, task.id, content)?;
            }
            _ => {} // Don't index Act/Decide for now
        }
        
        Ok(())
    }
    
    pub fn recall_context(&self, task: &Task) -> Result<String> {
        // Search vector store for relevant prior steps
        let search_query = format!("{} {}", task.goal, task.id);
        let relevant_steps = self.vector_store.search(&search_query, 5, SearchScope::Task(task.id))
            .unwrap_or_default();
        
        // For now, just return the task goal with context hint
        // In future, this would fetch full step content from memory
        let context = if relevant_steps.is_empty() {
            format!("Task: {} (ID: {})", task.goal, task.id)
        } else {
            format!("Task: {} (ID: {}) with {} relevant prior steps", task.goal, task.id, relevant_steps.len())
        };
        
        Ok(context)
    }
    
    pub fn cycle(&mut self) -> Result<Option<Task>> {
        let task = match self.taskmaster.next_task()? {
            Some(task) => task,
            None => return Ok(None),
        };
        
        // THINK phase
        let context = self.recall_context(&task)?;
        let thought = self.model.think(&context)?;
        self.store_and_log_step(CogState::Think, &thought, &task)?;
        
        // DECIDE phase
        let decision = self.model.decide(&thought)?;
        self.store_and_log_step(CogState::Decide, &decision, &task)?;
        
        // ACT phase (mock action)
        let action_result = "Action executed successfully".to_string();
        self.store_and_log_step(CogState::Act, &action_result, &task)?;
        
        // REFLECT phase
        let reflection = self.model.reflect(&task, &action_result)?;
        self.store_and_log_step(CogState::Reflect, &reflection, &task)?;
        
        // Log summary
        self.logger.log_summary(&task, &reflection)?;
        
        // Update task status based on reflection
        let is_complete = reflection.contains("complete") || reflection.contains("done");
        self.taskmaster.update_task(task.id, is_complete)?;
        
        Ok(Some(task))
    }
    
    pub fn add_task(&self, goal: String, priority: u8) -> Result<u64> {
        self.taskmaster.add_task(goal, priority)
    }
    
    pub fn list_tasks(&self) -> Result<Vec<Task>> {
        self.taskmaster.list_tasks()
    }
    
    pub fn vector_search(&self, query: &str, k: usize, scope: SearchScope) -> Result<Vec<(u64, f32)>> {
        self.vector_store.search(query, k, scope)
    }
    
    pub fn get_logger(&self) -> &Box<dyn CogLogger> {
        &self.logger
    }
}