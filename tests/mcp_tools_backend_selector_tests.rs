//! TDD Tests for MCP Tools Backend Selector Integration
//!
//! Tests that MCP tools use GraphBackendSelector instead of direct Neo4jClient

use std::sync::{Arc, Mutex};
use syncore::config::{GraphBackend as ConfigBackend, GraphConfig};
use syncore::db::DbManager;
use syncore::graph::create_graph_backend;
use syncore::mcp_tools::graph_suite::{GraphSuite, GraphSuiteArgs};
use syncore::memory::Memory;
use syncore::router::SynCoreState;
use syncore::snapshots::SnapshotHandle;
use syncore::tasks::Tasks;
use syncore::vector::{StubEmbeddings, VectorStore};
use tempfile;

// Global state for testing
static mut TEST_STATE: Option<Arc<SynCoreState>> = None;

fn init_test_state() -> Arc<SynCoreState> {
    unsafe {
        if TEST_STATE.is_none() {
            // Create temp directory for test databases
            let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
            let db_path = temp_dir.path().join("test.db");
            let code_graph_db_path = temp_dir.path().join("code_graph.db");

            // Create DbManager with long-lived connections
            let db_manager = Arc::new(
                DbManager::new(db_path.to_str().unwrap(), code_graph_db_path.to_str().unwrap())
                    .expect("Failed to create test DbManager"),
            );

            // Create graph backend using selector
            let graph_config = GraphConfig {
                backend: ConfigBackend::SqliteGraph,
                path: db_path.to_str().unwrap().to_string(),
                uri: String::new(),
                user: String::new(),
                password: String::new(),
                enabled: true,
            };

            let graph_backend = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current()
                    .block_on(async { create_graph_backend(&graph_config, "test_namespace").await })
            })
            .expect("Failed to create graph backend");

            // Create minimal components
            let memory = Memory::with_connection(db_manager.main_conn(), "test_cache")
                .expect("Failed to create Memory");
            let tasks =
                Tasks::with_connection(db_manager.main_conn()).expect("Failed to create Tasks");
            let code_store =
                Arc::new(Mutex::new(VectorStore::new(Box::new(StubEmbeddings::new(384).unwrap()))));
            let general_store = Arc::clone(&code_store);

            let mut state = SynCoreState::with_dual_stores(code_store, general_store).unwrap();

            state.logger = Arc::new(syncore::logger::MarkdownLogger::new("./logs"));

            state.graph_backend = None;

            // Keep temp_dir alive
            std::mem::forget(temp_dir);
            TEST_STATE = Some(Arc::new(state));
        }
        TEST_STATE.as_ref().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_graph_suite_uses_backend_selector() {
        let state = init_test_state();
        let suite = GraphSuite::new((*state).clone());

        // Test that GraphSuite can be created with backend selector
        assert!(state.graph_backend.is_some(), "Graph backend should be set");
        assert!(state.neo4j.is_none(), "Neo4j should be None when using SQLiteGraph");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_graph_suite_query_with_backend_selector() {
        let state = init_test_state();
        let suite = GraphSuite::new((*state).clone());

        // Test query command
        let args = GraphSuiteArgs {
            command: "query".to_string(),
            cypher: Some("MATCH (n) RETURN count(n) as count".to_string()),
            params: None,
            from_id: None,
            to_id: None,
            rel_type: None,
            from_label: None,
            to_label: None,
            query_text: None,
            seed_nodes: None,
        };

        let result = suite.execute(args);

        // Should succeed with SQLiteGraph backend
        assert!(
            result.success,
            "Query should succeed with SQLiteGraph backend: {:?}",
            result.error
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_graph_suite_relate_with_backend_selector() {
        let state = init_test_state();
        let suite = GraphSuite::new((*state).clone());

        // Test relate command
        let args = GraphSuiteArgs {
            command: "relate".to_string(),
            cypher: None,
            params: None,
            from_id: Some(1),
            to_id: Some(2),
            rel_type: Some("CALLS".to_string()),
            from_label: None,
            to_label: None,
            query_text: None,
            seed_nodes: None,
        };

        let result = suite.execute(args);

        // Should handle gracefully (foreign key error is expected when entities don't exist)
        // The important thing is that it uses the backend selector instead of direct Neo4j
        assert!(
            !result.success,
            "Relate should fail gracefully with non-existent entities: {:?}",
            result.error
        );
        assert!(
            result.error.as_ref().unwrap().contains("FOREIGN KEY"),
            "Should be foreign key error"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_backend_selector_fallback_to_neo4j() {
        // This test would require Neo4j to be running
        // For now, just test that the selector can handle Neo4j config
        let config = GraphConfig {
            backend: ConfigBackend::Neo4j,
            uri: "bolt://localhost:7687".to_string(),
            user: "neo4j".to_string(),
            password: "password".to_string(),
            path: String::new(),
            enabled: true,
        };

        // This would fail in test environment without Neo4j, but should not panic
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { create_graph_backend(&config, "test_namespace").await })
        });

        // Should either fail (no Neo4j) or succeed (Neo4j available)
        // The important thing is that it doesn't panic and handles both cases gracefully
        match result {
            Ok(_) => {
                // Neo4j is available - that's fine
                println!("Neo4j backend connected successfully");
            }
            Err(e) => {
                // Neo4j not available - expected in test environment
                assert!(
                    e.to_string().contains("Failed to connect")
                        || e.to_string().contains("connection"),
                    "Should fail with connection error, got: {}",
                    e
                );
            }
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_mcp_tools_backward_compatibility() {
        let state = init_test_state();

        // Test that existing MCP tools still work with backend selector
        assert!(state.graph_backend.is_some(), "Graph backend should be available");

        // The backend should be usable for graph operations
        if let Some(backend) = &state.graph_backend {
            // Test basic backend functionality
            let result = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current()
                    .block_on(async { backend.validate_structure().await })
            });

            // Should succeed with SQLiteGraph
            assert!(result.is_ok(), "Backend should validate structure: {:?}", result.err());
        }
    }
}
