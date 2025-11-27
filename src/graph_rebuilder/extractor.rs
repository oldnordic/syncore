//! Relationship Extractor - Extract code relationships using tree-sitter AST
//!
//! This module wraps the existing edge_extractor with a cleaner interface for
//! graph rebuild operations. Extracts:
//! - Imports: use declarations
//! - Calls: function/method calls
//! - Uses: type references
//! - Inherits: trait implementations
//! - References: constant/static references

use crate::code_graph::EdgeType;
use anyhow::{Context, Result};

/// Represents an extracted edge from source code
#[derive(Debug, Clone)]
pub struct ExtractedEdge {
    pub src_name: String,
    pub dst_name: String,
    pub edge_type: String,
}

impl ExtractedEdge {
    /// Convert EdgeType enum to string
    fn edge_type_str(et: &EdgeType) -> &'static str {
        match et {
            EdgeType::Calls => "calls",
            EdgeType::Imports => "imports",
            EdgeType::Uses => "uses",
            EdgeType::Inherits => "inherits",
            EdgeType::References => "references",
            EdgeType::Contains => "contains",
            EdgeType::Implements => "implements",
            EdgeType::UsesField => "uses_field",
            EdgeType::UsesType => "uses_type",
            EdgeType::ModuleChild => "module_child",
        }
    }
}

/// RelationshipExtractor extracts edges from source code using tree-sitter
pub struct RelationshipExtractor {
    parser: tree_sitter::Parser,
}

impl RelationshipExtractor {
    /// Create a new RelationshipExtractor
    pub fn new() -> Result<Self> {
        let parser = tree_sitter::Parser::new();
        // Language is set per-file in extract_from_source
        Ok(Self { parser })
    }

    /// Extract relationships from Rust source code
    ///
    /// Returns a list of edges with src_name, dst_name, edge_type
    pub fn extract_from_source(
        &mut self,
        source_code: &str,
        file_path: &str,
    ) -> Result<Vec<ExtractedEdge>> {
        // Detect language from file extension
        let lang = detect_language(file_path);

        if lang != "rust" {
            // For now, only support Rust extraction
            return Ok(vec![]);
        }

        // Set Rust language for parser
        self.parser
            .set_language(tree_sitter_rust::language())
            .context("Failed to set Rust language for tree-sitter")?;

        let tree = self
            .parser
            .parse(source_code, None)
            .context("Failed to parse source code")?;

        let root = tree.root_node();

        // Use existing edge extractor logic
        let raw_edges =
            crate::code_graph::edge_extractor::extract_edges_from_rust_ast(source_code, root)?;

        // Convert to our ExtractedEdge format
        // Replace "__FILE__" placeholder with actual file path for CONTAINS/MODULE_CHILD edges
        let edges = raw_edges
            .into_iter()
            .map(|e| {
                let src_name = if e.src_entity_name == "__FILE__" {
                    file_path.to_string()
                } else {
                    e.src_entity_name
                };
                ExtractedEdge {
                    src_name,
                    dst_name: e.dst_entity_name,
                    edge_type: ExtractedEdge::edge_type_str(&e.edge_type).to_string(),
                }
            })
            .collect();

        Ok(edges)
    }

    /// Extract edges from a file path (reads file content)
    pub fn extract_from_file(&mut self, file_path: &std::path::Path) -> Result<Vec<ExtractedEdge>> {
        let source_code = std::fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

        self.extract_from_source(&source_code, &file_path.to_string_lossy())
    }

    /// Extract edges from multiple files in a directory
    pub fn extract_from_directory(
        &mut self,
        dir_path: &std::path::Path,
    ) -> Result<Vec<ExtractedEdge>> {
        let mut all_edges = Vec::new();

        // Walk directory for Rust files
        for entry in walkdir::WalkDir::new(dir_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|ext| ext == "rs").unwrap_or(false))
        {
            match self.extract_from_file(entry.path()) {
                Ok(edges) => all_edges.extend(edges),
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to extract from {}: {}",
                        entry.path().display(),
                        e
                    );
                }
            }
        }

        Ok(all_edges)
    }
}

/// Detect language from file extension
fn detect_language(file_path: &str) -> &'static str {
    if file_path.ends_with(".rs") {
        "rust"
    } else if file_path.ends_with(".js") || file_path.ends_with(".ts") {
        "javascript"
    } else if file_path.ends_with(".py") {
        "python"
    } else if file_path.ends_with(".go") {
        "go"
    } else {
        "unknown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_imports() {
        let code = r#"
use std::io;
use crate::parser::Parser;

fn main() {}
        "#;

        let mut extractor = RelationshipExtractor::new().unwrap();
        let edges = extractor.extract_from_source(code, "test.rs").unwrap();

        // Should find import edges
        let import_edges: Vec<_> = edges.iter().filter(|e| e.edge_type == "imports").collect();
        assert!(!import_edges.is_empty(), "Should extract import edges");
    }

    #[test]
    fn test_extract_calls() {
        let code = r#"
fn helper() {}

fn main() {
    helper();
}
        "#;

        let mut extractor = RelationshipExtractor::new().unwrap();
        let edges = extractor.extract_from_source(code, "test.rs").unwrap();

        // Should find call edge from main to helper
        let call_edges: Vec<_> = edges.iter().filter(|e| e.edge_type == "calls").collect();
        assert!(
            call_edges
                .iter()
                .any(|e| e.src_name == "main" && e.dst_name == "helper"),
            "Should extract call from main to helper"
        );
    }
}
