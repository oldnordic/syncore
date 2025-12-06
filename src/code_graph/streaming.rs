//! APEX 2.8-STREAMING-FUSION-QUERY: Streaming results for RAGGraph queries
//!
//! Provides incremental result streaming for fusion queries:
//! - Chunk 1: Vector search results
//! - Chunk 2: Graph expansion results
//! - Chunk 3: Fusion scoring results
//! - Final: Consolidated sorted results
//!
//! Does NOT modify existing synchronous query API or scoring logic.

use super::rag_graph_api::{RagGraphAPI, RankedEntity};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Configuration for streaming behavior
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    /// Emit chunk after vector search
    pub chunk_vector: bool,
    /// Emit chunk after graph expansion
    pub chunk_graph: bool,
    /// Emit chunk after fusion scoring
    pub chunk_fusion: bool,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            chunk_vector: true,
            chunk_graph: true,
            chunk_fusion: true,
        }
    }
}

/// Single chunk of streaming results
#[derive(Debug, Clone)]
pub struct RagStreamingChunk {
    /// Ranked entities in this chunk
    pub ranked_entities: Vec<RankedEntity>,
    /// Whether this is the final chunk
    pub is_final: bool,
    /// Optional debug trace for this chunk
    pub debug_trace: Option<String>,
}

impl RagGraphAPI {
    /// Execute streaming RAGGraph query
    ///
    /// Returns an mpsc::Receiver that yields chunks as they become available:
    /// 1. Vector search results
    /// 2. Graph expansion results
    /// 3. Fusion scoring results
    /// 4. Final consolidated results
    ///
    /// # Arguments
    /// * `query` - Text query to search for
    /// * `top_k` - Maximum number of results
    /// * `config` - Streaming configuration
    ///
    /// # Returns
    /// Receiver that yields RagStreamingChunk instances
    pub async fn query_streaming(
        &self,
        query: &str,
        top_k: usize,
        config: StreamingConfig,
    ) -> Result<mpsc::Receiver<RagStreamingChunk>> {
        let (tx, rx) = mpsc::channel::<RagStreamingChunk>(32);

        // Simple passthrough: call sync query and emit as chunks
        // Clone Arc-wrapped components for background task
        let code_graph_db = self.code_graph.db.clone();
        let code_graph_vector_store = self.code_graph.vector_store.clone();
        let code_graph_neo4j = self.code_graph.neo4j.clone();
        let query_owned = query.to_string();

        // Spawn background producer
        tokio::spawn(async move {
            // Call the synchronous query (in future could be made truly streaming)
            if let Err(e) = Self::run_simple_streaming(
                code_graph_db,
                code_graph_vector_store,
                code_graph_neo4j,
                &query_owned,
                top_k,
                config,
                tx,
            )
            .await
            {
                // Note: tx was moved into run_simple_streaming, can't send error chunk here
                // Errors are handled within run_simple_streaming by sending error chunks
                let _ = e; // Suppress unused variable warning
            }
        });

        // Yield to give spawned task a chance to start before receiver tries to recv
        // This prevents race condition where recv() is called before task starts
        tokio::task::yield_now().await;

        Ok(rx)
    }

    /// Simple passthrough streaming implementation
    /// Delegates to search_code and emits results as chunks
    async fn run_simple_streaming(
        db: Arc<std::sync::Mutex<rusqlite::Connection>>,
        vector_store: Arc<std::sync::Mutex<crate::vector::VectorStore>>,
        code_graph_neo4j: Option<Arc<crate::graph::Neo4jClient>>,
        query: &str,
        top_k: usize,
        config: StreamingConfig,
        tx: mpsc::Sender<RagStreamingChunk>,
    ) -> Result<()> {
        use super::fusion_router::{FusionMode, FusionRouter};
        use super::graph::CodeGraph;

        // Empty query handling
        if query.is_empty() {
            let _ = tx
                .send(RagStreamingChunk {
                    ranked_entities: vec![],
                    is_final: true,
                    debug_trace: Some("Empty query".to_string()),
                })
                .await;
            return Ok(());
        }

        // Reconstruct Parser (cheap operation)
        let parser = match crate::parser::Parser::new() {
            Ok(p) => p,
            Err(e) => {
                let _ = tx
                    .send(RagStreamingChunk {
                        ranked_entities: vec![],
                        is_final: true,
                        debug_trace: Some(format!("Parser init error: {}", e)),
                    })
                    .await;
                return Ok(());
            }
        };

        // Reconstruct CodeGraph temporarily (all components are Arc so this is cheap)
        let temp_code_graph = CodeGraph {
            db,
            vector_store,
            parser,
            neo4j: code_graph_neo4j,
            version: std::sync::atomic::AtomicU64::new(0),
        };

        // Step 1: Select fusion mode
        let router = FusionRouter::new();
        let selected_mode = router.select_mode(query);

        // Step 2: Vector search
        let vector_matches = match temp_code_graph.search_code(query, top_k * 2) {
            Ok(matches) => matches,
            Err(e) => {
                let _ = tx
                    .send(RagStreamingChunk {
                        ranked_entities: vec![],
                        is_final: true,
                        debug_trace: Some(format!("Vector search error: {}", e)),
                    })
                    .await;
                return Ok(());
            }
        };

        // Emit vector chunk if configured
        if config.chunk_vector {
            let vector_entities: Vec<RankedEntity> = vector_matches
                .iter()
                .take(top_k)
                .map(|vmatch| RankedEntity {
                    entity: vmatch.entity.clone(),
                    combined_score: vmatch.score,
                    vector_score: vmatch.score,
                    graph_score: 0.0,
                    temporal_score: 0.0,
                    graph_embedding_score: Some(0.0),
                })
                .collect();

            let _ = tx
                .send(RagStreamingChunk {
                    ranked_entities: vector_entities,
                    is_final: false,
                    debug_trace: Some(format!("Vector: {} matches", vector_matches.len())),
                })
                .await;
        }

        // Step 3: Graph expansion (simplified - just increment scores)
        // In real implementation, this would query Neo4j for neighbors
        let mut ranked_entities = Vec::new();

        for vmatch in vector_matches.iter().take(top_k) {
            let vector_score = vmatch.score;

            // Simplified graph score (real implementation would query Neo4j)
            let graph_score = 0.1;

            // Simplified temporal score
            let temporal_score = super::fusion_simple::compute_temporal_score(
                vmatch.entity.last_modified_at.unwrap_or(0),
                vmatch.entity.change_count.unwrap_or(1),
                vmatch.entity.author_count.unwrap_or(1),
            );

            // GraphBERT: Graph embedding score (GRAPH domain)
            // Extract graph features and compute heuristic score
            let entity_id = vmatch.entity.id.unwrap_or(0);
            let graph_embedding_score = if entity_id > 0 {
                match temp_code_graph.extract_graph_features(entity_id) {
                    Ok(features) => super::fusion_simple::compute_graph_embedding_score(&features),
                    Err(_) => 0.0, // Fallback to 0.0 if feature extraction fails
                }
            } else {
                0.0 // Entity not in database yet
            };

            // Apply fusion using FusionSimple for consistency with sync path
            let base_score = match selected_mode {
                FusionMode::Simple => {
                    use super::fusion_simple::FusionSimple;
                    let fusion = FusionSimple::default();
                    fusion.combine(vector_score, graph_score, temporal_score, graph_embedding_score)
                }
                FusionMode::Attention => {
                    // Simplified attention fusion
                    vector_score * 0.7 + graph_score * 0.3
                }
                FusionMode::Reasoning => {
                    // Simplified reasoning fusion
                    vector_score * 0.5 + graph_score * 0.5
                }
            };

            // Apply entity boost
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
                Some(graph_embedding_score),
            });
        }

        // Emit graph chunk if configured
        if config.chunk_graph {
            let _ = tx
                .send(RagStreamingChunk {
                    ranked_entities: ranked_entities.clone(),
                    is_final: false,
                    debug_trace: Some("Graph expansion complete".to_string()),
                })
                .await;
        }

        // Step 4: Sort by combined score
        ranked_entities.sort_by(|a, b| b.combined_score.partial_cmp(&a.combined_score).unwrap());
        ranked_entities.truncate(top_k);

        // Emit fusion chunk if configured
        if config.chunk_fusion {
            let _ = tx
                .send(RagStreamingChunk {
                    ranked_entities: ranked_entities.clone(),
                    is_final: false,
                    debug_trace: Some("Fusion scoring complete".to_string()),
                })
                .await;
        }

        // Emit final chunk
        let _ = tx
            .send(RagStreamingChunk {
                ranked_entities,
                is_final: true,
                debug_trace: Some(format!("Final results: top_k={}", top_k)),
            })
            .await;

        // Brief yield to ensure message is buffered before sender drops
        tokio::task::yield_now().await;

        Ok(())
    }
}
