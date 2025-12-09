//! Tests for unified reasoning request parsing
//!
//! This test file verifies that typed helper functions eliminate
//! JSON roundtrips for MULTIHOP and FUSION reasoning tools while
//! preserving existing behavior and parameter semantics.

use serde_json::{json, Value};
use syncore::mcp_server::reasoning::request_parsing::{
    UnifiedReasoningRequest, RequestType, RequestParameters,
    build_unified_multihop_request_from_struct,
    build_unified_fusion_request_from_struct,
};
use syncore::mcp_server::types::{
    RagGraphMultihopRequest,
};
use syncore::code_graph::rag_graph::RagGraphQueryRequest;

/// Test multihop typed parsing equivalence
#[test]
fn test_multihop_typed_parsing_equivalence() {
    // Construct a dummy RagGraphMultihopRequest with seed_nodes + optional fields
    let multihop_request = RagGraphMultihopRequest {
        seed_nodes: vec![1, 2, 3],
        max_hops: Some(5),
        max_entities: Some(100),
        decay_factor: Some(0.8),
    };

    // Call the NEW typed helper
    let unified_request = build_unified_multihop_request_from_struct(&multihop_request)
        .expect("Failed to build unified multihop request");

    // Assert:
    // - The resulting unified request has seed_entities matching seed_nodes
    // - All other fields (max_hops, max_entities, decay_factor) are present as expected
    assert_eq!(unified_request.request_type, RequestType::MultiHop);

    if let RequestParameters::MultiHop {
        seed_entities,
        max_hops,
        max_entities,
        decay_factor,
    } = unified_request.parameters
    {
        assert_eq!(seed_entities, vec![1, 2, 3]);
        assert_eq!(max_hops, Some(5));
        assert_eq!(max_entities, Some(100));
        assert_eq!(decay_factor, Some(0.8));
    } else {
        panic!("Expected MultiHop parameters");
    }

    println!("✓ multihop typed helper successfully converted RagGraphMultihopRequest to UnifiedReasoningRequest");
}

/// Test fusion typed parsing equivalence
#[test]
fn test_fusion_typed_parsing_equivalence() {
    // Construct a dummy RagGraphQueryRequest for fusion with required fields
    let fusion_request = RagGraphQueryRequest {
        query: "test fusion query".to_string(),
        namespace: Some("test_namespace".to_string()),
        mode_hint: Some("reasoning".to_string()),
        top_k: Some(20),
        scope: Some("project".to_string()),
        project_label: Some("SynCore".to_string()),
        local_root: Some("/test/src".to_string()),
    };

    // Call the NEW typed helper for fusion
    let unified_request = build_unified_fusion_request_from_struct(&fusion_request)
        .expect("Failed to build unified fusion request");

    // Assert:
    // - All these fields are represented in the unified request
    // - enable_temporal is true in the unified request
    assert_eq!(unified_request.request_type, RequestType::Fusion);
    assert_eq!(unified_request.query, "test fusion query");
    assert_eq!(unified_request.namespace, Some("test_namespace".to_string()));
    assert_eq!(unified_request.mode_hint, Some("reasoning".to_string()));
    assert_eq!(unified_request.top_k, Some(20));
    assert_eq!(unified_request.scope, Some("project".to_string()));
    assert_eq!(unified_request.project_label, Some("SynCore".to_string()));
    assert_eq!(unified_request.local_root, Some("/test/src".to_string()));

    if let RequestParameters::Fusion {
        fusion_mode,
        entity_boost,
        enable_temporal,
    } = unified_request.parameters
    {
        assert_eq!(enable_temporal, true); // Critical: enable_temporal must be injected as true
        // fusion_mode and entity_boost should be None for this test case
        assert_eq!(fusion_mode, None);
        assert_eq!(entity_boost, None);
    } else {
        panic!("Expected Fusion parameters");
    }

    println!("✓ fusion typed helper successfully converted RagGraphQueryRequest to UnifiedReasoningRequest with enable_temporal=true");
}

/// Test multihop no JSON roundtrip
#[test]
fn test_multihop_no_json_roundtrip() {
    // This test verifies raggraph_multihop handler uses typed helper instead of JSON construction

    let handler_code = include_str!("../src/mcp_server/server.rs");

    // Find the raggraph_multihop handler section
    let multihop_section = handler_code.split("async fn raggraph_multihop").nth(1).unwrap_or("");
    let multihop_section = multihop_section.split("#[tool").nth(0).unwrap_or(multihop_section);

    // Verify the handler uses the typed helper
    assert!(
        multihop_section.contains("build_unified_multihop_request_from_struct"),
        "raggraph_multihop handler should use build_unified_multihop_request_from_struct()"
    );

    // Verify the handler does NOT construct JSON manually
    assert!(
        !multihop_section.contains("serde_json::json!"),
        "raggraph_multihop handler should not use serde_json::json!()"
    );

    assert!(
        !multihop_section.contains("seed_entities"),
        "raggraph_multihop handler should not manually construct seed_entities JSON"
    );

    println!("✓ raggraph_multihop handler correctly uses typed helper instead of JSON construction");
}

/// Test fusion no JSON roundtrip
#[test]
fn test_fusion_no_json_roundtrip() {
    // This test verifies code_graph_fusion_query handler uses typed helper instead of JSON construction

    let handler_code = include_str!("../src/mcp_server/server.rs");

    // Find the code_graph_fusion_query handler section
    let fusion_section = handler_code.split("async fn code_graph_fusion_query").nth(1).unwrap_or("");
    let fusion_section = fusion_section.split("#[tool").nth(0).unwrap_or(fusion_section);

    // Verify the handler uses the typed helper
    assert!(
        fusion_section.contains("build_unified_fusion_request_from_struct"),
        "code_graph_fusion_query handler should use build_unified_fusion_request_from_struct()"
    );

    // Verify the handler does NOT construct JSON manually
    assert!(
        !fusion_section.contains("serde_json::json!"),
        "code_graph_fusion_query handler should not use serde_json::json!()"
    );

    assert!(
        !fusion_section.contains("enable_temporal\": true"),
        "code_graph_fusion_query handler should not manually inject enable_temporal JSON"
    );

    println!("✓ code_graph_fusion_query handler correctly uses typed helper instead of JSON construction");
}

/// Regression test for raggraph_query
#[test]
fn test_raggraph_query_behavior_unchanged() {
    // This test ensures raggraph_query behavior remains exactly as-is

    // Construct a basic RagGraphQueryRequest like raggraph_query would receive
    let query_request = RagGraphQueryRequest {
        query: "test query".to_string(),
        namespace: Some("test".to_string()),
        mode_hint: Some("simple".to_string()),
        top_k: Some(10),
        scope: Some("local".to_string()),
        project_label: Some("test_project".to_string()),
        local_root: Some("/test".to_string()),
    };

    // Verify the request structure is valid
    assert_eq!(query_request.query, "test query");
    assert_eq!(query_request.namespace, Some("test".to_string()));
    assert_eq!(query_request.mode_hint, Some("simple".to_string()));
    assert_eq!(query_request.top_k, Some(10));

    // Verify that raggraph_query handler exists and has the right structure
    let handler_code = include_str!("../src/mcp_server/server.rs");

    // Find the raggraph_query handler section
    let raggraph_query_section = handler_code.split("async fn raggraph_query").nth(1).unwrap_or("");
    let raggraph_query_section = raggraph_query_section.split("#[tool").nth(0).unwrap_or(raggraph_query_section);

    // Verify the handler exists
    assert!(!raggraph_query_section.is_empty(), "raggraph_query handler should exist");

    // Verify raggraph_query does NOT use the new typed helpers (should remain unchanged)
    assert!(
        !raggraph_query_section.contains("build_unified_multihop_request_from_struct"),
        "raggraph_query handler should not use multihop typed helper"
    );

    assert!(
        !raggraph_query_section.contains("build_unified_fusion_request_from_struct"),
        "raggraph_query handler should not use fusion typed helper"
    );

    assert!(
        !raggraph_query_section.contains("parse_unified_request"),
        "raggraph_query handler should not use parse_unified_request()"
    );

    // Verify raggraph_query maintains its original structure
    assert!(
        raggraph_query_section.contains("Parameters<RagGraphQueryRequest>"),
        "raggraph_query handler should still use Parameters<RagGraphQueryRequest>"
    );

    println!("✓ raggraph_query handler behavior remains unchanged (does not use unified parsing)");
}