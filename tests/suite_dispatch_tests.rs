//! Suite dispatch tests for APEX v1.3 tool suites
//!
//! Tests the 5 unified suite tools:
//! - memory_suite
//! - code_suite
//! - graph_suite
//! - mapping_suite
//! - debug_suite

use syncore::mcp_tools::SuiteResult;

/// Test SuiteResult construction
#[test]
fn test_suite_result_ok_construction() {
    let result = SuiteResult::ok("test_command", serde_json::json!({"key": "value"}));

    assert!(result.success);
    assert_eq!(result.command, "test_command");
    assert!(result.error.is_none());
    assert_eq!(result.data["key"], "value");
}

#[test]
fn test_suite_result_err_construction() {
    let result = SuiteResult::err("failing_command", "Something went wrong");

    assert!(!result.success);
    assert_eq!(result.command, "failing_command");
    assert_eq!(result.error, Some("Something went wrong".to_string()));
    assert_eq!(result.data, serde_json::Value::Null);
}

/// Test suite argument deserialization
mod memory_suite_tests {
    use syncore::mcp_tools::memory_suite::MemorySuiteArgs;

    #[test]
    fn test_store_args() {
        let json = serde_json::json!({
            "command": "store",
            "key": "test_key",
            "value": "test_value",
            "dry_run": true
        });

        let args: MemorySuiteArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.command, "store");
        assert_eq!(args.key, Some("test_key".to_string()));
        assert_eq!(args.value, Some("test_value".to_string()));
        assert_eq!(args.dry_run, Some(true));
    }

    #[test]
    fn test_query_args() {
        let json = serde_json::json!({
            "command": "query",
            "key": "lookup_key"
        });

        let args: MemorySuiteArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.command, "query");
        assert_eq!(args.key, Some("lookup_key".to_string()));
        assert!(args.value.is_none());
    }

    #[test]
    fn test_vector_search_args() {
        let json = serde_json::json!({
            "command": "vector_search",
            "query": "semantic search query",
            "limit": 5
        });

        let args: MemorySuiteArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.command, "vector_search");
        assert_eq!(args.query, Some("semantic search query".to_string()));
        assert_eq!(args.limit, Some(5));
    }
}

mod code_suite_tests {
    use syncore::mcp_tools::code_suite::CodeSuiteArgs;

    #[test]
    fn test_index_args() {
        let json = serde_json::json!({
            "command": "index",
            "file_path": "/src/main.rs"
        });

        let args: CodeSuiteArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.command, "index");
        assert_eq!(args.file_path, Some("/src/main.rs".to_string()));
    }

    #[test]
    fn test_search_args() {
        let json = serde_json::json!({
            "command": "search",
            "query": "authentication logic",
            "limit": 10
        });

        let args: CodeSuiteArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.command, "search");
        assert_eq!(args.query, Some("authentication logic".to_string()));
        assert_eq!(args.limit, Some(10));
    }

    #[test]
    fn test_parse_args() {
        let json = serde_json::json!({
            "command": "parse",
            "file_path": "/path/to/file.rs"
        });

        let args: CodeSuiteArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.command, "parse");
        assert_eq!(args.file_path, Some("/path/to/file.rs".to_string()));
    }
}

mod graph_suite_tests {
    use syncore::mcp_tools::graph_suite::GraphSuiteArgs;

    #[test]
    fn test_query_args() {
        let json = serde_json::json!({
            "command": "query",
            "cypher": "MATCH (n) RETURN n LIMIT 10"
        });

        let args: GraphSuiteArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.command, "query");
        assert_eq!(args.cypher, Some("MATCH (n) RETURN n LIMIT 10".to_string()));
    }

    #[test]
    fn test_insert_args_with_params() {
        let json = serde_json::json!({
            "command": "insert",
            "cypher": "CREATE (n:Node {name: $name})",
            "params": {"name": "test_node"}
        });

        let args: GraphSuiteArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.command, "insert");
        assert!(args.cypher.is_some());
        assert!(args.params.is_some());
    }

    #[test]
    fn test_relate_args() {
        let json = serde_json::json!({
            "command": "relate",
            "from_id": 1,
            "to_id": 2,
            "rel_type": "CALLS",
            "from_label": "Function",
            "to_label": "Function"
        });

        let args: GraphSuiteArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.command, "relate");
        assert_eq!(args.from_id, Some(1));
        assert_eq!(args.to_id, Some(2));
        assert_eq!(args.rel_type, Some("CALLS".to_string()));
    }
}

mod mapping_suite_tests {
    use syncore::mcp_tools::mapping_suite::MappingSuiteArgs;

    #[test]
    fn test_record_args() {
        let json = serde_json::json!({
            "command": "record",
            "path": "/src/main.rs",
            "kind": "file",
            "language": "rust",
            "imports": ["crate::foo", "std::io"],
            "exports": ["main"]
        });

        let args: MappingSuiteArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.command, "record");
        assert_eq!(args.path, Some("/src/main.rs".to_string()));
        assert_eq!(args.kind, Some("file".to_string()));
        assert_eq!(args.language, Some("rust".to_string()));
        assert!(args.imports.is_some());
        assert!(args.exports.is_some());
    }

    #[test]
    fn test_get_args() {
        let json = serde_json::json!({
            "command": "get",
            "path": "/src/lib.rs"
        });

        let args: MappingSuiteArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.command, "get");
        assert_eq!(args.path, Some("/src/lib.rs".to_string()));
    }

    #[test]
    fn test_search_args() {
        let json = serde_json::json!({
            "command": "search",
            "query": "authentication"
        });

        let args: MappingSuiteArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.command, "search");
        assert_eq!(args.query, Some("authentication".to_string()));
    }

    #[test]
    fn test_deps_args() {
        let json = serde_json::json!({
            "command": "deps",
            "path": "/src/router.rs"
        });

        let args: MappingSuiteArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.command, "deps");
        assert_eq!(args.path, Some("/src/router.rs".to_string()));
    }
}

mod debug_suite_tests {
    use syncore::mcp_tools::debug_suite::DebugSuiteArgs;

    #[test]
    fn test_logs_tail_args() {
        let json = serde_json::json!({
            "command": "logs_tail",
            "n": 100,
            "file_path": "/var/log/app.log"
        });

        let args: DebugSuiteArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.command, "logs_tail");
        assert_eq!(args.n, Some(100));
        assert_eq!(args.file_path, Some("/var/log/app.log".to_string()));
    }

    #[test]
    fn test_project_hotspots_args() {
        let json = serde_json::json!({
            "command": "project_hotspots",
            "limit": 20,
            "min_loc": 100,
            "min_fan_in": 5
        });

        let args: DebugSuiteArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.command, "project_hotspots");
        assert_eq!(args.limit, Some(20));
        assert_eq!(args.min_loc, Some(100));
        assert_eq!(args.min_fan_in, Some(5));
    }

    #[test]
    fn test_project_cycles_args() {
        let json = serde_json::json!({
            "command": "project_cycles",
            "max_cycles": 15,
            "max_depth": 3
        });

        let args: DebugSuiteArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.command, "project_cycles");
        assert_eq!(args.max_cycles, Some(15));
        assert_eq!(args.max_depth, Some(3));
    }

    #[test]
    fn test_project_refactor_suggestions_args() {
        let json = serde_json::json!({
            "command": "project_refactor_suggestions",
            "limit": 10,
            "loc_threshold": 200,
            "fan_in_threshold": 10,
            "fan_out_threshold": 15
        });

        let args: DebugSuiteArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.command, "project_refactor_suggestions");
        assert_eq!(args.limit, Some(10));
        assert_eq!(args.loc_threshold, Some(200));
        assert_eq!(args.fan_in_threshold, Some(10));
        assert_eq!(args.fan_out_threshold, Some(15));
    }
}

/// Test deprecation warning emission
#[test]
fn test_deprecation_warning_function_exists() {
    // Just verify the function compiles and doesn't panic
    syncore::mcp_tools::emit_deprecation_warning("old_tool", "new_suite", "new_command");
}

/// Test suite result serialization
#[test]
fn test_suite_result_serialization() {
    let result = SuiteResult::ok(
        "serialize_test",
        serde_json::json!({
            "count": 42,
            "items": ["a", "b", "c"]
        }),
    );

    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"success\":true"));
    assert!(json.contains("\"command\":\"serialize_test\""));
    assert!(json.contains("\"count\":42"));

    // Error should not appear in serialized output
    assert!(!json.contains("\"error\":null"));
}

#[test]
fn test_suite_result_error_serialization() {
    let result = SuiteResult::err("error_test", "Test error message");

    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"success\":false"));
    assert!(json.contains("\"error\":\"Test error message\""));
}
