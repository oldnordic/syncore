//! Integration Tests for Real Code Graph Fusion Query
//!
//! These tests verify that code_graph_fusion_query returns REAL data from indexed files,
//! never mock data or placeholders.

use anyhow::Result;
use serde_json::json;
use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::router::SynCoreState;
    use crate::db::DbManager;
    use tempfile::TempDir;
    use std::path::Path;

    /// Test that fusion query returns real indexed files, not mock data
    #[tokio::test]
    async fn test_fusion_query_returns_real_data() -> Result<()> {
        // Setup temporary database
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");

        let db_manager = Arc::new(DbManager::new(&db_path.to_string_lossy())?);
        let state = create_test_state(db_manager).await?;

        // Verify we have real indexed data
        let check_result = state.db_manager.code_graph_conn().query_row(
            "SELECT COUNT(*) FROM code_entities WHERE file_path LIKE '%src%'",
            [],
            |row| row.get::<_, i64>(0),
        )?;

        assert!(check_result > 0, "Test requires indexed src files");

        // Execute fusion query
        let fusion_params = json!({
            "query": "router dependencies",
            "mode_hint": "simple",
            "top_k": 5,
            "scope": "project"
        });

        let result = state.mcp_delegate("code_graph_fusion_query", fusion_params).await?;

        // CRITICAL: Verify no mock data
        assert!(!result.get("results").unwrap().as_array().unwrap().is_empty(),
                "Fusion query must return real results");

        // Verify no mock paths
        let results = result.get("results").unwrap().as_array().unwrap();
        for item in results {
            let file_path = item.get("file_path").unwrap().as_str().unwrap();
            assert!(!file_path.contains("/mock/"),
                   "File path must not contain mock: {}", file_path);
            assert!(!file_path.contains("mock_result"),
                   "File path must not contain mock_result: {}", file_path);
            assert!(Path::new(file_path).exists(),
                   "File path must exist: {}", file_path);
        }

        Ok(())
    }

    /// Test that fusion query returns different results for different queries
    #[tokio::test]
    async fn test_fusion_query_returns_different_results() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");

        let db_manager = Arc::new(DbManager::new(&db_path.to_string_lossy())?);
        let state = create_test_state(db_manager).await?;

        // Query 1: router
        let router_params = json!({
            "query": "router",
            "mode_hint": "simple",
            "top_k": 3,
            "scope": "project"
        });

        let router_result = state.mcp_delegate("code_graph_fusion_query", router_params).await?;
        let router_results = router_result.get("results").unwrap().as_array().unwrap();

        // Query 2: vector
        let vector_params = json!({
            "query": "vector search",
            "mode_hint": "simple",
            "top_k": 3,
            "scope": "project"
        });

        let vector_result = state.mcp_delegate("code_graph_fusion_query", vector_params).await?;
        let vector_results = vector_result.get("results").unwrap().as_array().unwrap();

        // Results should be different
        assert!(!router_results.is_empty(), "Router query must return results");
        assert!(!vector_results.is_empty(), "Vector query must return results");

        // Extract file paths and verify they're different
        let mut router_paths: Vec<String> = router_results.iter()
            .map(|r| r.get("file_path").unwrap().as_str().unwrap().to_string())
            .collect();
        let mut vector_paths: Vec<String> = vector_results.iter()
            .map(|r| r.get("file_path").unwrap().as_str().unwrap().to_string())
            .collect();

        router_paths.sort();
        vector_paths.sort();

        assert_ne!(router_paths, vector_paths,
                 "Different queries should return different results");

        Ok(())
    }

    /// Test that fusion query respects scope parameter
    #[tokio::test]
    async fn test_fusion_query_respects_scope() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");

        let db_manager = Arc::new(DbManager::new(&db_path.to_string_lossy())?);
        let state = create_test_state(db_manager).await?;

        // Local scope query
        let local_params = json!({
            "query": "router",
            "mode_hint": "simple",
            "top_k": 10,
            "scope": "local",
            "local_root": "src/router.rs"
        });

        let local_result = state.mcp_delegate("code_graph_fusion_query", local_params).await?;
        let local_results = local_result.get("results").unwrap().as_array().unwrap();

        // Project scope query
        let project_params = json!({
            "query": "router",
            "mode_hint": "simple",
            "top_k": 10,
            "scope": "project"
        });

        let project_result = state.mcp_delegate("code_graph_fusion_query", project_params).await?;
        let project_results = project_result.get("results").unwrap().as_array().unwrap();

        // Local scope should be more specific
        assert!(!local_results.is_empty(), "Local scope must return results");
        assert!(!project_results.is_empty(), "Project scope must return results");

        // Verify local results are from the specific file
        for item in local_results {
            let file_path = item.get("file_path").unwrap().as_str().unwrap();
            assert!(file_path.contains("src/router.rs"),
                   "Local scope should return only router.rs: {}", file_path);
        }

        Ok(())
    }

    /// Test that fusion query never returns hardcoded mock strings
    #[tokio::test]
    async fn test_fusion_query_no_hardcoded_mocks() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");

        let db_manager = Arc::new(DbManager::new(&db_path.to_string_lossy())?);
        let state = create_test_state(db_manager).await?;

        let test_queries = vec![
            "function", "class", "module", "test", "example"
        ];

        for query in test_queries {
            let params = json!({
                "query": query,
                "mode_hint": "simple",
                "top_k": 5,
                "scope": "project"
            });

            let result = state.mcp_delegate("code_graph_fusion_query", params).await?;
            let results = result.get("results").unwrap().as_array().unwrap();

            // Check for known mock strings
            for item in results {
                let file_path = item.get("file_path").unwrap().as_str().unwrap();
                let id = item.get("id").unwrap().as_str().unwrap();

                // CRITICAL: These must never appear in production
                assert_ne!(id, "mock_result_1", "Found hardcoded mock_result_1");
                assert!(!file_path.contains("/mock/path.rs"), "Found hardcoded mock path");
                assert!(!file_path.contains("mock"), "File path contains 'mock': {}", file_path);
            }
        }

        Ok(())
    }

    /// Helper to create test state with minimal dependencies
    async fn create_test_state(db_manager: Arc<DbManager>) -> Result<SynCoreState> {
        use crate::memory::Memory;
        use crate::tasks::Tasks;
        use crate::vector::dual_service::DualEmbeddingService;
        use crate::vector::{VectorStore, SearchScope};

        let memory = Arc::new(Memory::new(":memory:")?);
        let tasks = Arc::new(Tasks::new(":memory:")?);
        let code_store = Arc::new(std::sync::Mutex::new(VectorStore::new(
            Box::new(crate::vector::RealEmbeddings::new(384)?)
        )));
        let general_store = Arc::new(std::sync::Mutex::new(VectorStore::new(
            Box::new(crate::vector::RealEmbeddings::new(384)?)
        )));

        Ok(SynCoreState {
            db_manager,
            memory,
            tasks,
            code_store,
            general_store,
            config: Default::default(),
            neo4j: None,
            message_bus: Default::default(),
            circuit_breaker: Default::default(),
            logger: Default::default(),
            snapshots: Default::default(),
            vector_service: Arc::new(DualEmbeddingService::new(
                code_store.clone(),
                general_store.clone(),
            )),
            llm_model: None,
        })
    }
}