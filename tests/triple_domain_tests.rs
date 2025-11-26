//! Triple-Domain Embedding Tests (TDD)
//!
//! Tests for GRAPH domain addition to existing CODE/GENERAL dual-domain architecture.
//! Written BEFORE implementation to define the contract.
//!
//! Test Coverage:
//! 1. EmbeddingDomain::Graph variant exists and behaves correctly
//! 2. GRAPH domain has distinct namespace mapping
//! 3. GRAPH domain has distinct index path
//! 4. EmbeddingConfig supports GRAPH domain
//! 5. Backward compatibility: CODE and GENERAL unchanged

use syncore::vector::domain::{EmbeddingConfig, EmbeddingDomain};

// ============================================================================
// PHASE 1 TESTS: Domain Extension
// ============================================================================

#[test]
fn test_graph_domain_variant_exists() {
    // GRAPH domain must be a valid enum variant
    let graph = EmbeddingDomain::Graph;
    assert_eq!(format!("{:?}", graph), "Graph");
}

#[test]
fn test_graph_domain_display() {
    let graph = EmbeddingDomain::Graph;
    assert_eq!(format!("{}", graph), "graph");
}

#[test]
fn test_graph_domain_from_namespace() {
    // GRAPH domain namespaces
    assert_eq!(
        EmbeddingDomain::from_namespace("graph_entity"),
        EmbeddingDomain::Graph
    );
    assert_eq!(
        EmbeddingDomain::from_namespace("graph_node"),
        EmbeddingDomain::Graph
    );
    assert_eq!(
        EmbeddingDomain::from_namespace("code_graph"),
        EmbeddingDomain::Graph
    );
}

#[test]
fn test_graph_domain_index_path_distinct() {
    let code_path = EmbeddingDomain::Code.default_index_path();
    let general_path = EmbeddingDomain::General.default_index_path();
    let graph_path = EmbeddingDomain::Graph.default_index_path();

    // All three domains must have distinct index paths
    assert_ne!(code_path, general_path);
    assert_ne!(code_path, graph_path);
    assert_ne!(general_path, graph_path);

    // GRAPH index path must contain "graph"
    assert!(graph_path.contains("graph"));
}

#[test]
fn test_graph_domain_equality() {
    assert_eq!(EmbeddingDomain::Graph, EmbeddingDomain::Graph);
    assert_ne!(EmbeddingDomain::Graph, EmbeddingDomain::Code);
    assert_ne!(EmbeddingDomain::Graph, EmbeddingDomain::General);
}

#[test]
fn test_graph_domain_serialization() {
    let graph = EmbeddingDomain::Graph;
    let json = serde_json::to_string(&graph).unwrap();
    assert_eq!(json, r#""graph""#);
}

#[test]
fn test_graph_domain_deserialization() {
    let graph: EmbeddingDomain = serde_json::from_str(r#""graph""#).unwrap();
    assert_eq!(graph, EmbeddingDomain::Graph);
}

#[test]
fn test_embedding_config_for_graph() {
    let config = EmbeddingConfig::for_graph();

    assert_eq!(config.domain, EmbeddingDomain::Graph);
    assert!(!config.model_name.is_empty());
    assert!(config.index_path.contains("graph"));
    assert_eq!(config.dimension, 384); // Same dimension as CODE/GENERAL for now
}

#[test]
fn test_embedding_config_for_domain_graph() {
    let config = EmbeddingConfig::for_domain(EmbeddingDomain::Graph);
    assert_eq!(config.domain, EmbeddingDomain::Graph);
}

#[test]
fn test_graph_config_validation() {
    let config = EmbeddingConfig::for_graph();
    assert!(config.validate().is_ok());
}

#[test]
fn test_triple_domain_index_paths_all_distinct() {
    let code_config = EmbeddingConfig::for_code();
    let general_config = EmbeddingConfig::for_general();
    let graph_config = EmbeddingConfig::for_graph();

    // All three must have distinct index paths
    assert_ne!(code_config.index_path, general_config.index_path);
    assert_ne!(code_config.index_path, graph_config.index_path);
    assert_ne!(general_config.index_path, graph_config.index_path);
}

// ============================================================================
// BACKWARD COMPATIBILITY TESTS
// ============================================================================

#[test]
fn test_code_domain_unchanged() {
    // APEX 2.0-E: CODE domain behavior (GPU upgrade)
    let code = EmbeddingDomain::Code;
    assert_eq!(format!("{}", code), "code");
    assert_eq!(
        EmbeddingDomain::from_namespace("code_entity"),
        EmbeddingDomain::Code
    );
    let config = EmbeddingConfig::for_code();
    assert_eq!(config.model_name, "bge-m3");  // APEX 2.0-E: GPU model
}

#[test]
fn test_general_domain_unchanged() {
    // APEX 2.0-E: GENERAL domain behavior (GPU upgrade)
    let general = EmbeddingDomain::General;
    assert_eq!(format!("{}", general), "general");
    assert_eq!(
        EmbeddingDomain::from_namespace("documents"),
        EmbeddingDomain::General
    );
    let config = EmbeddingConfig::for_general();
    assert_eq!(config.model_name, "bge-m3");  // APEX 2.0-E: GPU model
}

#[test]
fn test_unknown_namespace_still_defaults_to_general() {
    // Unknown namespaces should still default to GENERAL (not GRAPH)
    // This preserves existing behavior
    assert_eq!(
        EmbeddingDomain::from_namespace("unknown_namespace"),
        EmbeddingDomain::General
    );
}
