//! RAGGraph API - High-level interface combining vector search, graph expansion, and tri-mode fusion
//!
//! This module provides the main entry point for RAGGraph queries, integrating:
//! - Vector search for initial entity retrieval
//! - Graph expansion via Neo4j for multi-hop reasoning
//! - Tri-mode fusion (Simple/Attention/Reasoning) for score combination
//! - Automatic mode selection via FusionRouter
//!
//! Usage:
//! ```rust
//! let api = RagGraphAPI::new(code_graph, neo4j_client);
//! let result = api.query("find format function", None, None, Some(10)).await?;
//! ```

use super::fusion_attention::FusionAttention;
use super::fusion_reasoning::FusionReasoning;
use super::fusion_router::{FusionMode, FusionRouter};
use super::fusion_simple::FusionSimple;
use super::graph::CodeGraph;
use super::types::CodeEntity;
use crate::graph::Neo4jClient;
use crate::vector::VectorStore;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Request structure for RAGGraph queries
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RagGraphQueryRequest {
    /// The text query to search for
    pub query: String,
    /// Optional namespace for scoped search
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    /// Optional fusion mode hint ("simple", "attention", "reasoning")
    /// If None, router auto-selects based on query characteristics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode_hint: Option<String>,
    /// Maximum number of results to return
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
}

/// Response structure for RAGGraph queries
#[derive(Debug, Serialize, Deserialize)]
pub struct RagGraphQueryResponse {
    /// Ranked list of code entities with scores
    pub entities: Vec<RankedEntity>,
    /// Selected fusion mode used for this query
    pub selected_mode: String,
    /// Debug information about the query execution
    pub debug_info: HashMap<String, serde_json::Value>,
}

/// Ranked entity with combined score
#[derive(Debug, Serialize, Deserialize)]
pub struct RankedEntity {
    /// Code entity details
    pub entity: CodeEntity,
    /// Combined score from fusion
    pub combined_score: f32,
    /// Original vector search score
    pub vector_score: f32,
    /// Graph expansion score
    pub graph_score: f32,
}

/// High-level RAGGraph API
pub struct RagGraphAPI {
    code_graph: CodeGraph,
    neo4j: Neo4jClient,
    router: FusionRouter,
}

impl RagGraphAPI {
    /// Create new RAGGraph API instance
    ///
    /// # Arguments
    /// * `code_graph` - CodeGraph instance for entity storage and vector search
    /// * `neo4j` - Neo4jClient for graph traversal
    ///
    /// # Returns
    /// New RagGraphAPI instance
    pub fn new(code_graph: CodeGraph, neo4j: Neo4jClient) -> Self {
        Self {
            code_graph,
            neo4j,
            router: FusionRouter::new(),
        }
    }

    /// Execute a RAGGraph query
    ///
    /// # Arguments
    /// * `query` - Text query to search for
    /// * `namespace` - Optional namespace for scoped search
    /// * `mode_hint` - Optional fusion mode hint ("simple", "attention", "reasoning")
    /// * `top_k` - Maximum number of results to return (default: 10)
    ///
    /// # Returns
    /// RagGraphQueryResponse with ranked entities and debug info
    pub async fn query(
        &self,
        query: &str,
        namespace: Option<&str>,
        mode_hint: Option<&str>,
        top_k: Option<u32>,
    ) -> Result<RagGraphQueryResponse> {
        let k = top_k.unwrap_or(10) as usize;

        // Step 1: Select fusion mode (via hint or router)
        let selected_mode = if let Some(hint) = mode_hint {
            match hint {
                "simple" => FusionMode::Simple,
                "attention" => FusionMode::Attention,
                "reasoning" => FusionMode::Reasoning,
                _ => self.router.select_mode(query),
            }
        } else {
            self.router.select_mode(query)
        };

        // Step 2: Perform vector search
        let vector_matches = self.code_graph.search_code(query, k * 2)?;

        // Step 3: Perform graph expansion for each vector match
        let mut ranked_entities = Vec::new();
        let mut debug_info = HashMap::new();

        for vmatch in vector_matches.iter().take(k) {
            let entity_id = vmatch.entity.id.unwrap_or(0);
            let vector_score = vmatch.score;

            // Graph expansion: count incoming/outgoing edges
            let graph_score = self.compute_graph_score(entity_id).await?;

            // Step 4: Apply fusion based on selected mode
            let combined_score = match selected_mode {
                FusionMode::Simple => {
                    self.apply_simple_fusion(vector_score, graph_score, &mut debug_info)?
                }
                FusionMode::Attention => {
                    self.apply_attention_fusion(vector_score, graph_score, query, &mut debug_info)?
                }
                FusionMode::Reasoning => {
                    self.apply_reasoning_fusion(vector_score, graph_score, &mut debug_info)?
                }
            };

            ranked_entities.push(RankedEntity {
                entity: vmatch.entity.clone(),
                combined_score,
                vector_score,
                graph_score,
            });
        }

        // Step 5: Sort by combined score
        ranked_entities.sort_by(|a, b| b.combined_score.partial_cmp(&a.combined_score).unwrap());
        ranked_entities.truncate(k);

        // Add global debug info
        debug_info.insert("query_length".to_string(), serde_json::json!(query.len()));
        debug_info.insert(
            "total_matches".to_string(),
            serde_json::json!(vector_matches.len()),
        );
        if let Some(ns) = namespace {
            debug_info.insert("namespace".to_string(), serde_json::json!(ns));
        }

        Ok(RagGraphQueryResponse {
            entities: ranked_entities,
            selected_mode: format!("{:?}", selected_mode).to_lowercase(),
            debug_info,
        })
    }

    /// Compute graph score for an entity based on Neo4j connections
    async fn compute_graph_score(&self, entity_id: i64) -> Result<f32> {
        // Count incoming and outgoing edges in Neo4j
        // Match by properties instead of label (nodes have type-specific labels like Function, Class, etc.)
        let query = r#"
            MATCH (n {id: $id, namespace: $ns})
            OPTIONAL MATCH (n)-[r_out]->()
            OPTIONAL MATCH (n)<-[r_in]-()
            RETURN count(DISTINCT r_out) as out_degree, count(DISTINCT r_in) as in_degree
        "#;

        let params = vec![
            ("id", serde_json::json!(entity_id)),
            ("ns", serde_json::json!(self.neo4j.namespace())),
        ];

        let result = self.neo4j.execute_query(query, params).await?;

        // Extract degree counts from result
        let out_degree = result[0]
            .get("out_degree")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as f32;
        let in_degree = result[0]
            .get("in_degree")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as f32;

        // FIX 3: Log when entity not found in Neo4j (graph_score = 0)
        let total_degree = out_degree + in_degree;
        if total_degree == 0.0 {
            eprintln!(
                "[WARN] Neo4j graph_score = 0 for entity_id={}. \
                 Entity may not be synced to Neo4j or has no relationships. \
                 Run code_graph_sync_neo4j to sync entities and edges.",
                entity_id
            );
        }

        // Normalize to 0-1 range (assume max degree of 10)
        let normalized_score = (total_degree / 10.0).min(1.0);

        Ok(normalized_score)
    }

    /// Apply simple linear fusion
    fn apply_simple_fusion(
        &self,
        vector_score: f32,
        graph_score: f32,
        debug_info: &mut HashMap<String, serde_json::Value>,
    ) -> Result<f32> {
        let fusion = FusionSimple::new(0.6); // Default alpha = 0.6
        let result = fusion.combine(vector_score, graph_score);

        // Add debug info
        debug_info.insert("vector_score".to_string(), serde_json::json!(vector_score));
        debug_info.insert("graph_score".to_string(), serde_json::json!(graph_score));
        debug_info.insert("alpha".to_string(), serde_json::json!(0.6));

        Ok(result)
    }

    /// Apply attention-based fusion
    fn apply_attention_fusion(
        &self,
        vector_score: f32,
        graph_score: f32,
        context: &str,
        debug_info: &mut HashMap<String, serde_json::Value>,
    ) -> Result<f32> {
        use crate::vector::{Embeddings, HuggingFaceEmbeddings};

        let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
        let fusion = FusionAttention::new(embeddings);

        let result = fusion.combine(vector_score, graph_score, context)?;

        // Add debug info (recompute for debug - this is inefficient but maintains encapsulation)
        let embeddings_for_debug = HuggingFaceEmbeddings::new()?;
        let embedding = embeddings_for_debug.embed(context)?;
        let token_count = context.split_whitespace().count();
        let mean: f32 = embedding.iter().sum::<f32>() / embedding.len() as f32;
        let variance: f32 =
            embedding.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / embedding.len() as f32;
        let complexity = (token_count as f32 / 10.0) + variance * 100.0;
        let alpha = (0.3 + complexity / 5.0).clamp(0.3, 0.7);

        debug_info.insert("attention_alpha".to_string(), serde_json::json!(alpha));
        debug_info.insert(
            "context_complexity".to_string(),
            serde_json::json!(complexity),
        );

        Ok(result)
    }

    /// Apply reasoning fusion with higher-order terms
    fn apply_reasoning_fusion(
        &self,
        vector_score: f32,
        graph_score: f32,
        debug_info: &mut HashMap<String, serde_json::Value>,
    ) -> Result<f32> {
        use crate::vector::HuggingFaceEmbeddings;

        let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
        let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

        let fusion = FusionReasoning::new(self.neo4j.clone(), vector_store);

        let result = fusion.combine_higher_order(vector_score, graph_score);

        // Add debug info
        let gamma_term = 0.2 * graph_score * graph_score;
        debug_info.insert("gamma_term".to_string(), serde_json::json!(gamma_term));
        debug_info.insert("higher_order_score".to_string(), serde_json::json!(result));
        debug_info.insert("vector_score".to_string(), serde_json::json!(vector_score));
        debug_info.insert("graph_score".to_string(), serde_json::json!(graph_score));

        Ok(result)
    }
}
