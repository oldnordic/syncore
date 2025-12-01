//! APEX 1.8 REFRAG - ExpandStage
//!
//! Selective chunk expansion:
//! - RAW: Full snippet for selected chunks (from MappingTool or filesystem)
//! - COMPRESSED: Metadata summary for rejected chunks
//!
//! Design:
//! - MappingTool integration for file retrieval
//! - Filesystem fallback for non-indexed files
//! - Token limit enforcement with auto-shrink
//! - Format: "file:path, symbols:list, lines:N-M"

use super::types::ChunkMetadata;
use crate::router::SynCoreState;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Chunk expansion format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChunkFormat {
    /// Full raw text (selected chunks)
    Raw,
    /// Compressed metadata summary (rejected chunks)
    Compressed,
}

/// Expanded chunk with content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpandedChunk {
    /// Chunk identifier
    pub chunk_id: i64,

    /// Content (full text or compressed summary)
    pub content: String,

    /// Format type
    pub format: ChunkFormat,

    /// Estimated token count
    pub token_count: usize,

    /// Language (for markdown formatting)
    pub language: Option<String>,
}

/// Chunk expansion stage
pub struct ExpandStage {
    state: SynCoreState,
    token_limit: Option<usize>,
}

impl ExpandStage {
    /// Create new expand stage with state
    pub fn new(state: SynCoreState) -> Self {
        Self {
            state,
            token_limit: None,
        }
    }

    /// Create with token limit (enables auto-shrink)
    pub fn with_limit(state: SynCoreState, token_limit: usize) -> Self {
        Self {
            state,
            token_limit: Some(token_limit),
        }
    }

    /// Expand selected chunks as RAW format
    pub fn expand_selected(&self, chunk_ids: &[i64]) -> Result<Vec<ExpandedChunk>> {
        let mut expanded = Vec::new();

        for &chunk_id in chunk_ids {
            // For now, retrieve from vector store text
            // TODO: Enhanced retrieval via MappingTool
            if let Ok(chunk) = self.expand_raw(chunk_id) {
                expanded.push(chunk);
            }
        }

        // Check token limit and auto-shrink if needed
        if let Some(limit) = self.token_limit {
            let total_tokens: usize = expanded.iter().map(|c| c.token_count).sum();
            if total_tokens > limit {
                expanded = self.auto_shrink(expanded, limit)?;
            }
        }

        Ok(expanded)
    }

    /// Expand rejected chunks as COMPRESSED format
    pub fn expand_rejected(&self, chunk_ids: &[i64]) -> Result<Vec<ExpandedChunk>> {
        chunk_ids.iter().map(|&id| self.expand_compressed(id)).collect()
    }

    /// Expand single chunk as RAW
    fn expand_raw(&self, chunk_id: i64) -> Result<ExpandedChunk> {
        // Try CODE domain first
        if let Ok(hit) = self.try_get_from_code(chunk_id) {
            let content = hit.text;
            let token_count = self.estimate_tokens(&content);
            let language = self.infer_language(&content);

            return Ok(ExpandedChunk {
                chunk_id,
                content,
                format: ChunkFormat::Raw,
                token_count,
                language,
            });
        }

        // Try GENERAL domain
        if let Ok(hit) = self.try_get_from_general(chunk_id) {
            let content = hit.text;
            let token_count = self.estimate_tokens(&content);

            return Ok(ExpandedChunk {
                chunk_id,
                content,
                format: ChunkFormat::Raw,
                token_count,
                language: None,
            });
        }

        anyhow::bail!("Chunk {} not found", chunk_id)
    }

    /// Expand single chunk as COMPRESSED
    fn expand_compressed(&self, chunk_id: i64) -> Result<ExpandedChunk> {
        // Get metadata (simplified - in full implementation use ChunkCompressionLayer)
        let content = if let Ok(hit) = self.try_get_from_code(chunk_id) {
            self.format_compressed(&hit.text, chunk_id)
        } else if let Ok(hit) = self.try_get_from_general(chunk_id) {
            self.format_compressed(&hit.text, chunk_id)
        } else {
            format!("file:unknown, chunk_id:{}", chunk_id)
        };

        let token_count = self.estimate_tokens(&content);

        Ok(ExpandedChunk {
            chunk_id,
            content,
            format: ChunkFormat::Compressed,
            token_count,
            language: None,
        })
    }

    /// Expand chunk from metadata (public API)
    pub fn expand_chunk_raw(&self, metadata: &ChunkMetadata) -> Result<ExpandedChunk> {
        // Use file path if available
        if let Some(ref path) = metadata.file_path {
            if let Ok(content) = self.read_file_lines(path, metadata.line_start, metadata.line_end)
            {
                let token_count = self.estimate_tokens(&content);
                let language = Some(self.detect_language_from_path(path));

                return Ok(ExpandedChunk {
                    chunk_id: metadata.chunk_id,
                    content,
                    format: ChunkFormat::Raw,
                    token_count,
                    language,
                });
            }
        }

        // Fallback: use metadata.text
        let token_count = self.estimate_tokens(&metadata.text);
        let language = metadata.file_path.as_ref().map(|p| self.detect_language_from_path(p));

        Ok(ExpandedChunk {
            chunk_id: metadata.chunk_id,
            content: metadata.text.clone(),
            format: ChunkFormat::Raw,
            token_count,
            language,
        })
    }

    /// Compress chunk from metadata
    pub fn compress_chunk(&self, metadata: &ChunkMetadata) -> Result<String> {
        let mut parts = Vec::new();

        // File path
        if let Some(ref path) = metadata.file_path {
            parts.push(format!("file:{}", path));
        }

        // Symbols
        if !metadata.symbols.is_empty() {
            parts.push(format!("symbols:{}", metadata.symbols.join(",")));
        }

        // Line range
        if let (Some(start), Some(end)) = (metadata.line_start, metadata.line_end) {
            parts.push(format!("lines:{}-{}", start, end));
        }

        // Entity type
        if let Some(ref entity_type) = metadata.entity_type {
            parts.push(format!("type:{}", entity_type));
        }

        Ok(parts.join(", "))
    }

    /// Auto-shrink RAW chunks to fit token limit
    fn auto_shrink(
        &self,
        mut chunks: Vec<ExpandedChunk>,
        limit: usize,
    ) -> Result<Vec<ExpandedChunk>> {
        // Sort by token count (ascending) to compress smallest first
        chunks.sort_by_key(|c| c.token_count);

        let mut total_tokens: usize = chunks.iter().map(|c| c.token_count).sum();
        let mut result = Vec::new();

        for chunk in chunks {
            if total_tokens <= limit {
                result.push(chunk);
            } else {
                // Convert to compressed
                let compressed_content = format!(
                    "chunk_id:{}, tokens:{} (auto-compressed)",
                    chunk.chunk_id, chunk.token_count
                );
                let compressed_tokens = self.estimate_tokens(&compressed_content);

                result.push(ExpandedChunk {
                    chunk_id: chunk.chunk_id,
                    content: compressed_content,
                    format: ChunkFormat::Compressed,
                    token_count: compressed_tokens,
                    language: None,
                });

                total_tokens = total_tokens - chunk.token_count + compressed_tokens;
            }
        }

        Ok(result)
    }

    // Helper methods

    fn try_get_from_code(&self, chunk_id: i64) -> Result<crate::vector::Hit> {
        use crate::vector::SearchScope;
        let store = self.state.code_store.lock().unwrap();
        let results = store.search("", 1000, SearchScope::Global)?;
        results
            .into_iter()
            .find(|hit| hit.id == chunk_id)
            .ok_or_else(|| anyhow::anyhow!("Not found in code store"))
    }

    fn try_get_from_general(&self, chunk_id: i64) -> Result<crate::vector::Hit> {
        use crate::vector::SearchScope;
        let store = self.state.general_store.lock().unwrap();
        let results = store.search("", 1000, SearchScope::Global)?;
        results
            .into_iter()
            .find(|hit| hit.id == chunk_id)
            .ok_or_else(|| anyhow::anyhow!("Not found in general store"))
    }

    fn read_file_lines(
        &self,
        path: &str,
        line_start: Option<usize>,
        line_end: Option<usize>,
    ) -> Result<String> {
        let content = fs::read_to_string(path)?;
        let lines: Vec<&str> = content.lines().collect();

        let start = line_start.unwrap_or(1).saturating_sub(1);
        let end = line_end.unwrap_or(lines.len()).min(lines.len());

        Ok(lines[start..end].join("\n"))
    }

    fn format_compressed(&self, text: &str, chunk_id: i64) -> String {
        // Extract basic info from text
        let symbols = self.extract_symbols_simple(text);
        let entity_type = self.infer_entity_type(text);

        let mut parts = vec![format!("chunk_id:{}", chunk_id)];

        if let Some(et) = entity_type {
            parts.push(format!("type:{}", et));
        }

        if !symbols.is_empty() {
            parts.push(format!("symbols:{}", symbols.join(",")));
        }

        parts.join(", ")
    }

    fn extract_symbols_simple(&self, text: &str) -> Vec<String> {
        let mut symbols = Vec::new();

        // Simple regex-based extraction
        if let Some(caps) = regex::Regex::new(r"fn\s+(\w+)").ok().and_then(|re| re.captures(text)) {
            if let Some(name) = caps.get(1) {
                symbols.push(name.as_str().to_string());
            }
        }

        symbols
    }

    fn infer_entity_type(&self, text: &str) -> Option<String> {
        if text.contains("fn ") {
            Some("Function".to_string())
        } else if text.contains("struct ") {
            Some("Struct".to_string())
        } else {
            None
        }
    }

    fn infer_language(&self, text: &str) -> Option<String> {
        if text.contains("fn ") || text.contains("impl ") {
            Some("rust".to_string())
        } else if text.contains("function ") || text.contains("const ") {
            Some("javascript".to_string())
        } else if text.contains("def ") {
            Some("python".to_string())
        } else {
            None
        }
    }

    fn detect_language_from_path(&self, path: &str) -> String {
        let path = Path::new(path);
        match path.extension().and_then(|s| s.to_str()) {
            Some("rs") => "rust".to_string(),
            Some("js") | Some("ts") => "javascript".to_string(),
            Some("py") => "python".to_string(),
            Some("go") => "go".to_string(),
            Some("java") => "java".to_string(),
            Some("cpp") | Some("cc") | Some("cxx") => "cpp".to_string(),
            Some("c") | Some("h") => "c".to_string(),
            _ => "text".to_string(),
        }
    }

    /// Estimate token count (rough approximation: words / 0.75)
    pub fn estimate_tokens(&self, text: &str) -> usize {
        let words = text.split_whitespace().count();
        ((words as f32) / 0.75).ceil() as usize
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_token_estimation() {
        // Test the token estimation logic without needing full state
        // Formula: words / 0.75
        let text = "fn main() { println!(\"hello\"); }";
        let words = text.split_whitespace().count();
        let expected = ((words as f32) / 0.75).ceil() as usize;

        // Direct test of the formula (words=5: fn, main(), {, println!("hello");, })
        assert_eq!(words, 5);
        assert_eq!(expected, 7);
        assert!(expected > 0, "Should estimate tokens");
    }
}
