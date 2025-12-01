//! TOON Prompt Builder
//!
//! Generates deterministic LLM prompts from TOON state (graph + memory + pointer store)

use super::{MemoryEntry, ToonGraph};
use std::collections::HashMap;

pub struct ToonPromptBuilder {
    max_context_tokens: usize,
}

impl ToonPromptBuilder {
    pub fn new(max_context_tokens: usize) -> Self {
        Self {
            max_context_tokens,
        }
    }

    pub fn build_prompt(
        &self,
        graph: &ToonGraph,
        memory: &[MemoryEntry],
        pointer_store: &HashMap<String, MemoryEntry>,
    ) -> String {
        let mut prompt = String::new();

        // Static header
        prompt.push_str("# TOON Reasoning System\n\n");
        prompt.push_str("You are a TOON (Task-Oriented Optimization Network) reasoner.\n");
        prompt.push_str("Analyze the current state and respond with JSON operations.\n\n");

        // Graph structure
        prompt.push_str("## Current Graph\n\n");
        prompt.push_str(&format!("Entry Point: {}\n\n", graph.entry()));

        // Serialize all nodes in deterministic order (sorted by ID)
        let node_ids = graph.node_ids();

        for node_id in node_ids {
            if let Some(node) = graph.get_node(&node_id) {
                prompt.push_str(&format!("Node: {}\n", node.id));
                prompt.push_str(&format!("  Instruction: {:?}\n", node.instr));
                if !node.next.is_empty() {
                    prompt.push_str(&format!("  Next: {:?}\n", node.next));
                }
                prompt.push('\n');
            }
        }

        // Estimate token usage (rough: 4 chars = 1 token)
        let current_tokens = prompt.len() / 4;
        let remaining_tokens = if current_tokens < self.max_context_tokens {
            self.max_context_tokens - current_tokens
        } else {
            0
        };

        // Memory entries (limited by remaining tokens)
        if !memory.is_empty() {
            prompt.push_str("## Memory Entries\n\n");
            let mut added = 0;
            let mut entry_tokens = 0;

            for entry in memory {
                let entry_text = format!(
                    "ID: {}\nSummary: {}\nImportance: {}\nTags: {:?}\n\n",
                    entry.id, entry.summary, entry.importance, entry.tags
                );
                let tokens = entry_text.len() / 4;

                if entry_tokens + tokens > remaining_tokens {
                    break; // Token limit reached
                }

                prompt.push_str(&entry_text);
                entry_tokens += tokens;
                added += 1;
            }

            if added < memory.len() {
                prompt.push_str(&format!(
                    "... ({} more entries truncated)\n\n",
                    memory.len() - added
                ));
            }
        }

        // Pointer store
        if !pointer_store.is_empty() {
            prompt.push_str("## Pointer Store\n\n");

            // Deterministic ordering by ID
            let mut ptr_ids: Vec<_> = pointer_store.keys().cloned().collect();
            ptr_ids.sort();

            for ptr_id in ptr_ids {
                if let Some(entry) = pointer_store.get(&ptr_id) {
                    prompt
                        .push_str(&format!("Pointer: {}\nSummary: {}\n\n", ptr_id, entry.summary));
                }
            }
        }

        // Instructions for response format
        prompt.push_str("## Response Format\n\n");
        prompt.push_str("Respond with JSON in this format:\n");
        prompt.push_str("{\n");
        prompt.push_str("  \"ops\": [\n");
        prompt.push_str("    {\"type\": \"load_memory\", \"id\": \"entry_id\"},\n");
        prompt.push_str("    {\"type\": \"retrieve\", \"query\": \"search text\", \"k\": 5},\n");
        prompt.push_str("    {\"type\": \"fold_context\", \"context_ids\": [\"id1\", \"id2\"]},\n");
        prompt.push_str("    {\"type\": \"emit_pointer\", \"id\": \"ptr_id\"},\n");
        prompt.push_str("    {\"type\": \"noop\"}\n");
        prompt.push_str("  ]\n");
        prompt.push_str("}\n");

        prompt
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory_service::{ToonInstr, ToonNode};

    #[test]
    fn test_builder_deterministic() {
        let builder = ToonPromptBuilder::new(5000);

        let mut graph = ToonGraph::new("start".to_string());
        graph.add_node(ToonNode {
            id: "start".to_string(),
            instr: ToonInstr::NoOp,
            next: vec![],
        });

        let memory = vec![MemoryEntry {
            id: "m1".to_string(),
            summary: "Test 1".to_string(),
            importance: 0.5,
            tags: vec![],
            embedding: vec![0.1; 128],
        }];

        let mut pointer_store = HashMap::new();
        pointer_store.insert(
            "p1".to_string(),
            MemoryEntry {
                id: "p1".to_string(),
                summary: "Pointer 1".to_string(),
                importance: 0.5,
                tags: vec![],
                embedding: vec![0.2; 128],
            },
        );

        let prompt1 = builder.build_prompt(&graph, &memory, &pointer_store);
        let prompt2 = builder.build_prompt(&graph, &memory, &pointer_store);
        let prompt3 = builder.build_prompt(&graph, &memory, &pointer_store);

        assert_eq!(prompt1, prompt2);
        assert_eq!(prompt2, prompt3);
    }

    #[test]
    fn test_builder_includes_all_instruction_types() {
        let builder = ToonPromptBuilder::new(10000);

        let mut graph = ToonGraph::new("n1".to_string());
        graph.add_node(ToonNode {
            id: "n1".to_string(),
            instr: ToonInstr::LoadMemory {
                id: "M1".to_string(),
            },
            next: vec!["n2".to_string()],
        });
        graph.add_node(ToonNode {
            id: "n2".to_string(),
            instr: ToonInstr::Retrieve {
                query: "q".to_string(),
                k: 3,
            },
            next: vec!["n3".to_string()],
        });
        graph.add_node(ToonNode {
            id: "n3".to_string(),
            instr: ToonInstr::FoldContext {
                context_ids: vec!["c1".to_string()],
            },
            next: vec!["n4".to_string()],
        });
        graph.add_node(ToonNode {
            id: "n4".to_string(),
            instr: ToonInstr::EmitPointer {
                id: "P1".to_string(),
            },
            next: vec!["n5".to_string()],
        });
        graph.add_node(ToonNode {
            id: "n5".to_string(),
            instr: ToonInstr::NoOp,
            next: vec![],
        });

        let prompt = builder.build_prompt(&graph, &[], &HashMap::new());

        // All node IDs should be present
        assert!(prompt.contains("n1"));
        assert!(prompt.contains("n2"));
        assert!(prompt.contains("n3"));
        assert!(prompt.contains("n4"));
        assert!(prompt.contains("n5"));

        // All instruction types should be represented
        assert!(prompt.contains("LoadMemory"));
        assert!(prompt.contains("Retrieve"));
        assert!(prompt.contains("FoldContext"));
        assert!(prompt.contains("EmitPointer"));
        assert!(prompt.contains("NoOp"));
    }
}
