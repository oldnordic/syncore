//! APEX 1.8 REFRAG - ChunkCompressionLayer
//!
//! Reuses existing DualEmbeddingService embeddings without re-embedding.
//! Extracts metadata from existing vector store entries.
//!
//! Design:
//! - NO new embedding models
//! - NO re-embedding of existing text
//! - Metadata extraction from Hit structure
//! - Domain-aware retrieval from code_store/general_store

use super::types::{ChunkMetadata, Domain};
use crate::router::SynCoreState;
use crate::vector::{Hit, SearchScope};
use anyhow::Result;

/// Chunk compression layer using existing embeddings
pub struct ChunkCompressionLayer {
    state: SynCoreState,
}

impl ChunkCompressionLayer {
    /// Create new compression layer with state
    pub fn new(state: SynCoreState) -> Result<Self> {
        Ok(Self { state })
    }

    /// Get chunk metadata for a single chunk ID
    ///
    /// Retrieves from existing vector store without re-embedding.
    pub fn get_chunk(&self, chunk_id: i64) -> Result<ChunkMetadata> {
        // Try CODE domain first
        if let Ok(Some(metadata)) = self.try_get_from_code(chunk_id) {
            return Ok(metadata);
        }

        // Try GENERAL domain
        if let Ok(Some(metadata)) = self.try_get_from_general(chunk_id) {
            return Ok(metadata);
        }

        anyhow::bail!("Chunk {} not found in any store", chunk_id)
    }

    /// Get multiple chunks at once
    pub fn get_chunks(&self, chunk_ids: Vec<i64>) -> Result<Vec<ChunkMetadata>> {
        chunk_ids.into_iter().map(|id| self.get_chunk(id)).collect()
    }

    /// Extract metadata from existing vector store (CODE domain)
    fn try_get_from_code(&self, chunk_id: i64) -> Result<Option<ChunkMetadata>> {
        let store = self.state.code_store.lock().unwrap();

        // Search by ID to find the chunk
        // Note: This is a placeholder - actual implementation needs direct ID lookup
        // For now, we'll search and filter by ID
        let results = store.search("", 1000, SearchScope::Global)?;

        for hit in results {
            if hit.id == chunk_id {
                return Ok(Some(self.hit_to_metadata(hit, Domain::Code)?));
            }
        }

        Ok(None)
    }

    /// Extract metadata from existing vector store (GENERAL domain)
    fn try_get_from_general(&self, chunk_id: i64) -> Result<Option<ChunkMetadata>> {
        let store = self.state.general_store.lock().unwrap();

        let results = store.search("", 1000, SearchScope::Global)?;

        for hit in results {
            if hit.id == chunk_id {
                return Ok(Some(self.hit_to_metadata(hit, Domain::General)?));
            }
        }

        Ok(None)
    }

    /// Convert Hit to ChunkMetadata
    fn hit_to_metadata(&self, hit: Hit, domain: Domain) -> Result<ChunkMetadata> {
        let mut metadata = ChunkMetadata::new(hit.id, domain, hit.text.clone());

        // Extract metadata from text (basic parsing)
        metadata.symbols = self.extract_symbols(&hit.text);
        metadata.entity_type = self.infer_entity_type(&hit.text);

        // Note: Embedding is NOT re-computed - it exists in vector store already
        // We don't need to expose it for REFRAG pipeline (scores are derived from it)

        Ok(metadata)
    }

    /// Extract symbols from text (function names, struct names, etc.)
    fn extract_symbols(&self, text: &str) -> Vec<String> {
        let mut symbols = Vec::new();

        // Simple regex-based extraction (can be improved with tree-sitter)
        if let Some(caps) = regex::Regex::new(r"fn\s+(\w+)")
            .ok()
            .and_then(|re| re.captures(text))
        {
            if let Some(name) = caps.get(1) {
                symbols.push(name.as_str().to_string());
            }
        }

        if let Some(caps) = regex::Regex::new(r"struct\s+(\w+)")
            .ok()
            .and_then(|re| re.captures(text))
        {
            if let Some(name) = caps.get(1) {
                symbols.push(name.as_str().to_string());
            }
        }

        symbols
    }

    /// Infer entity type from text
    fn infer_entity_type(&self, text: &str) -> Option<String> {
        if text.contains("fn ") {
            Some("Function".to_string())
        } else if text.contains("struct ") {
            Some("Struct".to_string())
        } else if text.contains("impl ") {
            Some("Impl".to_string())
        } else if text.contains("use ") {
            Some("Import".to_string())
        } else {
            None
        }
    }

    /// Extract metadata for a specific chunk (public API)
    pub fn extract_metadata(&self, chunk_id: i64) -> Result<ChunkMetadata> {
        self.get_chunk(chunk_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TDD: Tests will fail until integration is complete

    #[test]
    #[ignore]
    fn test_compression_basic() {
        // Placeholder test - actual tests in tests/refrag_compression_test.rs
    }
}
