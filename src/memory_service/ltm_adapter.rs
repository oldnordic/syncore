//! Long-Term Memory Adapter for MemoryService
//!
//! Implements persistent storage using SQL (DbManager) + mock vector backend.
//! Phase 2: Uses DbManager for SQL, mock vector search for testing.
//! Future: Will integrate with Real HNSW + Neo4j backends.

use super::error::MemoryError;
use super::ram_cache::MemoryEntry;
use crate::db::DbManager;
use rusqlite::params;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

/// Long-term memory statistics
#[derive(Debug, Clone)]
pub struct LtmStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub sql_rows: usize,
}

/// Trait for long-term memory storage operations
pub trait LongTermStore {
    fn ltm_store(&mut self, entry: &MemoryEntry) -> Result<String, MemoryError>;
    fn ltm_query(&self, query_embedding: &[f32], k: usize)
        -> Result<Vec<MemoryEntry>, MemoryError>;
    fn ltm_stats(&self) -> Result<LtmStats, MemoryError>;
}

/// Long-term memory adapter using SQL + Mock vector backend
pub struct LtmAdapter {
    db_manager: Arc<DbManager>,
    dimension: usize,
    /// Mock vector storage: id -> (embedding, MemoryEntry)
    mock_vectors: Arc<Mutex<HashMap<String, (Vec<f32>, MemoryEntry)>>>,
}

impl LtmAdapter {
    /// Create a new LTM adapter with mock backend (for testing)
    pub fn new_with_mock(db_manager: DbManager, dimension: usize) -> Result<Self, MemoryError> {
        // Create ltm_memory table in main database
        let conn = db_manager.main_conn();
        let conn_lock = conn
            .lock()
            .map_err(|e| MemoryError::Internal(format!("Failed to lock connection: {}", e)))?;

        conn_lock
            .execute(
                "CREATE TABLE IF NOT EXISTS ltm_memory (
                id TEXT PRIMARY KEY,
                summary TEXT NOT NULL,
                importance REAL NOT NULL,
                tags TEXT NOT NULL,
                embedding BLOB NOT NULL
            )",
                [],
            )
            .map_err(|e| {
                MemoryError::Internal(format!("Failed to create ltm_memory table: {}", e))
            })?;

        drop(conn_lock);

        Ok(Self {
            db_manager: Arc::new(db_manager),
            dimension,
            mock_vectors: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Compute cosine similarity between two vectors
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot_product / (norm_a * norm_b)
    }

    /// Serialize tags to JSON string
    fn serialize_tags(tags: &[String]) -> String {
        serde_json::to_string(tags).unwrap_or_else(|_| "[]".to_string())
    }

    /// Deserialize tags from JSON string
    fn deserialize_tags(json: &str) -> Vec<String> {
        serde_json::from_str(json).unwrap_or_else(|_| vec![])
    }

    /// Serialize embedding to binary blob
    fn serialize_embedding(embedding: &[f32]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(embedding.len() * 4);
        for &val in embedding {
            bytes.extend_from_slice(&val.to_le_bytes());
        }
        bytes
    }

    /// Deserialize embedding from binary blob
    fn deserialize_embedding(bytes: &[u8]) -> Vec<f32> {
        let mut embedding = Vec::with_capacity(bytes.len() / 4);
        for chunk in bytes.chunks_exact(4) {
            let val = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            embedding.push(val);
        }
        embedding
    }
}

impl LongTermStore for LtmAdapter {
    fn ltm_store(&mut self, entry: &MemoryEntry) -> Result<String, MemoryError> {
        // Validate dimension
        if entry.embedding.len() != self.dimension {
            return Err(MemoryError::DimensionMismatch);
        }

        // Store in SQL database
        let conn = self.db_manager.main_conn();
        let conn_lock = conn
            .lock()
            .map_err(|e| MemoryError::Internal(format!("Failed to lock connection: {}", e)))?;

        let tags_json = Self::serialize_tags(&entry.tags);
        let embedding_blob = Self::serialize_embedding(&entry.embedding);

        conn_lock
            .execute(
                "INSERT OR REPLACE INTO ltm_memory (id, summary, importance, tags, embedding)
             VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    &entry.id,
                    &entry.summary,
                    entry.importance,
                    &tags_json,
                    &embedding_blob,
                ],
            )
            .map_err(|e| {
                MemoryError::Internal(format!("Failed to insert into ltm_memory: {}", e))
            })?;

        drop(conn_lock);

        // Store in mock vector index
        let mut vectors = self
            .mock_vectors
            .lock()
            .map_err(|e| MemoryError::Internal(format!("Failed to lock vectors: {}", e)))?;

        vectors.insert(entry.id.clone(), (entry.embedding.clone(), entry.clone()));

        Ok(entry.id.clone())
    }

    fn ltm_query(
        &self,
        query_embedding: &[f32],
        k: usize,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        // Validate dimension
        if query_embedding.len() != self.dimension {
            return Err(MemoryError::DimensionMismatch);
        }

        // Get all vectors from mock storage
        let vectors = self
            .mock_vectors
            .lock()
            .map_err(|e| MemoryError::Internal(format!("Failed to lock vectors: {}", e)))?;

        if vectors.is_empty() {
            return Ok(vec![]);
        }

        // Compute similarity scores for all entries
        let mut scored: Vec<(String, f32, MemoryEntry)> = vectors
            .iter()
            .map(|(id, (embedding, entry))| {
                let similarity = Self::cosine_similarity(query_embedding, embedding);
                (id.clone(), similarity, entry.clone())
            })
            .collect();

        // Sort deterministically: similarity DESC, then ID ASC for tie-breaking
        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });

        // Take top-k results
        let results: Vec<MemoryEntry> = scored
            .into_iter()
            .take(k)
            .map(|(_, _, entry)| entry)
            .collect();

        Ok(results)
    }

    fn ltm_stats(&self) -> Result<LtmStats, MemoryError> {
        // Count SQL rows
        let conn = self.db_manager.main_conn();
        let conn_lock = conn
            .lock()
            .map_err(|e| MemoryError::Internal(format!("Failed to lock connection: {}", e)))?;

        let sql_rows: usize = conn_lock
            .query_row("SELECT COUNT(*) FROM ltm_memory", [], |row| row.get(0))
            .map_err(|e| MemoryError::Internal(format!("Failed to count SQL rows: {}", e)))?;

        drop(conn_lock);

        // Count vectors in mock storage
        let vectors = self
            .mock_vectors
            .lock()
            .map_err(|e| MemoryError::Internal(format!("Failed to lock vectors: {}", e)))?;

        let node_count = vectors.len();

        Ok(LtmStats {
            node_count,
            edge_count: 0, // No graph edges in Phase 2
            sql_rows,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DbManager;

    fn create_test_adapter() -> LtmAdapter {
        let db_manager =
            DbManager::new(":memory:", ":memory:").expect("Failed to create test DbManager");
        LtmAdapter::new_with_mock(db_manager, 4).expect("Failed to create LtmAdapter")
    }

    #[test]
    fn test_ltm_adapter_store_and_query() {
        let mut adapter = create_test_adapter();

        let entry = MemoryEntry {
            id: "test1".to_string(),
            summary: "Test entry".to_string(),
            importance: 0.8,
            tags: vec!["test".to_string()],
            embedding: vec![1.0, 0.0, 0.0, 0.0],
        };

        let id = adapter.ltm_store(&entry).expect("Store should succeed");
        assert_eq!(id, "test1");

        let query = vec![1.0, 0.0, 0.0, 0.0];
        let results = adapter.ltm_query(&query, 1).expect("Query should succeed");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "test1");
    }

    #[test]
    fn test_ltm_adapter_dimension_validation() {
        let mut adapter = create_test_adapter();

        let entry = MemoryEntry {
            id: "bad".to_string(),
            summary: "Bad entry".to_string(),
            importance: 0.5,
            tags: vec![],
            embedding: vec![1.0, 0.0], // Wrong dimension
        };

        let result = adapter.ltm_store(&entry);
        assert!(result.is_err());

        match result {
            Err(MemoryError::DimensionMismatch) => {} // Expected
            _ => panic!("Expected DimensionMismatch"),
        }
    }

    #[test]
    fn test_ltm_adapter_stats() {
        let mut adapter = create_test_adapter();

        let stats = adapter.ltm_stats().expect("Stats should succeed");
        assert_eq!(stats.sql_rows, 0);

        let entry = MemoryEntry {
            id: "entry1".to_string(),
            summary: "Entry 1".to_string(),
            importance: 0.5,
            tags: vec![],
            embedding: vec![0.5; 4],
        };

        adapter.ltm_store(&entry).expect("Store should succeed");

        let stats = adapter.ltm_stats().expect("Stats should succeed");
        assert_eq!(stats.sql_rows, 1);
        assert_eq!(stats.node_count, 1);
    }
}
