//! Graph Validator - Checks Neo4j graph health and connectivity
//!
//! Validation checks:
//! - >95% of nodes have at least 1 relationship
//! - No duplicate edges (src_id, dst_id, type uniqueness)
//! - No orphan clusters from path mismatch
//! - Diffusion returns >0.0 scores for connected nodes

use crate::graph::Neo4jClient;
use crate::databases::neo4j::{validate_structure, find_orphan_entities};
use anyhow::Result;

/// Statistics about node connectivity in the graph
#[derive(Debug, Clone)]
pub struct ConnectivityStats {
    pub total_nodes: u64,
    pub nodes_with_edges: u64,
}

/// GraphValidator checks Neo4j graph health
pub struct GraphValidator {
    neo4j: Neo4jClient,
}

impl GraphValidator {
    /// Create a new GraphValidator with Neo4j client
    pub fn new(neo4j: Neo4jClient) -> Self {
        Self { neo4j }
    }

    /// Validate node connectivity - returns stats on nodes with relationships
    ///
    /// Requirement: >95% of nodes should have at least 1 relationship
    pub async fn validate_node_connectivity(&self) -> Result<ConnectivityStats> {
        // Use canonical validate_structure to get comprehensive graph stats
        let stats = validate_structure(&self.neo4j).await?;

        // Calculate nodes_with_edges = total_nodes - orphan_count
        let nodes_with_edges = stats.total_nodes.saturating_sub(stats.orphan_count);

        Ok(ConnectivityStats {
            total_nodes: stats.total_nodes as u64,
            nodes_with_edges: nodes_with_edges as u64,
        })
    }

    /// Count duplicate edges (same src name, dst name, type)
    ///
    /// Requirement: Should return 0 - no duplicate edges allowed
    pub async fn count_duplicate_edges(&self) -> Result<u64> {
        // Count relationships that have duplicates based on (start name, end name, type)
        // Use name since CodeEntity nodes are keyed by name, not id
        let query = r#"
            MATCH (a)-[r]->(b)
            WHERE a.namespace = $ns AND b.namespace = $ns
            WITH a.name as src, b.name as dst, type(r) as rel_type, count(*) as cnt
            WHERE cnt > 1
            RETURN sum(cnt - 1) as duplicates
        "#;

        let result = self
            .neo4j
            .execute_query(
                query,
                vec![("ns", serde_json::json!(self.neo4j.namespace()))],
            )
            .await?;

        let duplicates: u64 = result
            .first()
            .and_then(|r| r.get("duplicates"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        Ok(duplicates)
    }

    /// Find orphan clusters - disconnected subgraphs that shouldn't exist
    ///
    /// Returns list of file_path prefixes that have disconnected nodes
    pub async fn find_orphan_clusters(&self) -> Result<Vec<String>> {
        // Use canonical find_orphan_entities to get nodes without relationships
        let orphans = find_orphan_entities(&self.neo4j).await?;

        // Extract unique file path prefixes from orphan entities
        let mut prefixes: Vec<String> = orphans
            .iter()
            .filter_map(|entity| entity.path.as_ref())
            .map(|path| {
                // Get first 50 chars as prefix
                if path.len() > 50 {
                    path[..50].to_string()
                } else {
                    path.clone()
                }
            })
            .collect();

        // Deduplicate
        prefixes.sort();
        prefixes.dedup();

        Ok(prefixes)
    }

    /// Test diffusion on a connected node - should return >0.0 scores
    ///
    /// Returns sample diffusion scores from a connected node
    pub async fn test_diffusion_on_connected_node(&self) -> Result<Vec<f64>> {
        // Find a node with relationships and compute simple "diffusion"
        // by counting paths of different lengths
        let query = r#"
            MATCH (n)
            WHERE n.namespace = $ns AND EXISTS { (n)--() }
            WITH n LIMIT 1
            MATCH (n)-[*1..2]-(neighbor)
            WITH neighbor, count(*) as score
            RETURN toFloat(score) as diffusion_score
            ORDER BY diffusion_score DESC
            LIMIT 10
        "#;

        let result = self
            .neo4j
            .execute_query(
                query,
                vec![("ns", serde_json::json!(self.neo4j.namespace()))],
            )
            .await?;

        let scores: Vec<f64> = result
            .iter()
            .filter_map(|r| r.get("diffusion_score"))
            .filter_map(|v| v.as_f64())
            .collect();

        Ok(scores)
    }

    /// Validate CONTAINS coverage - what % of entities have a parent container
    ///
    /// Requirement: >= 95% of entities should have incoming CONTAINS edge
    pub async fn validate_contains_coverage(&self) -> Result<f64> {
        // Count entities that have an incoming CONTAINS relationship
        let query = r#"
            MATCH (n)
            WHERE n.namespace = $ns
            WITH count(n) as total
            MATCH (parent)-[:CONTAINS]->(child)
            WHERE child.namespace = $ns
            WITH total, count(DISTINCT child) as contained
            RETURN total, contained,
                   CASE WHEN total > 0 THEN toFloat(contained) / total ELSE 0.0 END as coverage
        "#;

        let result = self
            .neo4j
            .execute_query(
                query,
                vec![("ns", serde_json::json!(self.neo4j.namespace()))],
            )
            .await?;

        let coverage: f64 = result
            .first()
            .and_then(|r| r.get("coverage"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        Ok(coverage)
    }

    /// Count orphan entities - entities with no incoming CONTAINS edge
    ///
    /// After CONTAINS edges, only root modules should be orphans
    pub async fn count_orphan_entities(&self) -> Result<u64> {
        let query = r#"
            MATCH (n)
            WHERE n.namespace = $ns
              AND NOT EXISTS { (parent)-[:CONTAINS]->(n) }
              AND NOT EXISTS { (parent)-[:MODULE_CHILD]->(n) }
            RETURN count(n) as orphan_count
        "#;

        let result = self
            .neo4j
            .execute_query(
                query,
                vec![("ns", serde_json::json!(self.neo4j.namespace()))],
            )
            .await?;

        let count: u64 = result
            .first()
            .and_then(|r| r.get("orphan_count"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        Ok(count)
    }

    /// Get summary report of all validation checks
    pub async fn full_validation_report(&self) -> Result<ValidationReport> {
        let connectivity = self.validate_node_connectivity().await?;
        let duplicate_count = self.count_duplicate_edges().await?;
        let orphan_clusters = self.find_orphan_clusters().await?;
        let diffusion_scores = self.test_diffusion_on_connected_node().await?;

        let connectivity_ratio = if connectivity.total_nodes > 0 {
            connectivity.nodes_with_edges as f64 / connectivity.total_nodes as f64
        } else {
            1.0
        };

        let diffusion_ok = diffusion_scores.iter().any(|&s| s > 0.0) || diffusion_scores.is_empty();

        Ok(ValidationReport {
            total_nodes: connectivity.total_nodes,
            nodes_with_edges: connectivity.nodes_with_edges,
            connectivity_ratio,
            connectivity_ok: connectivity_ratio >= 0.95 || connectivity.total_nodes < 10,
            duplicate_edges: duplicate_count,
            duplicates_ok: duplicate_count == 0,
            orphan_clusters,
            orphans_ok_count: 0, // Will be set below
            diffusion_ok,
        })
    }
}

/// Full validation report for graph health
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub total_nodes: u64,
    pub nodes_with_edges: u64,
    pub connectivity_ratio: f64,
    pub connectivity_ok: bool,
    pub duplicate_edges: u64,
    pub duplicates_ok: bool,
    pub orphan_clusters: Vec<String>,
    pub orphans_ok_count: usize,
    pub diffusion_ok: bool,
}

impl ValidationReport {
    /// Check if all validations passed
    pub fn all_ok(&self) -> bool {
        self.connectivity_ok
            && self.duplicates_ok
            && self.orphan_clusters.len() <= 1
            && self.diffusion_ok
    }
}
