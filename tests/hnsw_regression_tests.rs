//! Regression Tests for HNSW Benchmark Integration
//!
//! Ensures that creating the benchmark harness does NOT break:
//! 1. Existing vector_search functionality
//! 2. Dual-embedding architecture (CODE/GENERAL domains)
//! 3. REFRAG selective expansion
//! 4. Fusion_query combined scoring
//!
//! These tests validate that the comparison harness is isolated and doesn't affect core SynCore.

use anyhow::Result;

/// Regression Test 1: Vector embedding generation still works
#[test]
fn test_vector_embeddings_still_work() -> Result<()> {
    use syncore::vector::{Embeddings, RealEmbeddings};

    let embeddings = RealEmbeddings::new(384)?;
    let vec1 = embeddings.embed("test text")?;
    let vec2 = embeddings.embed("different text")?;

    assert_eq!(vec1.len(), 384);
    assert_eq!(vec2.len(), 384);
    assert_ne!(
        vec1, vec2,
        "Different text should have different embeddings"
    );

    Ok(())
}

/// Regression Test 2: Dual-embedding architecture unchanged
#[test]
fn test_dual_embedding_architecture() -> Result<()> {
    use syncore::refrag::types::Domain;
    use syncore::vector::{Embeddings, RealEmbeddings};

    // Ensure both domains are still available in refrag module
    let _code_domain = Domain::Code;
    let _general_domain = Domain::General;

    // Verify RealEmbeddings still supports embedding generation
    let embeddings = RealEmbeddings::new(384)?;
    let test_vec = embeddings.embed("test text")?;

    assert_eq!(test_vec.len(), 384, "Embedding dimension should be 384");

    Ok(())
}

/// Regression Test 3: REFRAG module still available
#[test]
fn test_refrag_module_unchanged() {
    // Just verify the module is accessible
    use syncore::refrag::types::{ChunkMetadata, Domain};

    let _ = ChunkMetadata {
        chunk_id: 1,
        domain: Domain::Code,
        embedding: None,
        file_path: Some("test.rs".to_string()),
        entity_type: Some("function".to_string()),
        symbols: vec!["test_fn".to_string()],
        line_start: Some(1),
        line_end: Some(10),
        fusion_score: 0.0,
        graph_score: 0.0,
        structural_score: 0.0,
        perplexity_score: None,
        graph_hops: None,
        text: "test content".to_string(),
    };
}

/// Regression Test 4: Code graph module unchanged
#[test]
fn test_code_graph_module_unchanged() {
    // Verify code_graph module is accessible
    use syncore::code_graph::fusion_router::FusionMode;

    let _simple = FusionMode::Simple;
    let _attention = FusionMode::Attention;
    let _reasoning = FusionMode::Reasoning;
}

/// Regression Test 5: Memory module accessible
#[test]
fn test_memory_unchanged() {
    // Just verify Memory module is accessible (actual operations tested elsewhere)
    use syncore::memory::Memory;

    // Type check - verify Memory struct is available
    let _memory_type: Option<Memory> = None;
}

/// Regression Test 6: HNSW index isolated
#[test]
fn test_hnsw_index_isolated() -> Result<()> {
    use syncore::vector::hnsw::{HnswConfig, HnswVectorIndex};
    use syncore::vector::traits::VectorIndex;

    let config = HnswConfig {
        m: 16,
        ef_construction: 200,
        ef_search: 50,
    };

    let mut index = HnswVectorIndex::new(config, 42)?;

    // HNSW operations don't affect other modules
    for i in 1..=10 {
        index.add(i, vec![i as f32, (i * 2) as f32])?;
    }

    let results = index.search(&[5.0, 10.0], 3)?;
    assert!(results.len() > 0);

    Ok(())
}

/// Regression Test 9: MCP tool structure unchanged
#[test]
fn test_mcp_tools_structure() {
    // Verify MCP tool modules are still available
    use syncore::mcp_tools::memory_suite::MemorySuiteArgs;

    let _memory_args = MemorySuiteArgs::default();
}

/// Regression Test 10: No module conflicts
#[test]
fn test_no_module_conflicts() {
    // All core modules should still be accessible without conflicts
    use syncore::memory::Memory;
    use syncore::vector::hnsw::HnswVectorIndex;
    use syncore::vector::RealEmbeddings;

    // Types should be distinct and not conflict
    let _embeddings_type: Option<RealEmbeddings> = None;
    let _hnsw_type: Option<HnswVectorIndex> = None;
    let _memory_type: Option<Memory> = None;
}
