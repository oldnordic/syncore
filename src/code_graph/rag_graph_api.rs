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
use super::types::{CodeEntity, QueryScope};
use crate::graph::Neo4jClient;
use crate::vector::VectorStore;
use anyhow::{anyhow, Result};
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
    /// Query scope: "local" | "project" | "workspace" | "global" | "auto"
    /// Controls search breadth across projects. Default: "project"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// Project label for filtering (e.g., "SynCore", "OdinCode")
    /// Required for Project scope, optional for others
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_label: Option<String>,
    /// Local root path for Local scope filtering (e.g., "src/code_graph/")
    /// Only used when scope is "local"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_root: Option<String>,
}

/// Response structure for RAGGraph queries
#[derive(Debug, Serialize, Deserialize)]
pub struct RagGraphQueryResponse {
    /// Ranked list of code entities with scores
    pub entities: Vec<RankedEntity>,
    /// Selected fusion mode used for this query
    pub selected_mode: String,
    /// Applied query scope
    pub applied_scope: String,
    /// Debug information about the query execution
    pub debug_info: HashMap<String, serde_json::Value>,
}

/// Ranked entity with combined score (PHASE 5: includes temporal)
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
    /// Temporal score (PHASE 5: recency + churn)
    pub temporal_score: f32,
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

    /// Execute a RAGGraph query with default Global scope (backward compatible)
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
        // Default to Global scope for backward compatibility
        self.query_with_scope(
            query,
            namespace,
            mode_hint,
            top_k,
            QueryScope::Global,
            None,
            None,
        )
        .await
    }

    /// Execute a RAGGraph query with explicit scope control
    ///
    /// # Arguments
    /// * `query` - Text query to search for
    /// * `namespace` - Optional namespace for scoped search
    /// * `mode_hint` - Optional fusion mode hint ("simple", "attention", "reasoning")
    /// * `top_k` - Maximum number of results to return (default: 10)
    /// * `scope` - Query scope controlling search breadth
    /// * `project_label` - Project label for Project scope filtering
    /// * `local_root` - Local root path for Local scope filtering
    ///
    /// # Returns
    /// RagGraphQueryResponse with ranked entities and debug info
    pub async fn query_with_scope(
        &self,
        query: &str,
        namespace: Option<&str>,
        mode_hint: Option<&str>,
        top_k: Option<u32>,
        scope: QueryScope,
        project_label: Option<&str>,
        local_root: Option<&str>,
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

        // Step 2: Perform vector search (fetch more results for post-filtering)
        let fetch_multiplier = match scope {
            QueryScope::Global => 2,    // No filtering needed
            QueryScope::Workspace => 3, // Light filtering
            QueryScope::Project => 4,   // Moderate filtering
            QueryScope::Local => 5,     // Heavy filtering
            QueryScope::Auto => 2,      // Default to Global behavior
        };
        let vector_matches = self.code_graph.search_code(query, k * fetch_multiplier)?;

        // Step 3: Apply scope filtering
        let filtered_matches =
            self.apply_scope_filter(vector_matches, scope, project_label, local_root);

        // Step 4: Perform graph expansion for each filtered match
        let mut ranked_entities = Vec::new();
        let mut debug_info = HashMap::new();

        for vmatch in filtered_matches.iter().take(k) {
            let entity_id = vmatch.entity.id.unwrap_or(0);
            let vector_score = vmatch.score;

            // Graph expansion: compute depth-based score using multi-hop traversal
            let graph_score = self.compute_graph_score(entity_id)?;

            // PHASE 5: Compute temporal score from entity metadata
            let temporal_score = super::fusion_simple::compute_temporal_score(
                vmatch.entity.last_modified_at.unwrap_or(0),
                vmatch.entity.change_count.unwrap_or(1),
                vmatch.entity.author_count.unwrap_or(1),
            );

            // Step 5: Apply fusion based on selected mode (PHASE 5: now includes temporal)
            let base_score = match selected_mode {
                FusionMode::Simple => self.apply_simple_fusion(
                    vector_score,
                    graph_score,
                    temporal_score,
                    &mut debug_info,
                )?,
                FusionMode::Attention => {
                    self.apply_attention_fusion(vector_score, graph_score, query, &mut debug_info)?
                }
                FusionMode::Reasoning => {
                    self.apply_reasoning_fusion(vector_score, graph_score, &mut debug_info)?
                }
            };

            // STEP C + APEX v1.7 Phase 4: Apply entity type + body boost
            let entity_kind = vmatch.entity.entity_type.as_str();
            let has_body = vmatch.entity.body_snippet.is_some();
            let combined_score =
                super::entity_boost::apply_combined_boost(base_score, entity_kind, has_body);

            ranked_entities.push(RankedEntity {
                entity: vmatch.entity.clone(),
                combined_score,
                vector_score,
                graph_score,
                temporal_score,
            });
        }

        // Step 6: Sort by combined score
        ranked_entities.sort_by(|a, b| b.combined_score.partial_cmp(&a.combined_score).unwrap());
        ranked_entities.truncate(k);

        // Add global debug info
        debug_info.insert("query_length".to_string(), serde_json::json!(query.len()));
        debug_info.insert(
            "total_matches".to_string(),
            serde_json::json!(filtered_matches.len()),
        );
        debug_info.insert(
            "pre_filter_matches".to_string(),
            serde_json::json!(k * fetch_multiplier),
        );
        if let Some(ns) = namespace {
            debug_info.insert("namespace".to_string(), serde_json::json!(ns));
        }
        if let Some(pl) = project_label {
            debug_info.insert("project_label".to_string(), serde_json::json!(pl));
        }
        if let Some(lr) = local_root {
            debug_info.insert("local_root".to_string(), serde_json::json!(lr));
        }

        Ok(RagGraphQueryResponse {
            entities: ranked_entities,
            selected_mode: format!("{:?}", selected_mode).to_lowercase(),
            applied_scope: scope.as_str().to_string(),
            debug_info,
        })
    }

    /// Apply scope-based filtering to vector search results
    ///
    /// # Arguments
    /// * `matches` - Vector search results to filter
    /// * `scope` - Query scope controlling filter behavior
    /// * `project_label` - Project label for Project scope
    /// * `local_root` - Local root path for Local scope
    ///
    /// # Returns
    /// Filtered matches based on scope rules
    fn apply_scope_filter(
        &self,
        matches: Vec<super::types::CodeMatch>,
        scope: QueryScope,
        project_label: Option<&str>,
        local_root: Option<&str>,
    ) -> Vec<super::types::CodeMatch> {
        match scope {
            QueryScope::Global => {
                // No filtering - return all matches
                matches
            }
            QueryScope::Workspace => {
                // TODO: Filter to workspace projects when workspace concept is implemented
                // For now, same as Global
                matches
            }
            QueryScope::Project => {
                // Filter by project label (extracted from file path)
                if let Some(label) = project_label {
                    matches
                        .into_iter()
                        .filter(|m| self.matches_project(&m.entity.file_path, label))
                        .collect()
                } else {
                    // No project label specified, return all
                    matches
                }
            }
            QueryScope::Local => {
                // Filter by local root path
                if let Some(root) = local_root {
                    matches
                        .into_iter()
                        .filter(|m| m.entity.file_path.contains(root))
                        .collect()
                } else if let Some(label) = project_label {
                    // Fall back to project filtering if no local root
                    matches
                        .into_iter()
                        .filter(|m| self.matches_project(&m.entity.file_path, label))
                        .collect()
                } else {
                    matches
                }
            }
            QueryScope::Auto => {
                // Auto currently aliases to Global (engine decides later)
                matches
            }
        }
    }

    /// Check if a file path matches a project label
    ///
    /// # Arguments
    /// * `file_path` - Full file path to check
    /// * `project_label` - Project label to match against
    ///
    /// # Returns
    /// true if the file belongs to the project
    fn matches_project(&self, file_path: &str, project_label: &str) -> bool {
        // Case-insensitive match on project name in path
        let path_lower = file_path.to_lowercase();
        let label_lower = project_label.to_lowercase();

        // Match patterns like:
        // - /home/user/Projects/SynCore/... matches "SynCore"
        // - /workspace/my-project/... matches "my-project"
        path_lower.contains(&format!("/{}/", label_lower))
            || path_lower.contains(&format!("\\{}\\", label_lower))
            || path_lower.starts_with(&format!("{}/", label_lower))
            || path_lower.ends_with(&format!("/{}", label_lower))
    }

    /// Compute graph score for an entity based on multi-hop depth (TASK B)
    fn compute_graph_score(&self, entity_id: i64) -> Result<f32> {
        // Use sync SQLite-only multi-hop to avoid async Send trait violation
        // This computes depth-based graph score using only SQLite edges
        let db = self
            .code_graph
            .db_conn()
            .lock()
            .map_err(|e| anyhow!("Failed to lock database: {}", e))?;

        let multi_hop_result = super::multi_hop::multi_hop_sqlite(&db, entity_id, 20)?;

        // Find minimum depth (excluding self at depth 0)
        let min_depth = multi_hop_result
            .nodes
            .iter()
            .filter(|n| n.depth > 0)
            .map(|n| n.depth)
            .min();

        // Convert depth to score using Phase 5 formula
        let graph_score = super::fusion_simple::compute_graph_score(min_depth);
        Ok(graph_score)
    }

    /// Apply simple linear fusion
    fn apply_simple_fusion(
        &self,
        vector_score: f32,
        graph_score: f32,
        temporal_score: f32,
        debug_info: &mut HashMap<String, serde_json::Value>,
    ) -> Result<f32> {
        // PHASE 5: Use default 3-component weights (α=0.65, β=0.25, τ=0.10)
        let fusion = FusionSimple::default();
        let result = fusion.combine(vector_score, graph_score, temporal_score);

        // Add debug info
        debug_info.insert("vector_score".to_string(), serde_json::json!(vector_score));
        debug_info.insert("graph_score".to_string(), serde_json::json!(graph_score));
        debug_info.insert(
            "temporal_score".to_string(),
            serde_json::json!(temporal_score),
        );
        debug_info.insert("alpha".to_string(), serde_json::json!(0.65));
        debug_info.insert("beta".to_string(), serde_json::json!(0.25));
        debug_info.insert("tau".to_string(), serde_json::json!(0.10));

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
