//! Context Composer Module
//!
//! Composes unified ContextBundle by merging:
//! - RAGGraph results
//! - LTMC vector memory
//! - LTMC SQL memory
//! - LTMC graph memory (Neo4j)
//! - LTMC cache memory
//!
//! Applies deduplication, priority ranking, and token budget controls.

use super::context_bundle::*;
use super::router_logic::RoutingDecision;
use crate::code_graph::CodeGraph;
use crate::graph::Neo4jClient;
use crate::memory::Memory;
use anyhow::Result;
use std::collections::HashSet;

/// Configuration for LTMC lookups
#[derive(Debug, Clone)]
pub struct LtmcLookupConfig {
    /// Max vector search results
    pub vector_top_k: usize,
    /// Max SQL memory records
    pub sql_top_k: usize,
    /// Graph traversal depth (hops)
    pub graph_hops: usize,
    /// Cache entry depth
    pub cache_depth: usize,
}

impl Default for LtmcLookupConfig {
    fn default() -> Self {
        Self {
            vector_top_k: 10,
            sql_top_k: 10,
            graph_hops: 2,
            cache_depth: 5,
        }
    }
}

/// Context Composer
///
/// Merges RAGGraph and LTMC memory into unified ContextBundle
pub struct ContextComposer {
    config: LtmcLookupConfig,
}

impl ContextComposer {
    /// Create new ContextComposer with default config
    pub fn new() -> Self {
        Self {
            config: LtmcLookupConfig::default(),
        }
    }

    /// Create ContextComposer with custom config
    pub fn with_config(config: LtmcLookupConfig) -> Self {
        Self { config }
    }

    /// Compose unified ContextBundle
    ///
    /// # Arguments
    /// * `query` - User query text
    /// * `decision` - Routing decision from cognitive router
    /// * `code_graph` - CodeGraph instance
    /// * `memory` - LTMC Memory instance
    /// * `neo4j` - Optional Neo4j client for graph memory
    ///
    /// # Returns
    /// Unified ContextBundle with all memory systems merged
    pub async fn compose(
        &self,
        query: &str,
        decision: &RoutingDecision,
        code_graph: &CodeGraph,
        memory: &Memory,
        neo4j: Option<&Neo4jClient>,
    ) -> Result<ContextBundle> {
        let mut bundle =
            ContextBundle::with_mode(decision.mode_hint.as_deref().unwrap_or("simple"));

        // Track seen entity IDs for deduplication
        let mut seen_entities: HashSet<String> = HashSet::new();
        let mut dedup_count = 0;

        // 1. Fetch RAGGraph entities (if we had real results)
        // For now, RAGGraph integration happens at orchestrator level
        // This composer receives the results and enriches them with LTMC data

        // 2. Fetch LTMC vector memory hits
        let vector_hits = self.fetch_vector_memory(query, code_graph).await?;
        for hit in vector_hits {
            let key = format!("vec_{}", hit.id);
            if !seen_entities.contains(&key) {
                seen_entities.insert(key);
                bundle.add_vector_hit(hit);
            } else {
                dedup_count += 1;
            }
        }

        // 3. Fetch LTMC SQL memory
        let sql_records = self.fetch_sql_memory(query, memory)?;
        for record in sql_records {
            let key = format!("sql_{}", record.key);
            if !seen_entities.contains(&key) {
                seen_entities.insert(key);
                bundle.add_sql_record(record);
            } else {
                dedup_count += 1;
            }
        }

        // 4. Fetch LTMC graph memory (if Neo4j available)
        if let Some(neo4j_client) = neo4j {
            let graph_relations = self.fetch_graph_memory(query, neo4j_client).await?;
            for relation in graph_relations {
                let key = format!("graph_{}_{}", relation.source_id, relation.target_id);
                if !seen_entities.contains(&key) {
                    seen_entities.insert(key);
                    bundle.add_graph_relation(relation);
                } else {
                    dedup_count += 1;
                }
            }
        }

        // 5. Fetch LTMC cache entries
        let cache_entries = self.fetch_cache_memory(memory)?;
        for entry in cache_entries {
            bundle.add_cache_entry(entry);
        }

        // Mark deduplication
        bundle.mark_deduplicated(dedup_count);

        // Add reasoning metadata
        bundle.set_reasoning_trace(serde_json::json!({
            "query": query,
            "fusion_mode": decision.mode_hint,
            "top_k": decision.top_k,
            "depth": decision.depth,
            "reasoning": decision.reasoning
        }));

        Ok(bundle)
    }

    /// Fetch vector memory hits from LTMC
    async fn fetch_vector_memory(
        &self,
        query: &str,
        _code_graph: &CodeGraph,
    ) -> Result<Vec<LtmcVectorHit>> {
        let mut hits = Vec::new();

        // Search vector store (part of code_graph)
        // Note: VectorStore is inside CodeGraph's vector_store Arc<Mutex>
        // For now, return empty as we'd need to access the vector store directly
        // This would be populated when integrated with full LTMC vector backend

        // Placeholder: In real implementation, this would query LTMC vector DB
        let _ = query; // Use query to search

        // Limit results
        hits.truncate(self.config.vector_top_k);

        Ok(hits)
    }

    /// Fetch SQL memory records
    fn fetch_sql_memory(&self, query: &str, memory: &Memory) -> Result<Vec<LtmcSqlRecord>> {
        let mut records = Vec::new();

        // Try to query memory for relevant keys
        // Memory has query() method that searches by key
        if let Ok(value) = memory.query(query) {
            records.push(LtmcSqlRecord {
                key: query.to_string(),
                value: value.unwrap_or_default(),
                timestamp: None,
                relevance: 1.0,
            });
        }

        // Limit results
        records.truncate(self.config.sql_top_k);

        Ok(records)
    }

    /// Fetch graph memory relationships from Neo4j
    async fn fetch_graph_memory(
        &self,
        _query: &str,
        _neo4j: &Neo4jClient,
    ) -> Result<Vec<LtmcGraphRelation>> {
        let relations = Vec::new();

        // In real implementation, this would:
        // 1. Find relevant nodes in Neo4j based on query
        // 2. Traverse up to self.config.graph_hops
        // 3. Return relationships

        // For now, return empty
        Ok(relations)
    }

    /// Fetch recent cache entries
    fn fetch_cache_memory(&self, _memory: &Memory) -> Result<Vec<LtmcCacheEntry>> {
        let entries = Vec::new();

        // In real implementation, this would:
        // 1. Query Sled cache from Memory
        // 2. Get recent entries up to self.config.cache_depth
        // 3. Return entries

        // For now, return empty
        Ok(entries)
    }
}

impl Default for ContextComposer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_composer_creation() {
        let composer = ContextComposer::new();
        assert_eq!(composer.config.vector_top_k, 10);
    }

    #[test]
    fn test_composer_with_config() {
        let config = LtmcLookupConfig {
            vector_top_k: 5,
            sql_top_k: 3,
            graph_hops: 1,
            cache_depth: 2,
        };
        let composer = ContextComposer::with_config(config);
        assert_eq!(composer.config.vector_top_k, 5);
    }
}
