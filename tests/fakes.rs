//! Test Acceleration Layer - Fake implementations for heavy components
//!
//! This module provides zero-cost test doubles that replace heavyweight
//! production components (embeddings, vector stores, parsers) during testing.
//!
//! ## Design Principles
//! 1. No ML model loading
//! 2. No tree-sitter parsing
//! 3. No expensive I/O operations
//! 4. Maintain same interfaces as production
//! 5. Enable fast TDD feedback loops

use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use syncore::parser::{ClassInfo, CodeStructure, FunctionInfo, ImportInfo};
use syncore::vector::{Embeddings, Hit, SearchScope};

/// Fake embeddings that return fixed dummy vectors without ML models
pub struct FakeEmbeddings {
    dim: usize,
}

impl FakeEmbeddings {
    pub fn new(dim: usize) -> Result<Self> {
        Ok(Self {
            dim,
        })
    }
}

impl Embeddings for FakeEmbeddings {
    fn embed(&self, _text: &str) -> Result<Vec<f32>> {
        // Return zero vector - no ML computation
        Ok(vec![0.0; self.dim])
    }

    fn dim(&self) -> usize {
        self.dim
    }

    fn model_name(&self) -> &str {
        "fake-embeddings-test"
    }
}

/// Fake vector store using in-memory HashMap (no SQLite, no FAISS)
pub struct FakeVectorStore {
    vectors: Arc<Mutex<HashMap<i64, (String, Vec<f32>)>>>,
    next_id: Arc<Mutex<i64>>,
    dim: usize,
}

impl FakeVectorStore {
    pub fn new(dim: usize) -> Self {
        Self {
            vectors: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
            dim,
        }
    }

    pub fn insert(&mut self, text: String, vector: Vec<f32>) -> Result<i64> {
        let mut next_id = self.next_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;
        drop(next_id);

        let mut vectors = self.vectors.lock().unwrap();
        vectors.insert(id, (text, vector));
        Ok(id)
    }

    pub fn search(&self, _query: &str, limit: usize, _scope: SearchScope) -> Result<Vec<Hit>> {
        // Return dummy results - no actual similarity computation
        let vectors = self.vectors.lock().unwrap();
        let results: Vec<Hit> = vectors
            .iter()
            .take(limit)
            .map(|(id, (text, _vec))| Hit {
                id: *id,
                score: 0.5, // Dummy score
                task_id: None,
                text: text.clone(),
            })
            .collect();
        Ok(results)
    }

    pub fn count(&self) -> Result<usize> {
        let vectors = self.vectors.lock().unwrap();
        Ok(vectors.len())
    }
}

/// Fake parser that returns trivial code structure without tree-sitter
pub struct FakeParser;

impl FakeParser {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    pub fn parse_file(&mut self, file_path: &std::path::Path) -> Result<CodeStructure> {
        // Return minimal structure without parsing
        let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("test.rs");

        Ok(CodeStructure {
            file_path: file_path.to_string_lossy().to_string(),
            language: if file_name.ends_with(".rs") {
                "rust"
            } else if file_name.ends_with(".js") {
                "javascript"
            } else {
                "unknown"
            }
            .to_string(),
            functions: vec![FunctionInfo {
                name: "test_function".to_string(),
                line_number: 1,
                end_line: 5,
                parameters: vec![],
                return_type: None,
                docstring: None,
                visibility: Some("pub".to_string()),
            }],
            classes: vec![ClassInfo {
                name: "TestClass".to_string(),
                line_number: 7,
                methods: vec![],
                fields: vec![],
                docstring: None,
                class_type: "class".to_string(),
            }],
            imports: vec![ImportInfo {
                module: "std::collections::HashMap".to_string(),
                alias: None,
                line_number: 1,
                import_type: "use".to_string(),
            }],
            variables: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fake_embeddings_zero_cost() {
        let embeddings = FakeEmbeddings::new(8).unwrap();
        let vec = embeddings.embed("test").unwrap();
        assert_eq!(vec.len(), 8);
        assert_eq!(vec, vec![0.0; 8]);
    }

    #[test]
    fn test_fake_vector_store_in_memory() {
        let mut store = FakeVectorStore::new(8);
        let id = store.insert("test".to_string(), vec![0.0; 8]).unwrap();
        assert_eq!(id, 1);
        assert_eq!(store.count().unwrap(), 1);
    }

    #[test]
    fn test_fake_parser_trivial_output() {
        let mut parser = FakeParser::new().unwrap();
        let path = std::path::Path::new("/tmp/test.rs");
        let structure = parser.parse_file(&path).unwrap();
        assert_eq!(structure.functions.len(), 1);
        assert_eq!(structure.functions[0].name, "test_function");
        assert_eq!(structure.classes.len(), 1);
        assert_eq!(structure.classes[0].name, "TestClass");
    }
}
