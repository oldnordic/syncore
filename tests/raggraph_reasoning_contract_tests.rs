use crate::code_graph::rag_graph::RagGraphQueryRequest;
use crate::mcp_server::reasoning::{parse_unified_request, RequestType};
use crate::mcp_server::types::RagGraphMultihopRequest;

fn unified_query(raw_params: serde_json::Map<String, serde_json::Value>, request_type: RequestType) {
    let request = parse_unified_request(raw_params, request_type, None).expect("parse failed");
    match request.parameters {
        crate::mcp_server::reasoning::RequestParameters::Query { .. } | crate::mcp_server::reasoning::RequestParameters::Fusion { .. } => {}
        _ => panic!("Unexpected parameters"),
    }
}

#[test]
fn raggraph_query_and_fusion_share_fields() {
    let canonical = RagGraphQueryRequest {
        query: "test".into(),
        namespace: Some("ns".into()),
        mode_hint: Some("attention".into()),
        top_k: Some(5),
        scope: Some("project".into()),
        project_label: Some("SynCore".into()),
        local_root: Some("src".into()),
    };

    let raw_query = canonical.to_raw_params();
    let request = parse_unified_request(raw_query.clone(), RequestType::Query, None).unwrap();
    if let crate::mcp_server::reasoning::RequestParameters::Query { .. } = request.parameters {
    } else {
        panic!("Expected Query parameters");
    }

    let mut fusion_params = raw_query.clone();
    fusion_params.insert("enable_temporal".into(), serde_json::Value::Bool(true));
    let fusion_request = parse_unified_request(fusion_params, RequestType::Fusion, None).unwrap();
    if let crate::mcp_server::reasoning::RequestParameters::Fusion { enable_temporal, .. } = fusion_request.parameters {
        assert!(enable_temporal);
    } else {
        panic!("Expected Fusion parameters");
    }
}

#[test]
fn raggraph_multihop_params_map_seed_nodes() {
    let request = RagGraphMultihopRequest {
        seed_nodes: vec![1, 2],
        max_hops: Some(3),
        max_entities: Some(5),
        decay_factor: Some(0.5),
    };

    let raw = request.to_raw_params();
    let unified = parse_unified_request(raw, RequestType::MultiHop, None).unwrap();
    if let crate::mcp_server::reasoning::RequestParameters::MultiHop { seed_entities, max_hops, max_entities, decay_factor } = unified.parameters {
        assert_eq!(seed_entities, vec![1, 2]);
        assert_eq!(max_hops, Some(3));
        assert_eq!(max_entities, Some(5));
        assert_eq!(decay_factor, Some(0.5));
    } else {
        panic!("Expected MultiHop parameters");
    }
}
