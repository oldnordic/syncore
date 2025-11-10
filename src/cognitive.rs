use crate::vector::{VectorStore, SearchScope};
use crate::logger::MarkdownLogger;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveStep {
    pub step_id: u64,
    pub task_id: u64,
    pub step_type: String, // "think", "reflect", "observe"
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

// Simple nudge tracking
static NUDGE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct CognitiveEnhancer {
    vector_store: Arc<VectorStore>,
    logger: Arc<MarkdownLogger>,
}

impl CognitiveEnhancer {
    pub fn new(vector_store: Arc<VectorStore>, logger: Arc<MarkdownLogger>) -> Self {
        Self {
            vector_store,
            logger,
        }
    }
    
    // Context stitcher: hybrid recall combining semantic + temporal
    pub fn get_context_for_task(&self, task_id: u64, k: usize) -> anyhow::Result<Vec<CognitiveStep>> {
        // Get semantic matches for this task
        let semantic_hits = self.vector_store.search(&format!("task {}", task_id), k, SearchScope::Task(task_id))?;
        
        // Get recent chronological steps (mock implementation)
        let recent_steps = self.get_recent_steps_for_task(task_id, k)?;
        
        // Combine and deduplicate
        let mut context = Vec::new();
        let mut seen_step_ids = std::collections::HashSet::new();
        
        // Add semantic hits first
        for (step_id, _score) in semantic_hits {
            if seen_step_ids.insert(step_id) {
                if let Some(step) = self.get_step_by_id(step_id)? {
                    context.push(step);
                }
            }
        }
        
        // Add recent chronological steps
        for step in recent_steps {
            if seen_step_ids.insert(step.step_id) {
                context.push(step);
            }
        }
        
        Ok(context)
    }
    
    // Reflect guard: check consistency and suggest completion
    pub fn validate_reflection(&self, reflect_content: &str, observe_content: Option<&str>) -> Option<String> {
        let reflect_lower = reflect_content.to_lowercase();
        
        // Check if reflection mentions completion but observe shows success
        if let Some(observe) = observe_content {
            let observe_lower = observe.to_lowercase();
            
            if (observe_lower.contains("success") || observe_lower.contains("complete") || observe_lower.contains("done")) 
                && !reflect_lower.contains("complete") && !reflect_lower.contains("finish") {
                return Some(" (suggested: mark complete)".to_string());
            }
            
            // Check for inconsistent emotional tone
            if reflect_lower.contains("error") || reflect_lower.contains("failed") {
                if observe_lower.contains("success") {
                    return Some(" (note: reflection negative despite success)".to_string());
                }
            }
        }
        
        None
    }
    
    // Generate soft tool nudge based on confidence
    pub fn generate_nudge(&self, state_text: &str, tool_used: &str) -> Option<String> {
        let mut confidence: f32 = 0.5; // Base confidence
        
        // Boost confidence for complex states
        if state_text.len() > 100 {
            confidence += 0.1;
        }
        
        // Boost confidence for question-like states
        if state_text.contains("?") || state_text.contains("how") || state_text.contains("what") {
            confidence += 0.15;
        }
        
        // Adjust based on tool appropriateness
        match tool_used {
            "memory.store" if state_text.contains("remember") => confidence += 0.1,
            "vector.search" if state_text.contains("find") => confidence += 0.1,
            "task.create" if state_text.contains("goal") => confidence += 0.1,
            _ => {}
        }
        
        let final_confidence = (confidence).min(1.0);
        
        // Generate nudge based on confidence
        if final_confidence > 0.8 {
            Some(" (high confidence: proceed)".to_string())
        } else if final_confidence < 0.4 {
            Some(" (low confidence: consider alternatives)".to_string())
        } else {
            None
        }
    }
    
    // Mock implementations (would integrate with actual storage)
    fn get_recent_steps_for_task(&self, _task_id: u64, _k: usize) -> anyhow::Result<Vec<CognitiveStep>> {
        // Mock recent steps
        Ok(vec![])
    }
    
    fn get_step_by_id(&self, _step_id: u64) -> anyhow::Result<Option<CognitiveStep>> {
        // Mock step retrieval
        Ok(None)
    }
}

// Public metrics
pub fn get_nudge_count() -> u64 {
    NUDGE_COUNTER.load(Ordering::Relaxed)
}