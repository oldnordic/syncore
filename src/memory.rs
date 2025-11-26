use anyhow::Result;
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sled::Db;
use std::sync::{Arc, Mutex};

// NEW: Import DualEmbeddingService and Neo4j for semantic capabilities
use crate::vector::dual_service::DualEmbeddingService;
use crate::vector::domain::{EmbeddingService};
use crate::vector::{SearchScope, VectorStore};

/// Memory configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MemoryConfig {
    pub enable_semantic_search: bool,
    pub auto_summarize_threshold: usize,
    pub consolidation_similarity: f32,
    pub default_namespace: String,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enable_semantic_search: true,
            auto_summarize_threshold: 500, // chars
            consolidation_similarity: 0.9,
            default_namespace: "default".to_string(),
        }
    }
}

/// Memory entry with metadata
#[derive(Debug, Clone, Serialize)]
pub struct MemoryEntry {
    pub id: i64,
    pub key: String,
    pub value: String,
    pub summary: Option<String>,
    pub namespace: String,
    pub importance: f32,
    pub tags: Vec<String>,
    pub created_at: i64,
    pub last_accessed: i64,
    pub access_count: i64,
    pub embedding_id: Option<i64>,
}

/// Semantic search result
#[derive(Debug, Clone, Serialize)]
pub struct SemanticSearchResult {
    pub entry: MemoryEntry,
    pub similarity: f32,
    pub rank: usize,
}

pub struct Memory {
    db: Arc<Mutex<Connection>>,
    cache: Arc<Db>,
    // NEW: Semantic search infrastructure
    embeddings: Option<Arc<DualEmbeddingService>>,
    config: MemoryConfig,
}

impl std::fmt::Debug for Memory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Memory")
            .field("db", &"Arc<Mutex<Connection>>")
            .field("cache", &"Arc<Db>")
            .field("embeddings", &self.embeddings.is_some())
            .field("config", &self.config)
            .finish()
    }
}

impl Clone for Memory {
    fn clone(&self) -> Self {
        Self {
            db: Arc::clone(&self.db),
            cache: Arc::clone(&self.cache),
            embeddings: self.embeddings.clone(),
            config: self.config.clone(),
        }
    }
}

impl Memory {
    /// Create Memory using an existing database connection from DbManager.
    ///
    /// This is the preferred constructor when using DbManager. It reuses long-lived
    /// connections instead of creating new ones per-call.
    ///
    /// # Arguments
    ///
    /// * `db` - Arc<Mutex<Connection>> from DbManager.main_conn()
    /// * `cache_path` - Path for sled cache (typically derived from DB path)
    ///
    /// # Example
    ///
    /// ```rust
    /// let db_manager = DbManager::new("syncore.db", "syncore_code_graph.db")?;
    /// let memory = Memory::with_connection(
    ///     db_manager.main_conn(),
    ///     "syncore.db_cache"
    /// )?;
    /// ```
    pub fn with_connection(db: Arc<Mutex<Connection>>, cache_path: &str) -> Result<Self> {
        Self::with_connection_and_config(db, cache_path, MemoryConfig::default())
    }

    /// Create Memory with pre-existing DualEmbeddingService
    ///
    /// This is the preferred constructor when embeddings are already initialized
    /// (e.g., in SynCoreState). Avoids creating duplicate embedding services.
    pub fn with_embeddings(
        db: Arc<Mutex<Connection>>,
        cache_path: &str,
        embeddings: Arc<DualEmbeddingService>,
    ) -> Result<Self> {
        let cache = match sled::open(cache_path) {
            Ok(cache) => cache,
            Err(e) => {
                if e.to_string().contains("corrupted") || e.to_string().contains("offset None") {
                    eprintln!("Warning: Cache corrupted, attempting cleanup: {}", e);
                    if let Err(cleanup_err) = std::fs::remove_dir_all(cache_path) {
                        eprintln!("Failed to remove corrupted cache: {}", cleanup_err);
                    }
                    sled::open(cache_path).map_err(|err| {
                        anyhow::anyhow!("Failed to create fresh cache after cleanup: {}", err)
                    })?
                } else {
                    return Err(anyhow::anyhow!("Failed to open cache: {}", e));
                }
            }
        };

        Ok(Self {
            db,
            cache: Arc::new(cache),
            embeddings: Some(embeddings),
            config: MemoryConfig::default(),
        })
    }

    /// Create Memory with custom configuration
    pub fn with_connection_and_config(
        db: Arc<Mutex<Connection>>,
        cache_path: &str,
        config: MemoryConfig,
    ) -> Result<Self> {
        // Try to open cache, if corrupted, clean it and try again
        let cache = match sled::open(cache_path) {
            Ok(cache) => cache,
            Err(e) => {
                // Check if it's a corruption error and clean up if needed
                if e.to_string().contains("corrupted") || e.to_string().contains("offset None") {
                    eprintln!("Warning: Cache corrupted, attempting cleanup: {}", e);
                    // Remove corrupted cache directory
                    if let Err(cleanup_err) = std::fs::remove_dir_all(cache_path) {
                        eprintln!("Failed to remove corrupted cache: {}", cleanup_err);
                    }
                    // Try to create fresh cache
                    sled::open(cache_path).map_err(|err| {
                        anyhow::anyhow!("Failed to create fresh cache after cleanup: {}", err)
                    })?
                } else {
                    return Err(anyhow::anyhow!("Failed to open cache: {}", e));
                }
            }
        };

        // Initialize embeddings if semantic search is enabled
        let embeddings = if config.enable_semantic_search {
            match DualEmbeddingService::new() {
                Ok(service) => Some(Arc::new(service)),
                Err(e) => {
                    eprintln!("Warning: Failed to initialize embeddings: {}", e);
                    None
                }
            }
        } else {
            None
        };

        Ok(Self {
            db,
            cache: Arc::new(cache),
            embeddings,
            config,
        })
    }

    /// Legacy constructor - opens its own connection (deprecated, use with_connection instead).
    ///
    /// This method is kept for backward compatibility with existing code that hasn't
    /// been refactored to use DbManager yet.
    pub fn new(db_path: &str) -> Result<Self> {
        crate::db::ensure_schema(db_path)?;

        let conn = crate::db::open_db_with_wal(db_path)?;

        // Create unique cache directory based on the database path
        let cache_path = format!("{}_cache", db_path);

        Self::with_connection(Arc::new(Mutex::new(conn)), &cache_path)
    }

    // ========================================================================
    // EXISTING API (Backward Compatible)
    // ========================================================================

    pub fn store(&self, key: &str, value: &str) -> Result<()> {
        self.store_with_metadata(
            key,
            value,
            &self.config.default_namespace,
            &[],
            0.5, // default importance
        )?;
        Ok(())
    }

    pub fn query(&self, key: &str) -> Result<Option<String>> {
        // Update access tracking
        self.update_access(key)?;

        // Try cache first
        if let Some(v) = self.cache.get(key)? {
            return Ok(Some(String::from_utf8(v.to_vec())?));
        }

        // Fallback to database (search in default namespace for backward compatibility)
        let db = self.db.lock().unwrap();
        let default_ns = self.config.default_namespace.clone();
        let value = db
            .query_row("SELECT v FROM memory WHERE k=?1 AND namespace=?2",
                       rusqlite::params![key, default_ns], |r| {
                r.get::<_, String>(0)
            })
            .optional()?;

        Ok(value)
    }

    pub fn query_with_timestamp(&self, key: &str) -> Result<Option<(String, i64)>> {
        let db = self.db.lock().unwrap();
        let default_ns = self.config.default_namespace.clone();
        let result = db
            .query_row("SELECT v, ts FROM memory WHERE k=?1 AND namespace=?2",
                       rusqlite::params![key, default_ns], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })
            .optional()?;

        Ok(result)
    }

    pub fn delete(&self, key: &str) -> Result<()> {
        let db = self.db.lock().unwrap();
        let default_ns = self.config.default_namespace.clone();
        db.execute("DELETE FROM memory WHERE k=?1 AND namespace=?2",
                   rusqlite::params![key, default_ns])?;

        drop(db);
        self.cache.remove(key)?;
        self.cache.flush()?;

        Ok(())
    }

    /// Query with explicit namespace support (APEX 2.0-M-FIX)
    pub fn query_with_namespace(&self, key: &str, namespace: Option<&str>) -> Result<Option<String>> {
        let ns = namespace.unwrap_or(&self.config.default_namespace);

        // Update access tracking (namespace-aware)
        self.update_access_with_namespace(key, ns)?;

        // Try cache first (with namespace-aware key)
        let cache_key = format!("{}:{}", ns, key);
        if let Some(v) = self.cache.get(&cache_key)? {
            return Ok(Some(String::from_utf8(v.to_vec())?));
        }

        // Fallback to database
        let db = self.db.lock().unwrap();
        let value = db
            .query_row("SELECT v FROM memory WHERE k=?1 AND namespace=?2",
                       rusqlite::params![key, ns], |r| {
                r.get::<_, String>(0)
            })
            .optional()?;

        Ok(value)
    }

    /// Delete with explicit namespace support (APEX 2.0-M-FIX)
    pub fn delete_with_namespace(&self, key: &str, namespace: Option<&str>) -> Result<()> {
        let ns = namespace.unwrap_or(&self.config.default_namespace);

        let db = self.db.lock().unwrap();
        db.execute("DELETE FROM memory WHERE k=?1 AND namespace=?2",
                   rusqlite::params![key, ns])?;

        drop(db);

        // Remove from cache (with namespace-aware key)
        let cache_key = format!("{}:{}", ns, key);
        self.cache.remove(&cache_key)?;
        self.cache.flush()?;

        Ok(())
    }

    pub fn list_keys(&self, limit: Option<i64>) -> Result<Vec<String>> {
        let db = self.db.lock().unwrap();
        let mut query = "SELECT k FROM memory ORDER BY ts DESC".to_string();

        if let Some(limit) = limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        let mut stmt = db.prepare(&query)?;
        let keys = stmt.query_map([], |row| row.get(0))?;

        let mut result = Vec::new();
        for key in keys {
            result.push(key?);
        }

        Ok(result)
    }

    // ========================================================================
    // NEW SEMANTIC MEMORY API (APEX 1.9)
    // ========================================================================

    /// Store memory with metadata (tags, importance, namespace)
    pub fn store_with_metadata(
        &self,
        key: &str,
        value: &str,
        namespace: &str,
        tags: &[&str],
        importance: f32,
    ) -> Result<i64> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Auto-generate summary if value is long
        let summary = if value.len() > self.config.auto_summarize_threshold {
            Some(self.generate_summary(value))
        } else {
            None
        };

        // Insert/update memory entry using proper UPSERT
        let db = self.db.lock().unwrap();

        // Try to get existing ID and created_at (by key AND namespace for proper isolation)
        let existing: Option<(i64, i64)> = db.query_row(
            "SELECT id, created_at FROM memory WHERE k=?1 AND namespace=?2",
            rusqlite::params![key, namespace],
            |r| Ok((r.get(0)?, r.get(1)?))
        ).optional()?;

        let entry_id = if let Some((id, existing_created_at)) = existing {
            // UPDATE existing entry
            db.execute(
                "UPDATE memory SET v=?1, ts=?2, summary=?3, namespace=?4, importance=?5, last_accessed=?6
                 WHERE id=?7",
                (value, now, summary.as_ref(), namespace, importance, now, id),
            )?;
            id
        } else {
            // INSERT new entry
            db.execute(
                "INSERT INTO memory (k, v, ts, summary, namespace, importance, created_at, last_accessed, access_count)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0)",
                (key, value, now, summary.as_ref(), namespace, importance, now, now),
            )?;
            db.last_insert_rowid()
        };

        // Store tags
        for tag in tags {
            db.execute(
                "INSERT OR IGNORE INTO memory_tags (memory_id, tag) VALUES (?1, ?2)",
                (entry_id, tag),
            )?;
        }

        drop(db);

        // Update cache (namespace-aware key for APEX 2.0-M-FIX)
        let cache_key = format!("{}:{}", namespace, key);
        self.cache.insert(&cache_key, value.as_bytes())?;
        self.cache.flush()?;

        // Generate and store embedding if semantic search is enabled
        if let Some(ref embeddings) = self.embeddings {
            let store = embeddings.general_store();
            let mut store = store.lock().unwrap();

            // Store the actual VALUE text for semantic search (with GENERAL domain = all-MiniLM-L6-v2)
            // Use VectorStore's insert_text method (id, task_id, text, kind)
            store.insert_text(entry_id, None, value, "memory")?;

            // Note: embedding_id in this case is the same as entry_id
            let db = self.db.lock().unwrap();
            db.execute(
                "UPDATE memory SET embedding_id = ?1 WHERE id = ?2",
                (entry_id, entry_id),
            )?;
        }

        Ok(entry_id)
    }

    /// Semantic search using vector embeddings
    pub fn search_semantic(
        &self,
        query: &str,
        namespace: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SemanticSearchResult>> {
        let embeddings = self.embeddings.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Semantic search not enabled"))?;

        // Search vector store using its search method
        let store = embeddings.general_store();
        let store = store.lock().unwrap();
        let hits = store.search(query, limit * 2, SearchScope::Global)?;

        // Convert hits to memory entries by matching embedding_id
        let db = self.db.lock().unwrap();
        let mut results = Vec::new();

        for (rank, hit) in hits.iter().enumerate() {
            // hit.id is the entry_id we stored
            let entry: Option<MemoryEntry> = db
                .query_row(
                    "SELECT id, k, v, summary, namespace, importance, created_at, last_accessed, access_count, embedding_id
                     FROM memory WHERE id=?1",
                    [hit.id],
                    |r| {
                        Ok(MemoryEntry {
                            id: r.get(0)?,
                            key: r.get(1)?,
                            value: r.get(2)?,
                            summary: r.get(3)?,
                            namespace: r.get(4)?,
                            importance: r.get(5)?,
                            tags: vec![], // Loaded separately
                            created_at: r.get(6)?,
                            last_accessed: r.get(7)?,
                            access_count: r.get(8)?,
                            embedding_id: r.get(9)?,
                        })
                    },
                )
                .optional()?;

            if let Some(mut entry) = entry {
                // Filter by namespace if specified
                if let Some(ns) = namespace {
                    if entry.namespace != ns {
                        continue;
                    }
                }

                // Load tags
                entry.tags = self.load_tags(&db, entry.id)?;

                results.push(SemanticSearchResult {
                    entry,
                    similarity: hit.score,
                    rank,
                });

                if results.len() >= limit {
                    break;
                }
            }
        }

        Ok(results)
    }

    /// Hybrid search (semantic + keyword)
    pub fn search_hybrid(
        &self,
        query: &str,
        keywords: &[&str],
        namespace: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SemanticSearchResult>> {
        // Get semantic results
        let mut semantic_results = self.search_semantic(query, namespace, limit * 2)?;

        // Boost scores for keyword matches
        for result in &mut semantic_results {
            let mut keyword_boost = 0.0;
            for keyword in keywords {
                if result.entry.key.contains(keyword) || result.entry.value.contains(keyword) {
                    keyword_boost += 0.2; // 20% boost per keyword match
                }
            }
            result.similarity = (result.similarity + keyword_boost).min(1.0);
        }

        // Re-sort by adjusted similarity
        semantic_results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());

        // Re-rank
        for (rank, result) in semantic_results.iter_mut().enumerate() {
            result.rank = rank;
        }

        Ok(semantic_results.into_iter().take(limit).collect())
    }

    /// Query by tags
    pub fn query_by_tags(&self, tags: &[&str], namespace: Option<&str>) -> Result<Vec<MemoryEntry>> {
        let db = self.db.lock().unwrap();

        let mut query = "SELECT DISTINCT m.id, m.k, m.v, m.summary, m.namespace, m.importance,
                         m.created_at, m.last_accessed, m.access_count, m.embedding_id
                         FROM memory m
                         JOIN memory_tags mt ON m.id = mt.memory_id
                         WHERE mt.tag IN (".to_string();

        query.push_str(&vec!["?"; tags.len()].join(","));
        query.push(')');

        let namespace_str: Option<String> = namespace.map(|s| s.to_string());
        if namespace_str.is_some() {
            query.push_str(" AND m.namespace = ?");
        }

        let mut stmt = db.prepare(&query)?;
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = tags.iter()
            .map(|t| Box::new(t.to_string()) as Box<dyn rusqlite::ToSql>)
            .collect();
        if let Some(ref ns) = namespace_str {
            params.push(Box::new(ns.clone()));
        }

        let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|b| b.as_ref()).collect();
        let rows = stmt.query_map(params_refs.as_slice(), |r| {
            Ok(MemoryEntry {
                id: r.get(0)?,
                key: r.get(1)?,
                value: r.get(2)?,
                summary: r.get(3)?,
                namespace: r.get(4)?,
                importance: r.get(5)?,
                tags: vec![], // Loaded separately
                created_at: r.get(6)?,
                last_accessed: r.get(7)?,
                access_count: r.get(8)?,
                embedding_id: r.get(9)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            let mut entry = row?;
            entry.tags = self.load_tags(&db, entry.id)?;
            results.push(entry);
        }

        Ok(results)
    }

    /// Query by importance threshold
    pub fn query_by_importance(&self, min_importance: f32, limit: usize) -> Result<Vec<MemoryEntry>> {
        let db = self.db.lock().unwrap();

        let mut stmt = db.prepare(
            "SELECT id, k, v, summary, namespace, importance, created_at, last_accessed, access_count, embedding_id
             FROM memory WHERE importance >= ?1 ORDER BY importance DESC LIMIT ?2",
        )?;

        let rows = stmt.query_map([min_importance, limit as f32], |r| {
            Ok(MemoryEntry {
                id: r.get(0)?,
                key: r.get(1)?,
                value: r.get(2)?,
                summary: r.get(3)?,
                namespace: r.get(4)?,
                importance: r.get(5)?,
                tags: vec![], // Loaded separately
                created_at: r.get(6)?,
                last_accessed: r.get(7)?,
                access_count: r.get(8)?,
                embedding_id: r.get(9)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            let mut entry = row?;
            entry.tags = self.load_tags(&db, entry.id)?;
            results.push(entry);
        }

        Ok(results)
    }

    /// Query recent memories
    pub fn query_recent(&self, limit: usize, namespace: Option<&str>) -> Result<Vec<MemoryEntry>> {
        let db = self.db.lock().unwrap();

        let query = if namespace.is_some() {
            "SELECT id, k, v, summary, namespace, importance, created_at, last_accessed, access_count, embedding_id
             FROM memory WHERE namespace = ? ORDER BY created_at DESC LIMIT ?"
        } else {
            "SELECT id, k, v, summary, namespace, importance, created_at, last_accessed, access_count, embedding_id
             FROM memory ORDER BY created_at DESC LIMIT ?"
        };

        let mut stmt = db.prepare(query)?;

        let mut results = Vec::new();

        if let Some(ns) = namespace {
            let rows = stmt.query_map(rusqlite::params![ns, limit as i64], |r| {
                Ok(MemoryEntry {
                    id: r.get(0)?,
                    key: r.get(1)?,
                    value: r.get(2)?,
                    summary: r.get(3)?,
                    namespace: r.get(4)?,
                    importance: r.get(5)?,
                    tags: vec![], // Loaded separately
                    created_at: r.get(6)?,
                    last_accessed: r.get(7)?,
                    access_count: r.get(8)?,
                    embedding_id: r.get(9)?,
                })
            })?;
            for row in rows {
                let mut entry = row?;
                entry.tags = self.load_tags(&db, entry.id)?;
                results.push(entry);
            }
        } else {
            let rows = stmt.query_map([limit as i64], |r| {
                Ok(MemoryEntry {
                    id: r.get(0)?,
                    key: r.get(1)?,
                    value: r.get(2)?,
                    summary: r.get(3)?,
                    namespace: r.get(4)?,
                    importance: r.get(5)?,
                    tags: vec![], // Loaded separately
                    created_at: r.get(6)?,
                    last_accessed: r.get(7)?,
                    access_count: r.get(8)?,
                    embedding_id: r.get(9)?,
                })
            })?;
            for row in rows {
                let mut entry = row?;
                entry.tags = self.load_tags(&db, entry.id)?;
                results.push(entry);
            }
        }

        Ok(results)
    }

    /// Query memories since timestamp
    pub fn query_since(&self, timestamp: i64, namespace: Option<&str>) -> Result<Vec<MemoryEntry>> {
        let db = self.db.lock().unwrap();

        let query = if namespace.is_some() {
            "SELECT id, k, v, summary, namespace, importance, created_at, last_accessed, access_count, embedding_id
             FROM memory WHERE created_at >= ? AND namespace = ? ORDER BY created_at DESC"
        } else {
            "SELECT id, k, v, summary, namespace, importance, created_at, last_accessed, access_count, embedding_id
             FROM memory WHERE created_at >= ? ORDER BY created_at DESC"
        };

        let mut stmt = db.prepare(query)?;
        let mut results = Vec::new();

        if let Some(ns) = namespace {
            let rows = stmt.query_map(rusqlite::params![timestamp, ns], |r| {
                Ok(MemoryEntry {
                    id: r.get(0)?,
                    key: r.get(1)?,
                    value: r.get(2)?,
                    summary: r.get(3)?,
                    namespace: r.get(4)?,
                    importance: r.get(5)?,
                    tags: vec![], // Loaded separately
                    created_at: r.get(6)?,
                    last_accessed: r.get(7)?,
                    access_count: r.get(8)?,
                    embedding_id: r.get(9)?,
                })
            })?;
            for row in rows {
                let mut entry = row?;
                entry.tags = self.load_tags(&db, entry.id)?;
                results.push(entry);
            }
        } else {
            let rows = stmt.query_map([timestamp], |r| {
                Ok(MemoryEntry {
                    id: r.get(0)?,
                    key: r.get(1)?,
                    value: r.get(2)?,
                    summary: r.get(3)?,
                    namespace: r.get(4)?,
                    importance: r.get(5)?,
                    tags: vec![], // Loaded separately
                    created_at: r.get(6)?,
                    last_accessed: r.get(7)?,
                    access_count: r.get(8)?,
                    embedding_id: r.get(9)?,
                })
            })?;
            for row in rows {
                let mut entry = row?;
                entry.tags = self.load_tags(&db, entry.id)?;
                results.push(entry);
            }
        }

        Ok(results)
    }

    /// Query frequently accessed memories
    pub fn query_frequently_accessed(&self, limit: usize) -> Result<Vec<MemoryEntry>> {
        let db = self.db.lock().unwrap();

        let mut stmt = db.prepare(
            "SELECT id, k, v, summary, namespace, importance, created_at, last_accessed, access_count, embedding_id
             FROM memory ORDER BY access_count DESC LIMIT ?1",
        )?;

        let rows = stmt.query_map([limit], |r| {
            Ok(MemoryEntry {
                id: r.get(0)?,
                key: r.get(1)?,
                value: r.get(2)?,
                summary: r.get(3)?,
                namespace: r.get(4)?,
                importance: r.get(5)?,
                tags: vec![], // Loaded separately
                created_at: r.get(6)?,
                last_accessed: r.get(7)?,
                access_count: r.get(8)?,
                embedding_id: r.get(9)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            let mut entry = row?;
            entry.tags = self.load_tags(&db, entry.id)?;
            results.push(entry);
        }

        Ok(results)
    }

    /// Consolidate similar memories (deduplication)
    pub fn consolidate_similar(&self, similarity_threshold: f32) -> Result<Vec<i64>> {
        let embeddings = self.embeddings.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Semantic search not enabled"))?;

        let db = self.db.lock().unwrap();

        // Get all memories with embeddings
        let mut stmt = db.prepare(
            "SELECT id, k, embedding_id FROM memory WHERE embedding_id IS NOT NULL"
        )?;

        let memories: Vec<(i64, String, i64)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        drop(stmt);

        let store = embeddings.general_store();
        let store = store.lock().unwrap();

        let mut consolidated = Vec::new();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Find similar pairs
        for i in 0..memories.len() {
            for j in (i + 1)..memories.len() {
                let (id1, _, emb_id1) = &memories[i];
                let (id2, _, emb_id2) = &memories[j];

                // Calculate similarity between embeddings
                // (Simplified - in production use proper cosine similarity)
                let similarity = self.calculate_embedding_similarity(*emb_id1, *emb_id2, &store)?;

                if similarity >= similarity_threshold {
                    // Delete duplicate (keep the one with higher importance)
                    // Skip if either memory was already deleted in a previous iteration
                    let importance1: Option<f32> = db.query_row(
                        "SELECT importance FROM memory WHERE id = ?1",
                        [id1],
                        |r| r.get(0),
                    ).optional()?;

                    let importance2: Option<f32> = db.query_row(
                        "SELECT importance FROM memory WHERE id = ?1",
                        [id2],
                        |r| r.get(0),
                    ).optional()?;

                    let (Some(imp1), Some(imp2)) = (importance1, importance2) else {
                        continue; // One or both already deleted
                    };

                    let (keep_id, delete_id) = if imp1 >= imp2 {
                        (*id1, *id2)
                    } else {
                        (*id2, *id1)
                    };

                    // Record consolidation (source=deleted, target=kept)
                    db.execute(
                        "INSERT OR IGNORE INTO memory_consolidations (source_id, target_id, similarity, consolidated_at)
                         VALUES (?1, ?2, ?3, ?4)",
                        (delete_id, keep_id, similarity, now),
                    )?;

                    consolidated.push(delete_id);

                    // Delete the lower-importance duplicate
                    db.execute("DELETE FROM memory WHERE id = ?1", [delete_id])?;
                }
            }
        }

        Ok(consolidated)
    }

    /// Get related memories (semantic + graph if Neo4j available)
    pub fn get_related_memories(&self, key: &str, limit: usize) -> Result<Vec<MemoryEntry>> {
        // For now, use pure semantic search
        // TODO: Add Neo4j graph traversal when available
        let results = self.search_semantic(key, None, limit)?;
        Ok(results.into_iter().map(|r| r.entry).collect())
    }

    /// Link memories (Neo4j relationships) - stub for now
    pub fn link_memories(&self, from_key: &str, to_key: &str, relationship: &str) -> Result<()> {
        // TODO: Implement Neo4j integration when available
        // For now, just verify both keys exist
        let _ = self.query(from_key)?
            .ok_or_else(|| anyhow::anyhow!("Source key not found: {}", from_key))?;
        let _ = self.query(to_key)?
            .ok_or_else(|| anyhow::anyhow!("Target key not found: {}", to_key))?;

        Ok(())
    }

    /// Check if Neo4j is available
    pub fn has_neo4j(&self) -> bool {
        // TODO: Check if Neo4j client is configured
        false
    }

    /// Get memory statistics
    pub fn get_stats(&self) -> Result<(i64, Vec<String>)> {
        let db = self.db.lock().unwrap();

        let count: i64 = db.query_row("SELECT COUNT(*) FROM memory", [], |row| row.get(0))?;

        let namespaces: Vec<String> = db
            .prepare("SELECT DISTINCT namespace FROM memory")?
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;

        Ok((count, namespaces))
    }

    // ========================================================================
    // HELPER METHODS
    // ========================================================================

    fn load_tags(&self, db: &Connection, memory_id: i64) -> Result<Vec<String>> {
        let mut stmt = db.prepare("SELECT tag FROM memory_tags WHERE memory_id = ?1")?;
        let tags = stmt
            .query_map([memory_id], |r| r.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        Ok(tags)
    }

    fn update_access(&self, key: &str) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let db = self.db.lock().unwrap();
        let default_ns = self.config.default_namespace.clone();
        db.execute(
            "UPDATE memory SET last_accessed = ?1, access_count = access_count + 1 WHERE k = ?2 AND namespace = ?3",
            (now, key, default_ns),
        )?;
        Ok(())
    }

    fn update_access_with_namespace(&self, key: &str, namespace: &str) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let db = self.db.lock().unwrap();
        db.execute(
            "UPDATE memory SET last_accessed = ?1, access_count = access_count + 1 WHERE k = ?2 AND namespace = ?3",
            (now, key, namespace),
        )?;
        Ok(())
    }

    fn generate_summary(&self, text: &str) -> String {
        // Simple extractive summary: first 200 chars + "..."
        if text.len() <= 200 {
            text.to_string()
        } else {
            format!("{}...", &text[..200])
        }
    }

    fn calculate_embedding_similarity(
        &self,
        emb_id1: i64,
        emb_id2: i64,
        store: &VectorStore,
    ) -> Result<f32> {
        // Simplified similarity calculation
        // In production, use proper cosine similarity from vector store
        // For now, use a placeholder based on ID proximity
        let diff = (emb_id1 - emb_id2).abs() as f32;
        Ok((1.0 - (diff / 1000.0)).max(0.0))
    }
}
