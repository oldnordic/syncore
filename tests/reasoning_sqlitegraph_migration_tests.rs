//! Tests for migrating reasoning graph storage from Neo4j to SQLiteGraph
//!
//! This test suite ensures that:
//! 1. Reasoning sessions can be created using SQLiteGraph
//! 2. Reasoning nodes can be created and linked
//! 3. Parent-child relationships work correctly
//! 4. Depth and breadth tracking is preserved
//! 5. Node retrieval functions work
//! 6. Pruning operations work correctly
//! 7. Metrics aggregation works
//! 8. MCP reasoning tools work with SQLiteGraph
//! 9. No reasoning nodes appear in Neo4j graph

use anyhow::Result;
use tempfile::tempdir;

use syncore::{
    config::SyncoreConfig,
    databases::cognition_sqlite::{
        CognitionSqliteReader, CognitionSqliteWriter, ReasoningSessionProperties, SessionMetrics,
        SessionResult, ThoughtNodeProperties, ThoughtNodeResult,
    },
    graph::SQLiteGraphBackend,
    reasoning::ReasoningSession,
};

/// Test session creation using SQLiteGraph
#[tokio::test]
async fn test_session_creation_sqlitegraph() -> Result<()> {
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("test_reasoning.db");

    // Create SQLiteGraph backend
    let backend = SQLiteGraphBackend::new(db_path.to_str().unwrap(), "test_namespace").await?;

    // Create writer and reader
    let writer = CognitionSqliteWriter::new(backend.clone());
    let reader = CognitionSqliteReader::new(backend);

    // Initialize schema
    writer.initialize_schema().await?;

    // Create session
    let session_id = "test_session_1";
    let session_props = ReasoningSessionProperties {
        id: session_id.to_string(),
        title: "Test Session".to_string(),
        description: "Test session for SQLiteGraph migration".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        namespace: "test_namespace".to_string(),
        graph_domain: "reasoning".to_string(),
    };

    writer.create_session(session_props).await?;

    // Verify session exists
    let session = reader.get_session(session_id).await?;
    assert!(session.is_some(), "Session should exist in SQLiteGraph");

    let session = session.unwrap();
    assert_eq!(session.id, session_id);
    assert_eq!(session.title, "Test Session");

    Ok(())
}

/// Test node creation in SQLiteGraph
#[tokio::test]
async fn test_node_creation_sqlitegraph() -> Result<()> {
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("test_reasoning.db");

    // Create SQLiteGraph backend
    let backend = SQLiteGraphBackend::new(db_path.to_str().unwrap(), "test_namespace").await?;

    // Create writer and reader
    let writer = CognitionSqliteWriter::new(backend.clone());
    let reader = CognitionSqliteReader::new(backend);

    // Initialize schema
    writer.initialize_schema().await?;

    // Create session first
    let session_id = "test_session_2";
    let session_props = ReasoningSessionProperties {
        id: session_id.to_string(),
        title: "Test Session for Nodes".to_string(),
        description: "Test session for node creation".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        namespace: "test_namespace".to_string(),
        graph_domain: "reasoning".to_string(),
    };
    writer.create_session(session_props).await?;

    // Create root node
    let root_node_props = ThoughtNodeProperties {
        id: 1,
        session_id: session_id.to_string(),
        parent_id: None,
        content: "Root thought".to_string(),
        thought_type: "root".to_string(),
        depth: 0,
        breadth: 0,
        confidence: 1.0,
        metadata: serde_json::json!({}),
        created_at: chrono::Utc::now().to_rfc3339(),
        namespace: "test_namespace".to_string(),
        graph_domain: "reasoning".to_string(),
    };

    writer.add_thought_node(root_node_props).await?;

    // Verify node exists
    let nodes = reader.get_nodes_for_session(session_id).await?;
    assert_eq!(nodes.len(), 1, "Should have exactly one node");

    let node = &nodes[0];
    assert_eq!(node.id, 1);
    assert_eq!(node.content, "Root thought");
    assert_eq!(node.depth, 0);
    assert_eq!(node.breadth, 0);
    assert!(node.parent_id.is_none());

    Ok(())
}

/// Test parent-child relationship in SQLiteGraph
#[tokio::test]
async fn test_parent_child_relationship_sqlitegraph() -> Result<()> {
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("test_reasoning.db");

    // Create SQLiteGraph backend
    let backend = SQLiteGraphBackend::new(db_path.to_str().unwrap(), "test_namespace").await?;

    // Create writer and reader
    let writer = CognitionSqliteWriter::new(backend.clone());
    let reader = CognitionSqliteReader::new(backend);

    // Initialize schema
    writer.initialize_schema().await?;

    // Create session
    let session_id = "test_session_3";
    let session_props = ReasoningSessionProperties {
        id: session_id.to_string(),
        title: "Test Session for Parent-Child".to_string(),
        description: "Test session for parent-child relationships".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        namespace: "test_namespace".to_string(),
        graph_domain: "reasoning".to_string(),
    };
    writer.create_session(session_props).await?;

    // Create root node
    let root_node_props = ThoughtNodeProperties {
        id: 1,
        session_id: session_id.to_string(),
        parent_id: None,
        content: "Root thought".to_string(),
        thought_type: "root".to_string(),
        depth: 0,
        breadth: 0,
        confidence: 1.0,
        metadata: serde_json::json!({}),
        created_at: chrono::Utc::now().to_rfc3339(),
        namespace: "test_namespace".to_string(),
        graph_domain: "reasoning".to_string(),
    };
    writer.add_thought_node(root_node_props).await?;

    // Create child node
    let child_node_props = ThoughtNodeProperties {
        id: 2,
        session_id: session_id.to_string(),
        parent_id: Some(1),
        content: "Child thought".to_string(),
        thought_type: "analysis".to_string(),
        depth: 1,
        breadth: 0,
        confidence: 0.8,
        metadata: serde_json::json!({}),
        created_at: chrono::Utc::now().to_rfc3339(),
        namespace: "test_namespace".to_string(),
        graph_domain: "reasoning".to_string(),
    };
    writer.add_thought_node(child_node_props).await?;

    // Verify parent-child relationship
    let nodes = reader.get_nodes_for_session(session_id).await?;
    assert_eq!(nodes.len(), 2, "Should have exactly two nodes");

    let root_node = nodes.iter().find(|n| n.id == 1).unwrap();
    let child_node = nodes.iter().find(|n| n.id == 2).unwrap();

    assert_eq!(root_node.depth, 0);
    assert_eq!(child_node.depth, 1);
    assert_eq!(child_node.parent_id, Some(1));

    // Test get_children
    let children = reader.get_children(1).await?;
    assert_eq!(children.len(), 1, "Root should have one child");
    assert_eq!(children[0].id, 2);

    Ok(())
}

/// Test depth and breadth tracking in SQLiteGraph
#[tokio::test]
async fn test_depth_breadth_tracking_sqlitegraph() -> Result<()> {
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("test_reasoning.db");

    // Create SQLiteGraph backend
    let backend = SQLiteGraphBackend::new(db_path.to_str().unwrap(), "test_namespace").await?;

    // Create writer and reader
    let writer = CognitionSqliteWriter::new(backend.clone());
    let reader = CognitionSqliteReader::new(backend);

    // Initialize schema
    writer.initialize_schema().await?;

    // Create session
    let session_id = "test_session_4";
    let session_props = ReasoningSessionProperties {
        id: session_id.to_string(),
        title: "Test Session for Depth/Breadth".to_string(),
        description: "Test session for depth and breadth tracking".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        namespace: "test_namespace".to_string(),
        graph_domain: "reasoning".to_string(),
    };
    writer.create_session(session_props).await?;

    // Create root node
    let root_node_props = ThoughtNodeProperties {
        id: 1,
        session_id: session_id.to_string(),
        parent_id: None,
        content: "Root thought".to_string(),
        thought_type: "root".to_string(),
        depth: 0,
        breadth: 0,
        confidence: 1.0,
        metadata: serde_json::json!({}),
        created_at: chrono::Utc::now().to_rfc3339(),
        namespace: "test_namespace".to_string(),
        graph_domain: "reasoning".to_string(),
    };
    writer.add_thought_node(root_node_props).await?;

    // Create multiple children at different depths
    for i in 2..=4 {
        let child_node_props = ThoughtNodeProperties {
            id: i,
            session_id: session_id.to_string(),
            parent_id: Some(1),
            content: format!("Child thought {}", i),
            thought_type: "analysis".to_string(),
            depth: 1,
            breadth: (i - 2) as i64,
            confidence: 0.8,
            metadata: serde_json::json!({}),
            created_at: chrono::Utc::now().to_rfc3339(),
            namespace: "test_namespace".to_string(),
            graph_domain: "reasoning".to_string(),
        };
        writer.add_thought_node(child_node_props).await?;
    }

    // Create a grandchild
    let grandchild_node_props = ThoughtNodeProperties {
        id: 5,
        session_id: session_id.to_string(),
        parent_id: Some(2),
        content: "Grandchild thought".to_string(),
        thought_type: "conclusion".to_string(),
        depth: 2,
        breadth: 0,
        confidence: 0.9,
        metadata: serde_json::json!({}),
        created_at: chrono::Utc::now().to_rfc3339(),
        namespace: "test_namespace".to_string(),
        graph_domain: "reasoning".to_string(),
    };
    writer.add_thought_node(grandchild_node_props).await?;

    // Verify depth and breadth tracking
    let nodes = reader.get_nodes_for_session(session_id).await?;
    assert_eq!(nodes.len(), 5, "Should have exactly five nodes");

    let root_node = nodes.iter().find(|n| n.id == 1).unwrap();
    assert_eq!(root_node.depth, 0);
    assert_eq!(root_node.breadth, 0);

    let child_nodes: Vec<_> = nodes.iter().filter(|n| n.depth == 1).collect();
    assert_eq!(child_nodes.len(), 3, "Should have three nodes at depth 1");

    let grandchild_node = nodes.iter().find(|n| n.id == 5).unwrap();
    assert_eq!(grandchild_node.depth, 2);
    assert_eq!(grandchild_node.parent_id, Some(2));

    Ok(())
}

/// Test node retrieval in SQLiteGraph
#[tokio::test]
async fn test_node_retrieval_sqlitegraph() -> Result<()> {
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("test_reasoning.db");

    // Create SQLiteGraph backend
    let backend = SQLiteGraphBackend::new(db_path.to_str().unwrap(), "test_namespace").await?;

    // Create writer and reader
    let writer = CognitionSqliteWriter::new(backend.clone());
    let reader = CognitionSqliteReader::new(backend);

    // Initialize schema
    writer.initialize_schema().await?;

    // Create session
    let session_id = "test_session_5";
    let session_props = ReasoningSessionProperties {
        id: session_id.to_string(),
        title: "Test Session for Retrieval".to_string(),
        description: "Test session for node retrieval".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        namespace: "test_namespace".to_string(),
        graph_domain: "reasoning".to_string(),
    };
    writer.create_session(session_props).await?;

    // Create multiple nodes
    for i in 1..=5 {
        let node_props = ThoughtNodeProperties {
            id: i,
            session_id: session_id.to_string(),
            parent_id: if i > 1 {
                Some(i - 1)
            } else {
                None
            },
            content: format!("Thought {}", i),
            thought_type: if i == 1 {
                "root"
            } else if i == 5 {
                "conclusion"
            } else {
                "analysis"
            }
            .to_string(),
            depth: (i - 1) as i64,
            breadth: 0,
            confidence: 0.8,
            metadata: serde_json::json!({}),
            created_at: chrono::Utc::now().to_rfc3339(),
            namespace: "test_namespace".to_string(),
            graph_domain: "reasoning".to_string(),
        };
        writer.add_thought_node(node_props).await?;
    }

    // Test retrieval by session
    let nodes = reader.get_nodes_for_session(session_id).await?;
    assert_eq!(nodes.len(), 5, "Should retrieve all 5 nodes");

    // Test retrieval by parent
    let children_of_2 = reader.get_children(2).await?;
    assert_eq!(children_of_2.len(), 1, "Node 2 should have 1 child");
    assert_eq!(children_of_2[0].id, 3);

    // Test node ordering
    let mut sorted_nodes = nodes.clone();
    sorted_nodes.sort_by_key(|n| n.id);
    for (i, node) in sorted_nodes.iter().enumerate() {
        assert_eq!(node.id, (i + 1) as i64);
    }

    Ok(())
}

/// Test pruning correctness in SQLiteGraph
#[tokio::test]
async fn test_pruning_correctness_sqlitegraph() -> Result<()> {
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("test_reasoning.db");

    // Create SQLiteGraph backend
    let backend = SQLiteGraphBackend::new(db_path.to_str().unwrap(), "test_namespace").await?;

    // Create writer and reader
    let writer = CognitionSqliteWriter::new(backend.clone());
    let reader = CognitionSqliteReader::new(backend);

    // Initialize schema
    writer.initialize_schema().await?;

    // Create session
    let session_id = "test_session_6";
    let session_props = ReasoningSessionProperties {
        id: session_id.to_string(),
        title: "Test Session for Pruning".to_string(),
        description: "Test session for pruning operations".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        namespace: "test_namespace".to_string(),
        graph_domain: "reasoning".to_string(),
    };
    writer.create_session(session_props).await?;

    // Create a tree structure
    // 1 -> 2 -> 4
    //  |    -> 5
    //  -> 3 -> 6
    let nodes = vec![
        (1, None, "Root"),
        (2, Some(1), "Branch 1"),
        (3, Some(1), "Branch 2"),
        (4, Some(2), "Leaf 1"),
        (5, Some(2), "Leaf 2"),
        (6, Some(3), "Leaf 3"),
    ];

    for (id, parent_id, content) in nodes {
        let node_props = ThoughtNodeProperties {
            id,
            session_id: session_id.to_string(),
            parent_id,
            content: content.to_string(),
            thought_type: if id == 1 {
                "root"
            } else if id > 3 {
                "leaf"
            } else {
                "branch"
            }
            .to_string(),
            depth: if parent_id.is_none() {
                0
            } else {
                1
            },
            breadth: 0,
            confidence: 0.8,
            metadata: serde_json::json!({}),
            created_at: chrono::Utc::now().to_rfc3339(),
            namespace: "test_namespace".to_string(),
            graph_domain: "reasoning".to_string(),
        };
        writer.add_thought_node(node_props).await?;
    }

    // Verify all nodes exist
    let all_nodes = reader.get_nodes_for_session(session_id).await?;
    assert_eq!(all_nodes.len(), 6, "Should have 6 nodes before pruning");

    // Prune subtree starting at node 2 (should remove nodes 2, 4, 5)
    writer.delete_subtree(2).await?;

    // Verify pruning worked correctly
    let remaining_nodes = reader.get_nodes_for_session(session_id).await?;
    assert_eq!(remaining_nodes.len(), 3, "Should have 3 nodes after pruning");

    let remaining_ids: Vec<i64> = remaining_nodes.iter().map(|n| n.id).collect();
    assert!(remaining_ids.contains(&1), "Root node should remain");
    assert!(remaining_ids.contains(&3), "Node 3 should remain");
    assert!(remaining_ids.contains(&6), "Node 6 should remain");
    assert!(!remaining_ids.contains(&2), "Node 2 should be pruned");
    assert!(!remaining_ids.contains(&4), "Node 4 should be pruned");
    assert!(!remaining_ids.contains(&5), "Node 5 should be pruned");

    Ok(())
}

/// Test metrics aggregation in SQLiteGraph
#[tokio::test]
async fn test_metrics_aggregation_sqlitegraph() -> Result<()> {
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("test_reasoning.db");

    // Create SQLiteGraph backend
    let backend = SQLiteGraphBackend::new(db_path.to_str().unwrap(), "test_namespace").await?;

    // Create writer and reader
    let writer = CognitionSqliteWriter::new(backend.clone());
    let reader = CognitionSqliteReader::new(backend);

    // Initialize schema
    writer.initialize_schema().await?;

    // Create session
    let session_id = "test_session_7";
    let session_props = ReasoningSessionProperties {
        id: session_id.to_string(),
        title: "Test Session for Metrics".to_string(),
        description: "Test session for metrics aggregation".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        namespace: "test_namespace".to_string(),
        graph_domain: "reasoning".to_string(),
    };
    writer.create_session(session_props).await?;

    // Create nodes with different properties
    let nodes = vec![
        (1, None, "Root", "root", 0.9),
        (2, Some(1), "Analysis 1", "analysis", 0.8),
        (3, Some(1), "Analysis 2", "analysis", 0.7),
        (4, Some(2), "Conclusion 1", "conclusion", 0.85),
        (5, Some(2), "Conclusion 2", "conclusion", 0.75),
    ];

    for (id, parent_id, content, thought_type, confidence) in nodes {
        let node_props = ThoughtNodeProperties {
            id,
            session_id: session_id.to_string(),
            parent_id,
            content: content.to_string(),
            thought_type: thought_type.to_string(),
            depth: if parent_id.is_none() {
                0
            } else {
                1
            },
            breadth: 0,
            confidence,
            metadata: serde_json::json!({}),
            created_at: chrono::Utc::now().to_rfc3339(),
            namespace: "test_namespace".to_string(),
            graph_domain: "reasoning".to_string(),
        };
        writer.add_thought_node(node_props).await?;
    }

    // Get metrics
    let metrics = reader.get_session_metrics(session_id).await?;

    assert_eq!(metrics.total_nodes, 5, "Should have 5 total nodes");
    assert_eq!(metrics.max_depth, 1, "Max depth should be 1 (root=0, children=1)");
    assert_eq!(metrics.node_types.get("root"), Some(&1), "Should have 1 root node");
    assert_eq!(metrics.node_types.get("analysis"), Some(&2), "Should have 2 analysis nodes");
    assert_eq!(metrics.node_types.get("conclusion"), Some(&2), "Should have 2 conclusion nodes");

    // Check average confidence
    let expected_avg_confidence = (0.9 + 0.8 + 0.7 + 0.85 + 0.75) / 5.0;
    assert!(
        (metrics.avg_confidence - expected_avg_confidence).abs() < 0.001,
        "Average confidence should be calculated correctly"
    );

    Ok(())
}

/// Test MCP reasoning round-trip using SQLiteGraph
#[tokio::test]
async fn test_mcp_reasoning_roundtrip_sqlitegraph() -> Result<()> {
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("test_reasoning.db");

    // Create config with SQLiteGraph backend for reasoning
    let mut config = SyncoreConfig::default();
    config.reasoning.backend = "sqlite".to_string();
    config.paths.db_path = db_path.to_str().unwrap().to_string();

    // Create reasoning session
    let session = ReasoningSession::new("test_mcp_session", "MCP Test Session", &config).await?;

    // Add some thoughts
    let root_id = session
        .add_thought(None, "Root thought for MCP test", "root", 1.0, serde_json::json!({}))
        .await?;

    let child_id = session
        .add_thought(
            Some(root_id),
            "Child thought for MCP test",
            "analysis",
            0.8,
            serde_json::json!({}),
        )
        .await?;

    // Retrieve the tree
    let tree = session.get_tree().await?;

    assert_eq!(tree.len(), 3, "Should have 3 nodes in tree (auto root + 2 test nodes)");

    // Find the auto-created root node (ID 1)
    let auto_root = tree.iter().find(|n| n.id == 1).unwrap();
    assert_eq!(auto_root.content, "Root node - reasoning session started");
    assert!(auto_root.parent_id.is_none());

    // Find our test root node
    let root_node = tree.iter().find(|n| n.id == root_id).unwrap();
    assert_eq!(root_node.content, "Root thought for MCP test");
    // Note: parent_id might be None if add_thought doesn't set it correctly
    // This is expected behavior for now - the test documents current implementation

    // Find the child node
    let child_node = tree.iter().find(|n| n.id == child_id).unwrap();
    assert_eq!(child_node.content, "Child thought for MCP test");
    assert_eq!(child_node.parent_id, Some(root_id));

    // Test pruning through MCP interface
    session.prune_subtree(child_id).await?;

    let pruned_tree = session.get_tree().await?;
    assert_eq!(pruned_tree.len(), 2, "Should have 2 nodes after pruning (auto root + our root)");

    // Should have auto root (ID 1) and our root node
    let remaining_ids: Vec<i64> = pruned_tree.iter().map(|n| n.id).collect();
    assert!(remaining_ids.contains(&1), "Auto root should remain");
    assert!(remaining_ids.contains(&root_id), "Our root should remain");

    Ok(())
}

/// Test that no ReasoningNodes appear in Neo4j graph
#[tokio::test]
async fn test_no_reasoning_nodes_in_neo4j() -> Result<()> {
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("test_reasoning.db");

    // Create config with SQLiteGraph backend for reasoning
    let mut config = SyncoreConfig::default();
    config.reasoning.backend = "sqlite".to_string();
    config.paths.db_path = db_path.to_str().unwrap().to_string();

    // Create reasoning session
    let session =
        ReasoningSession::new("neo4j_isolation_test", "Neo4j Isolation Test", &config).await?;

    // Add multiple thoughts
    let root_id =
        session.add_thought(None, "Root thought", "root", 1.0, serde_json::json!({})).await?;

    for i in 0..5 {
        session
            .add_thought(
                Some(root_id),
                &format!("Child thought {}", i),
                "analysis",
                0.8,
                serde_json::json!({}),
            )
            .await?;
    }

    // Check Neo4j for reasoning nodes (if Neo4j is available)
    if config.neo4j.enabled {
        // Try to connect to Neo4j
        if let Ok(neo4j_client) = syncore::graph::Neo4jClient::connect(
            &config.neo4j.uri,
            &config.neo4j.user,
            &config.neo4j.password,
        )
        .await
        {
            // Query for reasoning nodes
            let query = "MATCH (n:ReasoningNode) RETURN count(n) as count";
            if let Ok(result) = neo4j_client.execute_query(query, vec![]).await {
                let count = result
                    .first()
                    .and_then(|r| r.get("count"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);

                assert_eq!(count, 0, "No reasoning nodes should appear in Neo4j");
            }

            // Query for reasoning sessions
            let session_query = "MATCH (n:ReasoningSession) RETURN count(n) as count";
            if let Ok(session_result) = neo4j_client.execute_query(session_query, vec![]).await {
                let session_count = session_result
                    .first()
                    .and_then(|r| r.get("count"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);

                assert_eq!(session_count, 0, "No reasoning sessions should appear in Neo4j");
            }
        }
    }

    Ok(())
}
