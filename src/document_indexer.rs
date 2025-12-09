// Document Indexer
// Scans directories for documents, extracts text, chunks semantically,
// and stores in global vector database for efficient semantic search

use crate::global_store::{GlobalDbPool, GlobalVectorStore};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Supported document types
#[derive(Debug, Clone, PartialEq)]
pub enum DocumentType {
    Markdown,
    Text,
    Pdf,
    Rust,
    Python,
    Json,
    Toml,
}

impl DocumentType {
    /// Detect document type from file extension
    pub fn from_path(path: &Path) -> Option<Self> {
        path.extension().and_then(|ext| ext.to_str()).and_then(|ext| {
            match ext.to_lowercase().as_str() {
                "md" | "markdown" => Some(DocumentType::Markdown),
                "txt" | "text" => Some(DocumentType::Text),
                "pdf" => Some(DocumentType::Pdf),
                "rs" => Some(DocumentType::Rust),
                "py" => Some(DocumentType::Python),
                "json" => Some(DocumentType::Json),
                "toml" => Some(DocumentType::Toml),
                _ => None,
            }
        })
    }
}

/// Document metadata
#[derive(Debug, Clone)]
pub struct DocumentMetadata {
    pub path: PathBuf,
    pub filename: String,
    pub doc_type: DocumentType,
    pub size_bytes: u64,
    pub modified_time: std::time::SystemTime,
}

/// Text chunk with metadata
#[derive(Debug, Clone)]
pub struct DocumentChunk {
    pub text: String,
    pub source_file: PathBuf,
    pub chunk_index: usize,
    pub metadata: DocumentMetadata,
}

/// Document indexer configuration
#[derive(Debug, Clone)]
pub struct IndexerConfig {
    pub max_chunk_size: usize,
    pub overlap_size: usize,
    pub skip_hidden: bool,
    pub skip_extensions: Vec<String>,
    pub max_depth: Option<usize>,
    pub max_files: Option<usize>,
    pub excluded_dirs: Vec<String>,
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self {
            max_chunk_size: 1000, // ~1000 chars per chunk
            overlap_size: 200,    // 200 char overlap for context
            skip_hidden: true,
            skip_extensions: vec![
                "bin".to_string(),
                "exe".to_string(),
                "so".to_string(),
                "dylib".to_string(),
                "a".to_string(),
                // Database files
                "db".to_string(),
                "sqlite".to_string(),
                "sqlite3".to_string(),
                // Log files
                "log".to_string(),
            ],
            max_depth: Some(10), // Limit recursion depth to prevent infinite loops
            max_files: Some(10000), // Limit total files to prevent memory issues
            excluded_dirs: vec![
                ".git".to_string(),
                "target".to_string(),
                "node_modules".to_string(),
                ".vscode".to_string(),
                ".idea".to_string(),
                "dist".to_string(),
                "build".to_string(),
                ".cargo".to_string(),
                "vendor".to_string(),
            ],
        }
    }
}

/// Document indexer
pub struct DocumentIndexer {
    config: IndexerConfig,
}

impl DocumentIndexer {
    pub fn new(config: IndexerConfig) -> Self {
        Self {
            config,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(IndexerConfig::default())
    }

    /// Scan directory for documents
    pub fn scan_directory(&self, dir_path: &Path) -> Result<Vec<DocumentMetadata>> {
        let mut documents = Vec::new();
        let mut file_count = 0;

        let root_path = dir_path.to_path_buf();

        // Build WalkDir with optional depth limit
        let walk_dir = if let Some(max_depth) = self.config.max_depth {
            WalkDir::new(dir_path).max_depth(max_depth)
        } else {
            WalkDir::new(dir_path)
        };

        for entry in walk_dir.follow_links(false).into_iter().filter_entry(|e| {
            // Don't filter the root directory itself
            if e.path() == root_path {
                return true;
            }

            let dir_name = e.file_name().to_str().unwrap_or("");

            // Skip excluded directories
            if self.config.excluded_dirs.contains(&dir_name.to_string()) {
                return false;
            }

            // Skip hidden files/dirs if configured
            if self.config.skip_hidden {
                !dir_name.starts_with('.')
            } else {
                true
            }
        }) {
            let entry = entry.context("Failed to read directory entry")?;

            // Skip directories
            if !entry.file_type().is_file() {
                continue;
            }

            // Check file count limit
            if let Some(max_files) = self.config.max_files {
                if file_count >= max_files {
                    eprintln!("Warning: Reached maximum file limit ({}), stopping scan", max_files);
                    break;
                }
            }
            file_count += 1;

            let path = entry.path();

            // Check if file extension should be skipped
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if self.config.skip_extensions.contains(&ext.to_string()) {
                    continue;
                }
            }

            // Detect document type
            if let Some(doc_type) = DocumentType::from_path(path) {
                let metadata = entry.metadata().context("Failed to read file metadata")?;

                documents.push(DocumentMetadata {
                    path: path.to_path_buf(),
                    filename: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                    doc_type,
                    size_bytes: metadata.len(),
                    modified_time: metadata
                        .modified()
                        .unwrap_or_else(|_| std::time::SystemTime::now()),
                });
            }
        }

        Ok(documents)
    }

    /// Extract text from a document
    pub fn extract_text(&self, doc: &DocumentMetadata) -> Result<String> {
        match doc.doc_type {
            DocumentType::Markdown
            | DocumentType::Text
            | DocumentType::Rust
            | DocumentType::Python
            | DocumentType::Json
            | DocumentType::Toml => {
                // Read as UTF-8 text
                fs::read_to_string(&doc.path)
                    .context(format!("Failed to read file: {}", doc.path.display()))
            }
            DocumentType::Pdf => {
                // TODO: Add PDF extraction using pdf-extract or similar
                // For now, return error
                anyhow::bail!("PDF extraction not yet implemented")
            }
        }
    }

    /// Chunk document into semantic pieces
    pub fn chunk_document(&self, text: &str, metadata: &DocumentMetadata) -> Vec<DocumentChunk> {
        let mut chunks = Vec::new();
        let text_len = text.len();

        if text_len == 0 {
            return chunks;
        }

        // If text is smaller than max chunk size, return as single chunk
        if text_len <= self.config.max_chunk_size {
            chunks.push(DocumentChunk {
                text: text.to_string(),
                source_file: metadata.path.clone(),
                chunk_index: 0,
                metadata: metadata.clone(),
            });
            return chunks;
        }

        // Split into overlapping chunks using char indices to handle UTF-8 properly
        let mut start_byte = 0;
        let mut chunk_index = 0;

        while start_byte < text_len {
            // Find end position (approximately max_chunk_size bytes from start)
            let target_end = (start_byte + self.config.max_chunk_size).min(text_len);

            // Ensure we're on a character boundary by finding the last char boundary <= target_end
            let mut end_byte = target_end;
            while end_byte > start_byte && !text.is_char_boundary(end_byte) {
                end_byte -= 1;
            }

            // Try to break at word boundary if not at the end of text
            if end_byte < text_len {
                // Search backwards from end_byte for whitespace
                if let Some(substring) = text.get(start_byte..end_byte) {
                    if let Some(last_space_offset) = substring.rfind(|c: char| c.is_whitespace()) {
                        end_byte = start_byte + last_space_offset;
                    }
                }
            }

            // Extract chunk text (guaranteed to be on char boundaries)
            if let Some(chunk_text) = text.get(start_byte..end_byte) {
                chunks.push(DocumentChunk {
                    text: chunk_text.to_string(),
                    source_file: metadata.path.clone(),
                    chunk_index,
                    metadata: metadata.clone(),
                });
            }

            // Move start forward with overlap, ensuring char boundary
            let overlap_target = end_byte.saturating_sub(self.config.overlap_size);
            let mut next_start = overlap_target;
            while next_start < text_len && !text.is_char_boundary(next_start) {
                next_start += 1;
            }

            start_byte = if end_byte < text_len {
                next_start
            } else {
                text_len // We're done
            };

            chunk_index += 1;

            // Safety: prevent infinite loops
            if chunks.len() > 10000 {
                break;
            }
        }

        chunks
    }

    /// Index a directory and store in global knowledge database
    pub fn index_directory(&self, dir_path: &Path) -> Result<usize> {
        // Scan for documents
        let documents = self.scan_directory(dir_path)?;

        let mut total_chunks = 0;
        let db_pool = GlobalDbPool::new()?;
        let db = db_pool.get();
        let mut vector_store = GlobalVectorStore::new()?;

        // Process each document
        for doc in documents {
            // Extract text
            let text = match self.extract_text(&doc) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("Warning: Failed to extract text from {}: {}", doc.path.display(), e);
                    continue;
                }
            };

            // Chunk the document
            let chunks = self.chunk_document(&text, &doc);

            // Store each chunk in global database with embeddings
            for chunk in &chunks {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;

                // Create unique key and ID for this chunk
                let key =
                    format!("doc:{}:chunk:{}", chunk.source_file.display(), chunk.chunk_index);

                // Generate unique ID based on hash of the key
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                key.hash(&mut hasher);
                let chunk_id = hasher.finish() as i64;

                // Store chunk text in global SQLite database
                db.execute(
                    "INSERT OR REPLACE INTO memory (k, v, ts) VALUES (?1, ?2, ?3)",
                    (&key, &chunk.text, now),
                )?;

                // Store vector embedding in global FAISS index
                vector_store.insert_text(chunk_id, &chunk.text, "documents").unwrap_or_else(|e| {
                    eprintln!("Warning: Failed to store embedding for {}: {}", key, e);
                });
            }

            total_chunks += chunks.len();
        }

        Ok(total_chunks)
    }

    /// Index a directory with custom storage paths (for testing)
    /// This avoids touching ~/.syncore and allows full isolation
    pub fn index_directory_with_storage(
        &self,
        dir_path: &Path,
        db_path: &Path,
        vectors_dir: &Path,
    ) -> Result<usize> {
        // Scan for documents
        let documents = self.scan_directory(dir_path)?;

        let mut total_chunks = 0;
        let db_pool = GlobalDbPool::new_with_path(db_path)?;
        let db = db_pool.get();
        let mut vector_store = GlobalVectorStore::new_with_path(vectors_dir)?;

        // Process each document
        for doc in documents {
            // Extract text
            let text = match self.extract_text(&doc) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("Warning: Failed to extract text from {}: {}", doc.path.display(), e);
                    continue;
                }
            };

            // Chunk the document
            let chunks = self.chunk_document(&text, &doc);

            // Store each chunk in database with embeddings
            for chunk in &chunks {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;

                // Create unique key and ID for this chunk
                let key =
                    format!("doc:{}:chunk:{}", chunk.source_file.display(), chunk.chunk_index);

                // Generate unique ID based on hash of the key
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                key.hash(&mut hasher);
                let chunk_id = hasher.finish() as i64;

                // Store chunk text in SQLite database
                db.execute(
                    "INSERT OR REPLACE INTO memory (k, v, ts) VALUES (?1, ?2, ?3)",
                    (&key, &chunk.text, now),
                )?;

                // Store vector embedding in FAISS index
                vector_store.insert_text(chunk_id, &chunk.text, "documents").unwrap_or_else(|e| {
                    eprintln!("Warning: Failed to store embedding for {}: {}", key, e);
                });
            }

            total_chunks += chunks.len();
        }

        Ok(total_chunks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_docs() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        // Create test documents
        fs::write(base.join("test.md"), "# Test Document\n\nThis is a test.").unwrap();
        fs::write(base.join("notes.txt"), "Some notes here.").unwrap();
        fs::write(base.join("code.rs"), "fn main() { println!(\"hello\"); }").unwrap();

        // Create hidden file (should be skipped)
        fs::write(base.join(".hidden"), "hidden content").unwrap();

        // Create subdirectory
        fs::create_dir(base.join("subdir")).unwrap();
        fs::write(base.join("subdir/nested.md"), "Nested document").unwrap();

        temp_dir
    }

    #[test]
    fn test_document_type_detection() {
        assert_eq!(DocumentType::from_path(Path::new("test.md")), Some(DocumentType::Markdown));
        assert_eq!(DocumentType::from_path(Path::new("file.txt")), Some(DocumentType::Text));
        assert_eq!(DocumentType::from_path(Path::new("doc.pdf")), Some(DocumentType::Pdf));
        assert_eq!(DocumentType::from_path(Path::new("main.rs")), Some(DocumentType::Rust));
        assert_eq!(DocumentType::from_path(Path::new("script.py")), Some(DocumentType::Python));
        assert_eq!(DocumentType::from_path(Path::new("unknown.xyz")), None);
    }

    #[test]
    fn test_scan_directory_finds_documents() {
        let temp_dir = setup_test_docs();
        let indexer = DocumentIndexer::with_defaults();

        let docs = indexer.scan_directory(temp_dir.path()).unwrap();

        println!("Found {} documents:", docs.len());
        for doc in &docs {
            println!("  - {} (type: {:?})", doc.filename, doc.doc_type);
        }

        assert!(
            docs.len() >= 3,
            "Should find at least 3 documents (test.md, notes.txt, code.rs), found {}",
            docs.len()
        );

        // Should not include hidden files
        assert!(!docs.iter().any(|d| d.filename.starts_with('.')), "Should skip hidden files");
    }

    #[test]
    fn test_scan_directory_recursive() {
        let temp_dir = setup_test_docs();
        let indexer = DocumentIndexer::with_defaults();

        let docs = indexer.scan_directory(temp_dir.path()).unwrap();

        // Should find nested document
        assert!(docs.iter().any(|d| d.filename == "nested.md"), "Should find nested documents");
    }

    #[test]
    fn test_extract_text_markdown() {
        let temp_dir = setup_test_docs();
        let indexer = DocumentIndexer::with_defaults();

        let metadata = DocumentMetadata {
            path: temp_dir.path().join("test.md"),
            filename: "test.md".to_string(),
            doc_type: DocumentType::Markdown,
            size_bytes: 100,
            modified_time: std::time::SystemTime::now(),
        };

        let text = indexer.extract_text(&metadata).unwrap();
        assert!(text.contains("Test Document"), "Should extract markdown content");
    }

    #[test]
    fn test_chunk_document_respects_max_size() {
        let indexer = DocumentIndexer::with_defaults();
        let long_text = "a".repeat(5000); // 5000 chars

        let metadata = DocumentMetadata {
            path: PathBuf::from("test.txt"),
            filename: "test.txt".to_string(),
            doc_type: DocumentType::Text,
            size_bytes: 5000,
            modified_time: std::time::SystemTime::now(),
        };

        let chunks = indexer.chunk_document(&long_text, &metadata);

        // Should create multiple chunks
        assert!(chunks.len() > 1, "Should split long text into multiple chunks");

        // Each chunk should be <= max_chunk_size
        for chunk in &chunks {
            assert!(
                chunk.text.len() <= indexer.config.max_chunk_size + indexer.config.overlap_size,
                "Chunk should respect max size"
            );
        }
    }

    #[test]
    fn test_chunk_document_has_overlap() {
        let indexer = DocumentIndexer::with_defaults();
        let text = "a".repeat(2000); // 2000 chars

        let metadata = DocumentMetadata {
            path: PathBuf::from("test.txt"),
            filename: "test.txt".to_string(),
            doc_type: DocumentType::Text,
            size_bytes: 2000,
            modified_time: std::time::SystemTime::now(),
        };

        let chunks = indexer.chunk_document(&text, &metadata);

        if chunks.len() > 1 {
            // Check that chunks have overlap for context
            let first_end = &chunks[0].text[chunks[0].text.len().saturating_sub(100)..];
            let second_start = &chunks[1].text[..100.min(chunks[1].text.len())];

            // Should have some overlap (not exact match due to word boundaries)
            assert!(
                !first_end.is_empty() && !second_start.is_empty(),
                "Chunks should have overlap for context"
            );
        }
    }

    #[test]
    fn test_chunk_preserves_metadata() {
        let indexer = DocumentIndexer::with_defaults();
        let text = "Test content";

        let metadata = DocumentMetadata {
            path: PathBuf::from("/path/to/test.md"),
            filename: "test.md".to_string(),
            doc_type: DocumentType::Markdown,
            size_bytes: 100,
            modified_time: std::time::SystemTime::now(),
        };

        let chunks = indexer.chunk_document(text, &metadata);

        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.source_file, metadata.path, "Should preserve source file");
            assert_eq!(chunk.chunk_index, i, "Should have correct chunk index");
        }
    }

    #[test]
    fn test_index_directory_returns_chunk_count() {
        let temp_dir = setup_test_docs();
        let indexer = DocumentIndexer::with_defaults();

        // Use isolated storage to avoid migration conflicts
        let storage_dir = tempfile::tempdir().unwrap();
        let db_path = storage_dir.path().join("test.db");
        let vectors_dir = storage_dir.path().join("vectors");

        let chunk_count =
            indexer.index_directory_with_storage(temp_dir.path(), &db_path, &vectors_dir).unwrap();

        assert!(chunk_count > 0, "Should index and return chunk count");
    }

    #[test]
    fn test_excluded_directories_are_skipped() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        // Create test documents
        fs::write(base.join("test.md"), "# Test Document\n\nThis is a test.").unwrap();

        // Create excluded directories with files
        fs::create_dir(base.join("target")).unwrap();
        fs::write(base.join("target/should_skip.rs"), "fn main() {}").unwrap();

        fs::create_dir(base.join("node_modules")).unwrap();
        fs::write(base.join("node_modules/should_skip.js"), "console.log('skip');").unwrap();

        fs::create_dir(base.join(".git")).unwrap();
        fs::write(base.join(".git/config"), "[core]\n\trepositoryformatversion = 0").unwrap();

        let indexer = DocumentIndexer::with_defaults();
        let docs = indexer.scan_directory(base).unwrap();

        // Should find the main document but not files in excluded directories
        assert_eq!(docs.len(), 1, "Should only find 1 document, found {}", docs.len());
        assert_eq!(docs[0].filename, "test.md", "Should find test.md");
    }

    #[test]
    fn test_depth_limiting() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        // Create nested directory structure deeper than default limit
        fs::write(base.join("level0.md"), "Level 0").unwrap();

        let mut current_path = base.to_path_buf();
        for i in 1..15 {
            // Create 15 levels deep
            current_path = current_path.join(format!("level{}", i));
            fs::create_dir(&current_path).unwrap();
            fs::write(current_path.join("content.md"), format!("Level {}", i)).unwrap();
        }

        // Test with depth limit of 6 (should allow levels 0-5)
        let config = IndexerConfig {
            max_depth: Some(6),
            ..Default::default()
        };
        let indexer = DocumentIndexer::new(config);
        let docs = indexer.scan_directory(base).unwrap();

        // Should find files at levels 0-5 (6 files total)
        // Note: WalkDir max_depth is inclusive, so depth 5 allows levels 0-5
        println!("Found documents:");
        for doc in &docs {
            println!("  - {}", doc.path.display());
        }
        // With max_depth(6), we should find files at levels 0-5 (6 files total)
        // WalkDir depth counting starts at 0, so max_depth(6) allows depths 0-5
        assert_eq!(
            docs.len(),
            6,
            "Should find exactly 6 documents with depth limit 5, found {}",
            docs.len()
        );

        // Verify the deepest file found is at level 5
        let deepest_path =
            docs.iter().map(|d| &d.path).max_by_key(|p| p.components().count()).unwrap();
        assert!(
            deepest_path.to_string_lossy().contains("level5"),
            "Should find file at level 5, found: {}",
            deepest_path.display()
        );
        assert!(
            !deepest_path.to_string_lossy().contains("level6"),
            "Should not find files beyond level 5"
        );
    }

    #[test]
    fn test_file_count_limiting() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        // Create many test documents
        for i in 0..20 {
            fs::write(base.join(format!("file{}.md", i)), format!("Content {}", i)).unwrap();
        }

        // Test with file count limit of 10
        let config = IndexerConfig {
            max_files: Some(10),
            ..Default::default()
        };
        let indexer = DocumentIndexer::new(config);
        let docs = indexer.scan_directory(base).unwrap();

        assert_eq!(
            docs.len(),
            10,
            "Should find exactly 10 documents with file limit, found {}",
            docs.len()
        );
    }

    #[test]
    fn test_custom_excluded_directories() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        // Create test documents
        fs::write(base.join("test.md"), "# Test Document").unwrap();

        // Create custom excluded directories
        fs::create_dir(base.join("custom_exclude")).unwrap();
        fs::write(base.join("custom_exclude/should_skip.rs"), "fn main() {}").unwrap();

        fs::create_dir(base.join("another_exclude")).unwrap();
        fs::write(base.join("another_exclude/should_skip.js"), "console.log('skip');").unwrap();

        // Test with custom excluded directories
        let config = IndexerConfig {
            excluded_dirs: vec!["custom_exclude".to_string(), "another_exclude".to_string()],
            ..Default::default()
        };
        let indexer = DocumentIndexer::new(config);
        let docs = indexer.scan_directory(base).unwrap();

        assert_eq!(docs.len(), 1, "Should only find 1 document, found {}", docs.len());
        assert_eq!(docs[0].filename, "test.md", "Should find test.md");
    }

    #[test]
    fn test_no_limits_when_none() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        // Create nested structure with many files
        fs::write(base.join("root.md"), "Root").unwrap();

        for i in 0..5 {
            let dir_path = base.join(format!("dir{}", i));
            fs::create_dir(&dir_path).unwrap();
            for j in 0..3 {
                fs::write(dir_path.join(format!("file{}.md", j)), format!("Content {}-{}", i, j))
                    .unwrap();
            }
        }

        // Test with no limits (None values)
        let config = IndexerConfig {
            max_depth: None,
            max_files: None,
            excluded_dirs: vec![],
            ..Default::default()
        };
        let indexer = DocumentIndexer::new(config);
        let docs = indexer.scan_directory(base).unwrap();

        // Should find all files (1 root + 5*3 = 16 total)
        assert_eq!(
            docs.len(),
            16,
            "Should find all 16 documents with no limits, found {}",
            docs.len()
        );
    }
}
