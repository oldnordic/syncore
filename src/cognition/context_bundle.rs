//! Context Bundle Module
//!
//! Defines the unified ContextBundle structure that fuses:
//! - RAGGraph ranked entities
//! - LTMC vector memory hits
//! - LTMC SQL memory records
//! - LTMC graph memory relationships
//! - LTMC cache entries
//! - Fusion metadata and reasoning traces

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Code entity with score from RAGGraph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeEntityWithScore {
    pub entity_id: Option<i64>,
    pub file_path: String,
    pub entity_type: String,
    pub name: String,
    pub signature: Option<String>,
    pub score: f32,
    pub rank: usize,
}

/// LTMC vector memory hit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LtmcVectorHit {
    pub id: String,
    pub content: String,
    pub similarity: f32,
    pub metadata: Option<Value>,
}

/// LTMC SQL memory record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LtmcSqlRecord {
    pub key: String,
    pub value: String,
    pub timestamp: Option<i64>,
    pub relevance: f32,
}

/// LTMC graph memory relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LtmcGraphRelation {
    pub source_id: String,
    pub target_id: String,
    pub relation_type: String,
    pub properties: Option<Value>,
}

/// LTMC cache entry (recent operations/actions)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LtmcCacheEntry {
    pub key: String,
    pub value: String,
    pub timestamp: Option<i64>,
}

/// Unified Context Bundle
///
/// Fuses all memory systems into a single structured context
/// for the worker model. Provides:
/// - Deduplication
/// - Priority scoring
/// - Token budget control
/// - Full traceability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBundle {
    /// RAGGraph ranked entities
    pub raggraph_entities: Vec<CodeEntityWithScore>,

    /// LTMC vector memory hits
    pub memory_vectors: Vec<LtmcVectorHit>,

    /// LTMC SQL memory records
    pub memory_sql: Vec<LtmcSqlRecord>,

    /// LTMC graph memory relationships
    pub memory_graph: Vec<LtmcGraphRelation>,

    /// Recent cache entries
    pub recent_cache_entries: Vec<LtmcCacheEntry>,

    /// Selected fusion mode
    pub fusion_mode: String,

    /// Fusion debug information
    pub fusion_debug: Option<Value>,

    /// Reasoning trace
    pub reasoning_trace: Option<Value>,

    /// Total entities across all sources
    pub total_entities: usize,

    /// Deduplication count
    pub deduplicated_count: usize,
}

impl ContextBundle {
    /// Create a new empty ContextBundle
    pub fn new() -> Self {
        Self {
            raggraph_entities: Vec::new(),
            memory_vectors: Vec::new(),
            memory_sql: Vec::new(),
            memory_graph: Vec::new(),
            recent_cache_entries: Vec::new(),
            fusion_mode: String::new(),
            fusion_debug: None,
            reasoning_trace: None,
            total_entities: 0,
            deduplicated_count: 0,
        }
    }

    /// Create a ContextBundle with fusion mode set
    pub fn with_mode(mode: &str) -> Self {
        Self {
            raggraph_entities: Vec::new(),
            memory_vectors: Vec::new(),
            memory_sql: Vec::new(),
            memory_graph: Vec::new(),
            recent_cache_entries: Vec::new(),
            fusion_mode: mode.to_string(),
            fusion_debug: None,
            reasoning_trace: None,
            total_entities: 0,
            deduplicated_count: 0,
        }
    }

    /// Add RAGGraph entity
    pub fn add_raggraph_entity(&mut self, entity: CodeEntityWithScore) {
        self.raggraph_entities.push(entity);
        self.total_entities += 1;
    }

    /// Add vector memory hit
    pub fn add_vector_hit(&mut self, hit: LtmcVectorHit) {
        self.memory_vectors.push(hit);
        self.total_entities += 1;
    }

    /// Add SQL memory record
    pub fn add_sql_record(&mut self, record: LtmcSqlRecord) {
        self.memory_sql.push(record);
        self.total_entities += 1;
    }

    /// Add graph relationship
    pub fn add_graph_relation(&mut self, relation: LtmcGraphRelation) {
        self.memory_graph.push(relation);
        self.total_entities += 1;
    }

    /// Add cache entry
    pub fn add_cache_entry(&mut self, entry: LtmcCacheEntry) {
        self.recent_cache_entries.push(entry);
    }

    /// Set fusion debug info
    pub fn set_fusion_debug(&mut self, debug: Value) {
        self.fusion_debug = Some(debug);
    }

    /// Set reasoning trace
    pub fn set_reasoning_trace(&mut self, trace: Value) {
        self.reasoning_trace = Some(trace);
    }

    /// Mark deduplication
    pub fn mark_deduplicated(&mut self, count: usize) {
        self.deduplicated_count = count;
    }

    /// Get total memory across all systems
    pub fn total_memory_entries(&self) -> usize {
        self.raggraph_entities.len()
            + self.memory_vectors.len()
            + self.memory_sql.len()
            + self.memory_graph.len()
            + self.recent_cache_entries.len()
    }

    /// Format as human-readable summary
    pub fn summary(&self) -> String {
        format!(
            "ContextBundle [mode={}]: {} RAGGraph, {} vectors, {} SQL, {} graph, {} cache (total: {}, deduped: {})",
            self.fusion_mode,
            self.raggraph_entities.len(),
            self.memory_vectors.len(),
            self.memory_sql.len(),
            self.memory_graph.len(),
            self.recent_cache_entries.len(),
            self.total_entities,
            self.deduplicated_count
        )
    }
}

impl Default for ContextBundle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_bundle_creation() {
        let bundle = ContextBundle::new();
        assert!(bundle.raggraph_entities.is_empty());
        assert!(bundle.memory_vectors.is_empty());
        assert_eq!(bundle.total_entities, 0);
    }

    #[test]
    fn test_context_bundle_with_mode() {
        let bundle = ContextBundle::with_mode("attention");
        assert_eq!(bundle.fusion_mode, "attention");
    }

    #[test]
    fn test_add_entities() {
        let mut bundle = ContextBundle::new();

        bundle.add_raggraph_entity(CodeEntityWithScore {
            entity_id: Some(1),
            file_path: "test.rs".to_string(),
            entity_type: "function".to_string(),
            name: "test_fn".to_string(),
            signature: None,
            score: 0.9,
            rank: 1,
        });

        assert_eq!(bundle.raggraph_entities.len(), 1);
        assert_eq!(bundle.total_entities, 1);
    }

    #[test]
    fn test_json_serialization() {
        let bundle = ContextBundle::with_mode("simple");
        let json = serde_json::to_string(&bundle).unwrap();
        assert!(json.contains("simple"));
        assert!(json.contains("raggraph_entities"));
    }
}
