//! Code Dependency Extractor using Tree-sitter
//! Extracts imports, function calls, trait implementations from Rust source code.

use anyhow::{anyhow, Result};
use tree_sitter::{Parser, Query, QueryCursor};

/// Extracted code dependencies from a source file
#[derive(Debug, Clone, Default)]
pub struct CodeDependencies {
    pub path: String,
    pub imports: Vec<String>,
    pub function_defs: Vec<String>,
    pub struct_defs: Vec<String>,
    pub calls: Vec<(String, String)>,      // (caller_fn, callee_fn)
    pub implements: Vec<(String, String)>, // (struct_name, trait_name)
}

pub struct CodeDependencyExtractor {
    parser: Parser,
}

impl CodeDependencyExtractor {
    pub fn new() -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_rust::language())
            .expect("Failed to set Rust language");
        CodeDependencyExtractor { parser }
    }

    pub fn extract_from_source(&mut self, source: &str, path: &str) -> Result<CodeDependencies> {
        let tree = self
            .parser
            .parse(source, None)
            .ok_or_else(|| anyhow!("Failed to parse source"))?;

        let root = tree.root_node();
        let source_bytes = source.as_bytes();

        let mut deps = CodeDependencies {
            path: path.to_string(),
            ..Default::default()
        };

        // Extract imports (use statements)
        self.extract_imports(&root, source_bytes, &mut deps)?;

        // Extract function definitions
        self.extract_function_defs(&root, source_bytes, &mut deps)?;

        // Extract struct definitions
        self.extract_struct_defs(&root, source_bytes, &mut deps)?;

        // Extract trait implementations
        self.extract_trait_impls(&root, source_bytes, &mut deps)?;

        // Extract function calls
        self.extract_function_calls(&root, source_bytes, &mut deps)?;

        Ok(deps)
    }

    fn extract_imports(
        &self,
        root: &tree_sitter::Node,
        source: &[u8],
        deps: &mut CodeDependencies,
    ) -> Result<()> {
        let query_str = r#"
            (use_declaration
                argument: (scoped_identifier) @import)
            (use_declaration
                argument: (identifier) @import)
            (use_declaration
                argument: (use_as_clause
                    path: (scoped_identifier) @import))
        "#;

        let query = Query::new(tree_sitter_rust::language(), query_str)
            .map_err(|e| anyhow!("Query error: {}", e))?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, *root, source);

        for m in matches {
            for capture in m.captures {
                let import_text = capture
                    .node
                    .utf8_text(source)
                    .map_err(|e| anyhow!("UTF8 error: {}", e))?;
                if !deps.imports.contains(&import_text.to_string()) {
                    deps.imports.push(import_text.to_string());
                }
            }
        }

        Ok(())
    }

    fn extract_function_defs(
        &self,
        root: &tree_sitter::Node,
        source: &[u8],
        deps: &mut CodeDependencies,
    ) -> Result<()> {
        let query_str = r#"
            (function_item
                name: (identifier) @fn_name)
        "#;

        let query = Query::new(tree_sitter_rust::language(), query_str)
            .map_err(|e| anyhow!("Query error: {}", e))?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, *root, source);

        for m in matches {
            for capture in m.captures {
                let fn_name = capture
                    .node
                    .utf8_text(source)
                    .map_err(|e| anyhow!("UTF8 error: {}", e))?;
                deps.function_defs.push(fn_name.to_string());
            }
        }

        Ok(())
    }

    fn extract_struct_defs(
        &self,
        root: &tree_sitter::Node,
        source: &[u8],
        deps: &mut CodeDependencies,
    ) -> Result<()> {
        let query_str = r#"
            (struct_item
                name: (type_identifier) @struct_name)
        "#;

        let query = Query::new(tree_sitter_rust::language(), query_str)
            .map_err(|e| anyhow!("Query error: {}", e))?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, *root, source);

        for m in matches {
            for capture in m.captures {
                let struct_name = capture
                    .node
                    .utf8_text(source)
                    .map_err(|e| anyhow!("UTF8 error: {}", e))?;
                deps.struct_defs.push(struct_name.to_string());
            }
        }

        Ok(())
    }

    fn extract_trait_impls(
        &self,
        root: &tree_sitter::Node,
        source: &[u8],
        deps: &mut CodeDependencies,
    ) -> Result<()> {
        let query_str = r#"
            (impl_item
                trait: (type_identifier) @trait_name
                type: (type_identifier) @struct_name)
        "#;

        let query = Query::new(tree_sitter_rust::language(), query_str)
            .map_err(|e| anyhow!("Query error: {}", e))?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, *root, source);

        for m in matches {
            let mut trait_name = String::new();
            let mut struct_name = String::new();

            for capture in m.captures {
                let text = capture
                    .node
                    .utf8_text(source)
                    .map_err(|e| anyhow!("UTF8 error: {}", e))?;
                let capture_name = &query.capture_names()[capture.index as usize];
                if capture_name == "trait_name" {
                    trait_name = text.to_string();
                } else if capture_name == "struct_name" {
                    struct_name = text.to_string();
                }
            }

            if !trait_name.is_empty() && !struct_name.is_empty() {
                deps.implements.push((struct_name, trait_name));
            }
        }

        Ok(())
    }

    fn extract_function_calls(
        &self,
        root: &tree_sitter::Node,
        source: &[u8],
        deps: &mut CodeDependencies,
    ) -> Result<()> {
        // Use recursive tree traversal to find all call_expressions within function bodies
        // This is more robust than pattern matching which can miss nested structures
        self.visit_function_calls(root, source, deps, None)
    }

    fn visit_function_calls(
        &self,
        node: &tree_sitter::Node,
        source: &[u8],
        deps: &mut CodeDependencies,
        current_function: Option<&str>,
    ) -> Result<()> {
        match node.kind() {
            "function_item" => {
                // Extract function name
                let mut fn_name = None;
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        if child.kind() == "identifier" {
                            fn_name = Some(
                                child
                                    .utf8_text(source)
                                    .map_err(|e| anyhow!("UTF8 error: {}", e))?
                                    .to_string(),
                            );
                            break;
                        }
                    }
                }

                // Traverse children with this function as context
                if let Some(name) = fn_name {
                    for i in 0..node.child_count() {
                        if let Some(child) = node.child(i) {
                            self.visit_function_calls(&child, source, deps, Some(&name))?;
                        }
                    }
                }
            }
            "call_expression" => {
                // Extract the callee identifier
                if let Some(fn_name) = current_function {
                    for i in 0..node.child_count() {
                        if let Some(child) = node.child(i) {
                            if child.kind() == "identifier" {
                                let callee = child
                                    .utf8_text(source)
                                    .map_err(|e| anyhow!("UTF8 error: {}", e))?;
                                deps.calls.push((fn_name.to_string(), callee.to_string()));
                                break;
                            }
                        }
                    }
                }
                // Also traverse children for nested calls
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        self.visit_function_calls(&child, source, deps, current_function)?;
                    }
                }
            }
            _ => {
                // Traverse all children
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        self.visit_function_calls(&child, source, deps, current_function)?;
                    }
                }
            }
        }

        Ok(())
    }
}

impl Default for CodeDependencyExtractor {
    fn default() -> Self {
        Self::new()
    }
}
