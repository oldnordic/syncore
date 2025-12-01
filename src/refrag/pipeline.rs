//! APEX 1.8 REFRAG - Pipeline Coordinator
//!
//! Orchestrates the full REFRAG selective-expansion pipeline:
//! 1. Retrieve candidates from VectorStore (via ChunkCompressionLayer)
//! 2. Select top-k chunks deterministically (via PerceiveSelector)
//! 3. Expand selected as RAW, rejected as COMPRESSED (via ExpandStage)
//! 4. Assemble hybrid prompt (via HybridPromptBuilder)
//!
//! Design:
//! - Single entry point: `query()` method
//! - Configurable via RefragConfig
//! - Returns RefragResult with prompt + metadata
//! - Token limit enforcement throughout pipeline

use super::{
    ChunkCompressionLayer, ChunkMetadata, ExpandStage, HybridPromptBuilder, PerceiveSelector,
    RefragConfig,
};
use crate::router::SynCoreState;
use crate::vector::SearchScope;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// RefragPipeline result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefragResult {
    /// Final assembled prompt
    pub prompt: String,
    /// Number of RAW chunks included
    pub raw_count: usize,
    /// Number of COMPRESSED chunks included
    pub compressed_count: usize,
    /// Total token count
    pub total_tokens: usize,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// REFRAG pipeline coordinator
pub struct RefragPipeline {
    state: SynCoreState,
    config: RefragConfig,
}

impl RefragPipeline {
    /// Create new pipeline with configuration
    pub fn new(state: SynCoreState, config: RefragConfig) -> Self {
        Self {
            state,
            config,
        }
    }

    /// Create pipeline with default configuration
    pub fn with_defaults(state: SynCoreState) -> Self {
        Self {
            state,
            config: RefragConfig::default(),
        }
    }

    /// Execute full REFRAG pipeline
    pub fn query(&self, query: &str) -> Result<RefragResult> {
        // Step 1: Retrieve candidates from VectorStore
        let candidates = self.retrieve_candidates(query)?;

        if candidates.is_empty() {
            return Ok(RefragResult {
                prompt: format!("## QUERY CONTEXT\n\n{}\n\nNo relevant chunks found.", query),
                raw_count: 0,
                compressed_count: 0,
                total_tokens: self.estimate_tokens(query),
                metadata: HashMap::new(),
            });
        }

        // Step 2: Select top-k chunks deterministically
        let selector = PerceiveSelector::new(self.config.selection_policy.clone());
        let selection_result = selector.select_chunks(query, candidates)?;

        // Step 3: Expand selected as RAW, rejected as COMPRESSED
        let expand_stage = ExpandStage::with_limit(self.state.clone(), self.config.max_tokens);

        let selected_ids: Vec<i64> = selection_result.selected.iter().map(|c| c.chunk_id).collect();
        let rejected_ids: Vec<i64> = selection_result.rejected.iter().map(|c| c.chunk_id).collect();

        let raw_chunks = expand_stage.expand_selected(&selected_ids)?;
        let compressed_chunks = expand_stage.expand_rejected(&rejected_ids)?;

        // Step 4: Assemble hybrid prompt
        let builder = HybridPromptBuilder::with_limit(self.config.max_tokens)
            .with_raw_chunks(raw_chunks.clone())
            .with_compressed_chunks(compressed_chunks.clone())
            .with_query(query)
            .auto_shrink();

        let prompt = builder.build()?;
        let total_tokens = self.calculate_total_tokens(&raw_chunks, &compressed_chunks, query);

        // Metadata
        let mut metadata = HashMap::new();
        metadata.insert(
            "selection_policy".to_string(),
            serde_json::to_value(&self.config.selection_policy)?,
        );
        metadata.insert("top_k_raw".to_string(), serde_json::to_value(self.config.top_k_raw)?);
        metadata.insert("max_tokens".to_string(), serde_json::to_value(self.config.max_tokens)?);
        metadata.insert(
            "candidates_retrieved".to_string(),
            serde_json::to_value(
                selection_result.selected.len() + selection_result.rejected.len(),
            )?,
        );

        Ok(RefragResult {
            prompt,
            raw_count: raw_chunks.len(),
            compressed_count: compressed_chunks.len(),
            total_tokens,
            metadata,
        })
    }

    /// Retrieve candidate chunks from VectorStore
    fn retrieve_candidates(&self, query: &str) -> Result<Vec<ChunkMetadata>> {
        let compression = ChunkCompressionLayer::new(self.state.clone())?;

        // Search both CODE and GENERAL stores
        let code_hits = {
            let store = self.state.code_store.lock().unwrap();
            store.search(query, 50, SearchScope::Global)?
        };

        let general_hits = {
            let store = self.state.general_store.lock().unwrap();
            store.search(query, 50, SearchScope::Global)?
        };

        // Convert hits to ChunkMetadata
        let mut candidates = Vec::new();

        for hit in code_hits {
            if let Ok(chunk) = compression.get_chunk(hit.id) {
                candidates.push(chunk);
            }
        }

        for hit in general_hits {
            if let Ok(chunk) = compression.get_chunk(hit.id) {
                candidates.push(chunk);
            }
        }

        Ok(candidates)
    }

    /// Calculate total token count
    fn calculate_total_tokens(
        &self,
        raw_chunks: &[super::ExpandedChunk],
        compressed_chunks: &[super::ExpandedChunk],
        query: &str,
    ) -> usize {
        let raw_tokens: usize = raw_chunks.iter().map(|c| c.token_count).sum();
        let compressed_tokens: usize = compressed_chunks.iter().map(|c| c.token_count).sum();
        let query_tokens = self.estimate_tokens(query);

        raw_tokens + compressed_tokens + query_tokens
    }

    /// Estimate token count (rough approximation)
    fn estimate_tokens(&self, text: &str) -> usize {
        let words = text.split_whitespace().count();
        ((words as f32) / 0.75).ceil() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector::{HuggingFaceEmbeddings, VectorStore};
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    fn create_test_state() -> Result<SynCoreState> {
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path().to_str().unwrap();

        std::env::set_var("DB_PATH", format!("{}/pipeline_test.db", temp_path));
        std::env::set_var("CODE_GRAPH_DB", format!("{}/code_graph_pipeline.db", temp_path));

        let code_embeddings = Box::new(HuggingFaceEmbeddings::new()?);
        let mut code_store = VectorStore::new(code_embeddings);
        code_store.set_index_path(format!("{}/pipeline_code.index", temp_path));
        let code_store = Arc::new(Mutex::new(code_store));

        let general_embeddings = Box::new(HuggingFaceEmbeddings::new()?);
        let mut general_store = VectorStore::new(general_embeddings);
        general_store.set_index_path(format!("{}/pipeline_general.index", temp_path));
        let general_store = Arc::new(Mutex::new(general_store));

        SynCoreState::with_dual_stores(code_store, general_store)
    }

    #[test]
    fn test_pipeline_creation() -> Result<()> {
        let state = create_test_state()?;
        let pipeline = RefragPipeline::with_defaults(state);
        assert_eq!(pipeline.config.top_k_raw, 5);
        Ok(())
    }

    #[test]
    fn test_pipeline_empty_results() -> Result<()> {
        let state = create_test_state()?;
        let pipeline = RefragPipeline::with_defaults(state);
        let result = pipeline.query("nonexistent query")?;
        assert_eq!(result.raw_count, 0);
        assert_eq!(result.compressed_count, 0);
        Ok(())
    }
}
