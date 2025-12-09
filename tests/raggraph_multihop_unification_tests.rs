//! Tests for raggraph_multihop unification and compatibility
//!
//! These tests validate the critical fixes needed for the multihop tool:
//! 1. Single authoritative RagGraphMultihopRequest definition
//! 2. Backward compatibility for query_text normalization
//! 3. MCP server unified request building
//! 4. Proper error handling for missing required fields
//! 5. Neo4jClient GraphBackend adapter
//! 6. format_error_response parameter fixes
//! 7. format_success_response parameter fixes

use serde_json::json;

// ============================================
// A) Ensure ONLY ONE authoritative struct exists
// ============================================

#[test]
fn test_multihop_single_authoritative_definition() {
    use syncore::mcp_server::types::RagGraphMultihopRequest as ServerReq;
    use syncore::raggraph::types::RagGraphMultihopRequest as GraphReq;

    // They MUST NOT diverge in field names or semantics
    // (Fail until unified)
    let server_fields = vec![
        "query",
        "query_text",
        "seed_nodes",
        "max_hops",
        "max_entities",
        "decay_factor",
        "namespace",
        "scope",
    ];

    let graph_fields = vec![
        "query_text",
        "seed_nodes",       // currently String vs i64 mismatch
        "max_hops",
        "max_entities",
        "decay_factor",
    ];

    // Failing assertion until fixed
    assert_eq!(server_fields, graph_fields);
}

// ============================================
// B) Ensure backward compatibility semantics remain valid
// ============================================

#[test]
fn test_backward_compatibility_query_text_normalization() {
    use syncore::raggraph::types::RagGraphMultihopRequest;

    let mut req = RagGraphMultihopRequest {
        query_text: "".into(),
        seed_nodes: vec![],
        max_hops: None,
        max_entities: None,
        decay_factor: None,
    };

    req.normalize_query_text();

    // Should auto-fill with default
    assert!(!req.query_text.is_empty());
}

// ============================================
// C) Ensure MCP server can build a unified request
// ============================================

#[test]
fn test_mcp_build_unified_multihop_request() {
    use syncore::mcp_server::reasoning::request_parsing;
    use syncore::mcp_server::types::RagGraphMultihopRequest;

    let req = RagGraphMultihopRequest {
        query: "test".into(),
        query_text: "test".into(),
        seed_nodes: vec![1, 2, 3],
        max_hops: Some(3),
        max_entities: Some(20),
        decay_factor: Some(0.5),
        namespace: None,
        scope: None,
    };

    // Should NOT panic and must map all fields 1:1
    let unified = request_parsing::build_unified_multihop_request_from_struct(&req)
        .expect("Expected unified request");

    assert_eq!(unified.query_text, "test");
    assert_eq!(unified.seed_nodes.len(), 3);
    assert_eq!(unified.max_hops, Some(3));
}

// ============================================
// D) Ensure MCP server returns an error when required fields missing
// ============================================

#[test]
fn test_mcp_multihop_missing_query_fails() {
    use syncore::mcp_server::reasoning::request_parsing;
    use syncore::mcp_server::types::RagGraphMultihopRequest;

    let req = RagGraphMultihopRequest {
        query: "".into(),       // missing query
        query_text: "".into(),
        seed_nodes: vec![1],
        max_hops: Some(1),
        max_entities: None,
        decay_factor: None,
        namespace: None,
        scope: None,
    };

    let result = request_parsing::build_unified_multihop_request_from_struct(&req);
    assert!(result.is_err(), "Missing query must cause error");
}

// ============================================
// E) Ensure Neo4jClient is properly adapted to GraphBackend
// ============================================

#[test]
fn test_neo4j_backend_wrapper_exists() {
    use syncore::graph::backend::GraphBackend;
    use syncore::graph::neo4j_client::Neo4jClient;

    fn assert_backend<T: GraphBackend>() {}

    // This MUST compile once the adapter is added
    // (Fails now)
    // assert_backend::<Neo4jClient>();

    assert!(false, "Neo4jClient must implement GraphBackend or have adapter");
}

// ============================================
// F) Ensure format_error_response receives full argument list
// ============================================

#[test]
fn test_format_error_response_parameter_count() {
    use syncore::mcp_server::reasoning::response_formatting::format_error_response;
    use syncore::mcp_server::reasoning::{ReasoningTrace, ReasoningEvaluation};

    // These are placeholders — purpose is signature validation only.
    let metadata = serde_json::json!({});
    let error = anyhow::anyhow!("test");
    let category = syncore::mcp_server::reasoning::ErrorCategory::Internal;

    // MUST compile only after fixing signature usage everywhere
    let _ = format_error_response(
        metadata.clone(),
        error,
        category,
        Some("ctx".into()),
        Some(metadata.clone()),
        None::<ReasoningTrace>,
        None::<ReasoningEvaluation>,
    );

    assert!(true);
}

// ============================================
// G) Ensure format_success_response receives 8 arguments
// ============================================

#[test]
fn test_format_success_response_parameter_count() {
    use syncore::mcp_server::reasoning::response_formatting::format_success_response;
    use syncore::mcp_server::reasoning::{ReasoningTrace, ReasoningEvaluation};

    let metadata = serde_json::json!({});

    let _ = format_success_response(
        metadata.clone(),
        serde_json::json!({"ok": true}),
        serde_json::json!({}),
        None,
        None,
        None::<ReasoningTrace>,
        None::<ReasoningEvaluation>,
    );

    assert!(true);
}