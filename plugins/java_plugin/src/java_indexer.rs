use crate::plugin_api::{Entity, Edge, EdgeKind, EntityKind, Span, PluginResult};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::{Language, Parser, Tree};
use walkdir::{WalkDir, DirEntry};

use tree_sitter_java::language;

pub struct JavaIndexer {
    parser: Parser,
}

impl JavaIndexer {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        let language = language();
        parser.set_language(language)
            .context("Failed to set tree-sitter-java language")?;
        
        Ok(JavaIndexer { parser })
    }

    pub fn index_directory(&mut self, root_path: &str) -> Result<PluginResult> {
        let mut entities = Vec::new();
        let mut edges = Vec::new();
        let mut file_count = 0;

        for entry in WalkDir::new(root_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| is_java_file(e))
        {
            let path = entry.path();
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok((file_entities, file_edges)) = self.parse_file(&content, path) {
                    entities.extend(file_entities);
                    edges.extend(file_edges);
                    file_count += 1;
                }
            }
        }

        let mut meta = HashMap::new();
        meta.insert("files_processed".to_string(), serde_json::Value::Number(serde_json::Number::from(file_count)));

        Ok(PluginResult {
            entities: Some(entities),
            edges: Some(edges),
            diagnostics: None,
            meta: Some(meta),
        })
    }

    fn parse_file(&mut self, content: &str, file_path: &Path) -> Result<(Vec<Entity>, Vec<Edge>)> {
        let tree = self.parser.parse(content, None)
            .context("Failed to parse Java file")?;

        let mut entities = Vec::new();
        let mut edges = Vec::new();
        let mut entity_id_counter = 0;

        // Extract package declaration
        if let Some(package_node) = find_package_declaration(&tree) {
            if let Some(package_name) = extract_package_name(&package_node, content) {
                let package_entity = Entity {
                    file_path: file_path.to_string_lossy().to_string(),
                    name: package_name.clone(),
                    kind: EntityKind::Package,
                    signature: Some(format!("package {}", package_name)),
                    span: node_to_span(&package_node),
                    extra: None,
                };
                entities.push(package_entity);
                entity_id_counter += 1;
            }
        }

        // Extract classes, interfaces, methods, and fields
        let root_node = tree.root_node();
        self.extract_declarations(&root_node, content, file_path, &mut entities, &mut edges, &mut entity_id_counter);

        Ok((entities, edges))
    }

    fn extract_declarations(
        &self,
        node: &tree_sitter::Node,
        source: &str,
        file_path: &Path,
        entities: &mut Vec<Entity>,
        edges: &mut Vec<Edge>,
        id_counter: &mut usize,
    ) {
        match node.kind() {
            "program" => {
                // Process all children
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.extract_declarations(&child, source, file_path, entities, edges, id_counter);
                }
            }
            "class_declaration" => {
                if let Some(class_entity) = self.extract_class(node, source, file_path, id_counter) {
                    let class_id = class_entity.name.clone();
                    
                    // Extract class members
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if child.kind() == "class_body" {
                            self.extract_class_members(&child, source, file_path, &class_id, entities, edges, id_counter);
                        }
                    }
                    
                    entities.push(class_entity);
                }
            }
            "interface_declaration" => {
                if let Some(interface_entity) = self.extract_interface(node, source, file_path, id_counter) {
                    let interface_id = interface_entity.name.clone();
                    
                    // Extract interface members
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if child.kind() == "interface_body" {
                            self.extract_class_members(&child, source, file_path, &interface_id, entities, edges, id_counter);
                        }
                    }
                    
                    entities.push(interface_entity);
                }
            }
            "import_declaration" => {
                if let Some(import_edge) = self.extract_import(node, source, file_path) {
                    edges.push(import_edge);
                }
            }
            _ => {}
        }
    }

    fn extract_class(&self, node: &tree_sitter::Node, source: &str, file_path: &Path, _id_counter: &mut usize) -> Option<Entity> {
        let name = extract_identifier(node, source)?;
        let signature = extract_node_text(node, source);

        Some(Entity {
            file_path: file_path.to_string_lossy().to_string(),
            name,
            kind: EntityKind::Class,
            signature: Some(signature),
            span: node_to_span(node),
            extra: None,
        })
    }

    fn extract_interface(&self, node: &tree_sitter::Node, source: &str, file_path: &Path, _id_counter: &mut usize) -> Option<Entity> {
        let name = extract_identifier(node, source)?;
        let signature = extract_node_text(node, source);

        Some(Entity {
            file_path: file_path.to_string_lossy().to_string(),
            name,
            kind: EntityKind::Interface,
            signature: Some(signature),
            span: node_to_span(node),
            extra: None,
        })
    }

    fn extract_class_members(
        &self,
        node: &tree_sitter::Node,
        source: &str,
        file_path: &Path,
        parent_id: &str,
        entities: &mut Vec<Entity>,
        edges: &mut Vec<Edge>,
        _id_counter: &mut usize,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "method_declaration" => {
                    if let Some(method_entity) = self.extract_method(&child, source, file_path) {
                        let method_id = method_entity.name.clone();
                        
                        // Create "contains" edge from class to method
                        edges.push(Edge {
                            from: parent_id.to_string(),
                            to: method_id.clone(),
                            kind: EdgeKind::Contains,
                        });
                        
                        entities.push(method_entity);
                    }
                }
                "field_declaration" => {
                    if let Some(field_entities) = self.extract_fields(&child, source, file_path) {
                        for field_entity in field_entities {
                            let field_id = field_entity.name.clone();
                            
                            // Create "contains" edge from class to field
                            edges.push(Edge {
                                from: parent_id.to_string(),
                                to: field_id.clone(),
                                kind: EdgeKind::Contains,
                            });
                            
                            entities.push(field_entity);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn extract_method(&self, node: &tree_sitter::Node, source: &str, file_path: &Path) -> Option<Entity> {
        let name = extract_identifier(node, source)?;
        let signature = extract_node_text(node, source);

        Some(Entity {
            file_path: file_path.to_string_lossy().to_string(),
            name,
            kind: EntityKind::Method,
            signature: Some(signature),
            span: node_to_span(node),
            extra: None,
        })
    }

    fn extract_fields(&self, node: &tree_sitter::Node, source: &str, file_path: &Path) -> Option<Vec<Entity>> {
        let mut fields = Vec::new();
        let mut cursor = node.walk();
        
        for child in node.children(&mut cursor) {
            if child.kind() == "variable_declarator" {
                if let Some(name) = extract_identifier(&child, source) {
                    fields.push(Entity {
                        file_path: file_path.to_string_lossy().to_string(),
                        name,
                        kind: EntityKind::Field,
                        signature: Some(extract_node_text(node, source)),
                        span: node_to_span(&child),
                        extra: None,
                    });
                }
            }
        }
        
        if fields.is_empty() {
            None
        } else {
            Some(fields)
        }
    }

    fn extract_import(&self, node: &tree_sitter::Node, source: &str, file_path: &Path) -> Option<Edge> {
        let import_text = extract_node_text(node, source);
        
        // Extract the imported class/package name
        if let Some(import_name) = import_text.strip_prefix("import ").and_then(|s| s.strip_suffix(";")) {
            Some(Edge {
                from: file_path.to_string_lossy().to_string(),
                to: import_name.trim().to_string(),
                kind: EdgeKind::Imports,
            })
        } else {
            None
        }
    }
}

fn is_java_file(entry: &DirEntry) -> bool {
    entry.path()
        .extension()
        .map(|ext| ext == "java")
        .unwrap_or(false)
}

fn find_package_declaration(tree: &Tree) -> Option<tree_sitter::Node<'_>> {
    let root = tree.root_node();
    let mut cursor = root.walk();
    
    for child in root.children(&mut cursor) {
        if child.kind() == "package_declaration" {
            return Some(child);
        }
    }
    
    None
}

fn extract_package_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    
    for child in node.children(&mut cursor) {
        if child.kind() == "scoped_identifier" || child.kind() == "identifier" {
            return Some(extract_node_text(&child, source));
        }
    }
    
    None
}

fn extract_identifier(node: &tree_sitter::Node, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            return Some(extract_node_text(&child, source));
        }
    }
    
    None
}

fn extract_node_text(node: &tree_sitter::Node, source: &str) -> String {
    source[node.byte_range()].to_string()
}

fn node_to_span(node: &tree_sitter::Node) -> Option<Span> {
    let start_position = node.start_position();
    let end_position = node.end_position();
    
    Some(Span {
        start_line: start_position.row as u32 + 1,
        start_col: start_position.column as u32 + 1,
        end_line: end_position.row as u32 + 1,
        end_col: end_position.column as u32 + 1,
    })
}