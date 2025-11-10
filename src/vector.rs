use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;

pub trait Embeddings: Send + Sync {
    fn embed(&self, text: &str) -> Result<Vec<f32>>;
    fn dim(&self) -> usize;
}

#[derive(Debug, Clone)]
pub struct MockEmbeddings {
    dim: usize,
}

impl MockEmbeddings {
    pub fn new(dim: usize) -> Self {
        Self { dim }
    }

    fn stable_hash(text: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }
}

impl Embeddings for MockEmbeddings {
    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let hash = Self::stable_hash(text);
        let mut embedding = Vec::with_capacity(self.dim);

        // Generate deterministic pseudo-random values from hash
        let mut seed = hash;
        for _ in 0..self.dim {
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            let value = (seed as f32) / (u64::MAX as f32);
            embedding.push(value * 2.0 - 1.0); // Normalize to [-1, 1]
        }

        // Normalize to unit length for cosine similarity
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in embedding.iter_mut() {
                *v /= norm;
            }
        }

        Ok(embedding)
    }

    fn dim(&self) -> usize {
        self.dim
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchScope {
    Global,
    Task(i64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hit {
    pub id: i64,
    pub score: f32,
    pub task_id: Option<i64>,
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
            dim: 384, // MiniLM dimension
            m: 32,
            ef_construction: 200,
            ef_search: 64,
        }
    }
}

// Simple vector store without HNSW for now - just linear search
pub struct VectorStore {
    embeddings: Box<dyn Embeddings>,
    vectors: Vec<(i64, Option<i64>, Vec<f32>)>, // (id, task_id, embedding)
    meta: VectorMeta,
    next_id: i64,
    index_path: String,
}

impl VectorStore {
    pub fn new(embeddings: Box<dyn Embeddings>) -> Self {
        let meta = VectorMeta::default();
        Self::with_meta(embeddings, meta)
    }

    pub fn with_meta(embeddings: Box<dyn Embeddings>, meta: VectorMeta) -> Self {
        Self {
            embeddings,
            vectors: Vec::new(),
            meta,
            next_id: 1,
            index_path: "vector.index".to_string(),
        }
    }

    pub fn insert_text(&mut self, id: i64, task_id: Option<i64>, text: &str, kind: &str) -> Result<()> {
        let embedding = self.embeddings.embed(text)?;
        self.vectors.push((id, task_id, embedding));
        self.save_snapshot()?;
        Ok(())
    }

    pub fn search(&self, query: &str, k: usize, scope: SearchScope) -> Result<Vec<Hit>> {
        let query_embedding = self.embeddings.embed(query)?;

        let mut results = Vec::new();

        for &(id, task_id, ref embedding) in &self.vectors {
            match scope {
                SearchScope::Global => {
                    let similarity = self.cosine_similarity(&query_embedding, embedding);
                    results.push(Hit { id, score: similarity, task_id });
                }
                SearchScope::Task(target_task_id) => {
                    if task_id == Some(target_task_id) {
                        let similarity = self.cosine_similarity(&query_embedding, embedding);
                        results.push(Hit { id, score: similarity, task_id });
                    }
                }
            }
        }

        // Sort by similarity (descending) and take top k
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(k);

        Ok(results)
    }

    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        dot_product // Since vectors are already normalized
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

        Ok(())
    }

    pub fn load_snapshot(&mut self) -> Result<()> {
        let vectors_path = format!("{}.vectors", self.index_path);
        let meta_path = format!("{}.meta", self.index_path);

        if Path::new(&vectors_path).exists() {
            let vectors_bytes = fs::read(vectors_path)?;
            self.vectors = bincode::deserialize(&vectors_bytes)?;
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
}

// Export the exact functions the user requested
pub fn insert_text(store: &mut VectorStore, id: i64, task_id: Option<i64>, text: &str, kind: &str) -> Result<()> {
    store.insert_text(id, task_id, text, kind)
}

pub fn search(store: &VectorStore, query: &str, k: usize, scope: SearchScope) -> Result<Vec<Hit>> {
    store.search(query, k, scope)
}
