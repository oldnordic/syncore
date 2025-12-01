//! Semantic edge extraction from AST using Tree-sitter
//!
//! PHASE 1: Extract semantic relationships from code:
//! - Function calls (CALLS)
//! - Trait implementations (IMPLEMENTS)
//! - Field access (USES_FIELD)
//! - Type usage (USES_TYPE)
//! - Imports (IMPORTS)
//! - Module hierarchy (MODULE_CHILD)

use anyhow::Result;
use tree_sitter::{Node, Parser};

use super::types::EdgeType;

/// Semantic edge extractor for Rust code
pub struct SemanticExtractor {
    parser: Parser,
}

/// Extracted semantic relationship
#[derive(Debug, Clone)]
pub struct SemanticEdge {
    pub source_name: String, // Caller/user entity name
    pub target_name: String, // Callee/used entity name
    pub edge_type: EdgeType,
    pub source_line: usize,
    pub target_line: Option<usize>, // None if target is external
}

impl SemanticExtractor {
    /// Create a new semantic extractor for Rust
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_rust::language())?;
        Ok(Self { parser })
    }

    /// Extract all semantic edges from Rust source code
    pub fn extract_edges(&mut self, source: &str, _file_path: &str) -> Result<Vec<SemanticEdge>> {
        let tree = self
            .parser
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse source"))?;

        let mut edges = Vec::new();

        // Extract function calls
        edges.extend(self.extract_calls(tree.root_node(), source)?);

        // Extract trait implementations
        edges.extend(self.extract_implements(tree.root_node(), source)?);

        // Extract field accesses
        edges.extend(self.extract_field_access(tree.root_node(), source)?);

        // Extract type usage
        edges.extend(self.extract_type_usage(tree.root_node(), source)?);

        Ok(edges)
    }

    /// Extract function call edges
    fn extract_calls(&self, root: Node, source: &str) -> Result<Vec<SemanticEdge>> {
        let mut edges = Vec::new();

        // Find all function/method definitions
        let functions = self.find_functions(root, source)?;

        // For each function, find call expressions
        for (func_name, func_node, func_line) in &functions {
            let calls = self.find_call_expressions_in_node(*func_node, source)?;
            for call in calls {
                edges.push(SemanticEdge {
                    source_name: func_name.clone(),
                    target_name: call,
                    edge_type: EdgeType::Calls,
                    source_line: *func_line,
                    target_line: None, // Will be resolved later
                });
            }
        }

        Ok(edges)
    }

    /// Extract trait implementation edges
    fn extract_implements(&self, root: Node, source: &str) -> Result<Vec<SemanticEdge>> {
        let mut edges = Vec::new();

        // Query for impl blocks: impl Trait for Type
        self.visit_node(root, &mut |node| {
            if node.kind() == "impl_item" {
                // Extract trait and type names
                if let Some((trait_name, type_name)) = self.parse_impl_block(node, source) {
                    edges.push(SemanticEdge {
                        source_name: type_name,
                        target_name: trait_name,
                        edge_type: EdgeType::Implements,
                        source_line: node.start_position().row + 1,
                        target_line: None,
                    });
                }
            }
        });

        Ok(edges)
    }

    /// Extract field access edges
    fn extract_field_access(&self, root: Node, source: &str) -> Result<Vec<SemanticEdge>> {
        let mut edges = Vec::new();

        // Find field_expression nodes (e.g., foo.bar)
        self.visit_node(root, &mut |node| {
            if node.kind() == "field_expression" {
                if let Some((object, field)) = self.parse_field_access(node, source) {
                    edges.push(SemanticEdge {
                        source_name: object,
                        target_name: field,
                        edge_type: EdgeType::UsesField,
                        source_line: node.start_position().row + 1,
                        target_line: None,
                    });
                }
            }
        });

        Ok(edges)
    }

    /// Extract type usage edges
    fn extract_type_usage(&self, root: Node, source: &str) -> Result<Vec<SemanticEdge>> {
        let mut edges = Vec::new();

        // Find type annotations and generic arguments
        self.visit_node(root, &mut |node| {
            match node.kind() {
                "type_identifier" | "generic_type" => {
                    if let Some(type_name) = self.get_node_text(node, source) {
                        // Find the containing function/struct
                        if let Some(parent_name) = self.find_parent_entity_name(node, source) {
                            edges.push(SemanticEdge {
                                source_name: parent_name,
                                target_name: type_name,
                                edge_type: EdgeType::UsesType,
                                source_line: node.start_position().row + 1,
                                target_line: None,
                            });
                        }
                    }
                }
                _ => {}
            }
        });

        Ok(edges)
    }

    // Helper methods

    fn find_functions<'a>(
        &self,
        root: Node<'a>,
        source: &str,
    ) -> Result<Vec<(String, Node<'a>, usize)>> {
        // Use recursive traversal without closures to avoid lifetime issues
        let mut functions = Vec::new();
        self.collect_functions_recursive(root, source, &mut functions);
        Ok(functions)
    }

    fn collect_functions_recursive<'a>(
        &self,
        node: Node<'a>,
        source: &str,
        functions: &mut Vec<(String, Node<'a>, usize)>,
    ) {
        if node.kind() == "function_item" || node.kind() == "function_signature_item" {
            if let Some(name) = self.get_function_name(node, source) {
                functions.push((name, node, node.start_position().row + 1));
            }
        }

        // Recurse into children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.collect_functions_recursive(child, source, functions);
            }
        }
    }

    fn find_call_expressions_in_node(&self, node: Node, source: &str) -> Result<Vec<String>> {
        let mut calls = Vec::new();

        self.visit_node(node, &mut |n| {
            if n.kind() == "call_expression" {
                if let Some(callee) = self.get_call_target(n, source) {
                    calls.push(callee);
                }
            }
        });

        Ok(calls)
    }

    fn get_function_name(&self, node: Node, source: &str) -> Option<String> {
        for i in 0..node.child_count() {
            let child = node.child(i)?;
            if child.kind() == "identifier" {
                return self.get_node_text(child, source);
            }
        }
        None
    }

    fn get_call_target(&self, node: Node, source: &str) -> Option<String> {
        // Get the function being called
        if let Some(func_node) = node.child_by_field_name("function") {
            return self.get_node_text(func_node, source);
        }
        None
    }

    fn parse_impl_block(&self, node: Node, source: &str) -> Option<(String, String)> {
        let mut trait_name = None;
        let mut type_name = None;

        for i in 0..node.child_count() {
            let child = node.child(i)?;
            if child.kind() == "type_identifier" {
                if trait_name.is_none() {
                    trait_name = self.get_node_text(child, source);
                } else {
                    type_name = self.get_node_text(child, source);
                }
            }
        }

        trait_name.and_then(|t| type_name.map(|ty| (t, ty)))
    }

    fn parse_field_access(&self, node: Node, source: &str) -> Option<(String, String)> {
        let value = node.child_by_field_name("value")?;
        let field = node.child_by_field_name("field")?;

        let object_name = self.get_node_text(value, source)?;
        let field_name = self.get_node_text(field, source)?;

        Some((object_name, field_name))
    }

    fn find_parent_entity_name(&self, node: Node, source: &str) -> Option<String> {
        let mut current = node;
        while let Some(parent) = current.parent() {
            match parent.kind() {
                "function_item" | "struct_item" => {
                    return self.get_function_name(parent, source);
                }
                _ => {}
            }
            current = parent;
        }
        None
    }

    fn get_node_text(&self, node: Node, source: &str) -> Option<String> {
        let start = node.start_byte();
        let end = node.end_byte();
        Some(source.get(start..end)?.to_string())
    }

    fn visit_node<F>(&self, node: Node, visitor: &mut F)
    where
        F: FnMut(Node),
    {
        visitor(node);
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.visit_node(child, visitor);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extractor_creation() {
        let extractor = SemanticExtractor::new();
        assert!(extractor.is_ok());
    }
}
