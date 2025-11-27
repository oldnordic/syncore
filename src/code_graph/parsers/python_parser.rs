//! Python Language Parser Implementation
//!
//! Extracts CodeEntity and CodeEdge structures from Python source code
//! using tree-sitter grammar and existing extraction patterns.

use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

use super::super::language_parser::LanguageParser;
use super::super::types::{CodeEdge, CodeEntity, EdgeType, EntityType};
use crate::parser::{ClassInfo, FunctionInfo, ImportInfo, Parser};

/// Strip Python docstring delimiters (triple quotes) from a docstring
fn strip_python_docstring(docstring: Option<String>) -> Option<String> {
    docstring.map(|s| {
        let s = s.trim();
        // Handle """ or ''' style docstrings
        if ((s.starts_with("\"\"\"") && s.ends_with("\"\"\""))
            || (s.starts_with("'''") && s.ends_with("'''")))
            && s.len() >= 6
        {
            return s[3..s.len() - 3].trim().to_string();
        }
        // Handle single-line # comments
        if let Some(stripped) = s.strip_prefix('#') {
            return stripped.trim().to_string();
        }
        s.to_string()
    })
}

/// Python language parser using tree-sitter
pub struct PythonLanguageParser {
    parser: Parser,
}

impl PythonLanguageParser {
    /// Create new Python language parser
    pub fn new() -> Result<Self> {
        let parser = Parser::new()?;
        Ok(Self { parser })
    }

    /// Convert FunctionInfo to CodeEntity
    fn function_to_entity(&self, func: &FunctionInfo, file_path: &str) -> CodeEntity {
        CodeEntity::new(
            file_path.to_string(),
            EntityType::Function,
            func.name.clone(),
            Some(format_function_signature(func)),
            func.line_number,
            func.end_line,
            strip_python_docstring(func.docstring.clone()),
            "python".to_string(),
        )
    }

    /// Convert ClassInfo to CodeEntity
    fn class_to_entity(&self, class: &ClassInfo, file_path: &str) -> CodeEntity {
        CodeEntity::new(
            file_path.to_string(),
            EntityType::Class,
            class.name.clone(),
            None,
            class.line_number,
            class.line_number,
            strip_python_docstring(class.docstring.clone()),
            "python".to_string(),
        )
    }

    /// Convert method to CodeEntity
    fn method_to_entity(
        &self,
        method: &FunctionInfo,
        class_name: &str,
        file_path: &str,
    ) -> CodeEntity {
        CodeEntity::new(
            file_path.to_string(),
            EntityType::Method,
            format!("{}.{}", class_name, method.name),
            Some(format_function_signature(method)),
            method.line_number,
            method.line_number,
            strip_python_docstring(method.docstring.clone()),
            "python".to_string(),
        )
    }

    /// Convert ImportInfo to CodeEntity
    fn import_to_entity(&self, import: &ImportInfo, file_path: &str) -> CodeEntity {
        CodeEntity::new(
            file_path.to_string(),
            EntityType::Import,
            import.module.clone(),
            import.alias.clone(),
            import.line_number,
            import.line_number,
            None,
            "python".to_string(),
        )
    }

    /// Extract function calls from source code for edge creation
    fn extract_function_calls(&self, source: &str, entities: &[CodeEntity]) -> Vec<CodeEdge> {
        let mut edges = Vec::new();
        let mut name_to_id: HashMap<String, i64> = HashMap::new();

        // Build entity name to ID mapping (using line number as temporary ID)
        for (idx, entity) in entities.iter().enumerate() {
            name_to_id.insert(entity.name.clone(), idx as i64);
        }

        // Simple regex-based function call detection
        // In production, this would use tree-sitter AST traversal
        let lines: Vec<&str> = source.lines().collect();
        for (line_num, line) in lines.iter().enumerate() {
            // Look for function calls: function_name(
            for (entity_name, &entity_id) in &name_to_id {
                if line.contains(&format!("{}(", entity_name)) {
                    // Find calling function by looking backwards for function definition
                    for calling_entity in entities {
                        if (calling_entity.entity_type == EntityType::Function
                            || calling_entity.entity_type == EntityType::Method)
                            && (line_num + 1 >= calling_entity.line_start
                                && line_num + 1 <= calling_entity.line_end)
                        {
                            edges.push(CodeEdge {
                                src_entity_id: name_to_id[&calling_entity.name.clone()],
                                dst_entity_id: entity_id,
                                edge_type: EdgeType::Calls,
                            });
                            break;
                        }
                    }
                }
            }
        }

        edges
    }
}

impl LanguageParser for PythonLanguageParser {
    fn supports(&self, file_path: &Path) -> bool {
        file_path.extension().map_or(false, |ext| ext == "py")
    }

    fn parse_entities(&self, file_path: &Path) -> Result<Vec<CodeEntity>> {
        let code_structure = self.parser.parse_file(file_path)?;
        let file_path_str = file_path.to_string_lossy().to_string();
        let mut entities = Vec::new();

        // Convert functions
        for func in &code_structure.functions {
            entities.push(self.function_to_entity(func, &file_path_str));
        }

        // Convert classes
        for class in &code_structure.classes {
            entities.push(self.class_to_entity(class, &file_path_str));

            // Convert methods
            for method in &class.methods {
                entities.push(self.method_to_entity(method, &class.name, &file_path_str));
            }
        }

        // Convert imports
        for import in &code_structure.imports {
            entities.push(self.import_to_entity(import, &file_path_str));
        }

        Ok(entities)
    }

    fn parse_edges(&self, file_path: &Path) -> Result<Vec<CodeEdge>> {
        let entities = self.parse_entities(file_path)?;
        let source_code = std::fs::read_to_string(file_path)?;

        let mut edges = self.extract_function_calls(&source_code, &entities);

        // Add import edges
        for import_entity in &entities {
            if import_entity.entity_type == EntityType::Import {
                for user_entity in &entities {
                    if user_entity.entity_type == EntityType::Function
                        || user_entity.entity_type == EntityType::Method
                    {
                        // Simple heuristic: if import appears before function, assume usage
                        if import_entity.line_start < user_entity.line_start {
                            edges.push(CodeEdge {
                                src_entity_id: user_entity.id.unwrap_or(0),
                                dst_entity_id: import_entity.id.unwrap_or(0),
                                edge_type: EdgeType::Imports,
                            });
                        }
                    }
                }
            }
        }

        Ok(edges)
    }
}

/// Format function signature from FunctionInfo
fn format_function_signature(func: &FunctionInfo) -> String {
    let params = func.parameters.join(", ");
    let return_type = func.return_type.as_deref().unwrap_or("");
    if return_type.is_empty() {
        format!("{}({})", func.name, params)
    } else {
        format!("{}({}) -> {}", func.name, params, return_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_python_parser_supports_py_files() {
        let parser = PythonLanguageParser::new().unwrap();
        assert!(parser.supports(Path::new("test.py")));
        assert!(!parser.supports(Path::new("test.rs")));
        assert!(!parser.supports(Path::new("test.js")));
    }

    #[test]
    fn test_parse_python_function() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.py");
        let python_code = r#"
def add(a, b):
    """Add two numbers."""
    return a + b
"#;
        fs::write(&file_path, python_code)?;

        let parser = PythonLanguageParser::new()?;
        let entities = parser.parse_entities(&file_path)?;

        assert_eq!(entities.len(), 1);
        let entity = &entities[0];
        assert_eq!(entity.name, "add");
        assert_eq!(entity.entity_type, EntityType::Function);
        assert_eq!(entity.language, "python");
        assert!(entity.signature.as_ref().unwrap().contains("add(a, b)"));
        assert_eq!(entity.docstring, Some("Add two numbers.".to_string()));

        Ok(())
    }

    #[test]
    fn test_parse_python_class() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.py");
        let python_code = r#"
class TestClass:
    """A test class."""
    
    def __init__(self, value):
        self.value = value
    
    def get_value(self):
        return self.value
"#;
        fs::write(&file_path, python_code)?;

        let parser = PythonLanguageParser::new()?;
        let entities = parser.parse_entities(&file_path)?;

        // Should have class and 2 methods
        assert_eq!(entities.len(), 3);

        let class_entity = entities
            .iter()
            .find(|e| e.name == "TestClass" && e.entity_type == EntityType::Class)
            .unwrap();
        assert_eq!(class_entity.line_start, 2);
        assert_eq!(class_entity.docstring, Some("A test class.".to_string()));

        let method_entity = entities
            .iter()
            .find(|e| e.name == "TestClass.__init__" && e.entity_type == EntityType::Method)
            .unwrap();
        assert!(method_entity
            .signature
            .as_ref()
            .unwrap()
            .contains("__init__(self, value)"));

        Ok(())
    }

    #[test]
    fn test_parse_python_import() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.py");
        let python_code = r#"
import os
import sys as system
from collections import defaultdict
from typing import List, Dict
"#;
        fs::write(&file_path, python_code)?;

        let parser = PythonLanguageParser::new()?;
        let entities = parser.parse_entities(&file_path)?;

        // Should have 4 import entities
        let import_entities: Vec<_> = entities
            .iter()
            .filter(|e| e.entity_type == EntityType::Import)
            .collect();
        assert_eq!(import_entities.len(), 4);

        let os_import = import_entities.iter().find(|e| e.name == "os").unwrap();
        assert_eq!(os_import.line_start, 2);

        let sys_import = import_entities.iter().find(|e| e.name == "sys").unwrap();
        assert_eq!(sys_import.signature, Some("system".to_string()));

        Ok(())
    }

    #[test]
    fn test_parse_edges() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.py");
        let python_code = r#"
import os

def main():
    path = os.path.join("dir", "file")
    print("Hello")
"#;
        fs::write(&file_path, python_code)?;

        let parser = PythonLanguageParser::new()?;
        let edges = parser.parse_edges(&file_path)?;

        // Should have at least import edges
        assert!(!edges.is_empty());

        Ok(())
    }

    #[test]
    fn test_parse_function_with_return_type() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.py");
        let python_code = r#"
def calculate(x: int, y: int) -> int:
    return x * y
"#;
        fs::write(&file_path, python_code)?;

        let parser = PythonLanguageParser::new()?;
        let entities = parser.parse_entities(&file_path)?;

        assert_eq!(entities.len(), 1);
        let entity = &entities[0];
        assert!(entity.signature.as_ref().unwrap().contains("-> int"));

        Ok(())
    }
}
