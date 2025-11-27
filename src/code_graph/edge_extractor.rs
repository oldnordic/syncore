//! Edge Extractor - Extracts code relationships from AST
//!
//! This module extracts edges (relationships) between code entities:
//! - Calls: Function/method calls
//! - Imports: Use declarations
//! - Uses: Type references
//! - References: Variable/constant references
//! - Inherits: Trait implementations
//!
//! REQUIREMENT: Module d 300 LOC

use anyhow::Result;
use std::collections::HashMap;
use tree_sitter::{Node, TreeCursor};

use super::types::EdgeType;

/// Represents a code edge (relationship) between entities
#[derive(Debug, Clone)]
pub struct ExtractedEdge {
    pub src_entity_name: String,
    pub dst_entity_name: String,
    pub edge_type: EdgeType,
}

/// Extract all edges from Rust source code AST
pub fn extract_edges_from_rust_ast(
    source_code: &str,
    root_node: Node,
) -> Result<Vec<ExtractedEdge>> {
    let mut edges = Vec::new();

    // First pass: Build entity name -> ID mapping
    let function_names = extract_function_names(source_code, root_node);
    let type_names = extract_type_names(source_code, root_node);
    let const_names = extract_const_names(source_code, root_node);

    // Second pass: Extract edges
    extract_calls(&mut edges, source_code, root_node, &function_names);
    extract_imports(&mut edges, source_code, root_node);
    extract_trait_impls(&mut edges, source_code, root_node, &type_names);
    extract_type_uses(&mut edges, source_code, root_node, &type_names);
    extract_references(&mut edges, source_code, root_node, &const_names);

    // Third pass: Extract CONTAINS and MODULE_CHILD edges
    extract_contains_edges(&mut edges, source_code, root_node);
    extract_module_child_edges(&mut edges, source_code, root_node);

    Ok(edges)
}

/// Extract function names from AST
fn extract_function_names(source_code: &str, root_node: Node) -> HashMap<String, ()> {
    let mut names = HashMap::new();
    let mut cursor = root_node.walk();

    visit_nodes(&mut cursor, source_code, &mut |node, src| {
        if node.kind() == "function_item" {
            if let Some(name_node) = node.child_by_field_name("name") {
                let name = &src[name_node.byte_range()];
                names.insert(name.to_string(), ());
            }
        }
    });

    names
}

/// Extract type names (struct, enum, trait)
fn extract_type_names(source_code: &str, root_node: Node) -> HashMap<String, ()> {
    let mut names = HashMap::new();
    let mut cursor = root_node.walk();

    visit_nodes(&mut cursor, source_code, &mut |node, src| {
        let kind = node.kind();
        if kind == "struct_item" || kind == "enum_item" || kind == "trait_item" {
            if let Some(name_node) = node.child_by_field_name("name") {
                let name = &src[name_node.byte_range()];
                names.insert(name.to_string(), ());
            }
        }
    });

    names
}

/// Extract function calls (Calls edges)
///
/// Handles both:
/// - Instance method calls: `obj.method()`
/// - Static method calls: `Type::method()` (e.g., `FusionAttention::new()`)
fn extract_calls(
    edges: &mut Vec<ExtractedEdge>,
    source_code: &str,
    root_node: Node,
    function_names: &HashMap<String, ()>,
) {
    let mut current_function = None;
    let mut cursor = root_node.walk();

    visit_nodes(&mut cursor, source_code, &mut |node, src| {
        // Track current function context
        if node.kind() == "function_item" {
            if let Some(name_node) = node.child_by_field_name("name") {
                current_function = Some(src[name_node.byte_range()].to_string());
            }
        }

        // Extract call expressions
        if node.kind() == "call_expression" {
            if let Some(caller) = &current_function {
                if let Some(function_node) = node.child_by_field_name("function") {
                    let callee_text = &src[function_node.byte_range()];

                    // Check for static method call: Type::method()
                    if callee_text.contains("::") {
                        // Parse "Type::method" or "module::Type::method"
                        let parts: Vec<&str> = callee_text.split("::").collect();
                        if parts.len() >= 2 {
                            let method_name = parts.last().unwrap().trim();
                            // Get the type name (last non-method part)
                            let type_name = parts[parts.len() - 2].trim();

                            // Create CALLS edge: caller -> Type::method
                            // Use "Type::method" format for dst to enable lookup
                            edges.push(ExtractedEdge {
                                src_entity_name: caller.clone(),
                                dst_entity_name: format!("{}::{}", type_name, method_name),
                                edge_type: EdgeType::Calls,
                            });

                            // Also create UsesType edge: caller -> Type
                            // This ensures types used in static calls are not flagged as dead
                            edges.push(ExtractedEdge {
                                src_entity_name: caller.clone(),
                                dst_entity_name: type_name.to_string(),
                                edge_type: EdgeType::UsesType,
                            });
                        }
                    } else {
                        // Instance method call: obj.method() or simple fn()
                        let callee = callee_text.split('.').next_back().unwrap_or(callee_text);
                        let callee = callee.trim();

                        // Only create edge if callee is a known function
                        if function_names.contains_key(callee) {
                            edges.push(ExtractedEdge {
                                src_entity_name: caller.clone(),
                                dst_entity_name: callee.to_string(),
                                edge_type: EdgeType::Calls,
                            });
                        }
                    }
                }
            }
        }
    });
}

/// Extract imports (Imports edges)
///
/// Creates self-referential edges for import entities (import -> import).
/// This allows imports to be discoverable in the graph and linked to usage.
fn extract_imports(edges: &mut Vec<ExtractedEdge>, source_code: &str, root_node: Node) {
    let mut cursor = root_node.walk();

    visit_nodes(&mut cursor, source_code, &mut |node, src| {
        if node.kind() == "use_declaration" {
            // Extract the use path
            if let Some(path_node) = node.child_by_field_name("argument") {
                let import_path = &src[path_node.byte_range()];

                // Create self-referential Import edge (import entity points to itself)
                // This makes imports discoverable in graph traversal
                edges.push(ExtractedEdge {
                    src_entity_name: import_path.to_string(),
                    dst_entity_name: import_path.to_string(),
                    edge_type: EdgeType::Imports,
                });
            }
        }
    });
}

/// Extract trait implementations (Inherits edges)
fn extract_trait_impls(
    edges: &mut Vec<ExtractedEdge>,
    source_code: &str,
    root_node: Node,
    _type_names: &HashMap<String, ()>,
) {
    let mut cursor = root_node.walk();

    visit_nodes(&mut cursor, source_code, &mut |node, src| {
        if node.kind() == "impl_item" {
            // Extract trait and type from: impl Trait for Type
            let mut struct_name = None;
            let mut trait_name = None;

            if let Some(trait_node) = node.child_by_field_name("trait") {
                trait_name = Some(src[trait_node.byte_range()].to_string());
            }

            if let Some(type_node) = node.child_by_field_name("type") {
                struct_name = Some(src[type_node.byte_range()].to_string());
            }

            // Create Inherits edge if both exist
            if let (Some(struct_nm), Some(trait_nm)) = (struct_name, trait_name) {
                // Don't check type_names - let indexer filter based on actual stored entities
                edges.push(ExtractedEdge {
                    src_entity_name: struct_nm,
                    dst_entity_name: trait_nm,
                    edge_type: EdgeType::Inherits,
                });
            }
        }
    });
}

/// Extract type usage (Uses edges)
fn extract_type_uses(
    edges: &mut Vec<ExtractedEdge>,
    source_code: &str,
    root_node: Node,
    type_names: &HashMap<String, ()>,
) {
    let mut cursor = root_node.walk();
    let mut current_context = None; // Track current struct/function

    visit_nodes(&mut cursor, source_code, &mut |node, src| {
        let kind = node.kind();

        // Track context
        if kind == "struct_item" || kind == "function_item" {
            if let Some(name_node) = node.child_by_field_name("name") {
                current_context = Some(src[name_node.byte_range()].to_string());
            }
        }

        // Extract type identifiers in type positions
        if kind == "type_identifier" {
            if let Some(context) = &current_context {
                let type_name = &src[node.byte_range()];

                // Only create edge if this is a known type
                if type_names.contains_key(type_name) && context != type_name {
                    edges.push(ExtractedEdge {
                        src_entity_name: context.clone(),
                        dst_entity_name: type_name.to_string(),
                        edge_type: EdgeType::Uses,
                    });
                }
            }
        }
    });
}

/// Extract constant names (const, static)
fn extract_const_names(source_code: &str, root_node: Node) -> HashMap<String, ()> {
    let mut names = HashMap::new();
    let mut cursor = root_node.walk();

    visit_nodes(&mut cursor, source_code, &mut |node, src| {
        let kind = node.kind();
        if kind == "const_item" || kind == "static_item" {
            if let Some(name_node) = node.child_by_field_name("name") {
                let name = &src[name_node.byte_range()];
                names.insert(name.to_string(), ());
            }
        }
    });

    names
}

/// Extract references to constants/statics (References edges)
fn extract_references(
    edges: &mut Vec<ExtractedEdge>,
    source_code: &str,
    root_node: Node,
    const_names: &HashMap<String, ()>,
) {
    let mut current_function = None;
    let mut cursor = root_node.walk();

    visit_nodes(&mut cursor, source_code, &mut |node, src| {
        // Track current function context
        if node.kind() == "function_item" {
            if let Some(name_node) = node.child_by_field_name("name") {
                current_function = Some(src[name_node.byte_range()].to_string());
            }
        }

        // Extract identifier references (not in type positions, not call expressions)
        if node.kind() == "identifier" {
            if let Some(function) = &current_function {
                let identifier = &src[node.byte_range()];

                // Check if this identifier is a known constant
                if const_names.contains_key(identifier) {
                    // Make sure it's not a function call (check parent isn't call_expression)
                    if let Some(parent) = node.parent() {
                        if parent.kind() != "call_expression" {
                            edges.push(ExtractedEdge {
                                src_entity_name: function.clone(),
                                dst_entity_name: identifier.to_string(),
                                edge_type: EdgeType::References,
                            });
                        }
                    }
                }
            }
        }
    });
}

/// Extract CONTAINS edges (file/struct/trait/enum → children)
///
/// Creates edges:
/// - File → function, struct, enum, trait, const, static, impl
/// - Struct → methods (via impl blocks)
/// - Trait → methods
/// - Enum → variants
fn extract_contains_edges(edges: &mut Vec<ExtractedEdge>, source_code: &str, root_node: Node) {
    // Use "__FILE__" as placeholder for file-level container
    // The caller should replace this with the actual file path
    const FILE_ENTITY: &str = "__FILE__";

    let mut cursor = root_node.walk();

    // First, extract file-level items
    for child in root_node.children(&mut cursor) {
        let kind = child.kind();
        match kind {
            "function_item" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = &source_code[name_node.byte_range()];
                    edges.push(ExtractedEdge {
                        src_entity_name: FILE_ENTITY.to_string(),
                        dst_entity_name: name.to_string(),
                        edge_type: EdgeType::Contains,
                    });
                }
            }
            "struct_item" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = &source_code[name_node.byte_range()];
                    edges.push(ExtractedEdge {
                        src_entity_name: FILE_ENTITY.to_string(),
                        dst_entity_name: name.to_string(),
                        edge_type: EdgeType::Contains,
                    });
                }
            }
            "enum_item" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let enum_name = &source_code[name_node.byte_range()];
                    edges.push(ExtractedEdge {
                        src_entity_name: FILE_ENTITY.to_string(),
                        dst_entity_name: enum_name.to_string(),
                        edge_type: EdgeType::Contains,
                    });
                    // Extract variants
                    extract_enum_variants(edges, source_code, child, enum_name);
                }
            }
            "trait_item" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let trait_name = &source_code[name_node.byte_range()];
                    edges.push(ExtractedEdge {
                        src_entity_name: FILE_ENTITY.to_string(),
                        dst_entity_name: trait_name.to_string(),
                        edge_type: EdgeType::Contains,
                    });
                    // Extract trait methods
                    extract_trait_methods(edges, source_code, child, trait_name);
                }
            }
            "impl_item" => {
                // Extract impl methods and connect to struct
                extract_impl_methods(edges, source_code, child);
            }
            "const_item" | "static_item" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = &source_code[name_node.byte_range()];
                    edges.push(ExtractedEdge {
                        src_entity_name: FILE_ENTITY.to_string(),
                        dst_entity_name: name.to_string(),
                        edge_type: EdgeType::Contains,
                    });
                }
            }
            _ => {}
        }
    }
}

/// Extract enum variant CONTAINS edges
fn extract_enum_variants(
    edges: &mut Vec<ExtractedEdge>,
    source_code: &str,
    enum_node: Node,
    enum_name: &str,
) {
    let mut cursor = enum_node.walk();
    for child in enum_node.children(&mut cursor) {
        if child.kind() == "enum_variant_list" {
            let mut variant_cursor = child.walk();
            for variant in child.children(&mut variant_cursor) {
                if variant.kind() == "enum_variant" {
                    if let Some(name_node) = variant.child_by_field_name("name") {
                        let variant_name = &source_code[name_node.byte_range()];
                        edges.push(ExtractedEdge {
                            src_entity_name: enum_name.to_string(),
                            dst_entity_name: variant_name.to_string(),
                            edge_type: EdgeType::Contains,
                        });
                    }
                }
            }
        }
    }
}

/// Extract trait method CONTAINS edges
fn extract_trait_methods(
    edges: &mut Vec<ExtractedEdge>,
    source_code: &str,
    trait_node: Node,
    trait_name: &str,
) {
    let mut cursor = trait_node.walk();
    for child in trait_node.children(&mut cursor) {
        if child.kind() == "declaration_list" {
            let mut decl_cursor = child.walk();
            for decl in child.children(&mut decl_cursor) {
                if decl.kind() == "function_signature_item" || decl.kind() == "function_item" {
                    if let Some(name_node) = decl.child_by_field_name("name") {
                        let method_name = &source_code[name_node.byte_range()];
                        edges.push(ExtractedEdge {
                            src_entity_name: trait_name.to_string(),
                            dst_entity_name: method_name.to_string(),
                            edge_type: EdgeType::Contains,
                        });
                    }
                }
            }
        }
    }
}

/// Extract impl block methods and connect to struct
fn extract_impl_methods(edges: &mut Vec<ExtractedEdge>, source_code: &str, impl_node: Node) {
    // Get the type being implemented (the struct/enum name)
    let struct_name = impl_node
        .child_by_field_name("type")
        .map(|n| source_code[n.byte_range()].to_string());

    if let Some(struct_nm) = struct_name {
        let mut cursor = impl_node.walk();
        for child in impl_node.children(&mut cursor) {
            if child.kind() == "declaration_list" {
                let mut decl_cursor = child.walk();
                for decl in child.children(&mut decl_cursor) {
                    if decl.kind() == "function_item" {
                        if let Some(name_node) = decl.child_by_field_name("name") {
                            let method_name = &source_code[name_node.byte_range()];
                            edges.push(ExtractedEdge {
                                src_entity_name: struct_nm.clone(),
                                dst_entity_name: method_name.to_string(),
                                edge_type: EdgeType::Contains,
                            });
                        }
                    }
                }
            }
        }
    }
}

/// Extract MODULE_CHILD edges from mod declarations
fn extract_module_child_edges(edges: &mut Vec<ExtractedEdge>, source_code: &str, root_node: Node) {
    const FILE_ENTITY: &str = "__FILE__";

    let mut cursor = root_node.walk();
    for child in root_node.children(&mut cursor) {
        if child.kind() == "mod_item" {
            if let Some(name_node) = child.child_by_field_name("name") {
                let mod_name = &source_code[name_node.byte_range()];
                edges.push(ExtractedEdge {
                    src_entity_name: FILE_ENTITY.to_string(),
                    dst_entity_name: mod_name.to_string(),
                    edge_type: EdgeType::ModuleChild,
                });
            }
        }
    }
}

/// Helper: Recursively visit all nodes in AST
fn visit_nodes<F>(cursor: &mut TreeCursor, source_code: &str, visitor: &mut F)
where
    F: FnMut(Node, &str),
{
    loop {
        let node = cursor.node();
        visitor(node, source_code);

        // Recurse into children
        if cursor.goto_first_child() {
            visit_nodes(cursor, source_code, visitor);
            cursor.goto_parent();
        }

        // Move to next sibling
        if !cursor.goto_next_sibling() {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_function_names() {
        let code = r#"
            fn main() {}
            fn helper() {}
        "#;

        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(unsafe { tree_sitter_rust::language() })
            .unwrap();
        let tree = parser.parse(code, None).unwrap();

        let names = extract_function_names(code, tree.root_node());
        assert!(names.contains_key("main"));
        assert!(names.contains_key("helper"));
    }

    #[test]
    fn test_extract_calls() {
        let code = r#"
            fn main() {
                helper();
            }
            fn helper() {}
        "#;

        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(unsafe { tree_sitter_rust::language() })
            .unwrap();
        let tree = parser.parse(code, None).unwrap();

        let function_names = extract_function_names(code, tree.root_node());
        let mut edges = Vec::new();
        extract_calls(&mut edges, code, tree.root_node(), &function_names);

        assert!(!edges.is_empty());
        assert!(edges.iter().any(|e| e.src_entity_name == "main"
            && e.dst_entity_name == "helper"
            && matches!(e.edge_type, EdgeType::Calls)));
    }
}
