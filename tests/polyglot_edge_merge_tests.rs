use syncore::polyglot::polyglot_aggregator::*;
use syncore::polyglot::polyglot_model::*;

fn create_dummy_edge(from_id: i64, to_id: i64, edge_type: &str) -> UnifiedEdge {
    UnifiedEdge {
        from_id,
        to_id,
        edge_type: edge_type.to_string(),
    }
}

#[test]
fn test_merge_edges() {
    let edge1 = create_dummy_edge(1, 2, "calls");
    let edge2 = create_dummy_edge(1, 2, "calls");

    let merged_edges = merge_edges(vec![edge1, edge2]);

    assert_eq!(merged_edges.len(), 1, "Should merge identical edges");
}
