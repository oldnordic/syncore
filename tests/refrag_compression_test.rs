//! APEX 1.8 REFRAG - ChunkCompressionLayer Tests
//!
//! Tests for chunk compression using existing DualEmbeddingService embeddings.
//! NO re-embedding, only metadata extraction and reuse.

use anyhow::Result;

// TDD: These imports will fail until we create the refrag module
// use syncore::refrag::compression::{ChunkCompressionLayer, ChunkMetadata};
// use syncore::refrag::types::Domain;
use std::sync::{Arc, Mutex};
use syncore::router::SynCoreState;
use syncore::vector::{HuggingFaceEmbeddings, VectorStore};
use tempfile::TempDir;

/// Helper: Create test state with dual stores
fn create_test_state() -> Result<SynCoreState> {
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path().to_str().unwrap();

    std::env::set_var("DB_PATH", format!("{}/test.db", temp_path));
    std::env::set_var("CODE_GRAPH_DB", format!("{}/test_code_graph.db", temp_path));

    let code_embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let mut code_store = VectorStore::new(code_embeddings);
    code_store.set_index_path(format!("{}/code.index", temp_path));
    let code_store = Arc::new(Mutex::new(code_store));

    let general_embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let mut general_store = VectorStore::new(general_embeddings);
    general_store.set_index_path(format!("{}/general.index", temp_path));
    let general_store = Arc::new(Mutex::new(general_store));

    SynCoreState::with_dual_stores(code_store, general_store)
}

#[test]
#[ignore] // TDD: Will fail until refrag module exists
fn test_compression_uses_existing_embeddings() -> Result<()> {
    // GIVEN a state with precomputed embeddings
    let state = create_test_state()?;

    // Insert some test vectors
    {
        let mut code_store = state.code_store.lock().unwrap();
        code_store.insert_text(
            1,
            None,
            "fn example() { println!(\"test\"); }",
            "code_entity",
        )?;
        code_store.insert_text(2, None, "struct Data { field: i32 }", "code_entity")?;
    }

    // WHEN we create compression layer
    // let compression = ChunkCompressionLayer::new(state.clone())?;

    // AND retrieve chunks
    // let chunks = compression.get_chunks(vec![1, 2])?;

    // THEN chunks should have embeddings from existing store
    // assert_eq!(chunks.len(), 2);
    // assert!(chunks[0].embedding.is_some(), "Should reuse existing embedding");
    // assert!(chunks[1].embedding.is_some(), "Should reuse existing embedding");

    // AND should NOT call embed() again (check via call count or mock)
    // This ensures no re-embedding occurs

    Ok(())
}

#[test]
#[ignore] // TDD: Will fail until refrag module exists
fn test_compression_metadata_extraction() -> Result<()> {
    // GIVEN code entities with known metadata
    let state = create_test_state()?;

    {
        let mut code_store = state.code_store.lock().unwrap();
        code_store.insert_text(
            10,
            None,
            "fn parse_config(path: &str) -> Result<Config>",
            "code_entity",
        )?;
    }

    // WHEN we extract metadata
    // let compression = ChunkCompressionLayer::new(state)?;
    // let metadata = compression.extract_metadata(10)?;

    // THEN metadata should include
    // assert_eq!(metadata.chunk_id, 10);
    // assert_eq!(metadata.domain, Domain::Code);
    // assert!(metadata.file_path.is_some());
    // assert!(metadata.entity_type.is_some()); // "Function"
    // assert!(metadata.symbols.contains(&"parse_config".to_string()));

    Ok(())
}

#[test]
#[ignore] // TDD: Will fail until refrag module exists
fn test_compression_chunk_id_stability() -> Result<()> {
    // GIVEN same input text
    let state = create_test_state()?;

    {
        let mut code_store = state.code_store.lock().unwrap();
        code_store.insert_text(42, None, "let x = 10;", "code_entity")?;
    }

    // WHEN we retrieve chunk multiple times
    // let compression = ChunkCompressionLayer::new(state)?;
    // let chunk1 = compression.get_chunk(42)?;
    // let chunk2 = compression.get_chunk(42)?;

    // THEN chunk_id should be stable
    // assert_eq!(chunk1.chunk_id, chunk2.chunk_id);
    // assert_eq!(chunk1.chunk_id, 42, "Should preserve original ID");

    Ok(())
}

#[test]
#[ignore] // TDD: Will fail until refrag module exists
fn test_compression_no_reembedding_when_exists() -> Result<()> {
    // GIVEN precomputed embeddings in store
    let state = create_test_state()?;

    {
        let mut code_store = state.code_store.lock().unwrap();
        code_store.insert_text(100, None, "test code", "code_entity")?;
    }

    // WHEN we create compression layer
    // let compression = ChunkCompressionLayer::new(state.clone())?;

    // AND request chunk
    // let chunk = compression.get_chunk(100)?;

    // THEN embedding should be retrieved (not recomputed)
    // assert!(chunk.embedding.is_some());
    // assert_eq!(chunk.embedding.as_ref().unwrap().len(), 384, "384-dim HuggingFace");

    // Verify no new inserts occurred
    let code_store = state.code_store.lock().unwrap();
    assert_eq!(code_store.len(), 1, "Should not insert new vectors");

    Ok(())
}

#[test]
#[ignore] // TDD: Will fail until refrag module exists
fn test_compression_handles_missing_embedding() -> Result<()> {
    // GIVEN a chunk ID with no precomputed embedding
    let state = create_test_state()?;

    // WHEN we try to get chunk for non-existent ID
    // let compression = ChunkCompressionLayer::new(state)?;
    // let result = compression.get_chunk(999);

    // THEN should return error (not panic)
    // assert!(result.is_err(), "Should handle missing chunk gracefully");

    Ok(())
}

#[test]
#[ignore] // TDD: Will fail until refrag module exists
fn test_compression_domain_separation() -> Result<()> {
    // GIVEN chunks in both CODE and GENERAL domains
    let state = create_test_state()?;

    {
        let mut code_store = state.code_store.lock().unwrap();
        code_store.insert_text(1, None, "fn main() {}", "code_entity")?;
    }

    {
        let mut general_store = state.general_store.lock().unwrap();
        general_store.insert_text(2, None, "Documentation text", "documents")?;
    }

    // WHEN we retrieve chunks
    // let compression = ChunkCompressionLayer::new(state)?;
    // let code_chunk = compression.get_chunk(1)?;
    // let general_chunk = compression.get_chunk(2)?;

    // THEN domains should be correctly identified
    // assert_eq!(code_chunk.domain, Domain::Code);
    // assert_eq!(general_chunk.domain, Domain::General);

    Ok(())
}
