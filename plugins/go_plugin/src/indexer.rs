use anyhow::{Context, Result};
use crate::plugin_api::{Edge, EdgeKind, Entity, EntityKind, PluginResult, Span};
use std::path::Path;
use tree_sitter::{Node, Parser};
use walkdir::WalkDir;

pub struct GoIndexer {
    parser: Parser,
}

impl GoIndexer {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_go::language())
            .context("Failed to load Go grammar")?;

        Ok(Self { parser })
    }

    pub fn index_directory(&mut self, root_path: &str) -> Result<PluginResult> {
        let mut all_entities = Vec::new();
        let mut all_edges = Vec::new();

        let walker = WalkDir::new(root_path)
            .into_iter()
            .filter_entry(|e| {
                // Don't filter the root directory
                if e.path() == std::path::Path::new(root_path) {
                    return true;
                }
                
                !is_hidden_or_vendor(e)
            })
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                let path = e.path();
                let extension = path.extension().and_then(|s| s.to_str());
                matches!(extension, Some("go"))
            });

        for entry in walker {
            let path = entry.path();
            if let Err(e) = self.index_file_into(path, &mut all_entities, &mut all_edges) {
                eprintln!("Failed to index file {}: {}", path.display(), e);
            }
        }

        Ok(PluginResult {
            entities: Some(all_entities),
            edges: Some(all_edges),
            diagnostics: None,
            meta: None,
        })
    }

    pub fn index_file(&mut self, file_path: &str) -> Result<PluginResult> {
        let mut entities = Vec::new();
        let mut edges = Vec::new();
        
        self.index_file_into(Path::new(file_path), &mut entities, &mut edges)?;
        
        Ok(PluginResult {
            entities: Some(entities),
            edges: Some(edges),
            diagnostics: None,
            meta: None,
        })
    }

    fn index_file_into(&mut self, file_path: &Path, entities: &mut Vec<Entity>, edges: &mut Vec<Edge>) -> Result<()> {
        let source = std::fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

        let tree = self.parser.parse(&source, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Go file: {}", file_path.display()))?;

        let root = tree.root_node();
        self.extract_entities(file_path, &source, root, entities, edges);
        
        Ok(())
    }

    fn extract_entities(&self, file_path: &Path, source: &str, node: Node, entities: &mut Vec<Entity>, edges: &mut Vec<Edge>) {
        // Skip empty nodes
        if node.byte_range().is_empty() {
            return;
        }
        
        match node.kind() {
            "source_file" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.extract_entities(file_path, source, child, entities, edges);
                }
            }
            "package_clause" => {
                // Try to find package_identifier by looking at children
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "package_identifier" {
                        let name = extract_text(source, &child);
                        entities.push(Entity {
                            file_path: file_path.to_string_lossy().to_string(),
                            name,
                            kind: EntityKind::Package,
                            signature: None,
                            span: Some(node_to_span(&child)),
                            extra: None,
                        });
                        break;
                    }
                }
                return; // Don't recurse into package_clause children
            }
            "import_declaration" => {
                // Handle both single imports and grouped imports
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    match child.kind() {
                        "import_spec" => {
                            // Single import: import "fmt"
                            if let Some(path_node) = child.child_by_field_name("path") {
                                let import_path = extract_text(source, &path_node);
                                let clean_path = import_path.trim_matches('"');
                                entities.push(Entity {
                                    file_path: file_path.to_string_lossy().to_string(),
                                    name: clean_path.to_string(),
                                    kind: EntityKind::Import,
                                    signature: None,
                                    span: Some(node_to_span(&path_node)),
                                    extra: None,
                                });
                            }
                        }
                        "import_spec_list" => {
                            // Grouped imports: import ("a" "b")
                            let mut inner_cursor = child.walk();
                            for import_spec in child.children(&mut inner_cursor) {
                                if import_spec.kind() == "import_spec" {
                                    if let Some(path_node) = import_spec.child_by_field_name("path") {
                                        let import_path = extract_text(source, &path_node);
                                        let clean_path = import_path.trim_matches('"');
                                        entities.push(Entity {
                                            file_path: file_path.to_string_lossy().to_string(),
                                            name: clean_path.to_string(),
                                            kind: EntityKind::Import,
                                            signature: None,
                                            span: Some(node_to_span(&path_node)),
                                            extra: None,
                                        });
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                return; // Don't recurse into import_declaration children
            }
            "function_declaration" => {
                // Find the function name (first identifier after "func")
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "identifier" {
                        let name = extract_text(source, &child);
                        entities.push(Entity {
                            file_path: file_path.to_string_lossy().to_string(),
                            name,
                            kind: EntityKind::Function,
                            signature: Some(extract_text(source, &node)),
                            span: Some(node_to_span(&child)),
                            extra: None,
                        });
                        break;
                    }
                }
                // Also extract call edges from the function body
                self.extract_function_calls(file_path, source, node, edges);
                return; // Don't recurse into function_declaration children
            }
            "method_declaration" => {
                // Find the method name (field_identifier after receiver)
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "field_identifier" {
                        let name = extract_text(source, &child);
                        entities.push(Entity {
                            file_path: file_path.to_string_lossy().to_string(),
                            name,
                            kind: EntityKind::Method,
                            signature: Some(extract_text(source, &node)),
                            span: Some(node_to_span(&child)),
                            extra: None,
                        });
                        break;
                    }
                }
                // Also extract call edges from the method body
                self.extract_function_calls(file_path, source, node, edges);
                return; // Don't recurse into method_declaration children
            }
            "type_declaration" => {
                // Try to find type_spec by looking at children
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "type_spec" {
                        self.extract_type_definition(file_path, source, child, entities);
                        break;
                    }
                }
                return; // Don't recurse into type_declaration children
            }
            "const_declaration" => {
                self.extract_const_var_declarations(file_path, source, node, EntityKind::Const, entities);
                return; // Don't recurse into const_declaration children
            }
            "var_declaration" => {
                self.extract_const_var_declarations(file_path, source, node, EntityKind::Var, entities);
                return; // Don't recurse into var_declaration children
            }
            _ => {}
        }

        // Only recurse if this node type doesn't handle its own children
        let should_recurse = !matches!(node.kind(), 
            "source_file" | "package_clause" | "import_declaration" | "function_declaration" | 
            "method_declaration" | "type_declaration" | "const_declaration" | 
            "var_declaration"
        );
        
        if should_recurse {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                self.extract_entities(file_path, source, child, entities, edges);
            }
        }
    }

    fn extract_type_definition(&self, file_path: &Path, source: &str, node: Node, entities: &mut Vec<Entity>) {
        if node.kind() == "type_spec" {
            if let Some(name_node) = node.child_by_field_name("name") {
                let name = extract_text(source, &name_node);
                let kind = if let Some(type_node) = node.child_by_field_name("type") {
                    match type_node.kind() {
                        "struct_type" => EntityKind::Struct,
                        "interface_type" => EntityKind::Interface,
                        _ => EntityKind::TypeAlias,
                    }
                } else {
                    EntityKind::TypeAlias
                };

                entities.push(Entity {
                    file_path: file_path.to_string_lossy().to_string(),
                    name,
                    kind,
                    signature: Some(extract_text(source, &node)),
                    span: Some(node_to_span(&name_node)),
                    extra: None,
                });
            }
        }
    }

    fn extract_const_var_declarations(&self, file_path: &Path, source: &str, node: Node, kind: EntityKind, entities: &mut Vec<Entity>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "const_spec" || child.kind() == "var_spec" {
                // Handle const/var with type: const Name string = "value"
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = extract_text(source, &name_node);
                    entities.push(Entity {
                        file_path: file_path.to_string_lossy().to_string(),
                        name,
                        kind: kind.clone(),
                        signature: Some(extract_text(source, &child)),
                        span: Some(node_to_span(&name_node)),
                        extra: None,
                    });
                }
            } else if child.kind() == "identifier" {
                // Handle simple const/var declarations like "const Version = "1.0.0""
                // But only if it's not part of a const_spec/var_spec
                let mut has_spec_siblings = false;
                let mut sibling_cursor = node.walk();
                for sibling in node.children(&mut sibling_cursor) {
                    if sibling.kind() == "const_spec" || sibling.kind() == "var_spec" {
                        has_spec_siblings = true;
                        break;
                    }
                }
                if !has_spec_siblings {
                    let name = extract_text(source, &child);
                    entities.push(Entity {
                        file_path: file_path.to_string_lossy().to_string(),
                        name,
                        kind: kind.clone(),
                        signature: Some(extract_text(source, &node)),
                        span: Some(node_to_span(&child)),
                        extra: None,
                    });
                }
            }
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn extract_function_calls(&self, file_path: &Path, source: &str, node: Node, edges: &mut Vec<Edge>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.extract_function_calls(file_path, source, child, edges);
        }

        if node.kind() == "call_expression" {
            if let Some(func_node) = node.child_by_field_name("function") {
                let func_name = extract_identifier_name(source, &func_node);
                if !func_name.is_empty() {
                    edges.push(Edge {
                        from: format!("{}:{}", file_path.to_string_lossy(), node.start_position().row),
                        to: func_name,
                        kind: EdgeKind::Calls,
                    });
                }
            }
        }
    }
}

fn is_hidden_or_vendor(entry: &walkdir::DirEntry) -> bool {
    let file_name = entry.file_name().to_string_lossy();
    let is_hidden = file_name.starts_with('.') && file_name != "." && file_name != "..";
    let is_vendor = file_name == "vendor";
    is_hidden || is_vendor
}

fn extract_text(source: &str, node: &Node) -> String {
    source[node.byte_range()].to_string()
}

fn extract_identifier_name(source: &str, node: &Node) -> String {
    match node.kind() {
        "identifier" => extract_text(source, node),
        "selector_expression" => {
            if let Some(field_node) = node.child_by_field_name("field") {
                extract_text(source, &field_node)
            } else {
                String::new()
            }
        }
        _ => String::new(),
    }
}

fn node_to_span(node: &Node) -> Span {
    let start = node.start_position();
    let end = node.end_position();
    Span {
        start_line: start.row as u32 + 1,
        start_col: start.column as u32,
        end_line: end.row as u32 + 1,
        end_col: end.column as u32,
    }
}