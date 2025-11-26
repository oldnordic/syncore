//! APEX 1.8 REFRAG - End-to-End Integration Tests
//!
//! Tests complete REFRAG pipeline:
//! query → embedding → retrieve → perceive → expand → hybrid prompt

use anyhow::Result;

// TDD: These imports will fail until we create the refrag module
// use syncore::refrag::{RefragPipeline, RefragConfig};
use syncore::router::SynCoreState;
use syncore::vector::{HuggingFaceEmbeddings, VectorStore};
use std::sync::{Arc, Mutex};
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

#[tokio::test]
#[ignore] // TDD: Will fail until refrag module exists
async fn test_refrag_end_to_end_pipeline() -> Result<()> {
    // GIVEN a populated code store
    let state = create_test_state()?;

    // Insert test code chunks
    {
        let mut code_store = state.code_store.lock().unwrap();
        code_store.insert_text(1, None, "fn parse_config(path: &str) -> Result<Config>", "code_entity")?;
        code_store.insert_text(2, None, "fn load_file(path: &str) -> Result<String>", "code_entity")?;
        code_store.insert_text(3, None, "struct Config { database: String }", "code_entity")?;
        code_store.insert_text(4, None, "impl Config { fn new() -> Self }", "code_entity")?;
        code_store.insert_text(5, None, "use std::fs;", "code_entity")?;
    }

    // WHEN we execute full REFRAG pipeline
    // let config = RefragConfig {
    //     top_k_raw: 2,
    //     max_tokens: 500,
    //     selection_policy: SelectionPolicy::TopK(2),
    // };
    // let pipeline = RefragPipeline::new(state, config)?;
    // let result = pipeline.query("How to parse config?").await?;

    // THEN should return hybrid prompt
    // assert!(result.prompt.len() > 0, "Should generate prompt");
    // assert_eq!(result.raw_count, 2, "Should have 2 raw chunks");
    // assert_eq!(result.compressed_count, 3, "Should have 3 compressed chunks");
    // assert!(result.total_tokens <= 500, "Should respect token limit");

    // AND raw chunks should be top-scored
    // assert!(result.prompt.contains("parse_config"));
    // assert!(result.prompt.contains("load_file"));

    // AND compressed chunks should be summaries
    // assert!(result.prompt.contains("file:") || result.prompt.contains("symbols:"));

    Ok(())
}

#[tokio::test]
#[ignore] // TDD: Will fail until refrag module exists
async fn test_refrag_deterministic_across_runs() -> Result<()> {
    // GIVEN same state and query
    let state = create_test_state()?;
    populate_test_chunks(&state)?;

    // WHEN we run pipeline multiple times
    // let config = RefragConfig::default();
    // let pipeline = RefragPipeline::new(state.clone(), config)?;

    // let result1 = pipeline.query("test query").await?;
    // let result2 = pipeline.query("test query").await?;
    // let result3 = pipeline.query("test query").await?;

    // THEN results should be identical (deterministic)
    // assert_eq!(result1.prompt, result2.prompt);
    // assert_eq!(result2.prompt, result3.prompt);
    // assert_eq!(result1.raw_count, result2.raw_count);
    // assert_eq!(result1.compressed_count, result2.compressed_count);

    Ok(())
}

#[tokio::test]
#[ignore] // TDD: Will fail until refrag module exists
async fn test_refrag_handles_empty_results() -> Result<()> {
    // GIVEN empty code store
    let state = create_test_state()?;

    // WHEN we query
    // let config = RefragConfig::default();
    // let pipeline = RefragPipeline::new(state, config)?;
    // let result = pipeline.query("nonexistent query").await?;

    // THEN should return empty prompt gracefully
    // assert_eq!(result.raw_count, 0);
    // assert_eq!(result.compressed_count, 0);
    // assert!(result.prompt.contains("No matching chunks found") || result.prompt.len() == 0);

    Ok(())
}

#[tokio::test]
#[ignore] // TDD: Will fail until refrag module exists
async fn test_refrag_respects_domain_separation() -> Result<()> {
    // GIVEN chunks in both CODE and GENERAL domains
    let state = create_test_state()?;

    {
        let mut code_store = state.code_store.lock().unwrap();
        code_store.insert_text(1, None, "fn code_func()", "code_entity")?;
    }

    {
        let mut general_store = state.general_store.lock().unwrap();
        general_store.insert_text(2, None, "Documentation about code", "documents")?;
    }

    // WHEN we query CODE domain
    // let config = RefragConfig::default();
    // let pipeline = RefragPipeline::new(state, config)?;
    // let result = pipeline.query_domain("function", Domain::Code).await?;

    // THEN should only return CODE chunks
    // assert!(result.prompt.contains("code_func"));
    // assert!(!result.prompt.contains("Documentation"), "Should not include GENERAL");

    Ok(())
}

#[tokio::test]
#[ignore] // TDD: Will fail until refrag module exists
async fn test_refrag_auto_shrink_on_overflow() -> Result<()> {
    // GIVEN many chunks exceeding token limit
    let state = create_test_state()?;

    {
        let mut code_store = state.code_store.lock().unwrap();
        for i in 1..=20 {
            let content = format!("fn func{}() {{ /* 50 tokens of code */ }}", i);
            code_store.insert_text(i, None, &content, "code_entity")?;
        }
    }

    // WHEN we query with low token limit
    // let config = RefragConfig {
    //     top_k_raw: 20,
    //     max_tokens: 300,  // Total chunks = 1000 tokens, but limit is 300
    //     ...
    // };
    // let pipeline = RefragPipeline::new(state, config)?;
    // let result = pipeline.query("functions").await?;

    // THEN should auto-shrink to fit
    // assert!(result.total_tokens <= 300, "Should not exceed limit");
    // assert!(result.raw_count < 20, "Should reduce raw count");
    // assert!(result.compressed_count > 0, "Should compress some chunks");

    Ok(())
}

#[tokio::test]
#[ignore] // TDD: Will fail until refrag module exists
async fn test_refrag_preserves_fusion_scores() -> Result<()> {
    // GIVEN chunks with known fusion scores
    let state = create_test_state()?;
    populate_test_chunks(&state)?;

    // WHEN we run pipeline
    // let config = RefragConfig::default();
    // let pipeline = RefragPipeline::new(state, config)?;
    // let result = pipeline.query("test").await?;

    // THEN fusion scores should be preserved in metadata
    // assert!(result.metadata.contains_key("fusion_scores"));
    // let scores = &result.metadata["fusion_scores"];
    // assert!(scores.len() > 0);

    Ok(())
}

#[tokio::test]
#[ignore] // TDD: Will fail until refrag module exists
async fn test_refrag_integration_with_mapping_suite() -> Result<()> {
    // GIVEN chunks with file paths
    let state = create_test_state()?;

    // TODO: Use MappingTool to register files
    // let mapping = MappingTool::new(state.clone());
    // mapping.record_file(...)?;

    // WHEN we expand chunks
    // let config = RefragConfig::default();
    // let pipeline = RefragPipeline::new(state, config)?;
    // let result = pipeline.query("test").await?;

    // THEN should retrieve from mapping_suite
    // assert!(result.prompt.len() > 0);

    Ok(())
}

#[tokio::test]
#[ignore] // TDD: Will fail until refrag module exists
async fn test_refrag_performance_benchmark() -> Result<()> {
    // GIVEN large code store (1000 chunks)
    let state = create_test_state()?;

    {
        let mut code_store = state.code_store.lock().unwrap();
        for i in 1..=1000 {
            let content = format!("fn func{}() {{ /* code */ }}", i);
            code_store.insert_text(i, None, &content, "code_entity")?;
        }
    }

    // WHEN we query and measure time
    // let config = RefragConfig::default();
    // let pipeline = RefragPipeline::new(state, config)?;

    // let start = std::time::Instant::now();
    // let _result = pipeline.query("performance test").await?;
    // let duration = start.elapsed();

    // THEN should complete within reasonable time
    // assert!(duration.as_secs() < 5, "Should complete within 5 seconds");

    Ok(())
}

// Helper function to populate test chunks
fn populate_test_chunks(state: &SynCoreState) -> Result<()> {
    let mut code_store = state.code_store.lock().unwrap();

    code_store.insert_text(1, None, "fn main() { println!(\"hello\"); }", "code_entity")?;
    code_store.insert_text(2, None, "struct Config { path: String }", "code_entity")?;
    code_store.insert_text(3, None, "impl Config { fn load() {} }", "code_entity")?;
    code_store.insert_text(4, None, "fn helper_func() -> i32 { 42 }", "code_entity")?;
    code_store.insert_text(5, None, "use std::fs::File;", "code_entity")?;

    Ok(())
}
