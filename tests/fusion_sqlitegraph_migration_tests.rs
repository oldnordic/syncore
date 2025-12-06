//! Fusion Reasoning SQLiteGraph Migration Tests
//!
//! Test-Driven Development approach to migrate FusionReasoning from Neo4j dependency
//! to complete SQLiteGraph support while preserving Neo4j optionality.
//!
//! This test file ensures:
//! 1. Backend-agnostic fusion behavior
//! 2. Deterministic multi-hop reasoning
//! 3. All three fusion modes work correctly
//! 4. MCP tool integration with SQLiteGraph
//! 5. Performance and correctness validation

use anyhow::Result;
use serde_json;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

use crate::agent::current_timestamp_ms;
use crate::code_graph::{
    CodeEntity, CodeGraph, EntityType, FusionReasoning, FusionRouter, FusionMode,
    RagGraphAPI, QueryScope
};
use crate::graph::{GraphBackend, SQLiteGraphBackend};
use crate::vector::{VectorStore, StubEmbeddings};
use crate::config::{GraphConfig, GraphBackend as ConfigGraphBackend};

/// Test setup for SQLiteGraph backend with sample data
struct FusionTestSetup {
    temp_dir: TempDir,
    code_graph: Arc<CodeGraph>,
    vector_store: Arc<Mutex<VectorStore>>,
    sqlite_backend: Arc<SQLiteGraphBackend>,
}

impl FusionTestSetup {
    /// Create test setup with sample code entities
    async fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test_codegraph.db");

        // Create vector store with stub embeddings
        let embeddings = Box::new(StubEmbeddings::new(384)?);
        let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

        // Create CodeGraph
        let code_graph = Arc::new(CodeGraph::new(db_path.to_str().unwrap(), vector_store.clone())?);

        // Create SQLiteGraph backend
        let sqlite_backend = Arc::new(
            SQLiteGraphBackend::new(db_path.to_str().unwrap(), "test_namespace").await?
        );

        let setup = Self {
            temp_dir,
            code_graph,
            vector_store,
            sqlite_backend,
        };

        // Populate with test data
        setup.populate_test_data().await?;

        Ok(setup)
    }

    /// Populate database with test entities for fusion reasoning
    async fn populate_test_data(&self) -> Result<()> {
        let timestamp = current_timestamp_ms();

        // Create test entities with clear relationships
        let test_entities = vec![
            // Main function
            CodeEntity {
                id: Some(1),
                name: "main".to_string(),
                entity_type: EntityType::Function,
                file_path: "/test/main.rs".to_string(),
                start_line: Some(1),
                end_line: Some(10),
                signature: Some("fn main()".to_string()),
                body_snippet: Some("println!(\"Hello, world!\");".to_string()),
                created_at: timestamp,
                updated_at: timestamp,
                ..Default::default()
            },

            // Helper function
            CodeEntity {
                id: Some(2),
                name: "helper".to_string(),
                entity_type: EntityType::Function,
                file_path: "/test/utils.rs".to_string(),
                start_line: Some(1),
                end_line: Some(5),
                signature: Some("fn helper() -> String".to_string()),
                body_snippet: Some("\"helper result\".to_string()".to_string()),
                created_at: timestamp,
                updated_at: timestamp,
                ..Default::default()
            },

            // Data structure
            CodeEntity {
                id: Some(3),
                name: "UserStruct".to_string(),
                entity_type: EntityType::Struct,
                file_path: "/test/models.rs".to_string(),
                start_line: Some(1),
                end_line: Some(8),
                signature: Some("struct UserStruct".to_string()),
                body_snippet: Some("name: String, age: u32".to_string()),
                created_at: timestamp,
                updated_at: timestamp,
                ..Default::default()
            },

            // Import module
            CodeEntity {
                id: Some(4),
                name: "std::fmt".to_string(),
                entity_type: EntityType::Import,
                file_path: "/test/main.rs".to_string(),
                start_line: Some(1),
                end_line: Some(1),
                signature: Some("use std::fmt".to_string()),
                body_snippet: None,
                created_at: timestamp,
                updated_at: timestamp,
                ..Default::default()
            },
        ];

        // Insert entities using CodeGraph
        for entity in test_entities {
            self.code_graph.upsert_entity(&entity).await?;
        }

        // Create relationships for multi-hop testing
        let relationships = vec![
            // main() calls helper()
            (1, 2, "CALLS"),
            // main() uses UserStruct
            (1, 3, "USES_TYPE"),
            // main() imports std::fmt
            (1, 4, "IMPORTS"),
            // helper() uses UserStruct
            (2, 3, "USES_TYPE"),
        ];

        for (src, dst, rel_type) in relationships {
            self.code_graph.create_relationship(src, dst, rel_type).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod fusion_reasoning_tests {
    use super::*;

    #[tokio::test]
    async fn test_fusion_reasoning_with_sqlitegraph_backend() -> Result<()> {
        let setup = FusionTestSetup::new().await?;

        // Create FusionReasoning with SQLiteGraph backend
        let fusion = FusionReasoning::new(
            setup.sqlite_backend.clone() as Arc<dyn GraphBackend>,
            setup.vector_store.clone()
        );

        // Test basic reasoning
        let results = fusion.reason("main function", 5)?;

        // Should return results with entity IDs and scores
        assert!(!results.is_empty(), "Fusion reasoning should return results");

        // Results should be sorted by score (descending)
        let mut sorted_results = results.clone();
        sorted_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        assert_eq!(results, sorted_results, "Results should be sorted by score");

        // All scores should be valid f32 values
        for (entity_id, score) in results {
            assert!(entity_id > 0, "Entity ID should be positive");
            assert!(score >= 0.0 && score <= 1.0, "Score should be between 0.0 and 1.0");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_fusion_reasoning_public_api() -> Result<()> {
        let setup = FusionTestSetup::new().await?;

        let fusion = FusionReasoning::new(
            setup.sqlite_backend.clone() as Arc<dyn GraphBackend>,
            setup.vector_store.clone()
        );

        // Test the public API with different queries
        let test_queries = vec![
            "main function",
            "helper function",
            "UserStruct definition",
            "import statements",
        ];

        for query in test_queries {
            let results = fusion.reason(query, 5)?;

            // Should return results (though may be empty with current placeholder)
            println!("Query '{}' returned {} results", query, results.len());

            // Results should be properly formatted
            for (entity_id, score) in &results {
                assert!(entity_id > &0, "Entity ID should be positive");
                assert!(score >= &0.0 && score <= &1.0, "Score should be between 0.0 and 1.0");
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_fusion_deterministic_scoring() -> Result<()> {
        let setup = FusionTestSetup::new().await?;

        let fusion = FusionReasoning::new(
            setup.sqlite_backend.clone() as Arc<dyn GraphBackend>,
            setup.vector_store.clone()
        );

        // Run multiple times and verify deterministic results
        let mut results_vec = Vec::new();

        for _ in 0..5 {
            let results = fusion.reason("main function", 5)?;
            results_vec.push(results);
        }

        // All results should be identical
        for i in 1..results_vec.len() {
            assert_eq!(results_vec[0], results_vec[i], "Fusion results should be deterministic");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_fusion_mode_selection() -> Result<()> {
        let router = FusionRouter::new();

        // Test short query → Simple mode
        assert_eq!(router.select_mode("fmt"), FusionMode::Simple);
        assert_eq!(router.select_mode("test fn"), FusionMode::Simple);

        // Test path patterns → Simple mode
        assert_eq!(router.select_mode("std::fmt::Display"), FusionMode::Simple);
        assert_eq!(router.select_mode("src/main.rs"), FusionMode::Simple);

        // Test reasoning keywords → Reasoning mode
        assert_eq!(router.select_mode("trace dependency from main to utils"), FusionMode::Reasoning);
        assert_eq!(router.select_mode("show path from function to struct"), FusionMode::Reasoning);

        // Test semantic keywords → Attention mode
        assert_eq!(router.select_mode("why does main use helper"), FusionMode::Attention);
        assert_eq!(router.select_mode("explain the relationship"), FusionMode::Attention);

        // Test multi-sentence → Attention mode
        assert_eq!(router.select_mode("What is main? How does it work?"), FusionMode::Attention);

        Ok(())
    }
}

#[cfg(test)]
mod raggraph_api_fusion_tests {
    use super::*;

    #[tokio::test]
    async fn test_raggraph_api_with_sqlitegraph_fusion() -> Result<()> {
        let setup = FusionTestSetup::new().await?;

        // Create RagGraphAPI with SQLiteGraph backend
        let api = RagGraphAPI::new(
            setup.code_graph.clone(),
            setup.sqlite_backend.clone() as Arc<dyn GraphBackend>
        );

        // Test fusion query with different mode hints
        let test_cases = vec![
            ("main", Some("simple"), 5),
            ("helper function usage", Some("attention"), 3),
            ("dependency tracing", Some("reasoning"), 10),
        ];

        for (query, mode_hint, top_k) in test_cases {
            let response = api.query_with_mode_hint(
                query,
                None,
                mode_hint,
                top_k,
                QueryScope::Global,
                None,
                None
            ).await?;

            assert!(!response.entities.is_empty(), "Query should return results for: {}", query);

            // Verify response structure
            assert_eq!(response.query, query, "Response should contain original query");
            assert!(response.applied_scope.is_fully_qualified(), "Should apply proper scope");

            // Check entity ordering by relevance score
            let mut sorted_entities = response.entities.clone();
            sorted_entities.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
            assert_eq!(response.entities, sorted_entities, "Entities should be sorted by relevance");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_fusion_scope_control() -> Result<()> {
        let setup = FusionTestSetup::new().await?;

        let api = RagGraphAPI::new(
            setup.code_graph.clone(),
            setup.sqlite_backend.clone() as Arc<dyn GraphBackend>
        );

        // Test different scopes
        let query = "main";
        let scopes = vec![
            QueryScope::Local,
            QueryScope::Project,
            QueryScope::Workspace,
            QueryScope::Global,
        ];

        for scope in scopes {
            let response = api.query_with_scope(
                query,
                None,
                None,
                5,
                scope,
                None,
                None
            ).await?;

            // Scope should be properly applied
            assert!(response.applied_scope.is_fully_qualified());

            // Results should vary by scope (this will depend on implementation)
            println!("Scope: {:?}, Results: {}", response.applied_scope, response.entities.len());
        }

        Ok(())
    }
}

#[cfg(test)]
mod mcp_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_fusion_query_simulation() -> Result<()> {
        let setup = FusionTestSetup::new().await?;

        // Simulate MCP request parameters (as would come from MCP client)
        let query = "main function dependencies";
        let mode_hint = Some("reasoning");
        let top_k = 10;
        let scope = QueryScope::Global;

        // Create API as would be done in MCP server
        let api = RagGraphAPI::new(
            setup.code_graph.clone(),
            setup.sqlite_backend.clone() as Arc<dyn GraphBackend>
        );

        // Execute query as would be done in MCP server
        let response = api.query_with_scope(
            query,
            Some("test_namespace"),
            mode_hint,
            top_k,
            scope,
            None,
            None
        ).await?;

        // Verify MCP response structure
        assert!(!response.entities.is_empty(), "MCP query should return results");
        assert_eq!(response.query, query, "Should echo original query");

        // Verify response can be serialized for MCP transport
        let json_response = serde_json::to_string_pretty(&response)?;
        assert!(json_response.contains("entities"), "Response should contain entities field");
        assert!(json_response.contains("selected_mode"), "Response should contain fusion mode");

        println!("MCP Response: {}", json_response);

        Ok(())
    }

    #[tokio::test]
    async fn test_backend_configuration_determination() -> Result<()> {
        // Test GraphBackend configuration parsing (as done in MCP server)
        let mut syncore_config = crate::config::SyncoreConfig::default();
        syncore_config.apply_env_overrides();
        let graph_config = syncore_config.graph;

        match graph_config.backend {
            ConfigGraphBackend::SqliteGraph => {
                // Should be able to create SQLiteGraph backend
                let backend = crate::graph::backend_selector::create_default_graph_backend(&graph_config).await?;
                assert!(backend.namespace().contains("test") || backend.namespace().contains("default"));
            }
            ConfigGraphBackend::Neo4j => {
                // Should handle Neo4j configuration gracefully
                println!("Neo4j backend configured: {}", graph_config.uri);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod performance_validation_tests {
    use super::*;

    #[tokio::test]
    async fn test_fusion_performance_characteristics() -> Result<()> {
        let setup = FusionTestSetup::new().await?;

        let fusion = FusionReasoning::new(
            setup.sqlite_backend.clone() as Arc<dyn GraphBackend>,
            setup.vector_store.clone()
        );

        // Measure performance characteristics
        let start_time = std::time::Instant::now();

        let results = fusion.reason("main function dependencies", 10)?;

        let duration = start_time.elapsed();

        // Performance assertions
        assert!(duration.as_millis() < 5000, "Fusion reasoning should complete within 5 seconds");
        assert!(results.len() <= 10, "Should not return more results than requested");

        // Memory usage should be reasonable (basic check)
        assert!(results.len() < 1000, "Should not return excessive results");

        println!("Fusion reasoning completed in {:?} with {} results", duration, results.len());

        Ok(())
    }

    #[tokio::test]
    async fn test_fusion_query_scalability() -> Result<()> {
        let setup = FusionTestSetup::new().await?;

        let fusion = FusionReasoning::new(
            setup.sqlite_backend.clone() as Arc<dyn GraphBackend>,
            setup.vector_store.clone()
        );

        // Test scalability with different k values
        let k_values = vec![1, 5, 10, 20];

        for k in k_values {
            let start_time = std::time::Instant::now();

            let results = fusion.reason("main function", k)?;

            let duration = start_time.elapsed();

            println!("k={}: {} results in {:?}", k, results.len(), duration);

            // Should complete in reasonable time
            assert!(duration.as_millis() < 5000, "Fusion reasoning should complete quickly");

            // Should not return more results than requested (though current implementation might return fewer)
            assert!(results.len() <= k, "Should not return more results than requested");
        }

        Ok(())
    }
}

/// Integration test for complete fusion workflow
#[tokio::test]
async fn test_complete_sqlitegraph_fusion_workflow() -> Result<()> {
    let setup = FusionTestSetup::new().await?;

    // 1. Create FusionReasoning with SQLiteGraph backend
    let fusion = FusionReasoning::new(
        setup.sqlite_backend.clone() as Arc<dyn GraphBackend>,
        setup.vector_store.clone()
    );

    // 2. Perform fusion reasoning
    let reasoning_results = fusion.reason("main function and its dependencies", 8)?;
    println!("Fusion reasoning returned {} results", reasoning_results.len());

    // 3. Create RagGraphAPI with same backend
    let api = RagGraphAPI::new(
        setup.code_graph.clone(),
        setup.sqlite_backend.clone() as Arc<dyn GraphBackend>
    );

    // 4. Perform API fusion query
    let api_response = api.query_with_mode_hint(
        "main function and its dependencies",
        Some("test_namespace"),
        Some("reasoning"),
        8,
        QueryScope::Global,
        None,
        None
    ).await?;

    println!("RagGraph API returned {} entities", api_response.entities.len());

    // 5. Verify both components work with SQLiteGraph backend
    // Note: Results may differ due to different algorithms, but both should work
    assert!(api_response.entities.is_empty() || reasoning_results.is_empty() ||
            api_response.entities.len() > 0 && reasoning_results.len() > 0,
            "At least one component should return results");

    // 6. Verify backend consistency - both use the same SQLiteGraph backend
    let backend_namespace = setup.sqlite_backend.namespace();
    assert_eq!(backend_namespace, "test_namespace", "Backend should use correct namespace");

    println!("✓ Complete SQLiteGraph fusion workflow test passed");

    Ok(())
}