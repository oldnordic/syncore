//! APEX 1.8 REFRAG - HybridPromptBuilder
//!
//! Assembles final LLM prompts with:
//! - TOP-K RAW BLOCKS (full snippets)
//! - COMPRESSED BLOCK SUMMARIES (metadata)
//! - QUERY CONTEXT
//!
//! Design:
//! - Deterministic assembly order: RAW → COMPRESSED → QUERY
//! - Token limit enforcement with auto-shrink
//! - Markdown code block formatting
//! - Section headers for clarity

use super::expand::{ChunkFormat, ExpandedChunk};
use anyhow::Result;

/// Hybrid prompt builder
pub struct HybridPromptBuilder {
    raw_chunks: Vec<ExpandedChunk>,
    compressed_chunks: Vec<ExpandedChunk>,
    query: Option<String>,
    token_limit: Option<usize>,
    auto_shrink_enabled: bool,
}

impl HybridPromptBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            raw_chunks: Vec::new(),
            compressed_chunks: Vec::new(),
            query: None,
            token_limit: None,
            auto_shrink_enabled: false,
        }
    }

    /// Create builder with token limit
    pub fn with_limit(token_limit: usize) -> Self {
        Self {
            raw_chunks: Vec::new(),
            compressed_chunks: Vec::new(),
            query: None,
            token_limit: Some(token_limit),
            auto_shrink_enabled: false,
        }
    }

    /// Add raw chunks (full snippets)
    pub fn with_raw_chunks(mut self, chunks: Vec<ExpandedChunk>) -> Self {
        self.raw_chunks = chunks;
        self
    }

    /// Add compressed chunks (metadata summaries)
    pub fn with_compressed_chunks(mut self, chunks: Vec<ExpandedChunk>) -> Self {
        self.compressed_chunks = chunks;
        self
    }

    /// Add query context
    pub fn with_query(mut self, query: impl Into<String>) -> Self {
        self.query = Some(query.into());
        self
    }

    /// Enable auto-shrink mechanism
    pub fn auto_shrink(mut self) -> Self {
        self.auto_shrink_enabled = true;
        self
    }

    /// Build final prompt
    pub fn build(mut self) -> Result<String> {
        // Check token limit and auto-shrink if needed
        if self.auto_shrink_enabled && self.token_limit.is_some() {
            self.apply_auto_shrink()?;
        }

        let mut sections = Vec::new();

        // Section 1: TOP-K RAW BLOCKS
        if !self.raw_chunks.is_empty() {
            sections.push(self.build_raw_section());
        }

        // Section 2: COMPRESSED BLOCK SUMMARIES
        if !self.compressed_chunks.is_empty() {
            sections.push(self.build_compressed_section());
        }

        // Section 3: QUERY CONTEXT
        if let Some(ref query) = self.query {
            sections.push(self.build_query_section(query));
        }

        Ok(sections.join("\n\n"))
    }

    /// Build RAW section with markdown code blocks
    fn build_raw_section(&self) -> String {
        let mut output = String::from("## TOP-K RAW BLOCKS\n\n");

        for (idx, chunk) in self.raw_chunks.iter().enumerate() {
            output.push_str(&format!("### Block {} (chunk_id: {})\n\n", idx + 1, chunk.chunk_id));

            // Markdown code block with language
            if let Some(ref lang) = chunk.language {
                output.push_str(&format!("```{}\n{}\n```\n\n", lang, chunk.content));
            } else {
                output.push_str(&format!("```\n{}\n```\n\n", chunk.content));
            }
        }

        output
    }

    /// Build COMPRESSED section with metadata
    fn build_compressed_section(&self) -> String {
        let mut output = String::from("## COMPRESSED BLOCK SUMMARIES\n\n");

        for (idx, chunk) in self.compressed_chunks.iter().enumerate() {
            output.push_str(&format!(
                "{}. {}\n",
                idx + 1,
                chunk.content
            ));
        }

        output.push('\n');
        output
    }

    /// Build QUERY section
    fn build_query_section(&self, query: &str) -> String {
        format!("## QUERY CONTEXT\n\n{}\n", query)
    }

    /// Apply auto-shrink to fit token limit
    fn apply_auto_shrink(&mut self) -> Result<()> {
        let limit = self.token_limit.unwrap();
        let current_total = self.total_tokens();

        if current_total <= limit {
            return Ok(());
        }

        // Strategy: Convert lowest-scoring RAW chunks to COMPRESSED
        // Sort raw chunks by token count (largest first to reduce faster)
        self.raw_chunks.sort_by(|a, b| b.token_count.cmp(&a.token_count));

        let mut total = current_total;
        let mut to_compress = Vec::new();

        while total > limit && !self.raw_chunks.is_empty() {
            if let Some(chunk) = self.raw_chunks.pop() {
                // Convert to compressed
                let compressed_content = format!(
                    "chunk_id:{}, tokens:{} (auto-compressed from RAW)",
                    chunk.chunk_id, chunk.token_count
                );
                let compressed_tokens = self.estimate_tokens(&compressed_content);

                to_compress.push(ExpandedChunk {
                    chunk_id: chunk.chunk_id,
                    content: compressed_content,
                    format: ChunkFormat::Compressed,
                    token_count: compressed_tokens,
                    language: None,
                });

                total = total - chunk.token_count + compressed_tokens;
            }
        }

        // Add compressed chunks
        self.compressed_chunks.extend(to_compress);

        Ok(())
    }

    /// Calculate total token count
    pub fn total_tokens(&self) -> usize {
        let raw_tokens: usize = self.raw_chunks.iter().map(|c| c.token_count).sum();
        let compressed_tokens: usize = self.compressed_chunks.iter().map(|c| c.token_count).sum();
        let query_tokens = self.query.as_ref().map(|q| self.estimate_tokens(q)).unwrap_or(0);

        raw_tokens + compressed_tokens + query_tokens
    }

    /// Get count of raw chunks
    pub fn raw_count(&self) -> usize {
        self.raw_chunks.len()
    }

    /// Get count of compressed chunks
    pub fn compressed_count(&self) -> usize {
        self.compressed_chunks.len()
    }

    /// Estimate token count (rough approximation)
    fn estimate_tokens(&self, text: &str) -> usize {
        let words = text.split_whitespace().count();
        ((words as f32) / 0.75).ceil() as usize
    }
}

impl Default for HybridPromptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let builder = HybridPromptBuilder::new();
        let prompt = builder.with_query("test query").build().unwrap();
        assert!(prompt.contains("test query"));
    }

    #[test]
    fn test_token_estimation() {
        let builder = HybridPromptBuilder::new();
        let tokens = builder.estimate_tokens("fn main() { println!(\"hello\"); }");
        assert!(tokens > 0);
    }

    #[test]
    fn test_section_order() {
        let raw = vec![ExpandedChunk {
            chunk_id: 1,
            content: "fn example() {}".to_string(),
            format: ChunkFormat::Raw,
            token_count: 10,
            language: Some("rust".to_string()),
        }];

        let compressed = vec![ExpandedChunk {
            chunk_id: 2,
            content: "file:test.rs, symbols:helper".to_string(),
            format: ChunkFormat::Compressed,
            token_count: 5,
            language: None,
        }];

        let builder = HybridPromptBuilder::new()
            .with_raw_chunks(raw)
            .with_compressed_chunks(compressed)
            .with_query("test query");

        let prompt = builder.build().unwrap();

        // Verify order: RAW → COMPRESSED → QUERY
        let raw_pos = prompt.find("fn example").unwrap();
        let compressed_pos = prompt.find("file:test.rs").unwrap();
        let query_pos = prompt.find("test query").unwrap();

        assert!(raw_pos < compressed_pos, "RAW should come before COMPRESSED");
        assert!(compressed_pos < query_pos, "COMPRESSED should come before QUERY");
    }
}
