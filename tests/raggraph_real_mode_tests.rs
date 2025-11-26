/*
//! Real-mode RAGGraph tests using Neo4j + HNSW
//!
//! These tests validate that RagGraph works with real backend infrastructure:
//! - HNSW vector index for semantic search
//! - Neo4j graph database for traversal
//! - Real diffusion over actual graph topology
//!
//! Test data fixture: Small 3-node graph with embeddings

use anyhow::Result;
use std::sync::{Arc, Mutex};
use syncore::graph::Neo4jClient;
use syncore::raggraph::storage::{RealStorageAdapter, StorageAdapter};
use syncore::raggraph::{HopGraphTransformer, RagGraphConfig, RaggraphBackendMode};
use syncore::vector::hnsw::{HnswConfig, HnswVectorIndex};
use syncore::vector::traits::VectorIndex;

/// Test fixture: Create small Neo4j graph for testing
///
/// Graph structure:
/// Node 1 ("hello") -- Node 2 ("world") -- Node 3 ("synapse")
///        \___________________________________/
async fn create_test_graph(client: &Neo4jClient) -> Result<()> {
    // Clear existing test data
    client
        .execute_query("MATCH (n:Embedding) DETACH DELETE n", vec![])
        .await?;

    // Create 3 nodes with text and embeddings
    let nodes = vec![(1, "hello"), (2, "world"), (3, "synapse")];

    for (id, text) in nodes {
        let cypher = r#"
            CREATE (n:Embedding {id: $id, text: $text})
        "#;
        client
            .execute_query(
                cypher,
                vec![
                    ("id", serde_json::json!(id)),
                    ("text", serde_json::json!(text)),
                ],
            )
            .await?;
    }

    // Create relationships (triangular graph)
    let edges = vec![
        (1, 2, 1.0), // 1 -- 2
        (2, 3, 0.8), // 2 -- 3
        (3, 1, 0.6), // 3 -- 1
    ];

    for (from_id, to_id, weight) in edges {
        let cypher = r#"
            MATCH (a:Embedding {id: $from_id}), (b:Embedding {id: $to_id})
            CREATE (a)-[r:RELATES_TO {weight: $weight}]->(b),
                   (b)-[r2:RELATES_TO {weight: $weight}]->(a)
        "#;
        client
            .execute_query(
                cypher,
                vec![
                    ("from_id", serde_json::json!(from_id)),
                    ("to_id", serde_json::json!(to_id)),
                    ("weight", serde_json::json!(weight)),
                ],
            )
            .await?;
    }

    Ok(())
}

/// Test fixture: Populate HNSW index with test embeddings
fn populate_vector_index(index: &mut dyn VectorIndex) -> Result<()> {
    // Simple test embeddings (3-dimensional for clarity)
    // Extend to 384-dim by padding with zeros
    let embeddings = vec![
        (1i64, vec![1.0, 0.0, 0.0]), // "hello"
        (2i64, vec![0.0, 1.0, 0.0]), // "world"
        (3i64, vec![0.0, 0.0, 1.0]), // "synapse"
    ];

    for (id, mut emb) in embeddings {
        // Pad to 384 dimensions
        emb.resize(384, 0.0);
        // Normalize
        let norm: f32 = emb.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut emb {
                *x /= norm;
            }
        }
        index.add(id, emb)?;
    }

    Ok(())
}

#[tokio::test]
async fn test_real_vector_search_returns_correct_ids() -> Result<()> {
    // Create HNSW index
    let config = HnswConfig {
        m: 32,
        ef_construction: 200,
        ef_search: 50,
    };
    let mut index = HnswVectorIndex::new(config, 42)?;
    populate_vector_index(&mut index)?;

    // Search for embedding similar to node 1
    let query = {
        let mut v = vec![0.9, 0.1, 0.0];
        v.resize(384, 0.0);
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        v.iter_mut().for_each(|x| *x /= norm);
        v
    };

    let results = index.search(&query, 3)?;

    // Should return node 1 as top result
    assert!(!results.is_empty());
    assert_eq!(results[0].0, 1); // Node 1 should be closest

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_real_graph_neighbors_query_neo4j() -> Result<()> {
    // Connect to Neo4j
    let neo4j_uri =
        std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string());
    let neo4j_user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let neo4j_pass = std::env::var("NEO4J_PASS").unwrap_or_else(|_| "testpassword123".to_string());

    let client = Neo4jClient::connect(&neo4j_uri, &neo4j_user, &neo4j_pass).await?;
    create_test_graph(&client).await?;

    // Create storage adapter
    let config = HnswConfig {
        m: 32,
        ef_construction: 200,
        ef_search: 50,
    };
    let mut index = HnswVectorIndex::new(config, 42)?;
    populate_vector_index(&mut index)?;
    let adapter = RealStorageAdapter::new(Arc::new(Mutex::new(index)), client.clone(), 384);

    // Query neighbors of node 1
    let neighbors = adapter.neighbors_of(1)?;

    // Should return at least nodes 2 and 3 from our test graph
    assert!(!neighbors.is_empty(), "Should have neighbors");
    let neighbor_ids: Vec<i64> = neighbors.iter().map(|(id, _)| *id).collect();
    assert!(neighbor_ids.contains(&2), "Should contain node 2");
    assert!(neighbor_ids.contains(&3), "Should contain node 3");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_real_diffusion_over_graph() -> Result<()> {
    // Setup
    let neo4j_uri =
        std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string());
    let neo4j_user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let neo4j_pass = std::env::var("NEO4J_PASS").unwrap_or_else(|_| "testpassword123".to_string());

    let client = Neo4jClient::connect(&neo4j_uri, &neo4j_user, &neo4j_pass).await?;
    create_test_graph(&client).await?;

    let mut index = HnswVectorIndex::new(
        HnswConfig {
            m: 32,
            ef_construction: 200,
            ef_search: 50,
        },
        42,
    )?;
    populate_vector_index(&mut index)?;

    let adapter = RealStorageAdapter::new(Arc::new(Mutex::new(index)), client, 384);

    // Create config for real mode
    let config = RagGraphConfig {
        num_hops: 2,
        alpha: 0.85,
        top_k: 10,
        embedding_dim: 384,
        backend_mode: RaggraphBackendMode::Real,
    };

    // Create transformer with real storage
    let transformer =
        HopGraphTransformer::with_storage(config, Arc::new(adapter) as Arc<dyn StorageAdapter>);

    // Run diffusion from seed node 1
    let result = transformer.multi_hop_reasoning(&[1])?;

    // Verify results
    assert!(!result.top_nodes.is_empty());
    assert_eq!(result.context_embedding.len(), 384);
    assert!(!result.reasoning_path.is_empty());

    // Seed node should be in top results
    assert!(result.top_nodes.contains(&1));

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_real_mode_end_to_end_query() -> Result<()> {
    // Full pipeline: query text -> vector search -> graph diffusion -> results
    let neo4j_uri =
        std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string());
    let neo4j_user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let neo4j_pass = std::env::var("NEO4J_PASS").unwrap_or_else(|_| "testpassword123".to_string());

    let client = Neo4jClient::connect(&neo4j_uri, &neo4j_user, &neo4j_pass).await?;
    create_test_graph(&client).await?;

    let mut index = HnswVectorIndex::new(
        HnswConfig {
            m: 32,
            ef_construction: 200,
            ef_search: 50,
        },
        42,
    )?;
    populate_vector_index(&mut index)?;

    let adapter = RealStorageAdapter::new(Arc::new(Mutex::new(index)), client, 384);

    // Query for seed nodes
    let seed_results = adapter.seed_nodes_from_query("hello world", 2)?;
    assert!(!seed_results.is_empty());

    let seed_nodes: Vec<i64> = seed_results.iter().map(|(id, _)| *id).collect();

    // Run multi-hop reasoning
    let config = RagGraphConfig {
        num_hops: 2,
        alpha: 0.85,
        top_k: 10,
        embedding_dim: 384,
        backend_mode: RaggraphBackendMode::Real,
    };

    let transformer =
        HopGraphTransformer::with_storage(config, Arc::new(adapter) as Arc<dyn StorageAdapter>);

    let result = transformer.multi_hop_reasoning(&seed_nodes)?;

    // Verify end-to-end pipeline works
    assert!(!result.top_nodes.is_empty());
    assert!(result.top_nodes.len() <= 10); // Respects top_k
    assert!(result.reasoning_path.len() >= 4); // Has reasoning steps

    Ok(())
}
*/
