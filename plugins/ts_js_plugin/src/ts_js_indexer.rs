use anyhow::{Context, Result};
use crate::plugin_api::{Edge, EdgeKind, Entity, EntityKind, PluginResult, Span};
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::{Node, Parser, Tree};
use walkdir::WalkDir;

pub struct TsJsIndexer {
    ts_parser: Parser,
    js_parser: Parser,
}

impl TsJsIndexer {
    pub fn new() -> Result<Self> {
        let mut ts_parser = Parser::new();
        ts_parser
            .set_language(tree_sitter_typescript::language_typescript())
            .context("Failed to load TypeScript grammar")?;

        let mut js_parser = Parser::new();
        js_parser
            .set_language(tree_sitter_javascript::language())
            .context("Failed to load JavaScript grammar")?;

        Ok(Self {
            ts_parser,
            js_parser,
        })
    }

    pub fn index_directory(&mut self, root_path: &str) -> Result<PluginResult> {
        let mut entities = Vec::new();
        let mut edges = Vec::new();

        let walker = WalkDir::new(root_path)
            .into_iter()
            .filter_entry(|e| !is_hidden(e))
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                let path = e.path();
                let extension = path.extension().and_then(|s| s.to_str());
                matches!(extension, Some("ts") | Some("tsx") | Some("js") | Some("jsx"))
            });

        for entry in walker {
            let path = entry.path();
            if let Err(e) = self.index_file(path, &mut entities, &mut edges) {
                eprintln!("Failed to index file {}: {}", path.display(), e);
            }
        }

        Ok(PluginResult {
            entities: Some(entities),
            edges: Some(edges),
            diagnostics: None,
            meta: None,
        })
    }

    pub fn index_file(&mut self, file_path: &Path, entities: &mut Vec<Entity>, edges: &mut Vec<Edge>) -> Result<()> {
        let content = std::fs::read_to_string(file_path)
            .context(format!("Failed to read file: {}", file_path.display()))?;

        let tree = if file_path.extension().and_then(|s| s.to_str()) == Some("ts")
            || file_path.extension().and_then(|s| s.to_str()) == Some("tsx")
        {
            self.ts_parser.parse(&content, None).ok_or_else(|| {
                anyhow::anyhow!("Failed to parse TypeScript file: {}", file_path.display())
            })?
        } else {
            self.js_parser.parse(&content, None).ok_or_else(|| {
                anyhow::anyhow!("Failed to parse JavaScript file: {}", file_path.display())
            })?
        };

        let root_node = tree.root_node();
        self.extract_entities(file_path, &root_node, &content, entities, edges)?;

        Ok(())
    }

    fn extract_entities(
        &self,
        file_path: &Path,
        root_node: &Node,
        content: &str,
        entities: &mut Vec<Entity>,
        edges: &mut Vec<Edge>,
    ) -> Result<()> {
        let file_path_str = file_path.to_string_lossy().to_string();

        // Walk the AST and extract entities
        let mut cursor = root_node.walk();
        for node in root_node.children(&mut cursor) {
            match node.kind() {
                "class_declaration" => {
                    self.extract_class(&file_path_str, &node, content, entities, edges)?;
                }
                "interface_declaration" => {
                    self.extract_interface(&file_path_str, &node, content, entities)?;
                }
                "function_declaration" => {
                    self.extract_function(&file_path_str, &node, content, entities)?;
                }
                "variable_declaration" | "lexical_declaration" => {
                    self.extract_variables(&file_path_str, &node, content, entities)?;
                }
                "import_statement" => {
                    self.extract_imports(&file_path_str, &node, content, entities)?;
                }
                "export_statement" => {
                    // Handle `export { foo, bar }` style exports
                    self.extract_exports(&file_path_str, &node, content, entities)?;
                    // Also handle `export class/interface/function/const` declarations
                    let mut export_cursor = node.walk();
                    for export_child in node.children(&mut export_cursor) {
                        match export_child.kind() {
                            "class_declaration" => {
                                self.extract_class(&file_path_str, &export_child, content, entities, edges)?;
                            }
                            "interface_declaration" => {
                                self.extract_interface(&file_path_str, &export_child, content, entities)?;
                            }
                            "function_declaration" => {
                                self.extract_function(&file_path_str, &export_child, content, entities)?;
                            }
                            "lexical_declaration" | "variable_declaration" => {
                                self.extract_variables(&file_path_str, &export_child, content, entities)?;
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn extract_class(
        &self,
        file_path: &str,
        class_node: &Node,
        content: &str,
        entities: &mut Vec<Entity>,
        edges: &mut Vec<Edge>,
    ) -> Result<()> {
        if let Some(name_node) = class_node.child_by_field_name("name") {
            let name = name_node.utf8_text(content.as_bytes())?;
            let span = node_to_span(&name_node);

            let entity = Entity {
                file_path: file_path.to_string(),
                name: name.to_string(),
                kind: EntityKind::Class,
                signature: None,
                span: Some(span),
                extra: None,
            };
            entities.push(entity);

            // Extract class members
            self.extract_class_members(file_path, class_node, content, entities, edges)?;
        }

        Ok(())
    }

    fn extract_interface(
        &self,
        file_path: &str,
        interface_node: &Node,
        content: &str,
        entities: &mut Vec<Entity>,
    ) -> Result<()> {
        if let Some(name_node) = interface_node.child_by_field_name("name") {
            let name = name_node.utf8_text(content.as_bytes())?;
            let span = node_to_span(&name_node);

            let entity = Entity {
                file_path: file_path.to_string(),
                name: name.to_string(),
                kind: EntityKind::Interface,
                signature: None,
                span: Some(span),
                extra: None,
            };
            entities.push(entity);
        }

        Ok(())
    }

    fn extract_function(
        &self,
        file_path: &str,
        function_node: &Node,
        content: &str,
        entities: &mut Vec<Entity>,
    ) -> Result<()> {
        if let Some(name_node) = function_node.child_by_field_name("name") {
            let name = name_node.utf8_text(content.as_bytes())?;
            let span = node_to_span(&name_node);

            let entity = Entity {
                file_path: file_path.to_string(),
                name: name.to_string(),
                kind: EntityKind::Function,
                signature: None,
                span: Some(span),
                extra: None,
            };
            entities.push(entity);
        }

        Ok(())
    }

    fn extract_variables(
        &self,
        file_path: &str,
        var_node: &Node,
        content: &str,
        entities: &mut Vec<Entity>,
    ) -> Result<()> {
        let mut cursor = var_node.walk();
        for child in var_node.children(&mut cursor) {
            if child.kind() == "variable_declarator" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = name_node.utf8_text(content.as_bytes())?;
                    let span = node_to_span(&name_node);

                    let entity = Entity {
                        file_path: file_path.to_string(),
                        name: name.to_string(),
                        kind: EntityKind::Variable,
                        signature: None,
                        span: Some(span),
                        extra: None,
                    };
                    entities.push(entity);
                }
            }
        }

        Ok(())
    }

    fn extract_imports(
        &self,
        file_path: &str,
        import_node: &Node,
        content: &str,
        entities: &mut Vec<Entity>,
    ) -> Result<()> {
        if let Some(clause_node) = import_node.child_by_field_name("import_clause") {
            let mut cursor = clause_node.walk();
            for child in clause_node.children(&mut cursor) {
                if child.kind() == "named_imports" {
                    let mut named_cursor = child.walk();
                    for named_child in child.children(&mut named_cursor) {
                        if named_child.kind() == "import_specifier" {
                            if let Some(name_node) = named_child.child_by_field_name("name") {
                                let name = name_node.utf8_text(content.as_bytes())?;
                                let span = node_to_span(&name_node);

                                let entity = Entity {
                                    file_path: file_path.to_string(),
                                    name: name.to_string(),
                                    kind: EntityKind::Import,
                                    signature: None,
                                    span: Some(span),
                                    extra: None,
                                };
                                entities.push(entity);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn extract_exports(
        &self,
        file_path: &str,
        export_node: &Node,
        content: &str,
        entities: &mut Vec<Entity>,
    ) -> Result<()> {
        let mut cursor = export_node.walk();
        for child in export_node.children(&mut cursor) {
            if child.kind() == "export_clause" {
                let mut export_cursor = child.walk();
                for export_child in child.children(&mut export_cursor) {
                    if export_child.kind() == "export_specifier" {
                        if let Some(name_node) = export_child.child_by_field_name("name") {
                            let name = name_node.utf8_text(content.as_bytes())?;
                            let span = node_to_span(&name_node);

                            let entity = Entity {
                                file_path: file_path.to_string(),
                                name: name.to_string(),
                                kind: EntityKind::Export,
                                signature: None,
                                span: Some(span),
                                extra: None,
                            };
                            entities.push(entity);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn extract_class_members(
        &self,
        file_path: &str,
        class_node: &Node,
        content: &str,
        entities: &mut Vec<Entity>,
        edges: &mut Vec<Edge>,
    ) -> Result<()> {
        let class_name = class_node
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(content.as_bytes()).ok())
            .unwrap_or("unknown");

        // Find the class_body node first, then iterate its children
        let class_body = class_node.child_by_field_name("body");
        let body_node = class_body.as_ref().unwrap_or(class_node);

        let mut cursor = body_node.walk();
        for child in body_node.children(&mut cursor) {
            match child.kind() {
                "method_definition" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = name_node.utf8_text(content.as_bytes())?;
                        let span = node_to_span(&name_node);

                        let entity = Entity {
                            file_path: file_path.to_string(),
                            name: name.to_string(),
                            kind: EntityKind::Method,
                            signature: None,
                            span: Some(span),
                            extra: None,
                        };
                        entities.push(entity);

                        // Add contains edge from class to method
                        let edge = Edge {
                            from: format!("{}:{}", file_path, class_name),
                            to: format!("{}:{}", file_path, name),
                            kind: EdgeKind::Contains,
                        };
                        edges.push(edge);
                    }
                }
                "public_field_definition" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = name_node.utf8_text(content.as_bytes())?;
                        let span = node_to_span(&name_node);

                        let entity = Entity {
                            file_path: file_path.to_string(),
                            name: name.to_string(),
                            kind: EntityKind::Property,
                            signature: None,
                            span: Some(span),
                            extra: None,
                        };
                        entities.push(entity);

                        // Add contains edge from class to property
                        let edge = Edge {
                            from: format!("{}:{}", file_path, class_name),
                            to: format!("{}:{}", file_path, name),
                            kind: EdgeKind::Contains,
                        };
                        edges.push(edge);
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}

fn node_to_span(node: &Node) -> Span {
    Span {
        start_line: (node.start_position().row + 1) as u32,
        start_col: node.start_position().column as u32,
        end_line: (node.end_position().row + 1) as u32,
        end_col: node.end_position().column as u32,
    }
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    // Skip hidden directories and node_modules, but don't filter the root directory
    if entry.depth() == 0 {
        return false;
    }
    entry.file_name()
        .to_str()
        .map(|s| s.starts_with('.') || s == "node_modules")
        .unwrap_or(false)
}