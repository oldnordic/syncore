//! Index Application for Multilanguage Code Analysis
//!
//! Provides unified interface for indexing multiple programming languages
//! using pluggable language parsers and existing CodeGraph infrastructure.

use anyhow::{anyhow, Result};
use std::path::Path;

use super::graph::CodeGraph;
use super::language_parser::LanguageParser;
use super::parsers::python_parser::PythonLanguageParser;

/// Index application that manages multiple language parsers
pub struct IndexApplication {
    code_graph: CodeGraph,
    python_parser: PythonLanguageParser,
}

impl IndexApplication {
    /// Create new index application with database path and vector store
    ///
    /// # Arguments
    /// * `db_path` - Path to SQLite database
    /// * `vector_store` - Vector store for embeddings
    ///
    /// # Returns
    /// IndexApplication instance or error
    pub fn new(
        db_path: &str,
        vector_store: std::sync::Arc<std::sync::Mutex<crate::vector::VectorStore>>,
    ) -> Result<Self> {
        let code_graph = CodeGraph::new(db_path, vector_store)?;
        let python_parser = PythonLanguageParser::new()?;

        Ok(Self {
            code_graph,
            python_parser,
        })
    }

    /// Index a single file using appropriate language parser
    ///
    /// # Arguments
    /// * `file_path` - Path to the file to index
    ///
    /// # Returns
    /// Number of entities indexed or error
    ///
    /// # Language Detection
    /// - `.rs` files -> Rust parser
    /// - `.py` files -> Python parser
    /// - Other extensions -> Error
    pub fn index_file(&mut self, file_path: &Path) -> Result<usize> {
        // Detect language by file extension
        let language = self.detect_language(file_path)?;

        match language.as_str() {
            "rust" => {
                // Use existing Rust indexing pipeline via CodeGraph::index_file()
                self.code_graph.index_file(file_path)
            }
            "python" => {
                // Use new Python parser for indexing
                self.index_python_file(file_path)
            }
            _ => Err(anyhow!("Unsupported language: {}", language)),
        }
    }

    /// Index multiple files using appropriate parsers
    ///
    /// # Arguments
    /// * `file_paths` - Iterator of file paths to index
    ///
    /// # Returns
    /// Total number of entities indexed across all files
    pub fn index_files<I>(&mut self, file_paths: I) -> Result<usize>
    where
        I: IntoIterator<Item: AsRef<Path>>,
    {
        let mut total_entities = 0;
        for file_path in file_paths {
            match self.index_file(file_path.as_ref()) {
                Ok(count) => total_entities += count,
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to index file {}: {}",
                        file_path.as_ref().display(),
                        e
                    );
                    // Continue with other files
                }
            }
        }
        Ok(total_entities)
    }

    /// Detect programming language from file extension
    ///
    /// # Arguments
    /// * `file_path` - Path to the file
    ///
    /// # Returns
    /// Language string or error if unsupported
    pub fn detect_language(&self, file_path: &Path) -> Result<String> {
        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| anyhow!("No file extension found"))?;

        match extension {
            "rs" => Ok("rust".to_string()),
            "py" => Ok("python".to_string()),
            _ => Err(anyhow!("Unsupported file extension: {}", extension)),
        }
    }

    /// Index Python file using Python parser and CodeGraph persistence
    ///
    /// # Arguments
    /// * `file_path` - Path to Python file
    ///
    /// # Returns
    /// Number of entities indexed
    fn index_python_file(&mut self, file_path: &Path) -> Result<usize> {
        // Extract entities using Python parser
        let entities = self.python_parser.parse_entities(file_path)?;

        // Extract edges using Python parser
        let edges = self.python_parser.parse_edges(file_path)?;

        // Store entities and edges using existing CodeGraph infrastructure
        // This reuses the same persistence logic as Rust indexing
        self.store_entities_and_edges(file_path, entities, edges)
    }

    /// Store entities and edges in database using CodeGraph methods
    ///
    /// # Arguments
    /// * `file_path` - Path to source file
    /// * `entities` - Vector of CodeEntity structs
    /// * `edges` - Vector of CodeEdge structs
    ///
    /// # Returns
    /// Number of entities stored
    fn store_entities_and_edges(
        &mut self,
        file_path: &Path,
        entities: Vec<super::types::CodeEntity>,
        edges: Vec<super::types::CodeEdge>,
    ) -> Result<usize> {
        let file_path_str = file_path
            .to_str()
            .ok_or_else(|| anyhow!("Invalid file path"))?
            .to_string();

        let mut db = self
            .code_graph
            .db
            .lock()
            .map_err(|e| anyhow!("Failed to lock database: {}", e))?;

        // Use transaction for atomic operations
        let tx = db.transaction()?;

        // Delete existing entities for this file to allow re-indexing
        tx.execute(
            "DELETE FROM code_entities WHERE file_path = ?",
            [&file_path_str],
        )?;

        // Store entities and collect IDs
        let mut entity_ids = Vec::new();
        let mut entities_indexed = 0;

        for entity in entities {
            let entity_id = self.store_entity_internal(&tx, &entity)?;
            entity_ids.push((entity_id, entity.name.clone()));
            entities_indexed += 1;
        }

        // Store edges using entity IDs
        for edge in edges {
            // Find entity IDs by name (simplified approach)
            let src_id = entity_ids
                .iter()
                .find(|(_, name)| *name == edge.src_entity_id.to_string())
                .map(|(id, _)| *id)
                .unwrap_or(0);

            let dst_id = entity_ids
                .iter()
                .find(|(_, name)| *name == edge.dst_entity_id.to_string())
                .map(|(id, _)| *id)
                .unwrap_or(0);

            if src_id > 0 && dst_id > 0 {
                self.store_edge_internal(&tx, src_id, dst_id, edge.edge_type)?;
            }
        }

        // Commit transaction
        tx.commit()?;

        // Create embeddings for entities (best-effort)
        for (entity_id, entity_name) in entity_ids {
            if let Err(e) = self.create_entity_embedding(&db, entity_id, &entity_name) {
                eprintln!(
                    "[WARN] Failed to create embedding for entity {}: {}",
                    entity_name, e
                );
            }
        }

        Ok(entities_indexed)
    }

    /// Store entity in database (internal method)
    fn store_entity_internal(
        &self,
        db: &rusqlite::Connection,
        entity: &super::types::CodeEntity,
    ) -> Result<i64> {
        db.execute(
            "INSERT INTO code_entities
             (file_path, entity_type, name, signature, line_start, line_end, docstring, language, indexed_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                &entity.file_path,
                entity.entity_type.as_str(),
                &entity.name,
                &entity.signature,
                entity.line_start as i64,
                entity.line_end as i64,
                &entity.docstring,
                &entity.language,
                chrono::Utc::now().timestamp(),
            ],
        )?;

        Ok(db.last_insert_rowid())
    }

    /// Store edge in database (internal method)
    fn store_edge_internal(
        &self,
        db: &rusqlite::Connection,
        src_id: i64,
        dst_id: i64,
        edge_type: super::types::EdgeType,
    ) -> Result<()> {
        db.execute(
            "INSERT OR IGNORE INTO code_edges (src_entity_id, dst_entity_id, edge_type)
             VALUES (?, ?, ?)",
            rusqlite::params![src_id, dst_id, edge_type.as_str()],
        )?;
        Ok(())
    }

    /// Create embedding for entity (internal method)
    fn create_entity_embedding(
        &self,
        db: &rusqlite::Connection,
        entity_id: i64,
        entity_name: &str,
    ) -> Result<()> {
        // Create text representation for embedding
        let text = format!("{}: {}", "entity", entity_name);

        // Store in vector store
        {
            let mut vector_store = self
                .code_graph
                .vector_store
                .lock()
                .map_err(|e| anyhow!("Failed to lock vector store: {}", e))?;

            vector_store.insert_text(entity_id, None, &text, "code_entity")?;
        }

        // Link embedding to entity
        db.execute(
            "INSERT INTO code_embeddings (entity_id, vector_id, model_version, created_at)
             VALUES (?, ?, ?, ?)",
            rusqlite::params![
                entity_id,
                entity_id,
                "all-MiniLM-L6-v2",
                chrono::Utc::now().timestamp(),
            ],
        )?;

        Ok(())
    }

    /// Get reference to underlying CodeGraph for advanced operations
    pub fn code_graph(&self) -> &CodeGraph {
        &self.code_graph
    }

    /// Get mutable reference to underlying CodeGraph for advanced operations
    pub fn code_graph_mut(&mut self) -> &mut CodeGraph {
        &mut self.code_graph
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector::{HuggingFaceEmbeddings, VectorStore};
    use std::fs;
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    fn create_test_index_app(db_path: &str) -> Result<IndexApplication> {
        let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
        let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
        IndexApplication::new(db_path, vector_store)
    }

    #[test]
    fn test_language_detection() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let app = create_test_index_app(db_path.to_str().unwrap())?;

        assert_eq!(app.detect_language(Path::new("test.rs"))?, "rust");
        assert_eq!(app.detect_language(Path::new("test.py"))?, "python");

        assert!(app.detect_language(Path::new("test.js")).is_err());
        assert!(app.detect_language(Path::new("test")).is_err());

        Ok(())
    }

    #[test]
    fn test_index_rust_file() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let mut app = create_test_index_app(db_path.to_str().unwrap())?;

        // Create test Rust file
        let rust_file = temp_dir.path().join("test.rs");
        fs::write(&rust_file, "pub fn test() { println!(\"Hello\"); }")?;

        let count = app.index_file(&rust_file)?;
        assert!(count >= 1); // Should index at least the function

        Ok(())
    }

    #[test]
    fn test_index_python_file() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let mut app = create_test_index_app(db_path.to_str().unwrap())?;

        // Create test Python file
        let py_file = temp_dir.path().join("test.py");
        fs::write(&py_file, "def test():\n    print(\"Hello\")")?;

        let count = app.index_file(&py_file)?;
        assert!(count >= 1); // Should index at least the function

        Ok(())
    }

    #[test]
    fn test_index_unsupported_file() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let mut app = create_test_index_app(db_path.to_str().unwrap())?;

        // Create test JavaScript file (unsupported)
        let js_file = temp_dir.path().join("test.js");
        fs::write(&js_file, "function test() { console.log(\"Hello\"); }")?;

        let result = app.index_file(&js_file);
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_index_multiple_files() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let mut app = create_test_index_app(db_path.to_str().unwrap())?;

        // Create test files
        let rust_file = temp_dir.path().join("test.rs");
        fs::write(&rust_file, "pub fn rust_func() {}")?;

        let py_file = temp_dir.path().join("test.py");
        fs::write(&py_file, "def python_func(): pass")?;

        let files = vec![&rust_file, &py_file];
        let total_count = app.index_files(&files)?;
        assert!(total_count >= 2); // Should index at least one function per file

        Ok(())
    }
}
