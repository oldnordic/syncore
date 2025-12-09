use serde::Serialize;
use serde_json::Value;

fn field_names<T: Serialize>(value: &T) -> Vec<String> {
    match serde_json::to_value(value).unwrap() {
        Value::Object(map) => map.keys().cloned().collect(),
        _ => vec![],
    }
}

#[test]
fn test_graph_suite_schema_parity() {
    use crate::mcp_server::types::GraphSuiteRequest;
    use crate::mcp_tools::graph_suite::GraphSuiteArgs;

    let request = GraphSuiteRequest {
        command: "query".into(),
        cypher: Some("Match".into()),
        params: Some(serde_json::json!({})),
        from_id: Some(1),
        to_id: Some(2),
        rel_type: Some("CALLS".into()),
        from_label: Some("src".into()),
        to_label: Some("dst".into()),
        query_text: Some("fn".into()),
        seed_nodes: Some(vec![1, 2]),
    };

    let args = GraphSuiteArgs {
        command: "query".into(),
        cypher: Some("Match".into()),
        params: Some(serde_json::json!({})),
        from_id: Some(1),
        to_id: Some(2),
        rel_type: Some("CALLS".into()),
        from_label: Some("src".into()),
        to_label: Some("dst".into()),
        query_text: Some("fn".into()),
        seed_nodes: Some(vec![1, 2]),
    };

    let req_fields = field_names(&request);
    let arg_fields = field_names(&args);

    for field in arg_fields {
        assert!(req_fields.contains(&field));
    }
    assert!(req_fields.contains(&"query_text".to_string()));
    assert!(req_fields.contains(&"seed_nodes".to_string()));
}

#[test]
fn test_mapping_suite_schema_parity() {
    use crate::mcp_server::types::MappingSuiteRequest;
    use crate::mcp_tools::mapping_suite::MappingSuiteArgs;

    let request = MappingSuiteRequest {
        command: "record".into(),
        path: Some("src/lib.rs".into()),
        kind: Some("file".into()),
        language: Some("rust".into()),
        imports: Some(vec!["foo".into()]),
        exports: Some(vec!["bar".into()]),
        dependencies: Some(vec!["baz".into()]),
        query: Some("test".into()),
        file_path: Some("src/lib.rs".into()),
        change_type: Some("create".into()),
        old_content: Some("".into()),
        new_content: Some("fn".into()),
        line_start: Some(1),
        line_end: Some(2),
        description: Some("desc".into()),
        task_id: Some(42),
    };

    let args = MappingSuiteArgs {
        command: "record".into(),
        path: Some("src/lib.rs".into()),
        kind: Some("file".into()),
        language: Some("rust".into()),
        imports: Some(vec!["foo".into()]),
        exports: Some(vec!["bar".into()]),
        dependencies: Some(vec!["baz".into()]),
        query: Some("test".into()),
        file_path: Some("src/lib.rs".into()),
        change_type: Some("create".into()),
        old_content: Some("".into()),
        new_content: Some("fn".into()),
        line_start: Some(1),
        line_end: Some(2),
        description: Some("desc".into()),
        task_id: Some(42),
    };

    let req_fields = field_names(&request);
    let arg_fields = field_names(&args);

    for field in arg_fields {
        assert!(req_fields.contains(&field));
    }
    assert!(req_fields.contains(&"file_path".to_string()));
    assert!(req_fields.contains(&"change_type".to_string()));
}

#[test]
fn test_debug_suite_schema_parity() {
    use crate::mcp_server::types::DebugSuiteRequest;
    use crate::mcp_tools::debug_suite::DebugSuiteArgs;

    let request = DebugSuiteRequest {
        command: "logs_tail".into(),
        file_path: Some("logs/syn".into()),
        n: Some(10),
        limit: Some(5),
        root: Some("src".into()),
        max_modules: Some(1),
        max_cycles: Some(2),
        max_depth: Some(3),
        min_loc: Some(4),
        min_fan_in: Some(5),
        min_fan_out: Some(6),
        min_entity_count: Some(7),
        exclude_public: Some(true),
        loc_threshold: Some(8),
        fan_in_threshold: Some(9),
        fan_out_threshold: Some(10),
        entity_threshold: Some(11),
        max_examples: Some(3),
        project_root: Some("target".into()),
        excluded_dirs: Some(vec!["target".into()]),
    };

    let args = DebugSuiteArgs {
        command: "logs_tail".into(),
        file_path: Some("logs/syn".into()),
        n: Some(10),
        limit: Some(5),
        root: Some("src".into()),
        max_modules: Some(1),
        max_cycles: Some(2),
        max_depth: Some(3),
        min_loc: Some(4),
        min_fan_in: Some(5),
        min_fan_out: Some(6),
        min_entity_count: Some(7),
        exclude_public: Some(true),
        loc_threshold: Some(8),
        fan_in_threshold: Some(9),
        fan_out_threshold: Some(10),
        entity_threshold: Some(11),
        max_examples: Some(3),
        project_root: Some("target".into()),
        excluded_dirs: Some(vec!["target".into()]),
    };
    let req_fields = field_names(&request);
    let arg_fields = field_names(&args);

    assert!(req_fields.contains(&"max_examples".to_string()));
    assert!(req_fields.contains(&"project_root".to_string()));
    assert!(req_fields.contains(&"excluded_dirs".to_string()));
    for field in arg_fields {
        assert!(req_fields.contains(&field));
    }
}
