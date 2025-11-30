//! TOON Controller
//!
//! Orchestrates TOON reasoning: prompt building, LLM interaction, execution

use super::{
    MemoryEntry, MemoryService, ToonDecoder, ToonExecutor, ToonGraph, ToonInstr, ToonNode,
    ToonPromptBuilder, ToonResult, ToonStepResult,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct ToonController {
    graph: ToonGraph,
    memory: Arc<Mutex<MemoryService>>,
    prompt_builder: ToonPromptBuilder,
    decoder: ToonDecoder,
    pointer_store: HashMap<String, MemoryEntry>,
}

impl ToonController {
    /// Create a new TOON controller
    ///
    /// # Arguments
    /// * `graph` - Initial TOON graph
    /// * `memory` - Memory service (shared with executor)
    /// * `max_context_tokens` - Maximum tokens for prompts
    pub fn new(
        graph: ToonGraph,
        memory: Arc<Mutex<MemoryService>>,
        max_context_tokens: usize,
    ) -> Self {
        Self {
            graph,
            memory,
            prompt_builder: ToonPromptBuilder::new(max_context_tokens),
            decoder: ToonDecoder::new(),
            pointer_store: HashMap::new(),
        }
    }

    /// Build LLM prompt from current state
    pub fn build_llm_prompt(&self) -> String {
        // Get all memory entries from RAM cache
        let memory_entries = {
            let mem_lock = self.memory.lock().unwrap();
            // Query with a broad embedding to get all entries
            let zero_embedding = vec![0.0; 128];
            mem_lock.retrieve(&zero_embedding, 1000) // Get many entries
        };

        self.prompt_builder
            .build_prompt(&self.graph, &memory_entries, &self.pointer_store)
    }

    /// Process LLM output: decode, execute, and update graph
    ///
    /// Returns the step results from executing the new operations
    pub fn step_llm(&mut self, llm_output: &str) -> Result<Vec<ToonStepResult>, String> {
        // Decode LLM output to instructions
        let instructions = self.decoder.decode_ops(llm_output)?;

        // Append instructions as new nodes to graph
        let base_node_idx = self.graph.node_ids().len();
        let mut node_ids = Vec::new();

        for (idx, _instr) in instructions.iter().enumerate() {
            let node_id = format!("llm_node_{}", base_node_idx + idx);
            node_ids.push(node_id.clone());
        }

        // Create all nodes with proper next pointers
        for (idx, (node_id, instr)) in node_ids.iter().zip(instructions.iter()).enumerate() {
            let next = if idx < instructions.len() - 1 {
                vec![node_ids[idx + 1].clone()]
            } else {
                vec![] // Terminal node
            };

            let node = ToonNode {
                id: node_id.clone(),
                instr: instr.clone(),
                next,
            };

            self.graph.add_node(node);
        }

        // If no nodes added, return empty results
        if node_ids.is_empty() {
            return Ok(vec![]);
        }

        // Create executor with updated graph
        // Note: Need to execute just the new nodes, not entire graph
        // Create a subgraph with only new nodes
        let mut subgraph = ToonGraph::new(node_ids[0].clone());
        for node_id in &node_ids {
            if let Some(node) = self.graph.get_node(node_id) {
                subgraph.add_node(node.clone());
            }
        }

        // Execute subgraph
        let mut executor = ToonExecutor::new(subgraph, Arc::clone(&self.memory));
        let results = executor
            .execute()
            .map_err(|e| format!("Execution failed: {:?}", e))?;

        // Update pointer store from executor
        for result in &results {
            match &result.result {
                ToonResult::Retrieved(entries) => {
                    for entry in entries {
                        self.pointer_store.insert(entry.id.clone(), entry.clone());
                    }
                }
                ToonResult::Loaded(entry) => {
                    self.pointer_store.insert(entry.id.clone(), entry.clone());
                }
                ToonResult::Folded { new_id } => {
                    // Load folded entry from memory
                    let mem_lock = self.memory.lock().unwrap();
                    let zero_embedding = vec![0.0; 128];
                    let entries = mem_lock.retrieve(&zero_embedding, 1000);
                    if let Some(entry) = entries.iter().find(|e| e.id == *new_id) {
                        self.pointer_store.insert(new_id.clone(), entry.clone());
                    }
                }
                _ => {}
            }
        }

        Ok(results)
    }

    /// Check if automatic folding is required and perform if needed
    ///
    /// Returns Some(folded_id) if folding occurred, None otherwise
    pub fn fold_if_required(&mut self) -> Result<Option<String>, String> {
        // Check if memory is over capacity
        let stats = {
            let mem_lock = self.memory.lock().unwrap();
            mem_lock.stats()
        };

        // If RAM cache is over capacity, trigger folding
        if stats.ram_size > stats.ram_capacity {
            // Get all entries from RAM
            let entries = {
                let mem_lock = self.memory.lock().unwrap();
                let zero_embedding = vec![0.0; 128];
                mem_lock.retrieve(&zero_embedding, 1000)
            };

            // Take entries to fold (oldest/least important)
            let to_fold: Vec<String> = entries.iter().take(5).map(|e| e.id.clone()).collect();

            if !to_fold.is_empty() {
                // Create FoldContext instruction
                let fold_instr = ToonInstr::FoldContext {
                    context_ids: to_fold,
                };

                // Execute fold operation
                let fold_node_id = format!("auto_fold_{}", self.graph.node_ids().len());
                let fold_node = ToonNode {
                    id: fold_node_id.clone(),
                    instr: fold_instr,
                    next: vec![],
                };

                // Create subgraph with just fold node
                let mut fold_graph = ToonGraph::new(fold_node_id.clone());
                fold_graph.add_node(fold_node.clone());

                // Add to main graph
                self.graph.add_node(fold_node);

                // Execute fold
                let mut executor = ToonExecutor::new(fold_graph, Arc::clone(&self.memory));
                let results = executor
                    .execute()
                    .map_err(|e| format!("Fold execution failed: {:?}", e))?;

                // Extract folded ID
                for result in results {
                    if let ToonResult::Folded { new_id } = result.result {
                        // Update pointer store
                        let mem_lock = self.memory.lock().unwrap();
                        let zero_embedding = vec![0.0; 128];
                        let entries = mem_lock.retrieve(&zero_embedding, 1000);
                        if let Some(entry) = entries.iter().find(|e| e.id == new_id) {
                            self.pointer_store.insert(new_id.clone(), entry.clone());
                        }

                        return Ok(Some(new_id));
                    }
                }
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_memory_service() -> Arc<Mutex<MemoryService>> {
        Arc::new(Mutex::new(MemoryService::new(128, 10)))
    }

    #[test]
    fn test_controller_builds_prompt() {
        let memory = create_test_memory_service();

        let mut graph = ToonGraph::new("start".to_string());
        graph.add_node(ToonNode {
            id: "start".to_string(),
            instr: ToonInstr::NoOp,
            next: vec![],
        });

        let controller = ToonController::new(graph, memory, 5000);
        let prompt = controller.build_llm_prompt();

        assert!(!prompt.is_empty());
        assert!(prompt.contains("start"));
    }

    #[test]
    fn test_controller_decodes_and_executes() {
        let memory = create_test_memory_service();

        // Pre-populate memory
        {
            let mut mem_lock = memory.lock().unwrap();
            mem_lock
                .store(MemoryEntry {
                    id: "test_entry".to_string(),
                    summary: "Test data".to_string(),
                    importance: 0.5,
                    tags: vec![],
                    embedding: vec![0.5; 128],
                })
                .unwrap();
        }

        let graph = ToonGraph::new("start".to_string());
        let mut controller = ToonController::new(graph, Arc::clone(&memory), 5000);

        let llm_output = r#"{
            "ops": [
                {"type": "retrieve", "query": "test", "k": 1}
            ]
        }"#;

        let result = controller.step_llm(llm_output);
        assert!(result.is_ok());

        let steps = result.unwrap();
        assert!(steps.len() > 0);
    }

    #[test]
    fn test_controller_returns_step_results() {
        let memory = create_test_memory_service();

        let graph = ToonGraph::new("start".to_string());
        let mut controller = ToonController::new(graph, memory, 5000);

        let llm_output = r#"{
            "ops": [
                {"type": "emit_pointer", "id": "PTR1"}
            ]
        }"#;

        let results = controller.step_llm(llm_output).unwrap();

        assert_eq!(results.len(), 1);

        if let ToonResult::Pointer(id) = &results[0].result {
            assert_eq!(id, "PTR1");
        } else {
            panic!("Expected Pointer result");
        }
    }
}
