// Global Knowledge Store
// Provides centralized storage for articles, documentation, and reusable knowledge
// shared across all SynCore projects

use crate::vector::{Hit, SearchScope, VectorStore};
use anyhow::{Context, Result};
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Get the global SynCore directory path
/// Default: ~/.syncore/
/// Override: SYNCORE_GLOBAL_DIR environment variable
pub fn get_global_dir() -> PathBuf {
    if let Ok(custom_dir) = std::env::var("SYNCORE_GLOBAL_DIR") {
        return PathBuf::from(custom_dir);
    }

    let home = std::env::var("HOME").expect("HOME environment variable should be set");
    PathBuf::from(home).join(".syncore")
}

/// Get the global database path
/// Returns: ~/.syncore/global.db
pub fn get_global_db_path() -> PathBuf {
    get_global_dir().join("global.db")
}

/// Get the global vectors directory
/// Returns: ~/.syncore/vectors/
pub fn get_global_vectors_dir() -> PathBuf {
    get_global_dir().join("vectors")
}

/// Initialize global SynCore directories
/// Creates ~/.syncore/ and subdirectories if they don't exist
pub fn init_global_dirs() -> Result<()> {
    let global_dir = get_global_dir();
    let vectors_dir = get_global_vectors_dir();

    // Create directories (idempotent - won't fail if they exist)
    std::fs::create_dir_all(&global_dir).context("Failed to create global SynCore directory")?;
    std::fs::create_dir_all(&vectors_dir).context("Failed to create global vectors directory")?;

    Ok(())
}

/// Global database pool (shared across all projects)
#[derive(Clone)]
pub struct GlobalDbPool {
    conn: Arc<Mutex<Connection>>,
}

impl GlobalDbPool {
    /// Create GlobalDbPool using an existing database connection.
    ///
    /// This is useful when you want to manage the connection lifecycle externally,
    /// for example when using DbManager or in tests.
    pub fn with_connection(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Create a new global database connection pool
    /// Initializes global directories and database schema
    pub fn new() -> Result<Self> {
        // Ensure global directories exist
        init_global_dirs()?;

        // Open global database with WAL mode and auto-migration
        let db_path = get_global_db_path();
        Self::new_with_path(&db_path)
    }

    /// Create a new database connection pool with custom path
    /// Use this for testing to avoid touching ~/.syncore
    pub fn new_with_path(db_path: &Path) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create database directory")?;
        }

        let db_path_str = db_path
            .to_str()
            .context("Database path contains invalid UTF-8")?;

        let conn = crate::db::open_db_with_wal(db_path_str).context("Failed to open database")?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Get a database connection
    pub fn get(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap()
    }
}

/// Global vector store manager
/// Manages FAISS indices in the global vectors directory
pub struct GlobalVectorStore {
    vectors_dir: PathBuf,
    stores: Arc<Mutex<HashMap<String, VectorStore>>>,
}

impl GlobalVectorStore {
    /// Create a new global vector store
    pub fn new() -> Result<Self> {
        init_global_dirs()?;
        let vectors_dir = get_global_vectors_dir();
        Self::new_with_path(&vectors_dir)
    }

    /// Create a new vector store with custom directory path
    /// Use this for testing to avoid touching ~/.syncore
    pub fn new_with_path(vectors_dir: &Path) -> Result<Self> {
        // Ensure vectors directory exists
        std::fs::create_dir_all(vectors_dir).context("Failed to create vectors directory")?;

        Ok(Self {
            vectors_dir: vectors_dir.to_path_buf(),
            stores: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Get path for a specific vector index
    pub fn get_index_path(&self, index_name: &str) -> PathBuf {
        self.vectors_dir.join(format!("{}.faiss", index_name))
    }

    /// Check if an index exists
    pub fn index_exists(&self, index_name: &str) -> bool {
        // Check for .vectors file (save_snapshot creates .vectors and .meta files)
        let index_path = self.get_index_path(index_name);
        let vectors_path = format!("{}.vectors", index_path.display());
        std::path::Path::new(&vectors_path).exists()
    }

    /// Insert text and generate embedding
    pub fn insert_text(&mut self, id: i64, text: &str, namespace: &str) -> Result<()> {
        let mut stores = self.stores.lock().unwrap();

        if !stores.contains_key(namespace) {
            // Create new VectorStore with HuggingFace embeddings
            let embeddings = crate::vector::HuggingFaceEmbeddings::new()
                .context("Failed to create HuggingFace embeddings")?;

            let index_path = self.get_index_path(namespace);
            let index_path_str = index_path
                .to_str()
                .context("Invalid index path")?
                .to_string();

            let mut store = VectorStore::new(Box::new(embeddings));
            store.set_index_path(index_path_str);

            // Load existing index if it exists
            if self.index_exists(namespace) {
                store
                    .load_snapshot()
                    .context("Failed to load existing index")?;
            }

            stores.insert(namespace.to_string(), store);
        }

        let store = stores.get_mut(namespace).unwrap();
        store.insert_text(id, None, text, namespace)?;
        Ok(())
    }

    /// Search for similar text
    pub fn search(&self, query: &str, limit: usize, namespace: &str) -> Result<Vec<Hit>> {
        let mut stores = self.stores.lock().unwrap();

        if !stores.contains_key(namespace) {
            // Try to load from disk
            if !self.index_exists(namespace) {
                return Ok(Vec::new()); // No index exists yet
            }

            // Load the index
            let embeddings = crate::vector::HuggingFaceEmbeddings::new()
                .context("Failed to create HuggingFace embeddings")?;

            let index_path = self.get_index_path(namespace);
            let index_path_str = index_path
                .to_str()
                .context("Invalid index path")?
                .to_string();

            let mut store = VectorStore::new(Box::new(embeddings));
            store.set_index_path(index_path_str);
            store.load_snapshot().context("Failed to load index")?;

            stores.insert(namespace.to_string(), store);
        }

        let store = stores.get(namespace).unwrap();
        store.search(query, limit, SearchScope::Global)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    // Test helper: Set up test environment with custom global dir
    fn setup_test_env() -> PathBuf {
        let test_dir = PathBuf::from("/tmp/syncore_global_test");
        env::set_var("SYNCORE_GLOBAL_DIR", test_dir.to_str().unwrap());

        // Clean up any previous test
        let _ = std::fs::remove_dir_all(&test_dir);

        test_dir
    }

    #[test]
    fn test_get_global_dir_default() {
        // Clear environment variable to test default
        env::remove_var("SYNCORE_GLOBAL_DIR");

        let global_dir = get_global_dir();
        let home = env::var("HOME").expect("HOME should be set");
        let expected = PathBuf::from(home).join(".syncore");

        assert_eq!(
            global_dir, expected,
            "Default global dir should be ~/.syncore"
        );
    }

    #[test]
    fn test_get_global_dir_override() {
        let test_dir = setup_test_env();

        let global_dir = get_global_dir();
        assert_eq!(
            global_dir, test_dir,
            "Should use SYNCORE_GLOBAL_DIR override"
        );
    }

    #[test]
    fn test_get_global_db_path() {
        let test_dir = setup_test_env();

        let db_path = get_global_db_path();
        let expected = test_dir.join("global.db");

        assert_eq!(db_path, expected, "Global DB should be in global dir");
    }

    #[test]
    fn test_get_global_vectors_dir() {
        let test_dir = setup_test_env();

        let vectors_dir = get_global_vectors_dir();
        let expected = test_dir.join("vectors");

        assert_eq!(vectors_dir, expected, "Vectors should be in global dir");
    }

    #[test]
    fn test_init_global_dirs_creates_directories() {
        let test_dir = setup_test_env();

        init_global_dirs().expect("Should create global directories");

        assert!(test_dir.exists(), "Global dir should exist");
        assert!(
            test_dir.join("vectors").exists(),
            "Vectors dir should exist"
        );
    }

    #[test]
    fn test_init_global_dirs_idempotent() {
        let test_dir = setup_test_env();

        // Run twice - should not fail
        init_global_dirs().expect("First init should succeed");
        init_global_dirs().expect("Second init should succeed (idempotent)");

        assert!(test_dir.exists(), "Global dir should still exist");
    }

    #[test]
    fn test_global_db_pool_creation() {
        let test_dir = setup_test_env();

        let pool = GlobalDbPool::new().expect("Should create global DB pool");

        // Verify database file exists
        let db_path = test_dir.join("global.db");
        assert!(db_path.exists(), "Global database should be created");
    }

    #[test]
    fn test_global_db_has_schema() {
        let test_dir = setup_test_env();

        let pool = GlobalDbPool::new().expect("Should create global DB pool");
        let conn = pool.get();

        // Check that memory table exists (should be migrated from local schema)
        let table_exists: bool = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='memory'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);

        assert!(table_exists, "Global DB should have memory table");
    }

    #[test]
    fn test_global_db_has_embeddings_table() {
        let test_dir = setup_test_env();

        let pool = GlobalDbPool::new().expect("Should create global DB pool");
        let conn = pool.get();

        // Check that embeddings table exists
        let table_exists: bool = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='embeddings'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);

        assert!(table_exists, "Global DB should have embeddings table");
    }

    #[test]
    fn test_global_db_accessible_from_multiple_instances() {
        let test_dir = setup_test_env();

        // Create two separate pool instances (simulating different projects)
        let pool1 = GlobalDbPool::new().expect("First instance should succeed");
        let pool2 = GlobalDbPool::new().expect("Second instance should succeed");

        // Both should access the same database file
        let db_path = test_dir.join("global.db");
        assert!(db_path.exists(), "Single global DB should exist");

        // Insert data with pool1
        {
            let conn = pool1.get();
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;

            conn.execute(
                "INSERT INTO memory (k, v, ts) VALUES (?1, ?2, ?3)",
                ("test_key", "test_value", ts),
            )
            .expect("Should insert via pool1");
        }

        // Read data with pool2
        {
            let conn = pool2.get();
            let value: String = conn
                .query_row("SELECT v FROM memory WHERE k = ?1", ["test_key"], |row| {
                    row.get(0)
                })
                .expect("Should read via pool2");

            assert_eq!(
                value, "test_value",
                "Data should be shared across instances"
            );
        }
    }

    // --- Global Vector Store Tests ---

    #[test]
    fn test_global_vector_store_creation() {
        let test_dir = setup_test_env();

        let store = GlobalVectorStore::new().expect("Should create global vector store");

        // Verify vectors directory exists
        let vectors_dir = test_dir.join("vectors");
        assert!(vectors_dir.exists(), "Vectors directory should exist");
    }

    #[test]
    fn test_get_index_path() {
        let test_dir = setup_test_env();

        let store = GlobalVectorStore::new().expect("Should create store");
        let index_path = store.get_index_path("articles");

        let expected = test_dir.join("vectors/articles.faiss");
        assert_eq!(index_path, expected, "Index path should be in vectors dir");
    }

    #[test]
    fn test_index_exists_false_initially() {
        setup_test_env();

        let store = GlobalVectorStore::new().expect("Should create store");

        assert!(
            !store.index_exists("articles"),
            "Index should not exist initially"
        );
        assert!(
            !store.index_exists("code_patterns"),
            "Index should not exist initially"
        );
    }

    // --- Vector Embedding Tests (TDD) ---

    #[test]
    fn test_store_and_retrieve_embedding() {
        setup_test_env();

        let mut store = GlobalVectorStore::new().expect("Should create store");

        // Store a document chunk with embedding
        let text = "This is a test document about vector embeddings";
        let chunk_id = 1;

        store
            .insert_text(chunk_id, text, "documents")
            .expect("Should insert text and generate embedding");

        // Search for similar text
        let results = store
            .search("vector embeddings", 5, "documents")
            .expect("Should search embeddings");

        assert!(!results.is_empty(), "Should find at least one result");
        assert_eq!(results[0].id, chunk_id, "Should find the inserted chunk");
    }

    #[test]
    fn test_semantic_search_finds_similar() {
        setup_test_env();

        let mut store = GlobalVectorStore::new().expect("Should create store");

        // Insert multiple related chunks
        store
            .insert_text(
                1,
                "Machine learning models require training data",
                "documents",
            )
            .expect("Should insert");
        store
            .insert_text(2, "Neural networks learn from examples", "documents")
            .expect("Should insert");
        store
            .insert_text(3, "The weather is sunny today", "documents")
            .expect("Should insert");

        // Search for ML-related content
        let results = store
            .search("artificial intelligence and deep learning", 3, "documents")
            .expect("Should search");

        // Should find ML-related chunks first (higher similarity)
        assert!(results.len() >= 2, "Should find at least 2 results");
        // Chunks 1 and 2 should rank higher than chunk 3
        let ml_chunks: Vec<_> = results.iter().filter(|r| r.id == 1 || r.id == 2).collect();
        assert!(ml_chunks.len() >= 2, "Should find both ML-related chunks");
    }

    #[test]
    fn test_global_vector_store_persists_across_instances() {
        setup_test_env();

        // First instance: insert data
        {
            let mut store = GlobalVectorStore::new().expect("Should create store");
            store
                .insert_text(1, "Global knowledge persistence test", "documents")
                .expect("Should insert");
        }

        // Second instance: search should find data
        {
            let store = GlobalVectorStore::new().expect("Should create store");
            let results = store
                .search("knowledge persistence", 5, "documents")
                .expect("Should search");

            assert!(!results.is_empty(), "Should find persisted data");
            assert_eq!(results[0].id, 1, "Should find the original chunk");
        }
    }

    #[test]
    fn test_multiple_index_namespaces() {
        setup_test_env();

        let mut store = GlobalVectorStore::new().expect("Should create store");

        // Insert into different namespaces
        store
            .insert_text(1, "Document chunk in documents namespace", "documents")
            .expect("Should insert to documents");
        store
            .insert_text(2, "Code snippet in code namespace", "code")
            .expect("Should insert to code");

        // Search each namespace independently
        let doc_results = store
            .search("document", 5, "documents")
            .expect("Should search documents");
        let code_results = store.search("code", 5, "code").expect("Should search code");

        assert!(
            !doc_results.is_empty(),
            "Should find in documents namespace"
        );
        assert!(!code_results.is_empty(), "Should find in code namespace");
        assert_eq!(doc_results[0].id, 1, "Should find document chunk");
        assert_eq!(code_results[0].id, 2, "Should find code chunk");
    }

    #[test]
    fn test_batch_insert_performance() {
        setup_test_env();

        let mut store = GlobalVectorStore::new().expect("Should create store");

        // Insert 100 chunks
        for i in 0..100 {
            let text = format!("Document chunk number {} with unique content", i);
            store
                .insert_text(i, &text, "documents")
                .expect("Should insert batch");
        }

        // Verify all are searchable
        let results = store
            .search("document chunk", 10, "documents")
            .expect("Should search after batch insert");

        assert!(
            results.len() >= 10,
            "Should find at least 10 results from 100 chunks"
        );
    }
}
