//! PHASE 4 TDD Tests: Multi-Hop Graph Reasoning
//!
//! These tests verify that multi-hop BFS traversal works correctly with:
//! - SQLite-only traversal
//! - Neo4j-enhanced traversal (dual-mode)
//! - Depth limiting
//! - Cycle detection
//! - Branch limiting
//!
//! All tests use REAL databases (no mocks).

use anyhow::Result;
use std::path::Path;
use std::sync::{Arc, Mutex};
use syncore::code_graph::multi_hop::{multi_hop, multi_hop_sqlite, neighbors_sqlite, MultiHopNode};
use syncore::code_graph::{CodeEntity, CodeGraph, EdgeType, EntityType};
use syncore::graph::Neo4jClient;
use syncore::vector::{HuggingFaceEmbeddings, VectorStore};

/// Test 1: Single-hop neighbors from SQLite
///
/// Creates a small graph (A->B, A->C) and verifies that neighbors_sqlite
/// returns both B and C in deterministic order.
#[test]
fn test_single_hop_neighbors_sqlite() -> Result<()> {
    // Create temporary database
    let db_path = "/tmp/test_phase4_single_hop.db";
    let _ = std::fs::remove_file(db_path);

    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let code_graph = CodeGraph::new(db_path, vector_store)?;

    let db = code_graph.db_conn();
    let conn = db.lock().unwrap();

    // Insert entities A, B, C
    conn.execute(
        "INSERT INTO code_entities (file_path, entity_type, name, signature, line_start, line_end, language, indexed_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params![
            "/test/a.rs",
            "function",
            "A",
            "fn A()",
            1,
            10,
            "rust",
            chrono::Utc::now().timestamp(),
        ],
    )?;
    let entity_a_id = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO code_entities (file_path, entity_type, name, signature, line_start, line_end, language, indexed_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params![
            "/test/b.rs",
            "function",
            "B",
            "fn B()",
            1,
            10,
            "rust",
            chrono::Utc::now().timestamp(),
        ],
    )?;
    let entity_b_id = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO code_entities (file_path, entity_type, name, signature, line_start, line_end, language, indexed_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params![
            "/test/c.rs",
            "function",
            "C",
            "fn C()",
            1,
            10,
            "rust",
            chrono::Utc::now().timestamp(),
        ],
    )?;
    let entity_c_id = conn.last_insert_rowid();

    // Create edges: A->B (calls), A->C (uses)
    conn.execute(
        "INSERT INTO code_edges (src_entity_id, dst_entity_id, edge_type)
         VALUES (?, ?, ?)",
        rusqlite::params![entity_a_id, entity_b_id, "calls"],
    )?;
    conn.execute(
        "INSERT INTO code_edges (src_entity_id, dst_entity_id, edge_type)
         VALUES (?, ?, ?)",
        rusqlite::params![entity_a_id, entity_c_id, "uses"],
    )?;

    // Query neighbors of A
    let neighbors = neighbors_sqlite(&conn, entity_a_id)?;

    // Verify we get both B and C
    assert_eq!(neighbors.len(), 2);

    // Verify deterministic ordering (by entity_id)
    assert_eq!(neighbors[0].0, entity_b_id);
    assert_eq!(neighbors[1].0, entity_c_id);

    // Verify edge types
    assert_eq!(neighbors[0].1, EdgeType::Calls);
    assert_eq!(neighbors[1].1, EdgeType::Uses);

    // Cleanup
    std::fs::remove_file(db_path)?;

    Ok(())
}

/// Test 2: Multi-hop SQLite with depth limit
///
/// Builds a chain A->B->C->D and verifies that multi_hop_sqlite(A, max_depth=2)
/// returns A, B, C but NOT D.
#[test]
fn test_multi_hop_sqlite_depth_limit() -> Result<()> {
    // Create temporary database
    let db_path = "/tmp/test_phase4_depth_limit.db";
    let _ = std::fs::remove_file(db_path);

    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let code_graph = CodeGraph::new(db_path, vector_store)?;

    let db = code_graph.db_conn();
    let conn = db.lock().unwrap();

    // Insert entities A, B, C, D
    let mut entity_ids = Vec::new();
    for name in &["A", "B", "C", "D"] {
        conn.execute(
            "INSERT INTO code_entities (file_path, entity_type, name, signature, line_start, line_end, language, indexed_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                format!("/test/{}.rs", name.to_lowercase()),
                "function",
                name,
                format!("fn {}()", name),
                1,
                10,
                "rust",
                chrono::Utc::now().timestamp(),
            ],
        )?;
        entity_ids.push(conn.last_insert_rowid());
    }

    // Create chain: A->B->C->D
    for i in 0..3 {
        conn.execute(
            "INSERT INTO code_edges (src_entity_id, dst_entity_id, edge_type)
             VALUES (?, ?, ?)",
            rusqlite::params![entity_ids[i], entity_ids[i + 1], "calls"],
        )?;
    }

    // Traverse with max_depth=2
    let result = multi_hop_sqlite(&conn, entity_ids[0], 2)?;

    // Should have A (depth 0), B (depth 1), C (depth 2)
    assert_eq!(result.nodes.len(), 3);

    assert_eq!(result.nodes[0].id, entity_ids[0]); // A at depth 0
    assert_eq!(result.nodes[0].depth, 0);

    assert_eq!(result.nodes[1].id, entity_ids[1]); // B at depth 1
    assert_eq!(result.nodes[1].depth, 1);

    assert_eq!(result.nodes[2].id, entity_ids[2]); // C at depth 2
    assert_eq!(result.nodes[2].depth, 2);

    // D should NOT be in result
    assert!(
        !result.nodes.iter().any(|n| n.id == entity_ids[3]),
        "D should not be included with max_depth=2"
    );

    // Cleanup
    std::fs::remove_file(db_path)?;

    Ok(())
}

/// Test 3: Multi-hop cycle detection
///
/// Builds a cycle A->B->C->A and ensures traversal stops safely without infinite loop.
#[test]
fn test_multi_hop_cycle_detection() -> Result<()> {
    // Create temporary database
    let db_path = "/tmp/test_phase4_cycle.db";
    let _ = std::fs::remove_file(db_path);

    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let code_graph = CodeGraph::new(db_path, vector_store)?;

    let db = code_graph.db_conn();
    let conn = db.lock().unwrap();

    // Insert entities A, B, C
    let mut entity_ids = Vec::new();
    for name in &["A", "B", "C"] {
        conn.execute(
            "INSERT INTO code_entities (file_path, entity_type, name, signature, line_start, line_end, language, indexed_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                format!("/test/{}.rs", name.to_lowercase()),
                "function",
                name,
                format!("fn {}()", name),
                1,
                10,
                "rust",
                chrono::Utc::now().timestamp(),
            ],
        )?;
        entity_ids.push(conn.last_insert_rowid());
    }

    // Create cycle: A->B->C->A
    conn.execute(
        "INSERT INTO code_edges (src_entity_id, dst_entity_id, edge_type)
         VALUES (?, ?, ?)",
        rusqlite::params![entity_ids[0], entity_ids[1], "calls"],
    )?;
    conn.execute(
        "INSERT INTO code_edges (src_entity_id, dst_entity_id, edge_type)
         VALUES (?, ?, ?)",
        rusqlite::params![entity_ids[1], entity_ids[2], "calls"],
    )?;
    conn.execute(
        "INSERT INTO code_edges (src_entity_id, dst_entity_id, edge_type)
         VALUES (?, ?, ?)",
        rusqlite::params![entity_ids[2], entity_ids[0], "calls"],
    )?;

    // Traverse with max_depth=5 (would loop infinitely without cycle detection)
    let result = multi_hop_sqlite(&conn, entity_ids[0], 5)?;

    // Should have exactly 3 nodes (A, B, C) despite depth=5
    assert_eq!(result.nodes.len(), 3);

    // Verify all three nodes are present
    let node_ids: Vec<i64> = result.nodes.iter().map(|n| n.id).collect();
    assert!(node_ids.contains(&entity_ids[0]));
    assert!(node_ids.contains(&entity_ids[1]));
    assert!(node_ids.contains(&entity_ids[2]));

    // Verify A is visited only once (at depth 0)
    let a_visits: Vec<&MultiHopNode> = result
        .nodes
        .iter()
        .filter(|n| n.id == entity_ids[0])
        .collect();
    assert_eq!(a_visits.len(), 1);
    assert_eq!(a_visits[0].depth, 0);

    // Cleanup
    std::fs::remove_file(db_path)?;

    Ok(())
}

/// Test 4: Multi-hop branch limit
///
/// Creates a node with 100 neighbors and verifies only first 20 (deterministic order)
/// are returned per level.
#[test]
fn test_multi_hop_branch_limit() -> Result<()> {
    // Create temporary database
    let db_path = "/tmp/test_phase4_branch_limit.db";
    let _ = std::fs::remove_file(db_path);

    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let code_graph = CodeGraph::new(db_path, vector_store)?;

    let db = code_graph.db_conn();
    let conn = db.lock().unwrap();

    // Insert root node A
    conn.execute(
        "INSERT INTO code_entities (file_path, entity_type, name, signature, line_start, line_end, language, indexed_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params![
            "/test/a.rs",
            "function",
            "A",
            "fn A()",
            1,
            10,
            "rust",
            chrono::Utc::now().timestamp(),
        ],
    )?;
    let entity_a_id = conn.last_insert_rowid();

    // Insert 100 neighbor nodes
    let mut neighbor_ids = Vec::new();
    for i in 0..100 {
        conn.execute(
            "INSERT INTO code_entities (file_path, entity_type, name, signature, line_start, line_end, language, indexed_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                format!("/test/node{}.rs", i),
                "function",
                format!("Node{}", i),
                format!("fn node{}()", i),
                1,
                10,
                "rust",
                chrono::Utc::now().timestamp(),
            ],
        )?;
        neighbor_ids.push(conn.last_insert_rowid());
    }

    // Create edges: A -> all 100 neighbors
    for neighbor_id in &neighbor_ids {
        conn.execute(
            "INSERT INTO code_edges (src_entity_id, dst_entity_id, edge_type)
             VALUES (?, ?, ?)",
            rusqlite::params![entity_a_id, neighbor_id, "calls"],
        )?;
    }

    // Traverse with max_depth=1
    let result = multi_hop_sqlite(&conn, entity_a_id, 1)?;

    // Should have A (depth 0) + first 20 neighbors (depth 1) = 21 nodes
    assert_eq!(result.nodes.len(), 21);

    // Verify A is at depth 0
    assert_eq!(result.nodes[0].id, entity_a_id);
    assert_eq!(result.nodes[0].depth, 0);

    // Verify next 20 nodes are at depth 1 and are the first 20 neighbors (deterministic)
    let depth1_nodes: Vec<&MultiHopNode> = result.nodes.iter().filter(|n| n.depth == 1).collect();
    assert_eq!(depth1_nodes.len(), 20);

    // Verify they are the first 20 neighbors in sorted order
    for (i, node) in depth1_nodes.iter().enumerate() {
        assert_eq!(node.id, neighbor_ids[i]);
    }

    // Cleanup
    std::fs::remove_file(db_path)?;

    Ok(())
}

/// Test 5: Multi-hop Neo4j union
///
/// A->B exists in SQLite, A->C exists only in Neo4j.
/// multi_hop(...) must return both B and C.
#[tokio::test]
async fn test_multi_hop_neo4j_union() -> Result<()> {
    // Create temporary database
    let db_path = "/tmp/test_phase4_neo4j_union.db";
    let _ = std::fs::remove_file(db_path);

    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    // Connect to Neo4j
    let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string());
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let pass = std::env::var("NEO4J_PASS").unwrap_or_else(|_| "testpassword123".to_string());

    let neo4j = Arc::new(Neo4jClient::connect(&uri, &user, &pass).await?);
    let code_graph = CodeGraph::new_with_neo4j(db_path, vector_store, neo4j.clone())?;

    let db = code_graph.db_conn();
    let conn = db.lock().unwrap();

    // Clean Neo4j test namespace
    neo4j
        .execute_query(
            "MATCH (n {namespace: $ns}) DETACH DELETE n",
            vec![("ns", serde_json::json!(neo4j.namespace()))],
        )
        .await?;

    // Insert entities A, B, C in SQLite
    conn.execute(
        "INSERT INTO code_entities (file_path, entity_type, name, signature, line_start, line_end, language, indexed_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params![
            "/test/a.rs",
            "function",
            "A",
            "fn A()",
            1,
            10,
            "rust",
            chrono::Utc::now().timestamp(),
        ],
    )?;
    let entity_a_id = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO code_entities (file_path, entity_type, name, signature, line_start, line_end, language, indexed_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params![
            "/test/b.rs",
            "function",
            "B",
            "fn B()",
            1,
            10,
            "rust",
            chrono::Utc::now().timestamp(),
        ],
    )?;
    let entity_b_id = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO code_entities (file_path, entity_type, name, signature, line_start, line_end, language, indexed_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params![
            "/test/c.rs",
            "function",
            "C",
            "fn C()",
            1,
            10,
            "rust",
            chrono::Utc::now().timestamp(),
        ],
    )?;
    let entity_c_id = conn.last_insert_rowid();

    // Create SQLite edge: A->B
    conn.execute(
        "INSERT INTO code_edges (src_entity_id, dst_entity_id, edge_type)
         VALUES (?, ?, ?)",
        rusqlite::params![entity_a_id, entity_b_id, "calls"],
    )?;

    // Create Neo4j nodes for A and C
    neo4j
        .execute_query(
            "CREATE (a:Function {id: $a_id, namespace: $ns})
             CREATE (c:Function {id: $c_id, namespace: $ns})",
            vec![
                ("a_id", serde_json::json!(entity_a_id)),
                ("c_id", serde_json::json!(entity_c_id)),
                ("ns", serde_json::json!(neo4j.namespace())),
            ],
        )
        .await?;

    // Create Neo4j edge: A->C (uses)
    neo4j
        .execute_query(
            "MATCH (a:Function {id: $a_id, namespace: $ns})
             MATCH (c:Function {id: $c_id, namespace: $ns})
             CREATE (a)-[:uses]->(c)",
            vec![
                ("a_id", serde_json::json!(entity_a_id)),
                ("c_id", serde_json::json!(entity_c_id)),
                ("ns", serde_json::json!(neo4j.namespace())),
            ],
        )
        .await?;

    drop(conn);

    // Perform multi-hop traversal (should union SQLite + Neo4j)
    let result = code_graph.multi_hop(entity_a_id, 1).await?;

    // Should have A (depth 0), B (depth 1 from SQLite), C (depth 1 from Neo4j)
    assert_eq!(result.nodes.len(), 3);

    assert_eq!(result.nodes[0].id, entity_a_id); // A at depth 0
    assert_eq!(result.nodes[0].depth, 0);

    // Verify both B and C are at depth 1
    let depth1_nodes: Vec<&MultiHopNode> = result.nodes.iter().filter(|n| n.depth == 1).collect();
    assert_eq!(depth1_nodes.len(), 2);

    let depth1_ids: Vec<i64> = depth1_nodes.iter().map(|n| n.id).collect();
    assert!(depth1_ids.contains(&entity_b_id), "B should be in result");
    assert!(depth1_ids.contains(&entity_c_id), "C should be in result");

    // Cleanup
    std::fs::remove_file(db_path)?;

    Ok(())
}

/// Test 6: Multi-hop depth 3 mixed graph
///
/// Builds a mixed SQLite/Neo4j 3-hop graph and ensures depth annotations are correct.
#[tokio::test]
async fn test_multi_hop_depth_3_mixed_graph() -> Result<()> {
    // Create temporary database
    let db_path = "/tmp/test_phase4_depth3_mixed.db";
    let _ = std::fs::remove_file(db_path);

    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    // Connect to Neo4j
    let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string());
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let pass = std::env::var("NEO4J_PASS").unwrap_or_else(|_| "testpassword123".to_string());

    let neo4j = Arc::new(Neo4jClient::connect(&uri, &user, &pass).await?);
    let code_graph = CodeGraph::new_with_neo4j(db_path, vector_store, neo4j.clone())?;

    let db = code_graph.db_conn();
    let conn = db.lock().unwrap();

    // Clean Neo4j test namespace
    neo4j
        .execute_query(
            "MATCH (n {namespace: $ns}) DETACH DELETE n",
            vec![("ns", serde_json::json!(neo4j.namespace()))],
        )
        .await?;

    // Insert entities A, B, C, D, E in SQLite
    let mut entity_ids = Vec::new();
    for name in &["A", "B", "C", "D", "E"] {
        conn.execute(
            "INSERT INTO code_entities (file_path, entity_type, name, signature, line_start, line_end, language, indexed_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                format!("/test/{}.rs", name.to_lowercase()),
                "function",
                name,
                format!("fn {}()", name),
                1,
                10,
                "rust",
                chrono::Utc::now().timestamp(),
            ],
        )?;
        entity_ids.push(conn.last_insert_rowid());
    }

    // Create SQLite edges: A->B, B->C
    conn.execute(
        "INSERT INTO code_edges (src_entity_id, dst_entity_id, edge_type)
         VALUES (?, ?, ?)",
        rusqlite::params![entity_ids[0], entity_ids[1], "calls"],
    )?;
    conn.execute(
        "INSERT INTO code_edges (src_entity_id, dst_entity_id, edge_type)
         VALUES (?, ?, ?)",
        rusqlite::params![entity_ids[1], entity_ids[2], "calls"],
    )?;

    // Create Neo4j nodes
    for (i, name) in ["A", "B", "C", "D", "E"].iter().enumerate() {
        neo4j
            .execute_query(
                &format!("CREATE (n:Function {{id: $id, name: $name, namespace: $ns}})"),
                vec![
                    ("id", serde_json::json!(entity_ids[i])),
                    ("name", serde_json::json!(name)),
                    ("ns", serde_json::json!(neo4j.namespace())),
                ],
            )
            .await?;
    }

    // Create Neo4j edges: A->D, D->E
    neo4j
        .execute_query(
            "MATCH (a:Function {id: $a_id, namespace: $ns})
             MATCH (d:Function {id: $d_id, namespace: $ns})
             CREATE (a)-[:uses]->(d)",
            vec![
                ("a_id", serde_json::json!(entity_ids[0])),
                ("d_id", serde_json::json!(entity_ids[3])),
                ("ns", serde_json::json!(neo4j.namespace())),
            ],
        )
        .await?;

    neo4j
        .execute_query(
            "MATCH (d:Function {id: $d_id, namespace: $ns})
             MATCH (e:Function {id: $e_id, namespace: $ns})
             CREATE (d)-[:calls]->(e)",
            vec![
                ("d_id", serde_json::json!(entity_ids[3])),
                ("e_id", serde_json::json!(entity_ids[4])),
                ("ns", serde_json::json!(neo4j.namespace())),
            ],
        )
        .await?;

    drop(conn);

    // Traverse with max_depth=3
    // Expected: A(0) -> B(1), D(1) -> C(2), E(2)
    let result = code_graph.multi_hop(entity_ids[0], 3).await?;

    // Verify nodes at each depth
    let depth0: Vec<&MultiHopNode> = result.nodes.iter().filter(|n| n.depth == 0).collect();
    let depth1: Vec<&MultiHopNode> = result.nodes.iter().filter(|n| n.depth == 1).collect();
    let depth2: Vec<&MultiHopNode> = result.nodes.iter().filter(|n| n.depth == 2).collect();

    assert_eq!(depth0.len(), 1); // A
    assert_eq!(depth0[0].id, entity_ids[0]);

    assert_eq!(depth1.len(), 2); // B, D
    let depth1_ids: Vec<i64> = depth1.iter().map(|n| n.id).collect();
    assert!(depth1_ids.contains(&entity_ids[1])); // B
    assert!(depth1_ids.contains(&entity_ids[3])); // D

    assert_eq!(depth2.len(), 2); // C, E
    let depth2_ids: Vec<i64> = depth2.iter().map(|n| n.id).collect();
    assert!(depth2_ids.contains(&entity_ids[2])); // C
    assert!(depth2_ids.contains(&entity_ids[4])); // E

    // Cleanup
    std::fs::remove_file(db_path)?;

    Ok(())
}
