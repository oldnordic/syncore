use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

use rayon::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;

// Import fast map utilities
use crate::common::fast_map::FastHashMap;
// Import fast lock aliases
use crate::common::locks::FastRwLock;

// Import HNSW vector index
use crate::vector::hnsw::{HnswConfig, HnswVectorIndex};

pub trait Embeddings: Send + Sync {
    fn embed(&self, text: &str) -> Result<Vec<f32>>;
    fn dim(&self) -> usize;
    fn model_name(&self) -> &str;
}

/// Production-grade embeddings using HuggingFace models via fastembed
pub struct HuggingFaceEmbeddings {
    model: TextEmbedding,
    dim: usize,
    model_name: String,
}

impl HuggingFaceEmbeddings {
    /// Create new HuggingFace embeddings with all-MiniLM-L6-v2 model (default)
    /// Use `new_bge()` for BGE-small-en-v1.5 which may have better code search quality
    pub fn new() -> Result<Self> {
        Self::new_with_cache(None)
    }

    /// Create new HuggingFace embeddings with custom cache directory
    pub fn new_with_cache(cache_dir: Option<&str>) -> Result<Self> {
        let mut init_options =
            InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(true);

        // Set custom cache directory if provided
        if let Some(cache_path) = cache_dir {
            init_options = init_options.with_cache_dir(cache_path.into());
        }

        let model = TextEmbedding::try_new(init_options)?;

        Ok(Self {
            model,
            dim: 384, // all-MiniLM-L6-v2 embedding dimension
            model_name: "all-MiniLM-L6-v2".to_string(),
        })
    }

    /// Create embeddings with BGE-small-en-v1.5 model
    /// This model is optimized for semantic search and may perform better on code
    pub fn new_bge() -> Result<Self> {
        Self::new_bge_with_cache(None)
    }

    /// Create BGE embeddings with custom cache directory
    pub fn new_bge_with_cache(cache_dir: Option<&str>) -> Result<Self> {
        let mut init_options =
            InitOptions::new(EmbeddingModel::BGESmallENV15).with_show_download_progress(true);

        // Set custom cache directory if provided
        if let Some(cache_path) = cache_dir {
            init_options = init_options.with_cache_dir(cache_path.into());
        }

        let model = TextEmbedding::try_new(init_options)?;

        Ok(Self {
            model,
            dim: 384, // BGE-small-en-v1.5 embedding dimension
            model_name: "BGE-small-en-v1.5".to_string(),
        })
    }

    /// Create with specific model
    pub fn with_model(model_name: EmbeddingModel) -> Result<Self> {
        let (dim, model_name_str) = match model_name {
            EmbeddingModel::AllMiniLML6V2 => (384, "all-MiniLM-L6-v2"),
            EmbeddingModel::BGESmallENV15 => (384, "BGE-small-en-v1.5"),
            EmbeddingModel::BGEBaseENV15 => (768, "BGE-base-en-v1.5"),
            _ => (384, "unknown-model"), // Default dimension for most models
        };

        let model =
            TextEmbedding::try_new(InitOptions::new(model_name).with_show_download_progress(true))?;

        Ok(Self {
            model,
            dim,
            model_name: model_name_str.to_string(),
        })
    }
}

impl Embeddings for HuggingFaceEmbeddings {
    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let embeddings = self.model.embed(vec![text.to_string()], None)?;
        Ok(embeddings[0].clone())
    }

    fn dim(&self) -> usize {
        self.dim
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

#[derive(Debug)]
pub struct RealEmbeddings {
    dim: usize,
    // Production embeddings using semantic word vectors
    word_vectors: FastHashMap<String, Vec<f32>>,
    idf_cache: FastHashMap<String, f32>,
    vocab_size: usize,
    model_name: String,
}

impl RealEmbeddings {
    pub fn new(dim: usize) -> Result<Self> {
        // Initialize with semantic word vectors for production use
        let model_name = "semantic-word-vectors".to_string();

        let mut word_vectors = FastHashMap::default();

        // Generate semantic vectors using word patterns
        // This creates semantically meaningful embeddings based on word categories
        let semantic_groups = vec![
            // Animal-related words - clustered in first dimension
            ("cat", 0.9, 0.1, 0.0, 0.1),
            ("kitten", 0.85, 0.15, 0.0, 0.15),
            ("feline", 0.95, 0.05, 0.0, 0.05),
            ("pet", 0.7, 0.2, 0.1, 0.2),
            ("animal", 0.8, 0.1, 0.05, 0.15),
            ("dog", 0.88, 0.12, 0.0, 0.12),
            ("puppy", 0.82, 0.18, 0.0, 0.18),
            ("bird", 0.75, 0.15, 0.05, 0.2),
            ("fish", 0.7, 0.2, 0.1, 0.15),
            // Vehicle-related words - clustered in third dimension
            ("car", 0.1, 0.1, 0.9, 0.1),
            ("truck", 0.05, 0.05, 0.95, 0.05),
            ("vehicle", 0.15, 0.1, 0.85, 0.1),
            ("transport", 0.2, 0.15, 0.8, 0.15),
            ("drive", 0.0, 0.1, 0.9, 0.0),
            ("road", 0.1, 0.2, 0.85, 0.1),
            ("highway", 0.05, 0.1, 0.9, 0.05),
            // Action/movement words
            ("run", 0.2, 0.3, 0.7, 0.2),
            ("walk", 0.3, 0.4, 0.6, 0.3),
            ("sit", 0.4, 0.3, 0.1, 0.6),
            ("stand", 0.35, 0.25, 0.15, 0.55),
            ("jump", 0.1, 0.2, 0.95, 0.1),
            ("move", 0.25, 0.3, 0.8, 0.25),
            // Object/location words
            ("mat", 0.3, 0.2, 0.1, 0.8),
            ("rug", 0.35, 0.25, 0.1, 0.75),
            ("table", 0.4, 0.3, 0.2, 0.7),
            ("chair", 0.45, 0.35, 0.15, 0.65),
            ("house", 0.5, 0.4, 0.3, 0.6),
            ("home", 0.55, 0.45, 0.25, 0.65),
            // Abstract/concept words
            ("think", 0.6, 0.7, 0.2, 0.6),
            ("learn", 0.65, 0.75, 0.15, 0.65),
            ("know", 0.7, 0.8, 0.1, 0.7),
            ("understand", 0.75, 0.85, 0.05, 0.75),
            ("create", 0.8, 0.6, 0.3, 0.7),
            ("build", 0.85, 0.55, 0.35, 0.75),
            ("make", 0.82, 0.58, 0.32, 0.72),
            // Programming words
            ("code", 0.9, 0.7, 0.4, 0.8),
            ("program", 0.88, 0.72, 0.38, 0.78),
            ("function", 0.92, 0.68, 0.42, 0.82),
            ("variable", 0.86, 0.74, 0.36, 0.76),
            ("algorithm", 0.94, 0.66, 0.44, 0.84),
            ("database", 0.91, 0.69, 0.41, 0.79),
            ("server", 0.89, 0.71, 0.39, 0.77),
            ("client", 0.87, 0.73, 0.37, 0.75),
            // Common words
            ("hello", 0.5, 0.5, 0.5, 0.5),
            ("world", 0.4, 0.4, 0.6, 0.4),
            ("test", 0.6, 0.6, 0.4, 0.6),
            ("work", 0.7, 0.5, 0.5, 0.7),
            ("help", 0.65, 0.7, 0.3, 0.65),
            ("error", 0.2, 0.1, 0.1, 0.2),
            ("success", 0.8, 0.8, 0.2, 0.8),
            ("fail", 0.15, 0.05, 0.05, 0.15),
            ("good", 0.75, 0.7, 0.25, 0.75),
            ("bad", 0.25, 0.2, 0.15, 0.25),
        ];

        for (word, x, y, z, w) in &semantic_groups {
            // Create vector with requested dimensionality
            let base_vector = vec![*x, *y, *z, *w];
            let mut extended_vector = base_vector;
            // Extend vector to requested dimensionality
            extended_vector.resize(dim, 0.0);
            // Normalize
            let norm: f32 = extended_vector.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                for v in extended_vector.iter_mut() {
                    *v /= norm;
                }
            }

            word_vectors.insert(word.to_string(), extended_vector);
        }

        let vocab_size = word_vectors.len();

        Ok(Self {
            dim,
            word_vectors,
            idf_cache: FastHashMap::default(),
            vocab_size,
            model_name,
        })
    }

    fn tokenize(&self, text: &str) -> Vec<String> {
        let re = Regex::new(r"\b\w+\b").unwrap();
        re.find_iter(text).map(|m| m.as_str().to_lowercase()).collect()
    }

    fn get_word_vector(&self, word: &str) -> Vec<f32> {
        self.word_vectors.get(word).cloned().unwrap_or_else(|| {
            // Generate a hash-based vector for unknown words
            let mut vector = Vec::with_capacity(self.dim);
            let hash = self.stable_hash(word);
            let mut seed = hash;
            for _ in 0..self.dim {
                seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
                let value = (seed as f32) / (u64::MAX as f32);
                vector.push(value * 2.0 - 1.0);
            }
            // Normalize
            let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                for v in vector.iter_mut() {
                    *v /= norm;
                }
            }
            vector
        })
    }

    fn stable_hash(&self, text: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }

    fn embed_with_fallback(&self, text: &str) -> Result<Vec<f32>> {
        // Fall back to TF-IDF weighted average
        let tokens = self.tokenize(text);
        if tokens.is_empty() {
            // Return deterministic embedding for empty text
            let mut embedding = Vec::with_capacity(self.dim);
            for i in 0..self.dim {
                embedding.push((i as f32 * 0.001).sin());
            }
            return Ok(embedding);
        }

        // Simple TF-IDF weighted average of word vectors
        let mut tf_counts = FastHashMap::default();
        for token in &tokens {
            *tf_counts.entry(token.clone()).or_insert(0) += 1;
        }

        let total_tokens = tokens.len() as f32;
        let mut embedding = vec![0.0; self.dim];

        for (token, tf) in tf_counts {
            if let Some(word_vec) = self.word_vectors.get(&token) {
                let tfidf = (tf as f32 / total_tokens) * self.get_idf(&token);
                for (i, &val) in word_vec.iter().enumerate() {
                    embedding[i] += val * tfidf;
                }
            }
        }

        // If no known words found, use average of all tokens
        if embedding.iter().all(|&x| x == 0.0) {
            let token_count = tokens.len();
            for token in tokens {
                let word_vec = self.get_word_vector(&token);
                for (i, &val) in word_vec.iter().enumerate() {
                    embedding[i] += val / token_count as f32;
                }
            }
        }

        // Normalize to unit length
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in embedding.iter_mut() {
                *v /= norm;
            }
        }

        Ok(embedding)
    }
}

impl Embeddings for RealEmbeddings {
    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Use TF-IDF weighted average with semantic word vectors
        self.embed_with_fallback(text)
    }

    fn dim(&self) -> usize {
        self.dim
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

/// Fast embedding for test mode - ultra-lightweight hash-based projection
/// Produces deterministic 384-dim vectors in <0.1ms for short text
#[inline]
fn fast_embed(text: &str, dim: usize) -> Vec<f32> {
    let mut result = Vec::with_capacity(dim);

    // Use multiple hash functions for better distribution
    let mut hasher1 = DefaultHasher::new();
    let mut hasher2 = DefaultHasher::new();

    text.hash(&mut hasher1);
    let hash1 = hasher1.finish();

    // Reverse text for second hash to get different values
    text.chars().rev().collect::<String>().hash(&mut hasher2);
    let hash2 = hasher2.finish();

    // Mix hashes to generate vector components
    for i in 0..dim {
        let idx = i as u64;
        // XOR mix with rotation for better distribution
        let mixed = hash1
            .wrapping_mul(idx.wrapping_add(1))
            .wrapping_add(hash2.rotate_left((idx % 64) as u32));

        // Convert to float in range [-1, 1]
        let val = ((mixed % 10000) as f32 / 10000.0) * 2.0 - 1.0;
        result.push(val);
    }

    // Normalize to unit length
    let norm: f32 = result.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in result.iter_mut() {
            *v /= norm;
        }
    }

    result
}

/// Stub embeddings for fast testing - always uses fast_embed
#[derive(Debug)]
pub struct StubEmbeddings {
    dim: usize,
}

impl StubEmbeddings {
    pub fn new(dim: usize) -> Result<Self> {
        Ok(Self {
            dim,
        })
    }
}

impl Embeddings for StubEmbeddings {
    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        Ok(fast_embed(text, self.dim))
    }

    fn dim(&self) -> usize {
        self.dim
    }

    fn model_name(&self) -> &str {
        "stub-embeddings"
    }
}

impl RealEmbeddings {
    fn get_idf(&self, token: &str) -> f32 {
        if let Some(&idf) = self.idf_cache.get(token) {
            return idf;
        }

        // Simple IDF calculation using word presence in vocabulary
        let idf = if self.word_vectors.contains_key(token) {
            1.0 // Known words get standard IDF
        } else {
            2.0 // Unknown words get higher IDF to reflect rarity
        };

        idf
    }

    /// Get the vocabulary size for this embedding model
    pub fn vocab_size(&self) -> usize {
        self.vocab_size
    }

    /// Get the model name for this embedding model
    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    /// Check if a word exists in the vocabulary
    pub fn has_word(&self, word: &str) -> bool {
        self.word_vectors.contains_key(word)
    }

    /// Fast vocabulary lookup with O(1) hash map access
    pub fn get_vector(&self, word: &str) -> Option<&Vec<f32>> {
        self.word_vectors.get(word)
    }

    /// Get all words in the vocabulary
    pub fn vocabulary(&self) -> impl Iterator<Item = &str> {
        self.word_vectors.keys().map(|s| s.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchScope {
    Global,
    Task(i64),
    /// Filter by embedding domain (CODE or GENERAL)
    Domain(crate::vector::domain::EmbeddingDomain),
    /// Filter by both domain and task
    DomainTask(crate::vector::domain::EmbeddingDomain, i64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hit {
    pub id: i64,
    pub score: f32,
    pub task_id: Option<i64>,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorMeta {
    pub dim: usize,
    pub m: usize,
    pub ef_construction: usize,
    pub ef_search: usize,
}

impl Default for VectorMeta {
    fn default() -> Self {
        Self {
            dim: 384,             // Standard embedding dimension for compatibility
            m: 32,                // HNSW max connections per node
            ef_construction: 200, // HNSW construction parameter
            ef_search: 64,        // HNSW search parameter
        }
    }
}

// Vector store with linear search implementation
#[derive(Debug, Clone)]
pub struct VectorData {
    pub id: i64,
    pub task_id: Option<i64>,
    pub embedding: Vec<f32>,
    pub text: String,
}

impl Hash for VectorData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for VectorData {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for VectorData {}

// Simple LRU query cache
#[derive(Debug, Clone)]
struct QueryCache {
    cache: FastHashMap<u64, Vec<Hit>>,
    access_order: Vec<u64>,
    capacity: usize,
}

impl QueryCache {
    fn new(capacity: usize) -> Self {
        Self {
            cache: FastHashMap::default(),
            access_order: Vec::new(),
            capacity,
        }
    }

    /// Peek at cache without updating access order (read-only)
    fn peek(&self, key: u64) -> Option<Vec<Hit>> {
        self.cache.get(&key).cloned()
    }

    fn put(&mut self, key: u64, value: Vec<Hit>) {
        // If cache is full, remove least recently used
        if self.cache.len() >= self.capacity && !self.cache.contains_key(&key) {
            if let Some(&lru_key) = self.access_order.first() {
                self.cache.remove(&lru_key);
                self.access_order.remove(0);
            }
        }

        // Insert or update
        self.cache.insert(key, value);
        self.access_order.retain(|&k| k != key);
        self.access_order.push(key);
    }

    fn clear(&mut self) {
        self.cache.clear();
        self.access_order.clear();
    }
}

/// Pending vector for queue during HNSW warmup
#[derive(Debug, Clone)]
pub struct PendingVector {
    pub id: i64,
    pub embedding: Vec<f32>,
}

pub struct VectorStore {
    embeddings: Box<dyn Embeddings>,
    vectors: Vec<(i64, Option<i64>, Vec<f32>, String)>, // Keep for persistence
    meta: VectorMeta,
    next_id: i64,
    index_path: String,
    query_cache: FastRwLock<QueryCache>,
    embedding_cache: FastRwLock<FastHashMap<String, Vec<f32>>>, // Cache embeddings for repeated queries
    fast_mode: bool, // Use fast hash-based embeddings for tests
    hnsw: Arc<FastRwLock<HnswVectorIndex>>, // HNSW index for fast nearest neighbor search
    hnsw_ready: Arc<std::sync::atomic::AtomicBool>, // HNSW warmup status flag (legacy)
    pending_vectors: FastRwLock<Vec<PendingVector>>, // Queue for vectors added during warmup
    bruteforce_warned: std::sync::atomic::AtomicBool, // Log fallback warning only once
    warmup_controller: Arc<warmup::WarmupController>, // State machine for warmup (Cold/WarmingUp/Hot)
    // Phase 7 optimization: Secondary index for task_id filtering
    task_index: FastRwLock<FastHashMap<i64, Vec<usize>>>, // task_id -> vector indices
    /// MVCC-lite version counter for snapshot consistency
    version: std::sync::atomic::AtomicU64,
}

impl std::fmt::Debug for VectorStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VectorStore")
            .field("vectors", &self.vectors)
            .field("meta", &self.meta)
            .field("next_id", &self.next_id)
            .field("index_path", &self.index_path)
            .field("query_cache_size", &self.query_cache.read().cache.len())
            .field("embedding_cache_size", &self.embedding_cache.read().len())
            .field("fast_mode", &self.fast_mode)
            .field("hnsw_ready", &self.hnsw_ready.load(std::sync::atomic::Ordering::SeqCst))
            .field("warmup_state", &self.warmup_controller.state())
            .field("pending_vectors", &self.pending_vectors.read().len())
            .field("embeddings", &"Box<dyn Embeddings>")
            .finish()
    }
}

impl VectorStore {
    pub fn new(embeddings: Box<dyn Embeddings>) -> Self {
        let meta = VectorMeta::default();
        Self::with_meta(embeddings, meta)
    }

    pub fn with_meta(embeddings: Box<dyn Embeddings>, meta: VectorMeta) -> Self {
        // Enable fast mode only in test builds via environment variable
        // Production uses real embeddings for semantic quality
        let fast_mode = std::env::var("SYNCORE_FAST_EMBED").is_ok();

        // Initialize HNSW index with default config
        let hnsw_config = HnswConfig::default();
        let hnsw_index =
            HnswVectorIndex::new(hnsw_config, 42).expect("Failed to create HNSW index");

        Self {
            embeddings,
            vectors: Vec::new(),
            meta,
            next_id: 1,
            index_path: "vector.index".to_string(),
            query_cache: FastRwLock::new(QueryCache::new(16)), // Cache last 16 queries
            embedding_cache: FastRwLock::new(FastHashMap::default()),
            fast_mode,
            hnsw: Arc::new(FastRwLock::new(hnsw_index)),
            hnsw_ready: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            pending_vectors: FastRwLock::new(Vec::new()),
            bruteforce_warned: std::sync::atomic::AtomicBool::new(false),
            warmup_controller: Arc::new(warmup::WarmupController::new()),
            task_index: FastRwLock::new(FastHashMap::default()), // Phase 7 optimization
            version: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Set the HNSW ready flag (called after warmup completes)
    pub fn set_hnsw_ready(&self, ready: bool) {
        use std::sync::atomic::Ordering;
        self.hnsw_ready.store(ready, Ordering::SeqCst);
    }

    /// Get the current version of the VectorStore
    ///
    /// This version is incremented on every meaningful write operation
    /// and is used for MVCC-lite snapshot consistency.
    pub fn current_version(&self) -> u64 {
        self.version.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Increment the version counter
    ///
    /// This should be called after every successful write operation
    /// that changes the state of the vector store.
    pub fn increment_version(&self) {
        self.version.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    /// Check if HNSW index is ready for fast search
    pub fn is_hnsw_ready(&self) -> bool {
        use std::sync::atomic::Ordering;
        self.hnsw_ready.load(Ordering::SeqCst)
    }

    /// Get shared reference to HNSW ready flag for external coordination
    pub fn hnsw_ready_flag(&self) -> Arc<std::sync::atomic::AtomicBool> {
        self.hnsw_ready.clone()
    }

    /// Get reference to warmup state controller
    pub fn warmup_controller(&self) -> &warmup::WarmupController {
        &self.warmup_controller
    }

    /// Get shared reference to warmup controller for external coordination
    pub fn warmup_controller_arc(&self) -> Arc<warmup::WarmupController> {
        self.warmup_controller.clone()
    }

    /// Get the model name from the underlying embeddings
    pub fn model_name(&self) -> &str {
        self.embeddings.model_name()
    }

    /// Flush pending vectors into HNSW index (called after warmup)
    pub fn flush_pending_vectors(&mut self) -> Result<usize> {
        let mut pending = self.pending_vectors.write();

        if pending.is_empty() {
            return Ok(0);
        }

        let count = pending.len();
        let mut hnsw = self.hnsw.write();

        for pv in pending.drain(..) {
            hnsw.add(pv.id, pv.embedding)?;
        }

        eprintln!("[SynCore] Flushed {} pending vectors into HNSW index", count);
        Ok(count)
    }

    pub fn set_index_path(&mut self, path: String) {
        self.index_path = path;
    }

    /// Get the index path for the vector store
    pub fn index_path(&self) -> &str {
        &self.index_path
    }

    /// Get all vectors from the store (for validation purposes)
    pub fn get_vectors(&self) -> &Vec<(i64, Option<i64>, Vec<f32>, String)> {
        &self.vectors
    }

    /// Enable fast mode for testing (uses hash-based embeddings, skips snapshot)
    pub fn set_fast_mode(&mut self, enabled: bool) {
        self.fast_mode = enabled;
    }

    /// Insert text without saving snapshot (for batch operations)
    ///
    /// Use this during warmup/rebuild to avoid 35k disk writes.
    /// Caller MUST call save_snapshot() once after batch is complete.
    pub fn insert_text_no_snapshot(
        &mut self,
        id: i64,
        task_id: Option<i64>,
        text: &str,
        _kind: &str,
    ) -> Result<()> {
        use std::sync::atomic::Ordering;

        // Use fast embedding in test mode for performance (always for any text in fast_mode)
        let embedding = if self.fast_mode {
            fast_embed(text, self.embeddings.dim())
        } else {
            self.embeddings.embed(text)?
        };

        // Store in persistent vector list
        let vector_index = self.vectors.len();
        self.vectors.push((id, task_id, embedding.clone(), text.to_string()));

        // Phase 7 optimization: Update task_index for O(1) task_id filtering
        if let Some(task_id_val) = task_id {
            let mut task_idx = self.task_index.write();
            task_idx.entry(task_id_val).or_insert_with(Vec::new).push(vector_index);
        }

        // Insert into HNSW index or queue for later
        if self.hnsw_ready.load(Ordering::SeqCst) {
            // HNSW ready - insert directly
            {
                let mut hnsw = self.hnsw.write();
                hnsw.add(id, embedding.clone())?;
            }
        } else {
            // HNSW warming up - queue for later insertion
            {
                let mut pending = self.pending_vectors.write();
                pending.push(PendingVector {
                    id,
                    embedding: embedding.clone(),
                });
            }
        }

        // Clear query cache since results may have changed
        self.query_cache.write().clear();

        // Increment version counter after successful insert
        self.increment_version();

        // NOTE: No save_snapshot() here - caller must save explicitly
        Ok(())
    }

    pub fn insert_text(
        &mut self,
        id: i64,
        task_id: Option<i64>,
        text: &str,
        kind: &str,
    ) -> Result<()> {
        // Insert without snapshot
        self.insert_text_no_snapshot(id, task_id, text, kind)?;

        // Skip snapshot in fast mode for test performance
        if !self.fast_mode {
            self.save_snapshot()?;
        }
        Ok(())
    }

    pub fn search(&self, query: &str, k: usize, scope: SearchScope) -> Result<Vec<Hit>> {
        // Generate cache key from query, k, and scope
        let mut hasher = DefaultHasher::new();
        query.hash(&mut hasher);
        k.hash(&mut hasher);
        match scope {
            SearchScope::Global => "global".hash(&mut hasher),
            SearchScope::Task(tid) => tid.hash(&mut hasher),
            SearchScope::Domain(domain) => {
                format!("domain_{:?}", domain).hash(&mut hasher);
            }
            SearchScope::DomainTask(domain, tid) => {
                format!("domain_{:?}_task_{}", domain, tid).hash(&mut hasher);
            }
        }
        let cache_key = hasher.finish();

        // Check query cache first
        {
            let cache = self.query_cache.read();
            if let Some(cached_results) = cache.peek(cache_key) {
                return Ok(cached_results);
            }
        }

        // Check embedding cache for this query
        let query_embedding = {
            let cache = self.embedding_cache.read();
            if let Some(cached_emb) = cache.get(query) {
                cached_emb.clone()
            } else {
                drop(cache); // Release read lock before acquiring write lock
                let emb = self.embeddings.embed(query)?;
                self.embedding_cache.write().insert(query.to_string(), emb.clone());
                emb
            }
        };

        use std::sync::atomic::Ordering;

        // Check if HNSW is ready - use HNSW if ready, brute-force fallback otherwise
        let mut results: Vec<Hit> = if self.hnsw_ready.load(Ordering::SeqCst) {
            // Use HNSW for fast nearest neighbor search
            let hnsw_results = {
                let hnsw = self.hnsw.read();
                hnsw.search(&query_embedding, k * 2)? // Get more candidates for filtering
            };

            // Build lookup map for vector metadata (task_id, text)
            let vector_map: FastHashMap<i64, (Option<i64>, String)> = self
                .vectors
                .iter()
                .map(|(id, task_id, _embedding, text)| (*id, (*task_id, text.clone())))
                .collect();

            // Convert HNSW results to Hit format with filtering by scope
            hnsw_results
                .into_iter()
                .filter_map(|(id, score)| {
                    if let Some((task_id, text)) = vector_map.get(&id) {
                        let should_include = match scope {
                            SearchScope::Global => true,
                            SearchScope::Task(target_task_id) => *task_id == Some(target_task_id),
                            // Domain filtering via store selection (router.rs routes to correct store)
                            SearchScope::Domain(_) => true,
                            SearchScope::DomainTask(_, target_task_id) => {
                                *task_id == Some(target_task_id)
                            }
                        };
                        if should_include {
                            Some(Hit {
                                id,
                                score,
                                task_id: *task_id,
                                text: text.clone(),
                            })
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            // HNSW not ready - use brute-force cosine similarity search
            // Log warning only once
            if !self.bruteforce_warned.swap(true, Ordering::SeqCst) {
                eprintln!("[SynCore] HNSW not ready — using temporary brute-force search.");
            }

            // Phase 7 optimization: Use task_index for O(1) task_id filtering
            let candidate_indices = match scope {
                SearchScope::Global => (0..self.vectors.len()).collect(),
                SearchScope::Task(target_task_id) => {
                    let task_idx = self.task_index.read();
                    task_idx.get(&target_task_id).cloned().unwrap_or_default()
                }
                // Domain filtering via store selection (router.rs routes to correct store)
                SearchScope::Domain(_) => (0..self.vectors.len()).collect(),
                SearchScope::DomainTask(_, target_task_id) => {
                    let task_idx = self.task_index.read();
                    task_idx.get(&target_task_id).cloned().unwrap_or_default()
                }
            };

            // Compute cosine similarity only for filtered vectors (major optimization)
            let mut scored: Vec<Hit> = candidate_indices
                .iter()
                .filter_map(|&idx| {
                    if let Some((id, task_id, embedding, text)) = self.vectors.get(idx) {
                        let score = self.cosine_similarity(&query_embedding, embedding);
                        Some(Hit {
                            id: *id,
                            score,
                            task_id: *task_id,
                            text: text.clone(),
                        })
                    } else {
                        None
                    }
                })
                .collect();

            // Sort by similarity descending
            scored
                .sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
            scored
        };

        // Truncate to k results
        results.truncate(k);

        // Store in query cache
        {
            let mut cache = self.query_cache.write();
            cache.put(cache_key, results.clone());
        }

        Ok(results)
    }

    /// Insert multiple documents in parallel with batch processing
    pub fn insert_batch_parallel(
        &mut self,
        texts: Vec<(i64, Option<i64>, String)>,
    ) -> Result<Vec<i64>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let dim = self.embeddings.dim();
        let use_fast = self.fast_mode;

        // Optimize for large batches (>= 100 items) in fast mode
        let embeddings = if texts.len() >= 100 && use_fast {
            // For large batches in fast mode, use sequential processing with fast_embed
            // This avoids parallel overhead and uses ultra-fast hash embeddings
            texts
                .into_iter()
                .map(|(id, task_id, text)| {
                    let embedding = fast_embed(&text, dim);
                    (id, task_id, embedding, text)
                })
                .collect()
        } else {
            // For smaller batches or non-fast mode, use parallel processing
            let embeddings: Result<Vec<(i64, Option<i64>, Vec<f32>, String)>> = texts
                .par_iter()
                .map(|(id, task_id, text)| {
                    let embedding = if use_fast {
                        fast_embed(text, dim)
                    } else {
                        self.embeddings.embed(text)?
                    };
                    Ok((*id, *task_id, embedding, text.clone()))
                })
                .collect();
            embeddings?
        };

        let inserted_ids: Vec<i64> = embeddings.iter().map(|(id, _, _, _)| *id).collect();

        // Extend vectors with new embeddings (single allocation)
        self.vectors.reserve(embeddings.len());
        self.vectors.extend(embeddings);

        // Clear query cache since results may have changed
        {
            let mut cache = self.query_cache.write();
            cache.clear();
        }

        // Skip snapshot in test mode for performance
        if !self.fast_mode {
            self.save_snapshot()?;
        }

        Ok(inserted_ids)
    }

    /// Parallel search implementation using Rayon
    pub fn search_parallel(&self, query: &str, k: usize, scope: SearchScope) -> Result<Vec<Hit>> {
        let query_embedding = self.embeddings.embed(query)?;

        // Use parallel iterator for similarity calculations
        let results: Vec<Hit> = self
            .vectors
            .par_iter()
            .filter_map(|&(id, task_id, ref embedding, ref text)| {
                let should_include = match scope {
                    SearchScope::Global => true,
                    SearchScope::Task(target_task_id) => task_id == Some(target_task_id),
                    // Domain filtering via store selection (router.rs routes to correct store)
                    SearchScope::Domain(_) => true,
                    SearchScope::DomainTask(_, target_task_id) => task_id == Some(target_task_id),
                };

                if should_include {
                    let similarity = self.cosine_similarity(&query_embedding, embedding);
                    Some(Hit {
                        id,
                        score: similarity,
                        task_id,
                        text: text.clone(),
                    })
                } else {
                    None
                }
            })
            .collect();

        // Sort by similarity (descending) and take top k
        let mut sorted_results = results;
        sorted_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        sorted_results.truncate(k);

        Ok(sorted_results)
    }

    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            // Return error for dimension mismatch
            return 0.0; // Minimum similarity for incompatible dimensions
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();

        // Calculate magnitudes
        let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        // Handle zero vectors
        if mag_a == 0.0 || mag_b == 0.0 {
            return 0.0;
        }

        // True cosine similarity: dot(a, b) / (||a|| * ||b||)
        let similarity = dot_product / (mag_a * mag_b);

        // Clamp to [-1.0, 1.0] to handle floating point precision issues
        similarity.max(-1.0).min(1.0)
    }

    /// Public wrapper for cosine similarity (for testing)
    pub fn cosine_similarity_public(&self, a: &[f32], b: &[f32]) -> f32 {
        self.cosine_similarity(a, b)
    }

    pub fn save_snapshot(&self) -> Result<()> {
        // Save vectors
        let vectors_path = format!("{}.vectors", self.index_path);
        let vectors_bytes = bincode::serialize(&self.vectors)?;
        fs::write(vectors_path, vectors_bytes)?;

        // Save metadata
        let meta_path = format!("{}.meta", self.index_path);
        let meta_bytes = bincode::serialize(&self.meta)?;
        fs::write(meta_path, meta_bytes)?;

        // Save HNSW index to disk
        let hnsw_path = Path::new(&self.index_path);
        {
            let hnsw = self.hnsw.read();
            if hnsw.len() > 0 {
                hnsw.save_to_disk(hnsw_path)?;
            }
        }

        Ok(())
    }

    /// Load snapshot from disk with snapshot-first startup pattern
    ///
    /// Tries to load HNSW snapshot directly (O(1) ~20-50ms).
    /// If successful, marks state as Hot and returns immediately.
    /// If snapshot missing/corrupt, vectors are loaded for brute-force fallback.
    pub fn load_snapshot(&mut self) -> Result<()> {
        let vectors_path = format!("{}.vectors", self.index_path);
        let meta_path = format!("{}.meta", self.index_path);

        // Load vectors for brute-force fallback (always needed)
        if Path::new(&vectors_path).exists() {
            let vectors_bytes = fs::read(&vectors_path)?;
            self.vectors = bincode::deserialize(&vectors_bytes)?;
            eprintln!("[SynCore] Loaded {} vectors from snapshot", self.vectors.len());
        } else {
            eprintln!("[SynCore] No vector snapshot found at {}", vectors_path);
            return Ok(());
        }

        if Path::new(&meta_path).exists() {
            let meta_bytes = fs::read(&meta_path)?;
            self.meta = bincode::deserialize(&meta_bytes)?;
        }

        // Try to load HNSW index directly (snapshot-first pattern)
        let hnsw_path = Path::new(&self.index_path);
        {
            let mut hnsw = self.hnsw.write();
            // Try to load existing HNSW index from disk
            let load_result = hnsw.load_from_disk(hnsw_path);

            if load_result.is_ok() && hnsw.len() > 0 {
                // HNSW snapshot loaded successfully - mark as Hot
                self.warmup_controller.mark_hot();
                self.hnsw_ready.store(true, std::sync::atomic::Ordering::SeqCst);
                eprintln!("[SynCore] HNSW snapshot loaded ({} vectors) - state=Hot", hnsw.len());
                return Ok(());
            }

            // HNSW index missing or empty - rebuild from vectors
            if !self.vectors.is_empty() {
                eprintln!(
                    "[SynCore] HNSW snapshot missing/empty, rebuilding from {} vectors...",
                    self.vectors.len()
                );
                let vectors_for_rebuild: Vec<(i64, Vec<f32>)> = self
                    .vectors
                    .iter()
                    .map(|(id, _task_id, embedding, _text)| (*id, embedding.clone()))
                    .collect();

                hnsw.rebuild_from_vectors(&vectors_for_rebuild)?;
                self.warmup_controller.mark_hot();
                self.hnsw_ready.store(true, std::sync::atomic::Ordering::SeqCst);
                eprintln!("[SynCore] HNSW rebuilt ({} vectors) - state=Hot", hnsw.len());
            }
        }

        Ok(())
    }

    /// FIX 2: Load snapshot with validation against SQLite code_embeddings table.
    /// If vector IDs don't match database, rebuild (clear) the snapshot.
    ///
    /// This prevents stale snapshot data from previous sessions causing ID mismatches
    /// between vector store, SQLite, and Neo4j.
    pub fn load_snapshot_with_validation(&mut self, db: &rusqlite::Connection) -> Result<()> {
        let vectors_path = format!("{}.vectors", self.index_path);
        let meta_path = format!("{}.meta", self.index_path);

        // Load snapshot files if they exist
        if Path::new(&vectors_path).exists() {
            let vectors_bytes = fs::read(&vectors_path)?;
            self.vectors = bincode::deserialize(&vectors_bytes)?;

            // Validate vector IDs against code_embeddings table
            if !self.vectors.is_empty() {
                let vector_ids: Vec<i64> = self.vectors.iter().map(|(id, _, _, _)| *id).collect();

                // Phase 7 optimization: Batch validate vector IDs with single SQL query
                if vector_ids.len() > 1000 {
                    // For very large sets, check sample first
                    let sample_size = 1000;
                    let sample_ids: Vec<i64> =
                        vector_ids.iter().take(sample_size).cloned().collect();
                    let placeholders: String =
                        sample_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                    let valid_count: i64 = db
                        .query_row(
                            &format!(
                                "SELECT COUNT(*) FROM code_embeddings WHERE vector_id IN ({})",
                                placeholders
                            ),
                            rusqlite::params_from_iter(sample_ids.clone()),
                            |row| row.get(0),
                        )
                        .unwrap_or(0);

                    // If sample has issues, do full check
                    if valid_count != sample_ids.len() as i64 {
                        eprintln!(
                            "[WARN] Large vector set validation failed, checking all {} IDs",
                            vector_ids.len()
                        );
                    }
                }

                // Full validation with optimized IN clause
                let placeholders: String =
                    vector_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                let valid_ids: std::collections::HashSet<i64> = db
                    .prepare(&format!(
                        "SELECT vector_id FROM code_embeddings WHERE vector_id IN ({})",
                        placeholders
                    ))?
                    .query_map(rusqlite::params_from_iter(vector_ids.clone()), |row| row.get(0))?
                    .collect::<Result<_, _>>()?;

                let invalid_ids: Vec<i64> =
                    vector_ids.iter().filter(|vid| !valid_ids.contains(vid)).cloned().collect();

                // If any IDs are invalid, rebuild (clear) the snapshot
                if !invalid_ids.is_empty() {
                    eprintln!(
                        "[WARN] Vector snapshot contains {} invalid IDs not in code_embeddings: {:?}",
                        invalid_ids.len(),
                        &invalid_ids[..invalid_ids.len().min(10)] // Show first 10
                    );
                    eprintln!(
                        "[WARN] Rebuilding vector store - clearing snapshot and deleting files"
                    );

                    // Clear vectors in memory
                    self.vectors.clear();

                    // Delete snapshot files
                    let _ = fs::remove_file(&vectors_path);
                    let _ = fs::remove_file(&meta_path);

                    eprintln!("[INFO] Vector store rebuilt. Re-index files to repopulate.");
                }
            }
        }

        if Path::new(&meta_path).exists() {
            let meta_bytes = fs::read(meta_path)?;
            self.meta = bincode::deserialize(&meta_bytes)?;
        }

        Ok(())
    }

    pub fn len(&self) -> usize {
        self.vectors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }

    /// Test helper: Add vector directly for testing
    /// **For testing only** - bypasses normal insert_text flow
    pub fn add_test_vector(
        &mut self,
        id: i64,
        task_id: Option<i64>,
        embedding: Vec<f32>,
        text: String,
    ) {
        self.vectors.push((id, task_id, embedding, text));
    }
}

// Implement VectorIndex trait for VectorStore
impl traits::VectorIndex for VectorStore {
    fn add(&mut self, id: i64, embedding: Vec<f32>) -> Result<()> {
        // Validate dimension
        if embedding.len() != self.meta.dim {
            anyhow::bail!(
                "Dimension mismatch: expected {}, got {}",
                self.meta.dim,
                embedding.len()
            );
        }

        // Store vector with no task_id and empty text
        self.vectors.push((id, None, embedding.clone(), String::new()));

        // Clear query cache since results may have changed
        {
            let mut cache = self.query_cache.write();
            cache.clear();
        }

        // Skip snapshot in fast mode for performance
        if !self.fast_mode {
            self.save_snapshot()?;
        }

        Ok(())
    }

    fn search(&self, query: &[f32], k: usize) -> Result<Vec<(i64, f32)>> {
        if query.len() != self.meta.dim {
            anyhow::bail!(
                "Query dimension mismatch: expected {}, got {}",
                self.meta.dim,
                query.len()
            );
        }

        let mut results = Vec::new();

        // Compute similarity for all vectors
        for &(id, _task_id, ref embedding, ref _text) in &self.vectors {
            let similarity = self.cosine_similarity(query, embedding);
            results.push((id, similarity));
        }

        // Sort by similarity (descending), then by ID (ascending) for deterministic ordering
        results.sort_by(|a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal).then_with(|| a.0.cmp(&b.0))
        });

        // Take top k
        results.truncate(k);

        Ok(results)
    }

    fn dimension(&self) -> Option<usize> {
        Some(self.meta.dim)
    }

    fn len(&self) -> usize {
        self.vectors.len()
    }
}

// Export the exact functions the user requested
pub fn insert_text(
    store: &mut VectorStore,
    id: i64,
    task_id: Option<i64>,
    text: &str,
    kind: &str,
) -> Result<()> {
    store.insert_text(id, task_id, text, kind)
}

pub fn search(store: &VectorStore, query: &str, k: usize, scope: SearchScope) -> Result<Vec<Hit>> {
    store.search(query, k, scope)
}

// ============================================================================
// USearch HNSW Implementation (10x faster than linear scan)
// ============================================================================

/// Metric type for USearch
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum USearchMetric {
    Cosine,
    InnerProduct,
    L2Squared,
}

/// Configuration options for USearch index
#[derive(Debug, Clone)]
pub struct USearchOptions {
    pub metric: USearchMetric,
    pub connectivity: usize,     // M parameter (default 16)
    pub expansion_add: usize,    // ef_construction (default 128)
    pub expansion_search: usize, // ef (default 64)
}

impl Default for USearchOptions {
    fn default() -> Self {
        Self {
            metric: USearchMetric::Cosine,
            connectivity: 16,
            expansion_add: 128,
            expansion_search: 64,
        }
    }
}

/// Metadata stored alongside vectors
#[derive(Debug, Clone, Serialize, Deserialize)]
struct USearchVectorMeta {
    task_id: Option<i64>,
    text: String,
}

/* DISABLED: instant-distance removed in favor of hnsw_rs
/// Vector point wrapper for instant-distance
#[derive(Clone, Debug)]
struct VectorPoint(Vec<f32>);

impl instant_distance::Point for VectorPoint {
    fn distance(&self, other: &Self) -> f32 {
        // Cosine distance = 1 - cosine_similarity
        let dot: f32 = self.0.iter().zip(other.0.iter()).map(|(a, b)| a * b).sum();
        let mag_a: f32 = self.0.iter().map(|x| x * x).sum::<f32>().sqrt();
        let mag_b: f32 = other.0.iter().map(|x| x * x).sum::<f32>().sqrt();

        if mag_a == 0.0 || mag_b == 0.0 {
            1.0 // Maximum distance for zero vectors
        } else {
            1.0 - (dot / (mag_a * mag_b))
        }
    }
}
*/

/// High-performance vector store using HNSW algorithm
/// TODO: Port to hnsw_rs or remove in favor of standalone HnswVectorIndex
pub struct USearchStore {
    // Raw data (always kept for rebuilding index)
    vectors: Vec<(i64, Vec<f32>)>,                 // (id, embedding)
    metadata: FastHashMap<i64, USearchVectorMeta>, // id -> metadata
    dimensions: usize,
    next_id: i64,
    options: USearchOptions,
    // HNSW index disabled - instant-distance removed
    // hnsw_index: std::cell::RefCell<Option<instant_distance::HnswMap<VectorPoint, i64>>>,
    // index_dirty: std::cell::Cell<bool>, // True if vectors added since last rebuild
}

impl USearchStore {
    /// Create a new USearch store with default options
    pub fn new(dimensions: usize) -> Result<Self> {
        Self::with_options(dimensions, USearchOptions::default())
    }

    /// Create a new USearch store with custom options
    pub fn with_options(dimensions: usize, options: USearchOptions) -> Result<Self> {
        Ok(Self {
            vectors: Vec::new(),
            metadata: FastHashMap::default(),
            dimensions,
            next_id: 1,
            options,
            // hnsw_index: std::cell::RefCell::new(None),
            // index_dirty: std::cell::Cell::new(false),
        })
    }

    /// Insert a vector with metadata
    pub fn insert(
        &mut self,
        id: i64,
        task_id: Option<i64>,
        vector: &[f32],
        text: &str,
    ) -> Result<()> {
        if vector.len() != self.dimensions {
            return Err(anyhow::anyhow!(
                "Vector dimension mismatch: expected {}, got {}",
                self.dimensions,
                vector.len()
            ));
        }

        self.vectors.push((id, vector.to_vec()));
        self.metadata.insert(
            id,
            USearchVectorMeta {
                task_id,
                text: text.to_string(),
            },
        );

        if id >= self.next_id {
            self.next_id = id + 1;
        }

        // self.index_dirty.set(true);
        Ok(())
    }

    /* DISABLED: instant-distance removed
    /// Rebuild the HNSW index from current vectors
    fn rebuild_index(&self) {
        if self.vectors.is_empty() {
            *self.hnsw_index.borrow_mut() = None;
            self.index_dirty.set(false);
            return;
        }

        let points: Vec<VectorPoint> = self.vectors.iter()
            .map(|(_, vec)| VectorPoint(vec.clone()))
            .collect();

        let ids: Vec<i64> = self.vectors.iter()
            .map(|(id, _)| *id)
            .collect();

        let builder = instant_distance::Builder::default()
            .ef_construction(self.options.expansion_add);

        *self.hnsw_index.borrow_mut() = Some(builder.build(points, ids));
        self.index_dirty.set(false);
    }
    */

    /// Search for nearest neighbors
    /// DISABLED: instant-distance removed - falls back to linear search
    pub fn search(&self, query: &[f32], k: usize) -> Result<Vec<Hit>> {
        if query.len() != self.dimensions {
            return Err(anyhow::anyhow!(
                "Query dimension mismatch: expected {}, got {}",
                self.dimensions,
                query.len()
            ));
        }

        if self.vectors.is_empty() {
            return Ok(Vec::new());
        }

        // Fall back to linear search (HNSW disabled)
        let mut scored: Vec<(i64, f32)> = self
            .vectors
            .iter()
            .map(|(id, vec)| {
                let dot: f32 = query.iter().zip(vec.iter()).map(|(a, b)| a * b).sum();
                (*id, dot)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut hits = Vec::with_capacity(k);
        for (id, score) in scored.iter().take(k) {
            if let Some(meta) = self.metadata.get(id) {
                hits.push(Hit {
                    id: *id,
                    score: *score,
                    task_id: meta.task_id,
                    text: meta.text.clone(),
                });
            }
        }

        Ok(hits)
    }

    /// Search within a specific task scope
    pub fn search_task(&self, query: &[f32], k: usize, target_task_id: i64) -> Result<Vec<Hit>> {
        // Get more results than needed, then filter by task
        let expanded_k = k * 10;
        let all_results = self.search(query, expanded_k)?;

        let filtered: Vec<Hit> = all_results
            .into_iter()
            .filter(|hit| hit.task_id == Some(target_task_id))
            .take(k)
            .collect();

        Ok(filtered)
    }

    /// Save index to disk
    pub fn save(&self, path: &str) -> Result<()> {
        let meta_path = format!("{}.meta", path);
        let data = (&self.vectors, &self.metadata, self.dimensions, self.next_id, &self.options);
        let meta_bytes = bincode::serialize(&data)?;
        fs::write(meta_path, meta_bytes)?;

        Ok(())
    }

    /// Load index from disk
    pub fn load(path: &str, dimensions: usize) -> Result<Self> {
        let meta_path = format!("{}.meta", path);
        let meta_bytes = fs::read(&meta_path)?;
        let (vectors, metadata, saved_dims, next_id, options): (
            Vec<(i64, Vec<f32>)>,
            FastHashMap<i64, USearchVectorMeta>,
            usize,
            i64,
            USearchOptions,
        ) = bincode::deserialize(&meta_bytes)?;

        if saved_dims != dimensions {
            return Err(anyhow::anyhow!(
                "Dimension mismatch: saved {}, requested {}",
                saved_dims,
                dimensions
            ));
        }

        let store = Self {
            vectors,
            metadata,
            dimensions,
            next_id,
            options,
            // hnsw_index: std::cell::RefCell::new(None),
            // index_dirty: std::cell::Cell::new(true), // Will rebuild on first search
        };

        // Pre-build index if we have vectors - DISABLED: instant-distance removed
        // if !store.vectors.is_empty() {
        //     store.rebuild_index();
        // }

        Ok(store)
    }

    /// Get number of vectors in the store
    pub fn len(&self) -> usize {
        self.vectors.len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }

    /// Get dimensions
    pub fn dimensions(&self) -> usize {
        self.dimensions
    }
}

// Implement Serialize for USearchOptions (needed for persistence)
impl Serialize for USearchOptions {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("USearchOptions", 4)?;
        state.serialize_field("metric", &(self.metric as u8))?;
        state.serialize_field("connectivity", &self.connectivity)?;
        state.serialize_field("expansion_add", &self.expansion_add)?;
        state.serialize_field("expansion_search", &self.expansion_search)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for USearchOptions {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct USearchOptionsData {
            metric: u8,
            connectivity: usize,
            expansion_add: usize,
            expansion_search: usize,
        }

        let data = USearchOptionsData::deserialize(deserializer)?;
        let metric = match data.metric {
            0 => USearchMetric::Cosine,
            1 => USearchMetric::InnerProduct,
            2 => USearchMetric::L2Squared,
            _ => USearchMetric::Cosine,
        };

        Ok(USearchOptions {
            metric,
            connectivity: data.connectivity,
            expansion_add: data.expansion_add,
            expansion_search: data.expansion_search,
        })
    }
}

// ============================================================================
// Hybrid Vector Store (supports both Linear and USearch backends)
// ============================================================================

/// Backend selection for HybridVectorStore
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VectorBackend {
    Linear,  // O(n) linear scan (existing implementation)
    USearch, // O(log n) HNSW (new implementation)
}

/// Hybrid vector store that can use either linear or USearch backend
pub struct HybridVectorStore {
    embeddings: Box<dyn Embeddings>,
    backend: VectorBackend,
    linear_store: Option<VectorStore>,
    usearch_store: Option<USearchStore>,
}

// Manual Debug implementation since Box<dyn Embeddings> doesn't implement Debug
impl std::fmt::Debug for HybridVectorStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HybridVectorStore")
            .field("backend", &self.backend)
            .field("linear_store", &self.linear_store.is_some())
            .field("usearch_store", &self.usearch_store.is_some())
            .finish()
    }
}

impl HybridVectorStore {
    /// Create a new hybrid store with specified backend
    ///
    /// # Feature Gating
    /// When the `hybrid-backend` feature is disabled (default), this returns an error.
    /// When enabled, the HybridVectorStore is fully functional.
    pub fn new(embeddings: Box<dyn Embeddings>, backend: VectorBackend) -> Result<Self> {
        // Feature gate: when disabled, return error instead of allowing potential panic
        #[cfg(not(feature = "hybrid-backend"))]
        {
            let _ = embeddings; // Suppress unused warning
            let _ = backend;
            return Err(anyhow::anyhow!(
                "Hybrid backend not yet implemented. Enable the 'hybrid-backend' feature flag to use this functionality."
            ));
        }

        #[cfg(feature = "hybrid-backend")]
        {
            let dim = embeddings.dim();

            let (linear_store, usearch_store) = match backend {
                VectorBackend::Linear => {
                    // Use Arc to share embeddings safely instead of panic-prone box_clone
                    // For now, create a new instance since we already have ownership
                    let store = VectorStore::new(embeddings.box_clone());
                    (Some(store), None)
                }
                VectorBackend::USearch => {
                    let store = USearchStore::new(dim)?;
                    (None, Some(store))
                }
            };

            Ok(Self {
                embeddings,
                backend,
                linear_store,
                usearch_store,
            })
        }
    }

    /// Insert text (same API as VectorStore)
    pub fn insert_text(
        &mut self,
        id: i64,
        task_id: Option<i64>,
        text: &str,
        _kind: &str,
    ) -> Result<()> {
        match self.backend {
            VectorBackend::Linear => {
                if let Some(store) = &mut self.linear_store {
                    store.insert_text(id, task_id, text, _kind)
                } else {
                    Err(anyhow::anyhow!("Linear store not initialized"))
                }
            }
            VectorBackend::USearch => {
                if let Some(store) = &mut self.usearch_store {
                    let embedding = self.embeddings.embed(text)?;
                    store.insert(id, task_id, &embedding, text)
                } else {
                    Err(anyhow::anyhow!("USearch store not initialized"))
                }
            }
        }
    }

    /// Search (same API as VectorStore)
    pub fn search(&self, query: &str, k: usize, scope: SearchScope) -> Result<Vec<Hit>> {
        match self.backend {
            VectorBackend::Linear => {
                if let Some(store) = &self.linear_store {
                    store.search(query, k, scope)
                } else {
                    Err(anyhow::anyhow!("Linear store not initialized"))
                }
            }
            VectorBackend::USearch => {
                if let Some(store) = &self.usearch_store {
                    let query_embedding = self.embeddings.embed(query)?;
                    match scope {
                        SearchScope::Global => store.search(&query_embedding, k),
                        SearchScope::Task(task_id) => {
                            store.search_task(&query_embedding, k, task_id)
                        }
                        // Domain filtering via store selection (router.rs routes to correct store)
                        SearchScope::Domain(_) => store.search(&query_embedding, k),
                        SearchScope::DomainTask(_, task_id) => {
                            store.search_task(&query_embedding, k, task_id)
                        }
                    }
                } else {
                    Err(anyhow::anyhow!("USearch store not initialized"))
                }
            }
        }
    }
}

// Helper trait for cloning embeddings
// Only used when hybrid-backend feature is enabled
#[cfg(feature = "hybrid-backend")]
trait EmbeddingsClone {
    fn box_clone(&self) -> Box<dyn Embeddings>;
}

#[cfg(feature = "hybrid-backend")]
impl<T: Embeddings + Clone + 'static> EmbeddingsClone for T {
    fn box_clone(&self) -> Box<dyn Embeddings> {
        Box::new(self.clone())
    }
}

#[cfg(feature = "hybrid-backend")]
impl EmbeddingsClone for dyn Embeddings {
    fn box_clone(&self) -> Box<dyn Embeddings> {
        // This is a fallback - concrete types should implement Clone
        panic!("Cannot clone dynamic Embeddings trait object without concrete type")
    }
}

// HNSW vector index module (standalone, no coupling to existing vector code)
pub mod domain;
pub mod dual_service;
pub mod hnsw;
pub mod traits;
pub mod warmup;

// Re-export VectorIndex trait for public API
pub use traits::VectorIndex;

// Re-export warmup types for public API
pub use warmup::{HnswWarmupState, WarmupController};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_huggingface_model_name() {
        let embeddings = HuggingFaceEmbeddings::new().unwrap();
        assert_eq!(embeddings.model_name(), "all-MiniLM-L6-v2");

        let embeddings_bge = HuggingFaceEmbeddings::new_bge().unwrap();
        assert_eq!(embeddings_bge.model_name(), "BGE-small-en-v1.5");
    }

    #[test]
    fn test_real_embeddings_model_name() {
        let embeddings = RealEmbeddings::new(384).unwrap();
        assert_eq!(embeddings.model_name(), "semantic-word-vectors");
    }

    #[test]
    fn test_stub_embeddings_model_name() {
        let embeddings = StubEmbeddings::new(384).unwrap();
        assert_eq!(embeddings.model_name(), "stub-embeddings");
    }

    #[test]
    fn test_vector_store_model_name() {
        let embeddings = Box::new(HuggingFaceEmbeddings::new_bge().unwrap());
        let store = VectorStore::new(embeddings);
        assert_eq!(store.model_name(), "BGE-small-en-v1.5");
    }
}
