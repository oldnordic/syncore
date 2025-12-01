//! Code Graph Extractor using Tree-sitter
//! Parses Rust source code to extract structural information for the Code Intelligence Graph.

use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use tree_sitter::{Parser, Query, QueryCursor};

/// Complete code graph extracted from a single file
#[derive(Debug, Clone, Default)]
pub struct CodeGraph {
    pub file_path: PathBuf,
    pub imports: Vec<ImportNode>,
    pub functions: Vec<FunctionNode>,
    pub calls: Vec<CallEdge>,
    pub structs: Vec<StructNode>,
    pub traits: Vec<TraitNode>,
    pub implementations: Vec<ImplementationEdge>,
}

/// Import/use statement
#[derive(Debug, Clone)]
pub struct ImportNode {
    pub path: String,
    pub line: usize,
}

/// Function definition (standalone or method)
#[derive(Debug, Clone)]
pub struct FunctionNode {
    pub name: String,
    pub qualified_path: String,
    pub is_public: bool,
    pub is_async: bool,
    pub parent_type: Option<String>,
    pub line_start: usize,
    pub line_end: usize,
}

/// Function call edge
#[derive(Debug, Clone)]
pub struct CallEdge {
    pub from: String,
    pub to: String,
    pub line: usize,
}

/// Struct definition
#[derive(Debug, Clone)]
pub struct StructNode {
    pub name: String,
    pub is_public: bool,
    pub line: usize,
}

/// Trait definition
#[derive(Debug, Clone)]
pub struct TraitNode {
    pub name: String,
    pub is_public: bool,
    pub line: usize,
}

/// Implementation edge (struct implements trait)
#[derive(Debug, Clone)]
pub struct ImplementationEdge {
    pub struct_name: String,
    pub trait_name: Option<String>,
    pub line: usize,
}

/// Node kind enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Function,
    Struct,
    Trait,
    Import,
}

/// Tree-sitter based code graph extractor
pub struct CodeGraphExtractor {
    _phantom: std::marker::PhantomData<()>,
}

impl CodeGraphExtractor {
    /// Create a new extractor with Rust language support
    pub fn new() -> Self {
        CodeGraphExtractor {
            _phantom: std::marker::PhantomData,
        }
    }

    /// Extract code graph from a Rust source file
    pub fn extract_file(&self, path: &Path) -> Result<CodeGraph> {
        let source = std::fs::read_to_string(path)
            .map_err(|e| anyhow!("Failed to read file {}: {}", path.display(), e))?;

        let mut parser = Parser::new();
        parser.set_language(tree_sitter_rust::language()).expect("Failed to set Rust language");

        let tree = parser.parse(&source, None).ok_or_else(|| anyhow!("Failed to parse source"))?;

        let root = tree.root_node();
        let source_bytes = source.as_bytes();

        let mut graph = CodeGraph {
            file_path: path.to_path_buf(),
            ..Default::default()
        };

        // Extract all components
        self.extract_imports(&root, source_bytes, &mut graph)?;
        self.extract_structs(&root, source_bytes, &mut graph)?;
        self.extract_traits(&root, source_bytes, &mut graph)?;
        self.extract_functions(&root, source_bytes, &mut graph, &[])?;
        self.extract_impl_blocks(&root, source_bytes, &mut graph)?;
        self.extract_calls(&root, source_bytes, &mut graph)?;

        Ok(graph)
    }

    fn extract_imports(
        &self,
        root: &tree_sitter::Node,
        source: &[u8],
        graph: &mut CodeGraph,
    ) -> Result<()> {
        let query_str = r#"
            (use_declaration) @use_decl
        "#;

        let query = Query::new(tree_sitter_rust::language(), query_str)
            .map_err(|e| anyhow!("Query error: {}", e))?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, *root, source);

        for m in matches {
            for capture in m.captures {
                let node = capture.node;
                let use_text = node.utf8_text(source).map_err(|e| anyhow!("UTF8 error: {}", e))?;

                // Extract the path from the use statement
                let path = self.extract_use_path(use_text);
                let line = node.start_position().row + 1;

                graph.imports.push(ImportNode {
                    path,
                    line,
                });
            }
        }

        Ok(())
    }

    fn extract_use_path(&self, use_text: &str) -> String {
        // Remove "use " prefix and trailing ";"
        let trimmed = use_text.trim().trim_start_matches("use ").trim_end_matches(';').trim();
        trimmed.to_string()
    }

    fn extract_structs(
        &self,
        root: &tree_sitter::Node,
        source: &[u8],
        graph: &mut CodeGraph,
    ) -> Result<()> {
        let query_str = r#"
            (struct_item
                name: (type_identifier) @name) @struct
        "#;

        let query = Query::new(tree_sitter_rust::language(), query_str)
            .map_err(|e| anyhow!("Query error: {}", e))?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, *root, source);

        for m in matches {
            let struct_node = m.captures.iter().find(|c| c.index == 1).map(|c| c.node);
            let name_node = m.captures.iter().find(|c| c.index == 0).map(|c| c.node);

            if let (Some(struct_node), Some(name_node)) = (struct_node, name_node) {
                let name = name_node
                    .utf8_text(source)
                    .map_err(|e| anyhow!("UTF8 error: {}", e))?
                    .to_string();

                let struct_text = struct_node.utf8_text(source).unwrap_or("");
                let is_public = struct_text.trim_start().starts_with("pub ");
                let line = struct_node.start_position().row + 1;

                graph.structs.push(StructNode {
                    name,
                    is_public,
                    line,
                });
            }
        }

        Ok(())
    }

    fn extract_traits(
        &self,
        root: &tree_sitter::Node,
        source: &[u8],
        graph: &mut CodeGraph,
    ) -> Result<()> {
        let query_str = r#"
            (trait_item
                name: (type_identifier) @name) @trait
        "#;

        let query = Query::new(tree_sitter_rust::language(), query_str)
            .map_err(|e| anyhow!("Query error: {}", e))?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, *root, source);

        for m in matches {
            let trait_node = m.captures.iter().find(|c| c.index == 1).map(|c| c.node);
            let name_node = m.captures.iter().find(|c| c.index == 0).map(|c| c.node);

            if let (Some(trait_node), Some(name_node)) = (trait_node, name_node) {
                let name = name_node
                    .utf8_text(source)
                    .map_err(|e| anyhow!("UTF8 error: {}", e))?
                    .to_string();

                let trait_text = trait_node.utf8_text(source).unwrap_or("");
                let is_public = trait_text.trim_start().starts_with("pub ");
                let line = trait_node.start_position().row + 1;

                graph.traits.push(TraitNode {
                    name,
                    is_public,
                    line,
                });
            }
        }

        Ok(())
    }

    fn extract_functions(
        &self,
        root: &tree_sitter::Node,
        source: &[u8],
        graph: &mut CodeGraph,
        module_path: &[String],
    ) -> Result<()> {
        // First, handle nested modules
        self.extract_module_functions(root, source, graph, module_path)?;

        // Extract top-level functions (not in impl blocks)
        let query_str = r#"
            (function_item
                name: (identifier) @name) @fn
        "#;

        let query = Query::new(tree_sitter_rust::language(), query_str)
            .map_err(|e| anyhow!("Query error: {}", e))?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, *root, source);

        for m in matches {
            let fn_node = m.captures.iter().find(|c| c.index == 1).map(|c| c.node);
            let name_node = m.captures.iter().find(|c| c.index == 0).map(|c| c.node);

            if let (Some(fn_node), Some(name_node)) = (fn_node, name_node) {
                // Skip functions inside impl blocks (handled separately)
                if self.is_inside_impl(fn_node) {
                    continue;
                }

                let name = name_node
                    .utf8_text(source)
                    .map_err(|e| anyhow!("UTF8 error: {}", e))?
                    .to_string();

                let fn_text = fn_node.utf8_text(source).unwrap_or("");
                let is_public = fn_text.trim_start().starts_with("pub ");
                let is_async = fn_text.contains("async fn ");
                let line_start = fn_node.start_position().row + 1;
                let line_end = fn_node.end_position().row + 1;

                let qualified_path = if module_path.is_empty() {
                    name.clone()
                } else {
                    format!("{}::{}", module_path.join("::"), name)
                };

                graph.functions.push(FunctionNode {
                    name,
                    qualified_path,
                    is_public,
                    is_async,
                    parent_type: None,
                    line_start,
                    line_end,
                });
            }
        }

        Ok(())
    }

    fn extract_module_functions(
        &self,
        root: &tree_sitter::Node,
        source: &[u8],
        graph: &mut CodeGraph,
        current_path: &[String],
    ) -> Result<()> {
        let query_str = r#"
            (mod_item
                name: (identifier) @mod_name
                body: (declaration_list) @body)
        "#;

        let query = Query::new(tree_sitter_rust::language(), query_str)
            .map_err(|e| anyhow!("Query error: {}", e))?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, *root, source);

        for m in matches {
            let mod_name_node = m.captures.iter().find(|c| c.index == 0).map(|c| c.node);
            let body_node = m.captures.iter().find(|c| c.index == 1).map(|c| c.node);

            if let (Some(mod_name_node), Some(body_node)) = (mod_name_node, body_node) {
                let mod_name = mod_name_node
                    .utf8_text(source)
                    .map_err(|e| anyhow!("UTF8 error: {}", e))?
                    .to_string();

                let mut new_path = current_path.to_vec();
                new_path.push(mod_name);

                // Recursively extract functions from nested module
                self.extract_functions(&body_node, source, graph, &new_path)?;
            }
        }

        Ok(())
    }

    fn extract_impl_blocks(
        &self,
        root: &tree_sitter::Node,
        source: &[u8],
        graph: &mut CodeGraph,
    ) -> Result<()> {
        let query_str = r#"
            (impl_item) @impl
        "#;

        let query = Query::new(tree_sitter_rust::language(), query_str)
            .map_err(|e| anyhow!("Query error: {}", e))?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, *root, source);

        for m in matches {
            if let Some(capture) = m.captures.first() {
                let impl_node = capture.node;
                self.process_impl_block(&impl_node, source, graph)?;
            }
        }

        Ok(())
    }

    fn process_impl_block(
        &self,
        impl_node: &tree_sitter::Node,
        source: &[u8],
        graph: &mut CodeGraph,
    ) -> Result<()> {
        let impl_text = impl_node.utf8_text(source).unwrap_or("");
        let line = impl_node.start_position().row + 1;

        // Extract struct name and optional trait
        let (struct_name, trait_name) = self.parse_impl_header(impl_text);

        if let Some(ref sn) = struct_name {
            graph.implementations.push(ImplementationEdge {
                struct_name: sn.clone(),
                trait_name: trait_name.clone(),
                line,
            });

            // Extract methods in this impl block
            self.extract_impl_methods(impl_node, source, graph, sn)?;
        }

        Ok(())
    }

    fn parse_impl_header(&self, impl_text: &str) -> (Option<String>, Option<String>) {
        let lines: Vec<&str> = impl_text.lines().collect();
        if lines.is_empty() {
            return (None, None);
        }

        let first_line = lines[0].trim();

        // Pattern: impl Trait for Struct
        if first_line.contains(" for ") {
            let parts: Vec<&str> = first_line.split(" for ").collect();
            if parts.len() >= 2 {
                let trait_part = parts[0].replace("impl ", "").trim().to_string();
                let struct_part = parts[1]
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_end_matches('{')
                    .to_string();
                return (Some(struct_part), Some(trait_part));
            }
        }

        // Pattern: impl Struct
        if first_line.starts_with("impl ") {
            let struct_name = first_line
                .replace("impl ", "")
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_end_matches('{')
                .to_string();
            return (Some(struct_name), None);
        }

        (None, None)
    }

    fn extract_impl_methods(
        &self,
        impl_node: &tree_sitter::Node,
        source: &[u8],
        graph: &mut CodeGraph,
        parent_type: &str,
    ) -> Result<()> {
        let query_str = r#"
            (function_item
                name: (identifier) @name) @fn
        "#;

        let query = Query::new(tree_sitter_rust::language(), query_str)
            .map_err(|e| anyhow!("Query error: {}", e))?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, *impl_node, source);

        for m in matches {
            let fn_node = m.captures.iter().find(|c| c.index == 1).map(|c| c.node);
            let name_node = m.captures.iter().find(|c| c.index == 0).map(|c| c.node);

            if let (Some(fn_node), Some(name_node)) = (fn_node, name_node) {
                let name = name_node
                    .utf8_text(source)
                    .map_err(|e| anyhow!("UTF8 error: {}", e))?
                    .to_string();

                let fn_text = fn_node.utf8_text(source).unwrap_or("");
                let is_public = fn_text.trim_start().starts_with("pub ");
                let is_async = fn_text.contains("async fn ");
                let line_start = fn_node.start_position().row + 1;
                let line_end = fn_node.end_position().row + 1;

                let qualified_path = format!("{}::{}", parent_type, name);

                graph.functions.push(FunctionNode {
                    name,
                    qualified_path,
                    is_public,
                    is_async,
                    parent_type: Some(parent_type.to_string()),
                    line_start,
                    line_end,
                });
            }
        }

        Ok(())
    }

    fn extract_calls(
        &self,
        root: &tree_sitter::Node,
        source: &[u8],
        graph: &mut CodeGraph,
    ) -> Result<()> {
        // For each function, extract calls made within it
        let query_str = r#"
            (function_item
                name: (identifier) @fn_name
                body: (block) @body) @fn
        "#;

        let query = Query::new(tree_sitter_rust::language(), query_str)
            .map_err(|e| anyhow!("Query error: {}", e))?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, *root, source);

        for m in matches {
            let fn_name_node = m.captures.iter().find(|c| c.index == 0).map(|c| c.node);
            let body_node = m.captures.iter().find(|c| c.index == 1).map(|c| c.node);

            if let (Some(fn_name_node), Some(body_node)) = (fn_name_node, body_node) {
                let caller_name = fn_name_node
                    .utf8_text(source)
                    .map_err(|e| anyhow!("UTF8 error: {}", e))?
                    .to_string();

                self.extract_calls_in_body(&caller_name, &body_node, source, graph)?;
            }
        }

        Ok(())
    }

    fn extract_calls_in_body(
        &self,
        caller: &str,
        body: &tree_sitter::Node,
        source: &[u8],
        graph: &mut CodeGraph,
    ) -> Result<()> {
        let query_str = r#"
            (call_expression
                function: (identifier) @callee)
            (call_expression
                function: (field_expression
                    field: (field_identifier) @callee))
            (call_expression
                function: (scoped_identifier
                    name: (identifier) @callee))
        "#;

        let query = Query::new(tree_sitter_rust::language(), query_str)
            .map_err(|e| anyhow!("Query error: {}", e))?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, *body, source);

        for m in matches {
            for capture in m.captures {
                let callee_name = capture
                    .node
                    .utf8_text(source)
                    .map_err(|e| anyhow!("UTF8 error: {}", e))?
                    .to_string();

                let line = capture.node.start_position().row + 1;

                graph.calls.push(CallEdge {
                    from: caller.to_string(),
                    to: callee_name,
                    line,
                });
            }
        }

        Ok(())
    }

    fn is_inside_impl(&self, node: tree_sitter::Node) -> bool {
        let mut current = node.parent();
        while let Some(parent) = current {
            if parent.kind() == "impl_item" {
                return true;
            }
            current = parent.parent();
        }
        false
    }
}

impl Default for CodeGraphExtractor {
    fn default() -> Self {
        Self::new()
    }
}
