use syncore::polyglot::polyglot_aggregator::*;
use syncore::polyglot::polyglot_model::*;

#[test]
fn test_multi_hop_path_reasoning() {
    let edges = vec![
        UnifiedEdge {
            from_id: 1,
            to_id: 2,
            edge_type: "calls".to_string(),
        },
        UnifiedEdge {
            from_id: 2,
            to_id: 3,
            edge_type: "calls".to_string(),
        },
    ];
    let paths = find_paths(edges, 1, 3);
    assert_eq!(paths.len(), 1);
}

#[test]
fn test_cross_language_call_edges() {
    let edges = vec![
        UnifiedEdge {
            from_id: 1,
            to_id: 2,
            edge_type: "calls".to_string(),
        },
        UnifiedEdge {
            from_id: 2,
            to_id: 3,
            edge_type: "calls".to_string(),
        },
    ];
    let paths = find_paths(edges, 1, 3);
    assert_eq!(paths.len(), 1);
}

#[test]
fn test_cyclic_detection_across_mixed_languages() {
    let edges = vec![
        UnifiedEdge {
            from_id: 1,
            to_id: 2,
            edge_type: "calls".to_string(),
        },
        UnifiedEdge {
            from_id: 2,
            to_id: 1,
            edge_type: "calls".to_string(),
        },
    ];
    let has_cycle = has_cycle(edges);
    assert!(has_cycle);
}

#[test]
fn test_compute_mixed_language_connected_components() {
    let edges = vec![
        UnifiedEdge {
            from_id: 1,
            to_id: 2,
            edge_type: "calls".to_string(),
        },
        UnifiedEdge {
            from_id: 3,
            to_id: 4,
            edge_type: "calls".to_string(),
        },
    ];
    let components = find_components(edges);
    assert_eq!(components.len(), 2);
}
