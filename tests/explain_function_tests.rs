// SPEC: SYNCORE-CODE-TOOLS-IMPROVEMENT-01 (APEX v1.2)
// STEP D: explain_function MCP tool tests
//
// Tests verify the function explanation feature returns:
// - Full signature and docstring
// - Callers and callees
// - Complexity metrics

use syncore::code_graph::explain::{
    ExplainFunctionRequest, ExplainFunctionResponse, FunctionExplainer,
};

/// Test: Response contains required fields
#[test]
fn test_response_has_required_fields() {
    let response = ExplainFunctionResponse {
        function_name: "my_function".to_string(),
        file_path: "src/test.rs".to_string(),
        signature: "fn my_function(x: i32) -> bool".to_string(),
        docstring: Some("Checks if x is positive".to_string()),
        line_start: 10,
        line_end: 25,
        callers: vec!["caller_a".to_string(), "caller_b".to_string()],
        callees: vec!["callee_x".to_string()],
        complexity: ComplexityMetrics {
            lines: 15,
            cyclomatic: 3,
            cognitive: 5,
        },
    };

    assert_eq!(response.function_name, "my_function");
    assert!(!response.signature.is_empty());
    assert!(!response.callers.is_empty());
}

use syncore::code_graph::explain::ComplexityMetrics;

/// Test: Complexity metrics calculation - simple function
#[test]
fn test_complexity_simple_function() {
    let code = r#"
fn simple() {
    println!("Hello");
}
"#;
    let metrics = ComplexityMetrics::from_code(code);
    assert!(metrics.lines <= 5);
    assert_eq!(metrics.cyclomatic, 1); // No branches
}

/// Test: Complexity metrics - function with if/else
#[test]
fn test_complexity_with_branches() {
    let code = r#"
fn with_branches(x: i32) -> bool {
    if x > 0 {
        true
    } else if x < 0 {
        false
    } else {
        true
    }
}
"#;
    let metrics = ComplexityMetrics::from_code(code);
    assert!(metrics.cyclomatic >= 3); // if + else if + else
}

/// Test: Complexity metrics - function with match
#[test]
fn test_complexity_with_match() {
    let code = r#"
fn with_match(opt: Option<i32>) -> i32 {
    match opt {
        Some(x) => x,
        None => 0,
    }
}
"#;
    let metrics = ComplexityMetrics::from_code(code);
    assert!(metrics.cyclomatic >= 2); // 2 match arms
}

/// Test: Complexity metrics - function with loops
#[test]
fn test_complexity_with_loops() {
    let code = r#"
fn with_loop() {
    for i in 0..10 {
        println!("{}", i);
    }
    while true {
        break;
    }
}
"#;
    let metrics = ComplexityMetrics::from_code(code);
    assert!(metrics.cyclomatic >= 3); // 2 loops + base
}

/// Test: Request parsing
#[test]
fn test_request_parsing() {
    let json = r#"{"function_name": "execute_plan", "file_path": "src/plan.rs"}"#;
    let request: ExplainFunctionRequest = serde_json::from_str(json).unwrap();

    assert_eq!(request.function_name, "execute_plan");
    assert_eq!(request.file_path, "src/plan.rs");
}

/// Test: Response serialization
#[test]
fn test_response_serialization() {
    let response = ExplainFunctionResponse {
        function_name: "test_fn".to_string(),
        file_path: "src/lib.rs".to_string(),
        signature: "fn test_fn()".to_string(),
        docstring: None,
        line_start: 1,
        line_end: 5,
        callers: vec![],
        callees: vec![],
        complexity: ComplexityMetrics {
            lines: 4,
            cyclomatic: 1,
            cognitive: 1,
        },
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("test_fn"));
    assert!(json.contains("cyclomatic"));
}

/// Test: Empty callers/callees for isolated function
#[test]
fn test_isolated_function() {
    let response = ExplainFunctionResponse {
        function_name: "isolated".to_string(),
        file_path: "src/lib.rs".to_string(),
        signature: "fn isolated()".to_string(),
        docstring: None,
        line_start: 1,
        line_end: 3,
        callers: vec![],
        callees: vec![],
        complexity: ComplexityMetrics {
            lines: 2,
            cyclomatic: 1,
            cognitive: 1,
        },
    };

    assert!(response.callers.is_empty());
    assert!(response.callees.is_empty());
}

/// Test: Cognitive complexity increases with nesting
#[test]
fn test_cognitive_complexity_nesting() {
    let code_flat = r#"
fn flat() {
    if x { }
    if y { }
}
"#;
    let code_nested = r#"
fn nested() {
    if x {
        if y {
            if z { }
        }
    }
}
"#;
    let flat_metrics = ComplexityMetrics::from_code(code_flat);
    let nested_metrics = ComplexityMetrics::from_code(code_nested);

    assert!(
        nested_metrics.cognitive > flat_metrics.cognitive,
        "Nested code should have higher cognitive complexity"
    );
}
