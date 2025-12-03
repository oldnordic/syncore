use crate::code_graph::CodeGraph;
use crate::vector::VectorStore;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Request to index a directory of code files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryIndexRequest {
    /// Path to the directory to index
    pub directory: String,
    /// Glob pattern to match files (e.g., "**/*.rs", "src/**/*.py")
    pub pattern: String,
}

/// Response containing indexing results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryIndexResponse {
    /// Number of files successfully indexed
    pub files_indexed: usize,
    /// Total number of entities (functions, classes, etc.) found
    pub total_entities: usize,
    /// Whether the operation completed successfully
    pub success: bool,
    /// Optional error message if not successful
    pub error: Option<String>,
}

/// Directory indexer that uses CodeGraph to index multiple files
pub struct DirectoryIndexer {
    code_graph: CodeGraph,
}

impl DirectoryIndexer {
    /// Create a new directory indexer with database path and vector store
    pub fn new(db_path: &str, vector_store: Arc<Mutex<VectorStore>>) -> Result<Self> {
        let code_graph = CodeGraph::new(db_path, vector_store)?;
        Ok(Self {
            code_graph,
        })
    }

    /// Index all files in a directory matching the given pattern
    ///
    /// Pattern examples:
    /// - "**/*.rs" - All Rust files recursively
    /// - "src/**/*.py" - All Python files under src/ recursively
    /// - "*.js" - All JavaScript files in root directory only
    pub fn index_directory(
        &mut self,
        request: &DirectoryIndexRequest,
    ) -> Result<DirectoryIndexResponse> {
        let directory = Path::new(&request.directory);

        // Verify directory exists
        if !directory.exists() {
            return Ok(DirectoryIndexResponse {
                files_indexed: 0,
                total_entities: 0,
                success: false,
                error: Some(format!("Directory not found: {}", request.directory)),
            });
        }

        if !directory.is_dir() {
            return Ok(DirectoryIndexResponse {
                files_indexed: 0,
                total_entities: 0,
                success: false,
                error: Some(format!("Path is not a directory: {}", request.directory)),
            });
        }

        // Build glob pattern matcher
        use glob::Pattern;
        let pattern = Pattern::new(&request.pattern)
            .map_err(|e| anyhow!("Invalid glob pattern '{}': {}", request.pattern, e))?;

        // Traverse directory and collect matching files
        use walkdir::WalkDir;
        let mut files_indexed = 0;
        let mut total_entities = 0;

        for entry in WalkDir::new(directory).follow_links(false) {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    // Log error but continue processing other files
                    eprintln!("Warning: Failed to read directory entry: {}", e);
                    continue;
                }
            };

            // Skip directories, only process files
            if !entry.file_type().is_file() {
                continue;
            }

            let file_path = entry.path();

            // Get path relative to directory for pattern matching
            let relative_path = match file_path.strip_prefix(directory) {
                Ok(p) => p,
                Err(_) => continue, // Skip if can't get relative path
            };

            // Check if file matches pattern
            if !pattern.matches_path(relative_path) {
                continue;
            }

            // Index the file
            match self.code_graph.index_file(file_path) {
                Ok(entity_count) => {
                    files_indexed += 1;
                    total_entities += entity_count;
                }
                Err(e) => {
                    // Log error but continue processing other files
                    eprintln!("Warning: Failed to index file {}: {}", file_path.display(), e);
                }
            }
        }

        Ok(DirectoryIndexResponse {
            files_indexed,
            total_entities,
            success: true,
            error: None,
        })
    }

    /// Get the underlying code graph (for testing)
    #[cfg(test)]
    pub fn code_graph(&self) -> &CodeGraph {
        &self.code_graph
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector::{HuggingFaceEmbeddings, VectorStore};
    use std::fs;
    use tempfile::TempDir;

    fn create_test_indexer(db_path: &str) -> Result<DirectoryIndexer> {
        let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
        let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
        DirectoryIndexer::new(db_path, vector_store)
    }

    #[test]
    fn test_request_serialization() {
        let request = DirectoryIndexRequest {
            directory: "/path/to/code".to_string(),
            pattern: "**/*.rs".to_string(),
        };

        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: DirectoryIndexRequest = serde_json::from_str(&serialized).unwrap();

        assert_eq!(request.directory, deserialized.directory);
        assert_eq!(request.pattern, deserialized.pattern);
    }

    #[test]
    fn test_response_serialization() {
        let response = DirectoryIndexResponse {
            files_indexed: 42,
            total_entities: 150,
            success: true,
            error: None,
        };

        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: DirectoryIndexResponse = serde_json::from_str(&serialized).unwrap();

        assert_eq!(response.files_indexed, deserialized.files_indexed);
        assert_eq!(response.total_entities, deserialized.total_entities);
        assert_eq!(response.success, deserialized.success);
    }

    #[test]
    fn test_indexer_creation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");

        let _indexer = create_test_indexer(db_path.to_str().unwrap())?;

        // Verify database file was created
        assert!(db_path.exists());

        Ok(())
    }

    #[test]
    fn test_nonexistent_directory() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let mut indexer = create_test_indexer(db_path.to_str().unwrap())?;

        let request = DirectoryIndexRequest {
            directory: "/nonexistent/directory".to_string(),
            pattern: "**/*.rs".to_string(),
        };

        let response = indexer.index_directory(&request)?;

        assert!(!response.success);
        assert_eq!(response.files_indexed, 0);
        assert!(response.error.is_some());
        assert!(response.error.unwrap().contains("not found"));

        Ok(())
    }

    #[test]
    fn test_file_path_instead_of_directory() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");

        // Create a test file
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content")?;

        let mut indexer = create_test_indexer(db_path.to_str().unwrap())?;

        let request = DirectoryIndexRequest {
            directory: test_file.to_str().unwrap().to_string(),
            pattern: "**/*.rs".to_string(),
        };

        let response = indexer.index_directory(&request)?;

        assert!(!response.success);
        assert_eq!(response.files_indexed, 0);
        assert!(response.error.is_some());
        assert!(response.error.unwrap().contains("not a directory"));

        Ok(())
    }

    #[test]
    #[ignore] // Only run when implementation is complete
    fn test_index_rust_files() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");

        // Create test directory structure
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir)?;

        // Create test Rust file
        let rust_file = src_dir.join("test.rs");
        fs::write(
            &rust_file,
            r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn multiply(x: i32, y: i32) -> i32 {
    x * y
}
"#,
        )?;

        // Create another Rust file
        let rust_file2 = src_dir.join("helper.rs");
        fs::write(
            &rust_file2,
            r#"
pub fn helper() {
    println!("Helper");
}
"#,
        )?;

        let mut indexer = create_test_indexer(db_path.to_str().unwrap())?;

        let request = DirectoryIndexRequest {
            directory: temp_dir.path().to_str().unwrap().to_string(),
            pattern: "**/*.rs".to_string(),
        };

        let response = indexer.index_directory(&request)?;

        assert!(response.success);
        assert_eq!(response.files_indexed, 2);
        assert!(response.total_entities >= 3); // At least add, multiply, helper functions
        assert!(response.error.is_none());

        Ok(())
    }

    #[test]
    #[ignore] // Only run when implementation is complete
    fn test_pattern_matching() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");

        // Create test files
        fs::write(temp_dir.path().join("test.rs"), "pub fn rust_func() {}")?;
        fs::write(temp_dir.path().join("test.py"), "def python_func(): pass")?;
        fs::write(temp_dir.path().join("test.js"), "function js_func() {}")?;

        let mut indexer = create_test_indexer(db_path.to_str().unwrap())?;

        // Test indexing only Python files
        let request = DirectoryIndexRequest {
            directory: temp_dir.path().to_str().unwrap().to_string(),
            pattern: "*.py".to_string(),
        };

        let response = indexer.index_directory(&request)?;

        assert!(response.success);
        assert_eq!(response.files_indexed, 1); // Only test.py

        Ok(())
    }

    #[test]
    #[ignore] // Only run when implementation is complete
    fn test_nested_directory_indexing() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");

        // Create nested structure
        let src_dir = temp_dir.path().join("src");
        let utils_dir = src_dir.join("utils");
        fs::create_dir_all(&utils_dir)?;

        fs::write(src_dir.join("main.rs"), "fn main() {}")?;
        fs::write(utils_dir.join("helper.rs"), "pub fn helper() {}")?;

        let mut indexer = create_test_indexer(db_path.to_str().unwrap())?;

        let request = DirectoryIndexRequest {
            directory: temp_dir.path().to_str().unwrap().to_string(),
            pattern: "**/*.rs".to_string(),
        };

        let response = indexer.index_directory(&request)?;

        assert!(response.success);
        assert_eq!(response.files_indexed, 2); // Both main.rs and helper.rs

        Ok(())
    }
}
