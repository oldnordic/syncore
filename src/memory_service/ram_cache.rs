//! RAM Cache for Short-Term Memory
//!
//! Provides:
//! - Fast in-memory storage with HashMap
//! - Vector similarity search using cosine similarity
//! - LRU eviction when capacity exceeded
//! - Deterministic ordering

use super::error::MemoryError;
use std::collections::HashMap;

/// Memory entry stored in cache
#[derive(Debug, Clone)]
pub struct MemoryEntry {
    pub id: String,
    pub summary: String,
    pub importance: f32,
    pub tags: Vec<String>,
    pub embedding: Vec<f32>,
}

/// RAM-based short-term memory cache
pub struct RamCache {
    /// Vector dimension
    dimension: usize,
    /// Maximum number of entries
    capacity: usize,
    /// Storage: ID -> Entry
    entries: HashMap<String, MemoryEntry>,
    /// Insertion order for LRU eviction
    insertion_order: Vec<String>,
}

impl RamCache {
    /// Create new RAM cache
    ///
    /// # Arguments
    /// * `dimension` - Embedding vector dimension
    /// * `capacity` - Maximum number of entries before eviction
    pub fn new(dimension: usize, capacity: usize) -> Self {
        Self {
            dimension,
            capacity,
            entries: HashMap::new(),
            insertion_order: Vec::new(),
        }
    }

    /// Insert entry into cache
    ///
    /// If capacity is exceeded, evicts oldest entry (FIFO)
    pub fn insert(&mut self, entry: MemoryEntry) -> Result<(), MemoryError> {
        // Validate dimension
        if entry.embedding.len() != self.dimension {
            return Err(MemoryError::DimensionMismatch);
        }

        // If entry already exists, remove it from insertion order
        if self.entries.contains_key(&entry.id) {
            self.insertion_order.retain(|id| id != &entry.id);
        }

        // Evict oldest entry if at capacity
        if self.entries.len() >= self.capacity && !self.entries.contains_key(&entry.id) {
            if let Some(oldest_id) = self.insertion_order.first().cloned() {
                self.entries.remove(&oldest_id);
                self.insertion_order.remove(0);
            }
        }

        // Insert new entry
        let id = entry.id.clone();
        self.entries.insert(id.clone(), entry);
        self.insertion_order.push(id);

        Ok(())
    }

    /// Search for entries by cosine similarity
    ///
    /// Returns top-k entries ordered by similarity (descending)
    /// Deterministic ordering: on similarity ties, sorts by ID (ascending)
    pub fn search(&self, query_embedding: &[f32], k: usize) -> Vec<MemoryEntry> {
        if self.entries.is_empty() {
            return Vec::new();
        }

        // Calculate cosine similarity for all entries
        let mut scored: Vec<(String, f32)> = self
            .entries
            .iter()
            .map(|(id, entry)| {
                let similarity = cosine_similarity(query_embedding, &entry.embedding);
                (id.clone(), similarity)
            })
            .collect();

        // Sort by similarity (descending), then by ID (ascending) for deterministic ordering
        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal).then_with(|| a.0.cmp(&b.0))
        });

        // Return top-k entries
        scored
            .into_iter()
            .take(k)
            .filter_map(|(id, _score)| self.entries.get(&id).cloned())
            .collect()
    }

    /// Get number of entries in cache
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Calculate cosine similarity between two vectors
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        // Test orthogonal vectors (similarity = 0)
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 0.0).abs() < 0.001);

        // Test identical vectors (similarity = 1)
        let a = vec![1.0, 1.0, 1.0];
        let b = vec![1.0, 1.0, 1.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        // Test opposite vectors (similarity = -1)
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![-1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - (-1.0)).abs() < 0.001);
    }
}
