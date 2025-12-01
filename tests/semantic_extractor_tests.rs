//! TDD tests for semantic edge extraction (PHASE 1)
//!
//! These tests define the expected behavior for semantic relationship extraction.

use anyhow::Result;
use syncore::code_graph::semantic_extractor::{SemanticEdge, SemanticExtractor};
use syncore::code_graph::EdgeType;

#[test]
fn test_extract_direct_calls_single_file() -> Result<()> {
    let mut extractor = SemanticExtractor::new()?;

    let source = r#"
fn caller() {
    callee();
}

fn callee() {
    println!("called");
}
"#;

    let edges = extractor.extract_edges(source, "test.rs")?;

    // Should find 1 call edge: caller -> callee
    let call_edges: Vec<_> = edges.iter().filter(|e| e.edge_type == EdgeType::Calls).collect();

    assert_eq!(call_edges.len(), 1, "Should find exactly 1 call edge");
    assert_eq!(call_edges[0].source_name, "caller");
    assert_eq!(call_edges[0].target_name, "callee");

    Ok(())
}

#[test]
fn test_extract_multiple_calls_in_function() -> Result<()> {
    let mut extractor = SemanticExtractor::new()?;

    let source = r#"
fn main() {
    foo();
    bar();
    baz();
}

fn foo() {}
fn bar() {}
fn baz() {}
"#;

    let edges = extractor.extract_edges(source, "test.rs")?;

    let call_edges: Vec<_> = edges.iter().filter(|e| e.edge_type == EdgeType::Calls).collect();

    // main() calls foo(), bar(), baz()
    assert_eq!(call_edges.len(), 3, "Should find 3 call edges");

    let targets: Vec<_> = call_edges.iter().map(|e| e.target_name.as_str()).collect();
    assert!(targets.contains(&"foo"));
    assert!(targets.contains(&"bar"));
    assert!(targets.contains(&"baz"));

    Ok(())
}

#[test]
fn test_extract_trait_implements_relations() -> Result<()> {
    let mut extractor = SemanticExtractor::new()?;

    let source = r#"
trait MyTrait {
    fn method(&self);
}

struct MyType;

impl MyTrait for MyType {
    fn method(&self) {}
}
"#;

    let edges = extractor.extract_edges(source, "test.rs")?;

    let impl_edges: Vec<_> = edges.iter().filter(|e| e.edge_type == EdgeType::Implements).collect();

    assert_eq!(impl_edges.len(), 1, "Should find 1 implements edge");
    assert_eq!(impl_edges[0].source_name, "MyType");
    assert_eq!(impl_edges[0].target_name, "MyTrait");

    Ok(())
}

#[test]
fn test_extract_struct_field_access() -> Result<()> {
    let mut extractor = SemanticExtractor::new()?;

    let source = r#"
struct Point {
    x: i32,
    y: i32,
}

fn process(p: Point) {
    let _a = p.x;
    let _b = p.y;
}
"#;

    let edges = extractor.extract_edges(source, "test.rs")?;

    let field_edges: Vec<_> = edges.iter().filter(|e| e.edge_type == EdgeType::UsesField).collect();

    // Should find 2 field accesses: p.x, p.y
    assert!(field_edges.len() >= 2, "Should find at least 2 field access edges");

    let fields: Vec<_> = field_edges.iter().map(|e| e.target_name.as_str()).collect();
    assert!(fields.contains(&"x"));
    assert!(fields.contains(&"y"));

    Ok(())
}

#[test]
fn test_extract_type_usage_edges() -> Result<()> {
    let mut extractor = SemanticExtractor::new()?;

    let source = r#"
struct MyType;

fn process(x: MyType) -> Option<MyType> {
    Some(x)
}
"#;

    let edges = extractor.extract_edges(source, "test.rs")?;

    let type_edges: Vec<_> = edges.iter().filter(|e| e.edge_type == EdgeType::UsesType).collect();

    // Function 'process' uses MyType and Option
    assert!(type_edges.len() >= 1, "Should find at least 1 type usage edge");

    let types: Vec<_> = type_edges.iter().map(|e| e.target_name.as_str()).collect();
    assert!(
        types.contains(&"MyType") || types.contains(&"Option"),
        "Should find MyType or Option usage"
    );

    Ok(())
}
