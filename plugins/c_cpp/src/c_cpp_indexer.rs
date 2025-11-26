use std::path::Path;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use tree_sitter::Parser;
use tree_sitter_c;
use tree_sitter_cpp;

pub struct CppIndexer {
    parser_c: Parser,
    parser_cpp: Parser,
}

impl CppIndexer {
    pub fn new() -> Self {
        let mut parser_c = Parser::new();
        parser_c.set_language(tree_sitter_c::language())
            .expect("Error loading C grammar");

        let mut parser_cpp = Parser::new();
        parser_cpp.set_language(tree_sitter_cpp::language())
            .expect("Error loading C++ grammar");

        Self {
            parser_c,
            parser_cpp,
        }
    }

    pub fn index_file(&mut self, file_path: &str) -> Result<(Vec<Value>, Vec<Value>)> {
        let content = std::fs::read_to_string(file_path)?;
        let is_cpp = Path::new(file_path)
            .extension()
            .map(|ext| ext == "cpp" || ext == "cxx" || ext == "cc" || ext == "hpp" || ext == "hxx" || ext == "hh")
            .unwrap_or(false);

        let tree = if is_cpp {
            self.parser_cpp.parse(&content, None)
                .ok_or_else(|| anyhow!("Failed to parse C++ file"))?
        } else {
            self.parser_c.parse(&content, None)
                .ok_or_else(|| anyhow!("Failed to parse C file"))?
        };

        let mut entities = Vec::new();
        let mut edges = Vec::new();

        self.index_node(tree.root_node(), &content, &mut entities, &mut edges);

        Ok((entities, edges))
    }

    fn index_node(&self, node: tree_sitter::Node, source: &str, entities: &mut Vec<Value>, edges: &mut Vec<Value>) {
        match node.kind() {
            "function_definition" => self.index_function(node, source, entities, edges),
            "class_specifier" => self.index_class(node, source, entities, edges),
            "struct_specifier" => self.index_struct(node, source, entities, edges),
            "enum_specifier" => self.index_enum(node, source, entities, edges),
            "type_definition" => self.index_typedef(node, source, entities, edges),
            "namespace_definition" => self.index_namespace(node, source, entities, edges),
            "preproc_def" => self.index_macro_definition(node, source, entities, edges),
            "preproc_function_def" => self.index_function_macro(node, source, entities, edges),
            "preproc_include" => self.index_include(node, source, entities, edges),
            "call_expression" => self.index_function_call(node, source, entities, edges),
            _ => {
                // Recurse into child nodes
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        self.index_node(child, source, entities, edges);
                    }
                }
            }
        }
    }

    fn index_function(&self, node: tree_sitter::Node, source: &str, entities: &mut Vec<Value>, edges: &mut Vec<Value>) {
        let name_node = node.child_by_field_name("declarator")
            .and_then(|n| n.child_by_field_name("declarator"))
            .and_then(|n| n.child_by_field_name("declarator"));

        let name = name_node
            .map(|n| &source[n.byte_range()])
            .unwrap_or("unknown")
            .to_string();

        let return_type_node = node.child_by_field_name("type");
        let return_type = return_type_node
            .map(|n| &source[n.byte_range()])
            .unwrap_or("void")
            .to_string();

        let mut parameters = Vec::new();
        if let Some(params_node) = node.child_by_field_name("declarator")
            .and_then(|n| n.child_by_field_name("parameters")) {

            for i in 0..params_node.child_count() {
                if let Some(param) = params_node.child(i) {
                    if param.kind() == "parameter_declaration" {
                        if let Some(param_type) = param.child_by_field_name("type") {
                            parameters.push(source[param_type.byte_range()].to_string());
                        }
                    }
                }
            }
        }

        // Check if this is a method
        let class_node = node.parent()
            .and_then(|p| if p.kind() == "class_specifier" || p.kind() == "struct_specifier" {
                Some(p)
            } else {
                None
            });

        let is_static = if class_node.is_some() {
            node.child(0)
                .map(|c| source[c.byte_range()].contains("static"))
                .unwrap_or(false)
        } else {
            false
        };

        let mut entity = json!({
            "type": "function",
            "name": name,
            "return_type": return_type,
            "parameters": parameters,
            "is_static": is_static,
            "line": node.start_position().row + 1,
            "column": node.start_position().column + 1,
            "end_line": node.end_position().row + 1,
            "end_column": node.end_position().column + 1,
        });

        if let Some(class_name) = class_node.and_then(|c| c.child(1)).map(|c| &source[c.byte_range()]) {
            if is_static {
                entity["type"] = Value::String("static_method".to_string());
            } else {
                entity["type"] = Value::String("method".to_string());
            }
            entity["class"] = Value::String(class_name.to_string());
        }

        let entity_id = entities.len();
        entities.push(entity);

        // Add method_of edge if this is a method
        if let Some(class_name) = class_node.and_then(|c| c.child(1)).map(|c| &source[c.byte_range()]) {
            edges.push(json!({
                "type": "method_of",
                "source": entity_id,
                "target": class_name.to_string(),
            }));
        }
    }

    fn index_class(&self, node: tree_sitter::Node, source: &str, entities: &mut Vec<Value>, edges: &mut Vec<Value>) {
        let name_node = node.child(1); // Name is typically the second child
        let name = name_node
            .map(|n| &source[n.byte_range()])
            .unwrap_or("unknown")
            .to_string();

        // Check for base classes
        let mut base_classes = Vec::new();
        if let Some(ancestor_list) = node.child_by_field_name("base_class_clause") {
            for i in 0..ancestor_list.child_count() {
                if let Some(base_class) = ancestor_list.child(i) {
                    if base_class.kind() == "type_identifier" {
                        base_classes.push(source[base_class.byte_range()].to_string());
                    }
                }
            }
        }

        let entity_id = entities.len();
        entities.push(json!({
            "type": "class",
            "name": name,
            "base_classes": base_classes,
            "line": node.start_position().row + 1,
            "column": node.start_position().column + 1,
            "end_line": node.end_position().row + 1,
            "end_column": node.end_position().column + 1,
        }));

        // Add inheritance edges
        for base_class in &base_classes {
            edges.push(json!({
                "type": "inherits",
                "source": entity_id,
                "target": base_class,
            }));
        }
    }

    fn index_struct(&self, node: tree_sitter::Node, source: &str, entities: &mut Vec<Value>, _edges: &mut Vec<Value>) {
        let name_node = node.child(1); // Name is typically the second child
        let name = name_node
            .map(|n| &source[n.byte_range()])
            .unwrap_or("unknown")
            .to_string();

        let _entity_id = entities.len();
        entities.push(json!({
            "type": "struct",
            "name": name,
            "line": node.start_position().row + 1,
            "column": node.start_position().column + 1,
            "end_line": node.end_position().row + 1,
            "end_column": node.end_position().column + 1,
        }));
    }

    fn index_enum(&self, node: tree_sitter::Node, source: &str, entities: &mut Vec<Value>, _edges: &mut Vec<Value>) {
        let name_node = node.child(1); // Name is typically the second child
        let name = name_node
            .map(|n| &source[n.byte_range()])
            .unwrap_or("unknown")
            .to_string();

        let mut values = Vec::new();

        // Find the enum body
        if let Some(body) = node.child(2) {
            for i in 0..body.child_count() {
                if let Some(child) = body.child(i) {
                    if child.kind() == "enumerator" {
                        if let Some(value_node) = child.child(0) {
                            values.push(source[value_node.byte_range()].to_string());
                        }
                    }
                }
            }
        }

        entities.push(json!({
            "type": "enum",
            "name": name,
            "values": values,
            "line": node.start_position().row + 1,
            "column": node.start_position().column + 1,
            "end_line": node.end_position().row + 1,
            "end_column": node.end_position().column + 1,
        }));
    }

    fn index_typedef(&self, node: tree_sitter::Node, source: &str, entities: &mut Vec<Value>, _edges: &mut Vec<Value>) {
        let type_node = node.child_by_field_name("type");
        let name_node = node.child_by_field_name("declarator");

        let target_type = type_node
            .map(|n| &source[n.byte_range()])
            .unwrap_or("unknown")
            .to_string();

        let name = name_node
            .and_then(|n| n.child_by_field_name("declarator"))
            .map(|n| &source[n.byte_range()])
            .unwrap_or("unknown")
            .to_string();

        entities.push(json!({
            "type": "typedef",
            "name": name,
            "target_type": target_type,
            "line": node.start_position().row + 1,
            "column": node.start_position().column + 1,
            "end_line": node.end_position().row + 1,
            "end_column": node.end_position().column + 1,
        }));
    }

    fn index_namespace(&self, node: tree_sitter::Node, source: &str, entities: &mut Vec<Value>, edges: &mut Vec<Value>) {
        let name_node = node.child(1); // Name is typically the second child
        let name = name_node
            .map(|n| &source[n.byte_range()])
            .unwrap_or("unknown")
            .to_string();

        // Check if this is a nested namespace
        let parent_node = node.parent()
            .and_then(|p| if p.kind() == "namespace_definition" {
                Some(p)
            } else {
                None
            });

        let parent_name = parent_node
            .and_then(|p| p.child(1))
            .map(|n| &source[n.byte_range()])
            .map(|s| s.to_string());

        let _entity_id = entities.len();
        entities.push(json!({
            "type": "namespace",
            "name": name,
            "parent": parent_name,
            "line": node.start_position().row + 1,
            "column": node.start_position().column + 1,
            "end_line": node.end_position().row + 1,
            "end_column": node.end_position().column + 1,
        }));

        // Add namespace relationship for entities inside this namespace
        let mut namespace_entities = Vec::new();
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.kind() != "name" && child.kind() != "{" && child.kind() != "}" {
                    self.index_node_with_namespace(child, source, &mut namespace_entities, edges, &name);
                }
            }
        }

        entities.extend(namespace_entities);
    }

    fn index_node_with_namespace(&self, node: tree_sitter::Node, source: &str, entities: &mut Vec<Value>, edges: &mut Vec<Value>, namespace: &str) {
        // Index nodes within a namespace
        match node.kind() {
            "function_definition" => {
                let (mut func_entities, mut func_edges) = self.index_function_with_namespace(node, source, namespace);
                entities.append(&mut func_entities);
                edges.append(&mut func_edges);
            },
            "class_specifier" => {
                let (mut class_entities, mut class_edges) = self.index_class_with_namespace(node, source, namespace);
                entities.append(&mut class_entities);
                edges.append(&mut class_edges);
            },
            _ => {
                // Recurse into child nodes
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        self.index_node_with_namespace(child, source, entities, edges, namespace);
                    }
                }
            }
        }
    }

    fn index_function_with_namespace(&self, node: tree_sitter::Node, source: &str, namespace: &str) -> (Vec<Value>, Vec<Value>) {
        let mut entities = Vec::new();
        let mut edges = Vec::new();

        let name_node = node.child_by_field_name("declarator")
            .and_then(|n| n.child_by_field_name("declarator"))
            .and_then(|n| n.child_by_field_name("declarator"));

        let name = name_node
            .map(|n| &source[n.byte_range()])
            .unwrap_or("unknown")
            .to_string();

        let return_type_node = node.child_by_field_name("type");
        let return_type = return_type_node
            .map(|n| &source[n.byte_range()])
            .unwrap_or("void")
            .to_string();

        let entity_id = entities.len();
        entities.push(json!({
            "type": "function",
            "name": name,
            "return_type": return_type,
            "namespace": namespace,
            "line": node.start_position().row + 1,
            "column": node.start_position().column + 1,
            "end_line": node.end_position().row + 1,
            "end_column": node.end_position().column + 1,
        }));

        edges.push(json!({
            "type": "belongs_to_namespace",
            "source": entity_id,
            "target": namespace,
        }));

        (entities, edges)
    }

    fn index_class_with_namespace(&self, node: tree_sitter::Node, source: &str, namespace: &str) -> (Vec<Value>, Vec<Value>) {
        let mut entities = Vec::new();
        let mut edges = Vec::new();

        let name_node = node.child(1); // Name is typically the second child
        let name = name_node
            .map(|n| &source[n.byte_range()])
            .unwrap_or("unknown")
            .to_string();

        let entity_id = entities.len();
        entities.push(json!({
            "type": "class",
            "name": name,
            "namespace": namespace,
            "line": node.start_position().row + 1,
            "column": node.start_position().column + 1,
            "end_line": node.end_position().row + 1,
            "end_column": node.end_position().column + 1,
        }));

        edges.push(json!({
            "type": "belongs_to_namespace",
            "source": entity_id,
            "target": namespace,
        }));

        (entities, edges)
    }

    fn index_macro_definition(&self, node: tree_sitter::Node, source: &str, entities: &mut Vec<Value>, edges: &mut Vec<Value>) {
        let name_node = node.child(1); // Name is typically the second child
        let name = name_node
            .map(|n| &source[n.byte_range()])
            .unwrap_or("unknown")
            .to_string();

        let value_node = node.child(2);
        let replacement = value_node
            .map(|n| &source[n.byte_range()])
            .unwrap_or("")
            .to_string();

        let _entity_id = entities.len();
        entities.push(json!({
            "type": "macro",
            "name": name,
            "replacement": replacement,
            "is_function_like": false,
            "line": node.start_position().row + 1,
            "column": node.start_position().column + 1,
            "end_line": node.end_position().row + 1,
            "end_column": node.end_position().column + 1,
        }));

        edges.push(json!({
            "type": "defines_macro",
            "source": name,
        }));
    }

    fn index_function_macro(&self, node: tree_sitter::Node, source: &str, entities: &mut Vec<Value>, edges: &mut Vec<Value>) {
        let name_node = node.child(1); // Name is typically the second child
        let name = name_node
            .map(|n| &source[n.byte_range()])
            .unwrap_or("unknown")
            .to_string();

        // Extract parameters
        let mut parameters = Vec::new();
        if let Some(params_node) = node.child(2) {
            for i in 0..params_node.child_count() {
                if let Some(param) = params_node.child(i) {
                    if param.kind() == "identifier" {
                        parameters.push(source[param.byte_range()].to_string());
                    }
                }
            }
        }

        let value_node = node.child(3);
        let replacement = value_node
            .map(|n| &source[n.byte_range()])
            .unwrap_or("")
            .to_string();

        let _entity_id = entities.len();
        entities.push(json!({
            "type": "macro",
            "name": name,
            "parameters": parameters,
            "replacement": replacement,
            "is_function_like": true,
            "line": node.start_position().row + 1,
            "column": node.start_position().column + 1,
            "end_line": node.end_position().row + 1,
            "end_column": node.end_position().column + 1,
        }));

        edges.push(json!({
            "type": "defines_macro",
            "source": name,
        }));
    }

    fn index_include(&self, node: tree_sitter::Node, source: &str, entities: &mut Vec<Value>, edges: &mut Vec<Value>) {
        let path_node = node.child(1); // Path is typically the second child
        let path = path_node
            .map(|n| &source[n.byte_range()])
            .unwrap_or("")
            .trim_matches(|c| c == '"' || c == '<' || c == '>')
            .to_string();

        let include_type = if path.contains('<') {
            "system"
        } else {
            "local"
        };

        // Create a header entity
        let _entity_id = entities.len();
        entities.push(json!({
            "type": "header",
            "name": path,
            "include_type": include_type,
            "line": node.start_position().row + 1,
            "column": node.start_position().column + 1,
        }));

        edges.push(json!({
            "type": "includes",
            "source": path,
            "include_type": include_type,
        }));
    }

    fn index_function_call(&self, node: tree_sitter::Node, source: &str, entities: &mut Vec<Value>, edges: &mut Vec<Value>) {
        let function_node = node.child_by_field_name("function");
        let function_name = function_node
            .and_then(|n| {
                // Handle different types of function calls
                match n.kind() {
                    "identifier" => Some(source[n.byte_range()].to_string()),
                    "field_expression" => {
                        // Handle method calls like obj.method()
                        let field_node = n.child_by_field_name("field");
                        field_node.map(|f| source[f.byte_range()].to_string())
                    }
                    "template_function" => {
                        // Handle template function calls
                        let name_node = n.child(0);
                        name_node.map(|name| source[name.byte_range()].to_string())
                    }
                    _ => None
                }
            })
            .unwrap_or("unknown".to_string());

        // Determine if this is a static method call
        let is_static_method = function_node
            .map(|n| n.kind() == "field_expression" && source[n.byte_range()].contains("::"))
            .unwrap_or(false);

        // Determine the caller function
        let mut current = node.parent();
        let mut caller_name = "unknown".to_string();

        while let Some(parent) = current {
            if parent.kind() == "function_definition" {
                if let Some(declarator) = parent.child_by_field_name("declarator")
                    .and_then(|d| d.child_by_field_name("declarator"))
                    .and_then(|d| d.child_by_field_name("declarator")) {

                    caller_name = source[declarator.byte_range()].to_string();
                    break;
                }
            }
            current = parent.parent();
        }

        let call_type = if is_static_method { "static" } else { "instance" };

        edges.push(json!({
            "type": "calls",
            "source": caller_name,
            "target": function_name,
            "call_type": call_type,
        }));

        // Recurse into child nodes
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.index_node(child, source, entities, edges);
            }
        }
    }
}
