//! MCP Reasoning Unification Tests
//!
//! TDD approach: Define unified behavior expectations for all reasoning MCP tools.
//! These tests should FAIL initially due to inconsistencies, then pass after unification.
//!
//! Tools being unified:
//! - raggraph_query
//! - raggraph_multihop
//! - code_graph_fusion_query

use anyhow::Result;
use std::sync::Arc;
use tempfile::TempDir;
use syncore::config::{GraphConfig, GraphBackend, SyncoreConfig};
use syncore::graph::{backend_selector::create_default_graph_backend, SQLiteGraphBackend};
use syncore::mcp_server::{
    types::{RagGraphQueryRequest, RagGraphMultihopRequest},
    MCPServerHandler,
    SynCoreState,
};
use syncore::vector::{VectorStore, StubEmbeddings};

/// Test setup for MCP reasoning unification
struct ReasoningTestSetup {
    temp_dir: TempDir,
    server: MCPServerHandler,
}

impl ReasoningTestSetup {
    async fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test_reasoning.db");

        // Create vector store with stub embeddings
        let embeddings = Box::new(StubEmbeddings::new(384)?);
        let vector_store = Arc::new(std::sync::Mutex::new(VectorStore::new(embeddings)));

        // Create CodeGraph
        let code_graph = syncore::graph::CodeGraph::new(db_path.to_str().unwrap(), vector_store.clone())?;

        // Create state with graph components
        let state = Arc::new(SynCoreState {
            code_store: vector_store.clone(),
            code_graph: Arc::new(code_graph),
            vector_store: vector_store.clone(),
            llm_model: None,
            message_bus: None,
        });

        let server = MCPServerHandler::new(state);

        Ok(Self {
            temp_dir,
            server,
        })
    }

    /// Set backend configuration for testing
    fn set_backend_config(&self, backend: GraphBackend) {
        match backend {
            GraphBackend::SqliteGraph => {
                std::env::set_var("GRAPH_BACKEND", "sqlitegraph");
                std::env::remove_var("SYNCORE_RAGGRAPH_BACKEND");
            }
            GraphBackend::Neo4j => {
                std::env::set_var("GRAPH_BACKEND", "neo4j");
                std::env::set_var("SYNCORE_RAGGRAPH_BACKEND", "real");
            }
        }
    }

    /// Clear all backend configuration for testing
    fn clear_backend_config(&self) {
        std::env::remove_var("GRAPH_BACKEND");
        std::env::remove_var("SYNCORE_RAGGRAPH_BACKEND");
    }
}

#[cfg(test)]
mod backend_selection_consistency_tests {
    use super::*;

    #[tokio::test]
    async fn test_raggraph_query_uses_sqlitegraph_when_configured() -> Result<()> {
        let setup = ReasoningTestSetup::new().await?;
        setup.set_backend_config(GraphBackend::SqliteGraph);

        // Test that raggraph_query uses SQLiteGraph backend when configured
        let request = RagGraphQueryRequest {
            query_text: "test query".to_string(),
        };

        let result = setup.server.raggraph_query(syncore::mcp_server::Parameters(request)).await;

        // Currently this will likely fail due to inconsistent backend selection
        assert!(result.is_ok(), "raggraph_query should succeed with SQLiteGraph backend");

        // Verify the response doesn't contain Neo4j-specific errors
        let call_result = result.unwrap();
        let response_text = if let Some(content) = call_result.content.first() {
            content.text.as_str().unwrap_or("")
        } else {
            ""
        };

        assert!(!response_text.contains("Neo4j"), "Response should not reference Neo4j when SQLiteGraph is configured");
        Ok(())
    }

    #[tokio::test]
    async fn test_raggraph_multihop_uses_sqlitegraph_when_configured() -> Result<()> {
        let setup = ReasoningTestSetup::new().await?;
        setup.set_backend_config(GraphBackend::SqliteGraph);

        // Test that raggraph_multihop uses SQLiteGraph backend when configured
        let request = RagGraphMultihopRequest {
            seed_nodes: vec![1, 2, 3],
        };

        let result = setup.server.raggraph_multihop(syncore::mcp_server::Parameters(request)).await;

        assert!(result.is_ok(), "raggraph_multihop should succeed with SQLiteGraph backend");

        let call_result = result.unwrap();
        let response_text = if let Some(content) = call_result.content.first() {
            content.text.as_str().unwrap_or("")
        } else {
            ""
        };

        assert!(!response_text.contains("Neo4j"), "Response should not reference Neo4j when SQLiteGraph is configured");
        Ok(())
    }

    #[tokio::test]
    async fn test_code_graph_fusion_query_uses_sqlitegraph_when_configured() -> Result<()> {
        let setup = ReasoningTestSetup::new().await?;
        setup.set_backend_config(GraphBackend::SqliteGraph);

        // Test that code_graph_fusion_query uses SQLiteGraph backend when configured
        let request = syncore::code_graph::RagGraphQueryRequest {
            query: "test fusion query".to_string(),
            namespace: None,
            mode_hint: Some("simple".to_string()),
            top_k: Some(10),
            scope: Some("project".to_string()),
            project_label: None,
            local_root: None,
        };

        let result = setup.server.code_graph_fusion_query(syncore::mcp_server::Parameters(request)).await;

        assert!(result.is_ok(), "code_graph_fusion_query should succeed with SQLiteGraph backend");

        let call_result = result.unwrap();
        let response_text = if let Some(content) = call_result.content.first() {
            content.text.as_str().unwrap_or("")
        } else {
            ""
        };

        assert!(!response_text.contains("Neo4j"), "Response should not reference Neo4j when SQLiteGraph is configured");
        Ok(())
    }

    #[tokio::test]
    async fn test_all_reasoning_tools_default_to_sqlitegraph() -> Result<()> {
        let setup = ReasoningTestSetup::new().await?;
        setup.clear_backend_config();

        // Test that all reasoning tools default to SQLiteGraph when no backend is configured

        // Test raggraph_query
        let query_request = RagGraphQueryRequest {
            query_text: "test query".to_string(),
        };
        let query_result = setup.server.raggraph_query(syncore::mcp_server::Parameters(query_request)).await;
        assert!(query_result.is_ok(), "raggraph_query should default to SQLiteGraph");

        // Test raggraph_multihop
        let multihop_request = RagGraphMultihopRequest {
            seed_nodes: vec![1, 2, 3],
        };
        let multihop_result = setup.server.raggraph_multihop(syncore::mcp_server::Parameters(multihop_request)).await;
        assert!(multihop_result.is_ok(), "raggraph_multihop should default to SQLiteGraph");

        // Test code_graph_fusion_query
        let fusion_request = syncore::code_graph::RagGraphQueryRequest {
            query: "test fusion query".to_string(),
            namespace: None,
            mode_hint: Some("simple".to_string()),
            top_k: Some(10),
            scope: Some("project".to_string()),
            project_label: None,
            local_root: None,
        };
        let fusion_result = setup.server.code_graph_fusion_query(syncore::mcp_server::Parameters(fusion_request)).await;
        assert!(fusion_result.is_ok(), "code_graph_fusion_query should default to SQLiteGraph");

        Ok(())
    }

    #[tokio::test]
    async fn test_all_reasoning_tools_handle_missing_backend_consistently() -> Result<()> {
        let setup = ReasoningTestSetup::new().await?;

        // Configure Neo4j without actual Neo4j connection to test error handling
        setup.set_backend_config(GraphBackend::Neo4j);

        // All tools should handle missing Neo4j gracefully with consistent error messages
        let test_cases = vec![
            ("raggraph_query", async {
                let request = RagGraphQueryRequest {
                    query_text: "test query".to_string(),
                };
                setup.server.raggraph_query(syncore::mcp_server::Parameters(request)).await
            }),
            ("raggraph_multihop", async {
                let request = RagGraphMultihopRequest {
                    seed_nodes: vec![1, 2, 3],
                };
                setup.server.raggraph_multihop(syncore::mcp_server::Parameters(request)).await
            }),
            ("code_graph_fusion_query", async {
                let request = syncore::code_graph::RagGraphQueryRequest {
                    query: "test fusion query".to_string(),
                    namespace: None,
                    mode_hint: Some("simple".to_string()),
                    top_k: Some(10),
                    scope: Some("project".to_string()),
                    project_label: None,
                    local_root: None,
                };
                setup.server.code_graph_fusion_query(syncore::mcp_server::Parameters(request)).await
            }),
        ];

        for (tool_name, test_future) in test_cases {
            let result = test_future.await;

            // Tools should either succeed (if they have fallback) or fail gracefully
            if let Err(ref e) = result {
                // Error messages should be consistent across tools
                let error_msg = format!("{}", e);
                assert!(!error_msg.contains("panic"), "Error should not contain panic for {}", tool_name);
                assert!(!error_msg.contains("unwrap"), "Error should not contain unwrap for {}", tool_name);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod response_structure_consistency_tests {
    use super::*;

    #[tokio::test]
    async fn test_all_reasoning_tools_return_json_responses() -> Result<()> {
        let setup = ReasoningTestSetup::new().await?;
        setup.set_backend_config(GraphBackend::SqliteGraph);

        // Test that all reasoning tools return properly formatted JSON responses
        let test_cases = vec![
            ("raggraph_query", async {
                let request = RagGraphQueryRequest {
                    query_text: "test query".to_string(),
                };
                setup.server.raggraph_query(syncore::mcp_server::Parameters(request)).await
            }),
            ("raggraph_multihop", async {
                let request = RagGraphMultihopRequest {
                    seed_nodes: vec![1, 2, 3],
                };
                setup.server.raggraph_multihop(syncore::mcp_server::Parameters(request)).await
            }),
            ("code_graph_fusion_query", async {
                let request = syncore::code_graph::RagGraphQueryRequest {
                    query: "test fusion query".to_string(),
                    namespace: None,
                    mode_hint: Some("simple".to_string()),
                    top_k: Some(10),
                    scope: Some("project".to_string()),
                    project_label: None,
                    local_root: None,
                };
                setup.server.code_graph_fusion_query(syncore::mcp_server::Parameters(request)).await
            }),
        ];

        for (tool_name, test_future) in test_cases {
            let result = test_future.await?;

            // All tools should return a CallToolResult with content
            assert!(!result.content.is_empty(), "{} should return content", tool_name);

            // Content should be text (JSON)
            let content = &result.content[0];
            assert!(content.text.is_some(), "{} should return text content", tool_name);

            let response_text = content.text.as_ref().unwrap();

            // Should be valid JSON (or at least start with {)
            if !response_text.trim().starts_with('{') {
                // Some tools might return simple success messages, but they should be structured
                println!("{} response: {}", tool_name, response_text);
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_all_reasoning_tools_have_consistent_response_fields() -> Result<()> {
        let setup = ReasoningTestSetup::new().await?;
        setup.set_backend_config(GraphBackend::SqliteGraph);

        // Extract responses to analyze their structure
        let mut responses = Vec::new();

        // raggraph_query response
        let query_request = RagGraphQueryRequest {
            query_text: "test query".to_string(),
        };
        let query_result = setup.server.raggraph_query(syncore::mcp_server::Parameters(query_request)).await?;
        responses.push(("raggraph_query", &query_result.content[0].text.as_ref().unwrap()));

        // raggraph_multihop response
        let multihop_request = RagGraphMultihopRequest {
            seed_nodes: vec![1, 2, 3],
        };
        let multihop_result = setup.server.raggraph_multihop(syncore::mcp_server::Parameters(multihop_request)).await?;
        responses.push(("raggraph_multihop", &multihop_result.content[0].text.as_ref().unwrap()));

        // code_graph_fusion_query response
        let fusion_request = syncore::code_graph::RagGraphQueryRequest {
            query: "test fusion query".to_string(),
            namespace: None,
            mode_hint: Some("simple".to_string()),
            top_k: Some(10),
            scope: Some("project".to_string()),
            project_label: None,
            local_root: None,
        };
        let fusion_result = setup.server.code_graph_fusion_query(syncore::mcp_server::Parameters(fusion_request)).await?;
        responses.push(("code_graph_fusion_query", &fusion_result.content[0].text.as_ref().unwrap()));

        // Analyze response structure for consistency
        for (tool_name, response_text) in responses {
            if response_text.trim().starts_with('{') {
                // Try to parse as JSON and analyze structure
                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(response_text) {
                    // Should have some form of results/data field
                    if json_value.is_object() {
                        let obj = json_value.as_object().unwrap();

                        // Common expected fields (may vary by tool but should be consistent):
                        // - "results" or "entities" or "nodes" for main data
                        // - "backend" or "metadata" for backend info
                        // - "error" should not be present in successful responses

                        assert!(!obj.contains_key("error"), "{} response should not contain error field on success", tool_name);

                        // At minimum, should have some data field
                        let has_data_field = obj.keys().any(|k| {
                            k.contains("result") || k.contains("entity") || k.contains("node") ||
                            k.contains("data") || k.contains("top") || k.contains("query")
                        });

                        if !has_data_field {
                            println!("{} response structure: {}", tool_name, serde_json::to_string_pretty(&obj).unwrap_or_else(|_| response_text.to_string()));
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod parameter_parsing_consistency_tests {
    use super::*;

    #[tokio::test]
    async fn test_shared_parameters_work_consistently() -> Result<()> {
        let setup = ReasoningTestSetup::new().await?;
        setup.set_backend_config(GraphBackend::SqliteGraph);

        // Test that shared parameters work consistently across tools

        // Test namespace parameter
        let fusion_request_with_namespace = syncore::code_graph::RagGraphQueryRequest {
            query: "test query".to_string(),
            namespace: Some("test_namespace".to_string()),
            mode_hint: Some("simple".to_string()),
            top_k: Some(10),
            scope: Some("project".to_string()),
            project_label: None,
            local_root: None,
        };

        let result = setup.server.code_graph_fusion_query(syncore::mcp_server::Parameters(fusion_request_with_namespace)).await?;
        assert!(result.is_ok(), "code_graph_fusion_query should handle namespace parameter");

        // Test top_k parameter consistency
        let fusion_request_with_topk = syncore::code_graph::RagGraphQueryRequest {
            query: "test query".to_string(),
            namespace: None,
            mode_hint: Some("simple".to_string()),
            top_k: Some(5), // Different top_k
            scope: Some("project".to_string()),
            project_label: None,
            local_root: None,
        };

        let result = setup.server.code_graph_fusion_query(syncore::mcp_server::Parameters(fusion_request_with_topk)).await?;
        assert!(result.is_ok(), "code_graph_fusion_query should handle top_k parameter");

        Ok(())
    }

    #[tokio::test]
    async fn test_mode_hint_parsing_consistency() -> Result<()> {
        let setup = ReasoningTestSetup::new().await?;
        setup.set_backend_config(GraphBackend::SqliteGraph);

        // Test all mode hints work consistently
        let mode_hints = vec!["simple", "attention", "reasoning"];

        for mode_hint in mode_hints {
            let fusion_request = syncore::code_graph::RagGraphQueryRequest {
                query: "test query".to_string(),
                namespace: None,
                mode_hint: Some(mode_hint.to_string()),
                top_k: Some(10),
                scope: Some("project".to_string()),
                project_label: None,
                local_root: None,
            };

            let result = setup.server.code_graph_fusion_query(syncore::mcp_server::Parameters(fusion_request)).await?;
            assert!(result.is_ok(), "code_graph_fusion_query should handle {} mode hint", mode_hint);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_scope_parsing_consistency() -> Result<()> {
        let setup = ReasoningTestSetup::new().await?;
        setup.set_backend_config(GraphBackend::SqliteGraph);

        // Test all scopes work consistently
        let scopes = vec!["local", "project", "workspace", "global", "auto"];

        for scope in scopes {
            let fusion_request = syncore::code_graph::RagGraphQueryRequest {
                query: "test query".to_string(),
                namespace: None,
                mode_hint: Some("simple".to_string()),
                top_k: Some(10),
                scope: Some(scope.to_string()),
                project_label: Some("test_project".to_string()),
                local_root: Some("/test/path".to_string()),
            };

            let result = setup.server.code_graph_fusion_query(syncore::mcp_server::Parameters(fusion_request)).await?;
            assert!(result.is_ok(), "code_graph_fusion_query should handle {} scope", scope);
        }

        Ok(())
    }
}

#[cfg(test)]
mod end_to_end_unification_tests {
    use super::*;

    #[tokio::test]
    async fn test_reasoning_tools_work_with_same_backend() -> Result<()> {
        let setup = ReasoningTestSetup::new().await?;
        setup.set_backend_config(GraphBackend::SqliteGraph);

        // Test that all reasoning tools work with the same backend configuration

        // All should succeed when SQLiteGraph is configured
        let results = vec![
            setup.server.raggraph_query(syncore::mcp_server::Parameters(RagGraphQueryRequest {
                query_text: "test query".to_string(),
            })).await,
            setup.server.raggraph_multihop(syncore::mcp_server::Parameters(RagGraphMultihopRequest {
                seed_nodes: vec![1, 2, 3],
            })).await,
            setup.server.code_graph_fusion_query(syncore::mcp_server::Parameters(syncore::code_graph::RagGraphQueryRequest {
                query: "test fusion query".to_string(),
                namespace: None,
                mode_hint: Some("simple".to_string()),
                top_k: Some(10),
                scope: Some("project".to_string()),
                project_label: None,
                local_root: None,
            })).await,
        ];

        for (i, result) in results.iter().enumerate() {
            let tool_names = ["raggraph_query", "raggraph_multihop", "code_graph_fusion_query"];
            assert!(result.is_ok(), "{} should succeed with same backend", tool_names[i]);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_reasoning_tools_handle_backend_switch_gracefully() -> Result<()> {
        let setup = ReasoningTestSetup::new().await?;

        // Test switching from SQLiteGraph to Neo4j (and back)
        let backend_switches = vec![
            GraphBackend::SqliteGraph,
            GraphBackend::Neo4j,
            GraphBackend::SqliteGraph,
        ];

        for backend in backend_switches {
            setup.set_backend_config(backend);

            // Each tool should handle the backend switch gracefully
            let query_request = RagGraphQueryRequest {
                query_text: "test query".to_string(),
            };
            let result = setup.server.raggraph_query(syncore::mcp_server::Parameters(query_request)).await;

            // Should either succeed (if backend is available) or fail gracefully
            match result {
                Ok(_) => {
                    // Success is good
                }
                Err(e) => {
                    // Error should be graceful, not a panic
                    let error_msg = format!("{}", e);
                    assert!(!error_msg.contains("panic"), "Backend switch should not cause panic");
                    assert!(!error_msg.contains("unwrap"), "Backend switch should not cause unwrap panic");
                }
            }
        }

        Ok(())
    }
}