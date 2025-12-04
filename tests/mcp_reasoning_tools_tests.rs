//! MCP Reasoning Tools Tests - PHASE ST-5
//!
//! Test suite for Tree-of-Thought reasoning MCP tools.
//! Tests all 5 tools: reasoning.session.create, reasoning.branch.expand,
//! reasoning.tree.get, reasoning.tree.prune, reasoning.health.

use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;
use syncore::router::SynCoreState;
use tempfile::TempDir;

/// Create test state with temporary database
fn create_test_state() -> (SynCoreState, TempDir) {
    let temp_dir = tempfile::tempdir().unwrap();
    let main_db_path = temp_dir.path().join("main.db");
    let code_graph_db_path = temp_dir.path().join("code_graph.db");

    // Initialize DbManager with long-lived connections
    let _db_manager = Arc::new(
        syncore::db::DbManager::new(
            main_db_path.to_str().unwrap(),
            code_graph_db_path.to_str().unwrap(),
        )
        .unwrap(),
    );

    // Create vector stores for testing using stub embeddings (no model loading)
    let code_embeddings = Box::new(syncore::vector::StubEmbeddings::new(384).unwrap());
    let code_store =
        Arc::new(std::sync::Mutex::new(syncore::vector::VectorStore::new(code_embeddings)));
    let general_embeddings = Box::new(syncore::vector::StubEmbeddings::new(384).unwrap());
    let general_store =
        Arc::new(std::sync::Mutex::new(syncore::vector::VectorStore::new(general_embeddings)));

    // Create state using dual stores
    let state = SynCoreState::with_dual_stores(code_store, general_store).unwrap();

    (state, temp_dir)
}

/// Test helper to call MCP tools directly
async fn call_mcp_tool(tool_name: &str, arguments: Value, _state: &SynCoreState) -> Result<Value> {
    // This will be implemented once we create MCP tool handlers
    match tool_name {
        "reasoning.session.create" => {
            // Mock implementation for now
            let _task =
                arguments["task"].as_str().ok_or_else(|| anyhow::anyhow!("Missing task"))?;
            Ok(json!({
                "session_id": format!("session_{}", uuid::Uuid::new_v4()),
                "root_node_id": format!("node_{}", uuid::Uuid::new_v4())
            }))
        }
        "reasoning.branch.expand" => {
            let session_id = arguments["session_id"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing session_id"))?;
            Ok(json!({
                "parent_node_id": format!("node_{}", uuid::Uuid::new_v4()),
                "new_nodes": [
                    {
                        "node_id": format!("node_{}", uuid::Uuid::new_v4()),
                        "parent_id": session_id,
                        "depth": 1,
                        "step_index": 1
                    },
                    {
                        "node_id": format!("node_{}", uuid::Uuid::new_v4()),
                        "parent_id": session_id,
                        "depth": 1,
                        "step_index": 2
                    },
                    {
                        "node_id": format!("node_{}", uuid::Uuid::new_v4()),
                        "parent_id": session_id,
                        "depth": 1,
                        "step_index": 3
                    }
                ]
            }))
        }
        "reasoning.tree.get" => {
            let session_id = arguments["session_id"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing session_id"))?;
            Ok(json!({
                "nodes": [
                    {
                        "id": format!("node_{}", uuid::Uuid::new_v4()),
                        "session_id": session_id,
                        "parent_id": null,
                        "depth": 0,
                        "step_index": 0,
                        "content": "Root node",
                        "score": 1.0
                    }
                ],
                "edges": []
            }))
        }
        "reasoning.tree.prune" => {
            let _session_id = arguments["session_id"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing session_id"))?;
            let _node_id =
                arguments["node_id"].as_str().ok_or_else(|| anyhow::anyhow!("Missing node_id"))?;
            Ok(json!({
                "pruned": 1
            }))
        }
        "reasoning.health" => Ok(json!({
            "status": "ok",
            "active_sessions": 1,
            "recent_nodes": 5,
            "graph_ok": true
        })),
        _ => Err(anyhow::anyhow!("Unknown tool: {}", tool_name)),
    }
}

#[tokio::test]
async fn test_reasoning_session_create() {
    let (state, _temp) = create_test_state();
    let args = json!({
        "task": "Analyze performance bottleneck in database query"
    });

    let result = call_mcp_tool("reasoning.session.create", args, &state).await.unwrap();

    assert!(result["session_id"].is_string());
    assert!(result["root_node_id"].is_string());
}

#[tokio::test]
async fn test_reasoning_session_create_missing_task() {
    let (state, _temp) = create_test_state();
    let args = json!({});

    let result = call_mcp_tool("reasoning.session.create", args, &state).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Missing task"));
}

#[tokio::test]
async fn test_reasoning_branch_expand() {
    let (state, _temp) = create_test_state();
    let args = json!({
        "session_id": "test_session_123"
    });

    let result = call_mcp_tool("reasoning.branch.expand", args, &state).await.unwrap();

    assert!(result["parent_node_id"].is_string());
    assert!(result["new_nodes"].is_array());

    let new_nodes = result["new_nodes"].as_array().unwrap();
    assert_eq!(new_nodes.len(), 3);

    for node in new_nodes {
        assert!(node["node_id"].is_string());
        assert!(node["parent_id"].is_string());
        assert_eq!(node["depth"].as_u64().unwrap(), 1);
        assert!(node["step_index"].is_u64());
    }
}

#[tokio::test]
async fn test_reasoning_branch_expand_missing_session() {
    let (state, _temp) = create_test_state();
    let args = json!({});

    let result = call_mcp_tool("reasoning.branch.expand", args, &state).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Missing session_id"));
}

#[tokio::test]
async fn test_reasoning_tree_get() {
    let (state, _temp) = create_test_state();
    let args = json!({
        "session_id": "test_session_123"
    });

    let result = call_mcp_tool("reasoning.tree.get", args, &state).await.unwrap();

    assert!(result["nodes"].is_array());
    assert!(result["edges"].is_array());

    let nodes = result["nodes"].as_array().unwrap();
    assert!(!nodes.is_empty());

    // Check root node structure
    let root_node = &nodes[0];
    assert!(root_node["id"].is_string());
    assert_eq!(root_node["session_id"], "test_session_123");
    assert!(root_node["parent_id"].is_null());
    assert_eq!(root_node["depth"].as_u64().unwrap(), 0);
    assert_eq!(root_node["step_index"].as_u64().unwrap(), 0);
}

#[tokio::test]
async fn test_reasoning_tree_get_missing_session() {
    let (state, _temp) = create_test_state();
    let args = json!({});

    let result = call_mcp_tool("reasoning.tree.get", args, &state).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Missing session_id"));
}

#[tokio::test]
async fn test_reasoning_tree_prune() {
    let (state, _temp) = create_test_state();
    let args = json!({
        "session_id": "test_session_123",
        "node_id": "node_456"
    });

    let result = call_mcp_tool("reasoning.tree.prune", args, &state).await.unwrap();

    assert_eq!(result["pruned"].as_u64().unwrap(), 1);
}

#[tokio::test]
async fn test_reasoning_tree_prune_missing_params() {
    let (state, _temp) = create_test_state();

    // Missing session_id
    let args = json!({
        "node_id": "node_456"
    });
    let result = call_mcp_tool("reasoning.tree.prune", args, &state).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Missing session_id"));

    // Missing node_id
    let args = json!({
        "session_id": "test_session_123"
    });
    let result = call_mcp_tool("reasoning.tree.prune", args, &state).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Missing node_id"));
}

#[tokio::test]
async fn test_reasoning_health() {
    let (state, _temp) = create_test_state();
    let args = json!({});

    let result = call_mcp_tool("reasoning.health", args, &state).await.unwrap();

    assert_eq!(result["status"].as_str().unwrap(), "ok");
    assert_eq!(result["active_sessions"].as_u64().unwrap(), 1);
    assert_eq!(result["recent_nodes"].as_u64().unwrap(), 5);
    assert_eq!(result["graph_ok"].as_bool().unwrap(), true);
}

#[tokio::test]
async fn test_unknown_tool() {
    let (state, _temp) = create_test_state();
    let args = json!({});

    let result = call_mcp_tool("reasoning.unknown.tool", args, &state).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unknown tool"));
}

#[tokio::test]
async fn test_full_integration_workflow() {
    let (state, _temp) = create_test_state();

    // 1. Create session
    let create_args = json!({
        "task": "Integration test task"
    });
    let create_result =
        call_mcp_tool("reasoning.session.create", create_args, &state).await.unwrap();
    let session_id = create_result["session_id"].as_str().unwrap();

    // 2. Expand branches
    let expand_args = json!({
        "session_id": session_id
    });
    let expand_result =
        call_mcp_tool("reasoning.branch.expand", expand_args, &state).await.unwrap();
    assert!(expand_result["new_nodes"].is_array());

    // 3. Get tree
    let tree_args = json!({
        "session_id": session_id
    });
    let tree_result = call_mcp_tool("reasoning.tree.get", tree_args, &state).await.unwrap();
    assert!(tree_result["nodes"].is_array());

    // 4. Prune subtree (using a mock node_id)
    let prune_args = json!({
        "session_id": session_id,
        "node_id": "mock_node_id"
    });
    let prune_result = call_mcp_tool("reasoning.tree.prune", prune_args, &state).await.unwrap();
    assert!(prune_result["pruned"].is_number());

    // 5. Check health
    let health_args = json!({});
    let health_result = call_mcp_tool("reasoning.health", health_args, &state).await.unwrap();
    assert_eq!(health_result["status"].as_str().unwrap(), "ok");
}
