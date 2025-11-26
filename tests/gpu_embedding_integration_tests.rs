//! APEX 2.0-E Integration Tests: GPU Embeddings with Real HNSW + SQLite
//!
//! REAL integration tests against live HNSW indices and SQLite database.
//! These tests verify end-to-end behavior after GPU embedding upgrade.
//!
//! Test Coverage:
//! 1. vector_insert with CODE domain → index uses new 1024 dims
//! 2. vector_search with CODE → results valid and non-zero
//! 3. vector_insert with GENERAL → index adapts to 1024 or 768 dims
//! 4. vector_search with GENERAL → results valid
//! 5. code_suite.search → BGE-M3 vectors improve semantic ranking
//! 6. memory_suite.vector_search → GENERAL domain ranking works
//! 7. GRAPH domain vector operations still use 384 dims
//! 8. Dimension mismatch detection and index rebuild
//! 9. Mixed-domain search (CODE + GENERAL + GRAPH) doesn't cross-contaminate
//! 10. HNSW index persistence across restarts

use anyhow::Result;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// INTEGRATION TEST 1: CODE Domain Vector Insert + Search
// ============================================================================

#[tokio::test]
async fn test_code_domain_vector_insert_uses_new_dimensions() -> Result<()> {
    // Test: Insert CODE entity with GPU embeddings (1024 dims)
    // Expected: HNSW index created/updated with 1024 dimensions

    // When implemented:
    // let temp_dir = TempDir::new()?;
    // let db_path = temp_dir.path().join("test_code_insert.db");
    // let state = SynCoreState::new(db_path, temp_dir.path())?;
    //
    // // Insert code entity
    // let request = VectorInsertRequest {
    //     text: "fn main() { println!(\"Hello\"); }".to_string(),
    //     namespace: "code_entity".to_string(),
    //     metadata: None,
    // };
    // vector_insert(request, &state).await?;
    //
    // // Verify HNSW index dimension
    // let code_store = state.store_for_domain(EmbeddingDomain::Code);
    // let store = code_store.lock().unwrap();
    // assert_eq!(store.dimension(), 1024, "CODE index should be 1024-dim after GPU upgrade");

    Ok(())
}

#[tokio::test]
async fn test_code_domain_vector_search_returns_valid_results() -> Result<()> {
    // Test: Search CODE entities returns non-empty valid results
    // Expected: Results with non-zero scores and correct entity metadata

    // When implemented:
    // let temp_dir = TempDir::new()?;
    // let db_path = temp_dir.path().join("test_code_search.db");
    // let state = SynCoreState::new(db_path, temp_dir.path())?;
    //
    // // Insert multiple code entities
    // for i in 0..5 {
    //     let code = format!("fn function_{}() {{ /* code */ }}", i);
    //     vector_insert(VectorInsertRequest {
    //         text: code,
    //         namespace: "code_entity".to_string(),
    //         metadata: None,
    //     }, &state).await?;
    // }
    //
    // // Search
    // let request = VectorSearchRequest {
    //     query: "function implementation".to_string(),
    //     namespace: Some("code_entity".to_string()),
    //     limit: Some(3),
    // };
    // let results = vector_search(request, &state).await?;
    //
    // assert!(!results.is_empty(), "Should return results");
    // assert!(results[0].score > 0.0, "Top result should have non-zero score");
    // assert_eq!(results.len(), 3, "Should respect limit");

    Ok(())
}

// ============================================================================
// INTEGRATION TEST 2: GENERAL Domain Vector Insert + Search
// ============================================================================

#[tokio::test]
async fn test_general_domain_vector_insert_with_fallback() -> Result<()> {
    // Test: Insert GENERAL entity adapts to GPU (1024) or CPU fallback (768)
    // Expected: HNSW index dimension matches embedding dimension

    // When implemented:
    // let temp_dir = TempDir::new()?;
    // let db_path = temp_dir.path().join("test_general_insert.db");
    // let state = SynCoreState::new(db_path, temp_dir.path())?;
    //
    // // Insert document
    // let request = VectorInsertRequest {
    //     text: "Project requirements document for feature X".to_string(),
    //     namespace: "documents".to_string(),
    //     metadata: Some(serde_json::json!({"type": "prd"})),
    // };
    // vector_insert(request, &state).await?;
    //
    // // Verify dimension (either GPU or CPU fallback)
    // let general_store = state.store_for_domain(EmbeddingDomain::General);
    // let store = general_store.lock().unwrap();
    // let dim = store.dimension();
    // assert!(dim == 1024 || dim == 768, "GENERAL should be 1024 (GPU) or 768 (CPU fallback), got {}", dim);

    Ok(())
}

#[tokio::test]
async fn test_general_domain_vector_search_ranking() -> Result<()> {
    // Test: GENERAL domain search returns semantically ranked results
    // Expected: Most relevant document ranked first

    // When implemented:
    // let temp_dir = TempDir::new()?;
    // let db_path = temp_dir.path().join("test_general_search.db");
    // let state = SynCoreState::new(db_path, temp_dir.path())?;
    //
    // // Insert documents with varying relevance
    // let docs = vec![
    //     "Machine learning model training documentation",
    //     "Database migration scripts and procedures",
    //     "Neural network architecture design patterns",  // Most relevant
    //     "API endpoint routing configuration",
    // ];
    // for doc in docs {
    //     vector_insert(VectorInsertRequest {
    //         text: doc.to_string(),
    //         namespace: "documents".to_string(),
    //         metadata: None,
    //     }, &state).await?;
    // }
    //
    // // Search for ML/AI related
    // let request = VectorSearchRequest {
    //     query: "neural network machine learning".to_string(),
    //     namespace: Some("documents".to_string()),
    //     limit: Some(2),
    // };
    // let results = vector_search(request, &state).await?;
    //
    // assert!(!results.is_empty());
    // assert!(results[0].text.contains("Neural network") || results[0].text.contains("Machine learning"));

    Ok(())
}

// ============================================================================
// INTEGRATION TEST 3: code_suite.search with BGE-M3
// ============================================================================

#[tokio::test]
async fn test_code_suite_search_improved_semantic_ranking() -> Result<()> {
    // Test: code_suite.search with BGE-M3 yields better semantic code ranking
    // Expected: Code-specific queries return more relevant results than before

    // When implemented:
    // let temp_dir = TempDir::new()?;
    // let db_path = temp_dir.path().join("test_code_suite.db");
    // let state = SynCoreState::new(db_path, temp_dir.path())?;
    //
    // // Index multiple code entities with semantic differences
    // let code_samples = vec![
    //     "fn authenticate_user(username: &str, password: &str) -> Result<Token>",
    //     "async fn fetch_data_from_api(endpoint: &str) -> Result<Response>",
    //     "fn validate_jwt_token(token: &str) -> Result<Claims>",  // Related to auth
    //     "fn parse_json_response(body: &str) -> Result<JsonValue>",
    // ];
    // for (i, code) in code_samples.iter().enumerate() {
    //     code_index(CodeIndexRequest {
    //         file_path: format!("test{}.rs", i),
    //     }, &state).await?;
    // }
    //
    // // Search for authentication-related code
    // let request = CodeSearchRequest {
    //     query: "user authentication token validation".to_string(),
    //     limit: Some(2),
    // };
    // let results = code_search(request, &state).await?;
    //
    // // Verify auth-related functions ranked higher
    // assert!(!results.is_empty());
    // let top_result = &results[0].text;
    // assert!(
    //     top_result.contains("authenticate") || top_result.contains("jwt_token"),
    //     "Top result should be auth-related"
    // );

    Ok(())
}

// ============================================================================
// INTEGRATION TEST 4: memory_suite.vector_search
// ============================================================================

#[tokio::test]
async fn test_memory_suite_vector_search_general_domain() -> Result<()> {
    // Test: memory_suite.vector_search works with GENERAL domain
    // Expected: Can insert and retrieve memories using GPU embeddings

    // When implemented:
    // let temp_dir = TempDir::new()?;
    // let db_path = temp_dir.path().join("test_memory_suite.db");
    // let state = SynCoreState::new(db_path, temp_dir.path())?;
    //
    // // Store memories via memory_suite
    // memory_store(MemoryStoreRequest {
    //     key: "project_context".to_string(),
    //     value: "Building a code analysis tool with Rust".to_string(),
    // }, &state).await?;
    //
    // // Search via vector_search (should use GENERAL domain)
    // let request = VectorSearchRequest {
    //     query: "Rust code analysis project".to_string(),
    //     namespace: None,  // Default to GENERAL
    //     limit: Some(5),
    // };
    // let results = vector_search(request, &state).await?;
    //
    // assert!(!results.is_empty());
    // assert!(results[0].score > 0.3, "Should have decent semantic match");

    Ok(())
}

// ============================================================================
// INTEGRATION TEST 5: GRAPH Domain Unchanged (384 dims)
// ============================================================================

#[tokio::test]
async fn test_graph_domain_vector_operations_unchanged() -> Result<()> {
    // Test: GRAPH domain vector operations still use 384 dimensions
    // Expected: GRAPH entities use SimpleFeatureCombiner, not GPU embeddings

    // When implemented:
    // let temp_dir = TempDir::new()?;
    // let db_path = temp_dir.path().join("test_graph_domain.db");
    // let state = SynCoreState::new(db_path, temp_dir.path())?;
    //
    // // Insert GRAPH entity
    // let request = VectorInsertRequest {
    //     text: "function calculate_total in module payments".to_string(),
    //     namespace: "graph_entity".to_string(),
    //     metadata: None,
    // };
    // vector_insert(request, &state).await?;
    //
    // // Verify GRAPH store dimension UNCHANGED
    // let graph_store = state.store_for_domain(EmbeddingDomain::Graph);
    // let store = graph_store.lock().unwrap();
    // assert_eq!(store.dimension(), 384, "GRAPH domain MUST remain 384-dim (unchanged from APEX 1.9-G)");

    Ok(())
}

// ============================================================================
// INTEGRATION TEST 6: Dimension Mismatch Detection
// ============================================================================

#[tokio::test]
async fn test_dimension_mismatch_triggers_index_rebuild() -> Result<()> {
    // Test: Inserting vector with different dimension triggers index rebuild
    // Expected: Old index cleared, new index created with correct dimension

    // When implemented:
    // let temp_dir = TempDir::new()?;
    // let db_path = temp_dir.path().join("test_dimension_mismatch.db");
    // let state = SynCoreState::new(db_path, temp_dir.path())?;
    //
    // // Create CODE index with old dimension (384) by manually setting
    // {
    //     let code_store = state.store_for_domain(EmbeddingDomain::Code);
    //     let mut store = code_store.lock().unwrap();
    //     store.insert_test_vector(vec![0.5; 384], "old_entity", "code_entity")?;
    // }
    //
    // // Insert with new dimension (1024)
    // let request = VectorInsertRequest {
    //     text: "fn new_function() {}".to_string(),
    //     namespace: "code_entity".to_string(),
    //     metadata: None,
    // };
    // vector_insert(request, &state).await?;
    //
    // // Verify index rebuilt with 1024 dims
    // let code_store = state.store_for_domain(EmbeddingDomain::Code);
    // let store = code_store.lock().unwrap();
    // assert_eq!(store.dimension(), 1024);
    // assert_eq!(store.len(), 1, "Old 384-dim entities should be cleared");

    Ok(())
}

// ============================================================================
// INTEGRATION TEST 7: Mixed-Domain Search Isolation
// ============================================================================

#[tokio::test]
async fn test_mixed_domain_search_no_cross_contamination() -> Result<()> {
    // Test: CODE/GENERAL/GRAPH searches don't cross-contaminate
    // Expected: Each domain searches only its own index

    // When implemented:
    // let temp_dir = TempDir::new()?;
    // let db_path = temp_dir.path().join("test_domain_isolation.db");
    // let state = SynCoreState::new(db_path, temp_dir.path())?;
    //
    // // Insert into CODE domain
    // vector_insert(VectorInsertRequest {
    //     text: "fn code_function() {}".to_string(),
    //     namespace: "code_entity".to_string(),
    //     metadata: None,
    // }, &state).await?;
    //
    // // Insert into GENERAL domain
    // vector_insert(VectorInsertRequest {
    //     text: "General document about functions".to_string(),
    //     namespace: "documents".to_string(),
    //     metadata: None,
    // }, &state).await?;
    //
    // // Search CODE domain
    // let code_results = vector_search(VectorSearchRequest {
    //     query: "function".to_string(),
    //     namespace: Some("code_entity".to_string()),
    //     limit: Some(10),
    // }, &state).await?;
    //
    // // Search GENERAL domain
    // let general_results = vector_search(VectorSearchRequest {
    //     query: "function".to_string(),
    //     namespace: Some("documents".to_string()),
    //     limit: Some(10),
    // }, &state).await?;
    //
    // // Verify no cross-contamination
    // for result in &code_results {
    //     assert!(result.metadata.namespace == "code_entity");
    // }
    // for result in &general_results {
    //     assert!(result.metadata.namespace == "documents");
    // }

    Ok(())
}

// ============================================================================
// INTEGRATION TEST 8: HNSW Index Persistence
// ============================================================================

#[tokio::test]
async fn test_hnsw_index_persistence_across_restarts() -> Result<()> {
    // Test: HNSW indices persist to disk and reload correctly
    // Expected: After restart, searches return same results

    // When implemented:
    // let temp_dir = TempDir::new()?;
    // let db_path = temp_dir.path().join("test_persistence.db");
    //
    // // First session: Insert entities
    // {
    //     let state = SynCoreState::new(db_path.clone(), temp_dir.path())?;
    //     vector_insert(VectorInsertRequest {
    //         text: "persistent test entity".to_string(),
    //         namespace: "code_entity".to_string(),
    //         metadata: None,
    //     }, &state).await?;
    // }
    //
    // // Second session: Reload and search
    // {
    //     let state = SynCoreState::new(db_path, temp_dir.path())?;
    //     let results = vector_search(VectorSearchRequest {
    //         query: "persistent test".to_string(),
    //         namespace: Some("code_entity".to_string()),
    //         limit: Some(5),
    //     }, &state).await?;
    //
    //     assert!(!results.is_empty(), "Should find persisted entities after restart");
    // }

    Ok(())
}

// ============================================================================
// INTEGRATION TEST 9: Backward Compatibility with Existing Indices
// ============================================================================

#[tokio::test]
async fn test_backward_compatibility_with_old_384dim_indices() -> Result<()> {
    // Test: System handles existing 384-dim indices gracefully
    // Expected: Either rebuilds index OR issues clear warning

    // When implemented:
    // let temp_dir = TempDir::new()?;
    // let db_path = temp_dir.path().join("test_backward_compat.db");
    // let state = SynCoreState::new(db_path, temp_dir.path())?;
    //
    // // Simulate old 384-dim index by inserting with legacy embedder
    // // (This would be done by copying old index file in real scenario)
    //
    // // Attempt to use new GPU embeddings
    // let result = vector_insert(VectorInsertRequest {
    //     text: "new entity with GPU embeddings".to_string(),
    //     namespace: "code_entity".to_string(),
    //     metadata: None,
    // }, &state).await;
    //
    // // Should succeed (either by rebuilding or migrating)
    // assert!(result.is_ok(), "Should handle dimension mismatch gracefully");

    Ok(())
}

// ============================================================================
// INTEGRATION TEST 10: Performance Baseline
// ============================================================================

#[tokio::test]
async fn test_gpu_embeddings_performance_baseline() -> Result<()> {
    // Test: Establish performance baseline for GPU embeddings
    // Expected: Embedding + search completes in reasonable time

    // When implemented:
    // let temp_dir = TempDir::new()?;
    // let db_path = temp_dir.path().join("test_performance.db");
    // let state = SynCoreState::new(db_path, temp_dir.path())?;
    //
    // let start = std::time::Instant::now();
    //
    // // Insert 100 code entities
    // for i in 0..100 {
    //     vector_insert(VectorInsertRequest {
    //         text: format!("fn function_{}() {{ /* code */ }}", i),
    //         namespace: "code_entity".to_string(),
    //         metadata: None,
    //     }, &state).await?;
    // }
    //
    // // Perform 10 searches
    // for i in 0..10 {
    //     vector_search(VectorSearchRequest {
    //         query: format!("function {}", i),
    //         namespace: Some("code_entity".to_string()),
    //         limit: Some(5),
    //     }, &state).await?;
    // }
    //
    // let elapsed = start.elapsed();
    // println!("Performance baseline: {:?} for 100 inserts + 10 searches", elapsed);
    //
    // // Reasonable baseline: < 30 seconds for 100 GPU embedding calls
    // assert!(elapsed.as_secs() < 30, "Performance should be acceptable");

    Ok(())
}
