//! TOON Reasoning Engine
//!
//! Provides instruction graph and execution engine for TOON internal reasoning.

use super::ram_cache::MemoryEntry;
use super::toon_error::ToonError;
use super::toon_result::{ToonResult, ToonStepResult};
use super::MemoryService;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

/// TOON instruction types
#[derive(Debug, Clone, PartialEq)]
pub enum ToonInstr {
    /// Load memory entry by ID
    LoadMemory { id: String },

    /// Retrieve memory entries by query
    Retrieve { query: String, k: usize },

    /// Fold context from multiple entries into new summary
    FoldContext { context_ids: Vec<String> },

    /// Emit a pointer token
    EmitPointer { id: String },

    /// No operation
    NoOp,
}

/// Node in the TOON instruction graph
#[derive(Debug, Clone)]
pub struct ToonNode {
    pub id: String,
    pub instr: ToonInstr,
    pub next: Vec<String>, // Deterministically ordered successor IDs
}

/// TOON instruction graph
pub struct ToonGraph {
    nodes: HashMap<String, ToonNode>,
    entry: String,
}

impl ToonGraph {
    /// Create a new graph with specified entry point
    pub fn new(entry: String) -> Self {
        Self {
            nodes: HashMap::new(),
            entry,
        }
    }

    /// Add or update a node in the graph
    pub fn add_node(&mut self, node: ToonNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    /// Get a node by ID
    pub fn get_node(&self, id: &str) -> Option<&ToonNode> {
        self.nodes.get(id)
    }

    /// Get the entry point node ID
    pub fn entry(&self) -> &str {
        &self.entry
    }

    /// Get all node IDs in deterministic order (sorted)
    pub fn node_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.nodes.keys().cloned().collect();
        ids.sort();
        ids
    }

    /// Get reference to nodes HashMap (for iteration)
    pub fn nodes(&self) -> &HashMap<String, ToonNode> {
        &self.nodes
    }
}

/// TOON execution engine
pub struct ToonExecutor {
    graph: ToonGraph,
    memory: Arc<Mutex<MemoryService>>,
    /// Pointer store for dereferencing pointers to actual memory entries
    pointer_store: HashMap<String, MemoryEntry>,
}

impl ToonExecutor {
    /// Create a new executor with graph and memory service
    pub fn new(graph: ToonGraph, memory: Arc<Mutex<MemoryService>>) -> Self {
        Self {
            graph,
            memory,
            pointer_store: HashMap::new(),
        }
    }

    /// Execute the entire graph starting from entry point
    ///
    /// Returns sequence of step results in execution order
    pub fn execute(&mut self) -> Result<Vec<ToonStepResult>, ToonError> {
        let mut results = Vec::new();
        let mut visited = HashSet::new();
        let mut current = self.graph.entry().to_string();

        loop {
            // Check for execution loops
            if visited.contains(&current) {
                return Err(ToonError::ExecutionLoopDetected(current));
            }
            visited.insert(current.clone());

            // Execute current node
            let step_result = self.step(&current)?;
            let next_nodes = self
                .graph
                .get_node(&current)
                .ok_or_else(|| ToonError::NodeNotFound(current.clone()))?
                .next
                .clone();

            results.push(step_result);

            // Determine next node
            if next_nodes.is_empty() {
                // Terminal node - execution complete
                break;
            } else {
                // Take first successor (deterministic order preserved)
                current = next_nodes[0].clone();
            }
        }

        Ok(results)
    }

    /// Execute a single node
    pub fn step(&mut self, node_id: &str) -> Result<ToonStepResult, ToonError> {
        let node = self
            .graph
            .get_node(node_id)
            .ok_or_else(|| ToonError::NodeNotFound(node_id.to_string()))?;

        let result =
            match &node.instr {
                ToonInstr::LoadMemory { id } => {
                    let entry = self
                        .resolve_pointer(id)
                        .ok_or_else(|| ToonError::InvalidPointer(id.clone()))?;
                    ToonResult::Loaded(entry)
                }

                ToonInstr::Retrieve { query, k } => {
                    let memory_lock = self.memory.lock().map_err(|e| {
                        ToonError::Internal(format!("Failed to lock memory: {}", e))
                    })?;

                    // Generate query embedding (simplified - hash-based for determinism)
                    let query_embedding = Self::hash_text_to_embedding(query);
                    let entries = memory_lock.retrieve(&query_embedding, *k);

                    // Store entries in pointer store for later reference
                    for entry in &entries {
                        self.pointer_store.insert(entry.id.clone(), entry.clone());
                    }

                    ToonResult::Retrieved(entries)
                }

                ToonInstr::FoldContext { context_ids } => {
                    // Fold all context entries into a single summary
                    let mut combined_text = String::new();
                    let mut combined_tags = Vec::new();

                    for ctx_id in context_ids {
                        if let Some(entry) = self.pointer_store.get(ctx_id) {
                            combined_text.push_str(&entry.summary);
                            combined_text.push(' ');
                            combined_tags.extend(entry.tags.clone());
                        }
                    }

                    // Deduplicate tags
                    combined_tags.sort();
                    combined_tags.dedup();

                    // Create folded summary (deterministic)
                    let summary = format!("FOLDED[{}]", combined_text.trim());
                    let new_id = format!("FOLD_{}", context_ids.join("_"));

                    // Generate embedding for folded content
                    let embedding = Self::hash_text_to_embedding(&combined_text);

                    let folded_entry = MemoryEntry {
                        id: new_id.clone(),
                        summary,
                        importance: 0.8, // Folded context has high importance
                        tags: combined_tags,
                        embedding,
                    };

                    // Store in memory service
                    let mut memory_lock = self.memory.lock().map_err(|e| {
                        ToonError::Internal(format!("Failed to lock memory: {}", e))
                    })?;

                    memory_lock.store(folded_entry.clone())?;

                    // Add to pointer store
                    self.pointer_store.insert(new_id.clone(), folded_entry);

                    ToonResult::Folded { new_id }
                }

                ToonInstr::EmitPointer { id } => ToonResult::Pointer(id.clone()),

                ToonInstr::NoOp => ToonResult::Completed,
            };

        Ok(ToonStepResult::new(node_id.to_string(), result))
    }

    /// Resolve a pointer to its memory entry
    ///
    /// Does NOT automatically load from memory - only from pointer store
    pub fn resolve_pointer(&self, id: &str) -> Option<MemoryEntry> {
        self.pointer_store.get(id).cloned()
    }

    /// Generate deterministic embedding from text (hash-based)
    fn hash_text_to_embedding(text: &str) -> Vec<f32> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        const DIMENSION: usize = 128;
        let mut embedding = Vec::with_capacity(DIMENSION);

        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let base_hash = hasher.finish();

        for i in 0..DIMENSION {
            let mut h = DefaultHasher::new();
            (base_hash.wrapping_add(i as u64)).hash(&mut h);
            let val = (h.finish() as f32) / (u64::MAX as f32);
            embedding.push(val * 2.0 - 1.0); // Range: [-1, 1]
        }

        // Normalize to unit length
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut embedding {
                *x /= norm;
            }
        }

        embedding
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory_service::MemoryService;

    #[test]
    fn test_toon_graph_basic() {
        let graph = ToonGraph::new("start".to_string());
        assert_eq!(graph.entry(), "start");
    }

    #[test]
    fn test_toon_instr_creation() {
        let instr = ToonInstr::Retrieve {
            query: "test".to_string(),
            k: 5,
        };
        match instr {
            ToonInstr::Retrieve { query, k } => {
                assert_eq!(query, "test");
                assert_eq!(k, 5);
            }
            _ => panic!("Wrong instruction type"),
        }
    }

    #[test]
    fn test_hash_text_to_embedding_deterministic() {
        let emb1 = ToonExecutor::hash_text_to_embedding("test text");
        let emb2 = ToonExecutor::hash_text_to_embedding("test text");
        assert_eq!(emb1, emb2, "Hash should be deterministic");
    }

    #[test]
    fn test_hash_text_to_embedding_different() {
        let emb1 = ToonExecutor::hash_text_to_embedding("text1");
        let emb2 = ToonExecutor::hash_text_to_embedding("text2");
        assert_ne!(
            emb1, emb2,
            "Different texts should have different embeddings"
        );
    }
}
