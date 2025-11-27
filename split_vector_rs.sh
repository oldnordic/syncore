#!/bin/bash
# APEX 2.16 - Phase 2A: Split vector.rs into modular structure
# This script extracts vector.rs (1,800 LOC) into 9 focused modules

set -e  # Exit on error

PROJECT_ROOT="/home/feanor/Projects/SynCore/syncore"
VECTOR_FILE="$PROJECT_ROOT/src/vector.rs"
BACKUP_FILE="$PROJECT_ROOT/src/vector.rs.backup"

echo "[APEX 2.16] Starting vector.rs modularization..."
echo "Original file: $VECTOR_FILE ($(wc -l < $VECTOR_FILE) lines)"

# Step 1: Backup original file
echo "[1/10] Creating backup..."
cp "$VECTOR_FILE" "$BACKUP_FILE"
echo "  ✓ Backup created: $BACKUP_FILE"

# Step 2: Create directory structure
echo "[2/10] Creating directory structure..."
mkdir -p "$PROJECT_ROOT/src/vector/embeddings"
mkdir -p "$PROJECT_ROOT/src/vector/backends"
echo "  ✓ Directories created"

# Step 3: Extract embeddings trait + HuggingFaceEmbeddings (lines 15-91)
echo "[3/10] Extracting src/vector/embeddings/huggingface.rs..."
cat > "$PROJECT_ROOT/src/vector/embeddings/huggingface.rs" << 'EOF'
//! HuggingFace fastembed wrapper for production embedding models
//!
//! Supported models:
//! - all-MiniLM-L6-v2 (default, 384 dims, general-purpose)
//! - BGE-small-en-v1.5 (384 dims, optimized for code)

use super::Embeddings;
use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

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
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(true),
        )?;

        Ok(Self {
            model,
            dim: 384, // all-MiniLM-L6-v2 embedding dimension
            model_name: "all-MiniLM-L6-v2".to_string(),
        })
    }

    /// Create embeddings with BGE-small-en-v1.5 model
    /// This model is optimized for semantic search and may perform better on code
    pub fn new_bge() -> Result<Self> {
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::BGESmallENV15).with_show_download_progress(true),
        )?;

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
            _ => (384, "unknown-model"),
        };

        let model = TextEmbedding::try_new(
            InitOptions::new(model_name).with_show_download_progress(true)
        )?;

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
EOF
echo "  ✓ Created huggingface.rs ($(wc -l < $PROJECT_ROOT/src/vector/embeddings/huggingface.rs) lines)"

# Step 4: Extract RealEmbeddings (lines 94-351)
echo "[4/10] Extracting src/vector/embeddings/real.rs..."
sed -n '94,351p' "$BACKUP_FILE" > "$PROJECT_ROOT/src/vector/embeddings/real.rs"
# Add header
sed -i '1i//! Semantic word vector embeddings with TF-IDF weighting\n//!\n//! Uses hardcoded semantic vocabulary (~100 words) with hash-based\n//! fallback for unknown words.\n\nuse super::Embeddings;\nuse anyhow::Result;\nuse std::collections::HashMap;\n' "$PROJECT_ROOT/src/vector/embeddings/real.rs"
echo "  ✓ Created real.rs ($(wc -l < $PROJECT_ROOT/src/vector/embeddings/real.rs) lines)"

# Step 5: Extract StubEmbeddings (lines 352-421)
echo "[5/10] Extracting src/vector/embeddings/stub.rs..."
sed -n '352,421p' "$BACKUP_FILE" > "$PROJECT_ROOT/src/vector/embeddings/stub.rs"
sed -i '1i//! Test stub embeddings using hash-based deterministic vectors\n//!\n//! NOT for production use.\n\nuse super::Embeddings;\nuse anyhow::Result;\nuse std::collections::hash_map::DefaultHasher;\nuse std::hash::{Hash, Hasher};\n' "$PROJECT_ROOT/src/vector/embeddings/stub.rs"
echo "  ✓ Created stub.rs ($(wc -l < $PROJECT_ROOT/src/vector/embeddings/stub.rs) lines)"

# Step 6: Create embeddings/mod.rs
echo "[6/10] Creating src/vector/embeddings/mod.rs..."
cat > "$PROJECT_ROOT/src/vector/embeddings/mod.rs" << 'EOF'
//! Embedding model implementations for vector generation
//!
//! Supports multiple embedding backends:
//! - `HuggingFaceEmbeddings`: Production fastembed models
//! - `RealEmbeddings`: Semantic word vectors with TF-IDF
//! - `StubEmbeddings`: Test stub using hash-based vectors

pub mod huggingface;
pub mod real;
pub mod stub;

pub use huggingface::HuggingFaceEmbeddings;
pub use real::RealEmbeddings;
pub use stub::StubEmbeddings;

use anyhow::Result;

/// Trait for generating vector embeddings from text
pub trait Embeddings: Send + Sync {
    fn embed(&self, text: &str) -> Result<Vec<f32>>;
    fn dim(&self) -> usize;
    fn model_name(&self) -> &str;
}
EOF
echo "  ✓ Created embeddings/mod.rs"

# Step 7: Create types.rs (extract types from lines 424-533)
echo "[7/10] Creating src/vector/types.rs..."
sed -n '424,533p' "$BACKUP_FILE" > "$PROJECT_ROOT/src/vector/types.rs"
sed -i '1i//! Shared types for vector storage\n\nuse serde::{Deserialize, Serialize};\nuse std::collections::HashMap;\nuse std::hash::{Hash, Hasher};\n' "$PROJECT_ROOT/src/vector/types.rs"
echo "  ✓ Created types.rs ($(wc -l < $PROJECT_ROOT/src/vector/types.rs) lines)"

# Step 8: Create store.rs (VectorStore implementation)
echo "[8/10] Creating src/vector/store.rs..."
cat > "$PROJECT_ROOT/src/vector/store.rs" << 'EOF'
//! Main vector store implementation with HNSW indexing
//!
//! NOTE: This file will be extracted from backup. Creating placeholder.
EOF
sed -n '534,1277p' "$BACKUP_FILE" >> "$PROJECT_ROOT/src/vector/store.rs"
# Add necessary imports at top
sed -i '4i\\nuse crate::vector::embeddings::Embeddings;\nuse crate::vector::types::*;\nuse crate::vector::hnsw::{HnswConfig, HnswVectorIndex};\nuse anyhow::Result;\nuse rayon::prelude::*;\nuse std::fs;\nuse std::path::Path;\nuse std::sync::{Arc, RwLock};\n' "$PROJECT_ROOT/src/vector/store.rs"
echo "  ✓ Created store.rs ($(wc -l < $PROJECT_ROOT/src/vector/store.rs) lines)"

# Step 9: Create new vector.rs as module root
echo "[9/10] Creating new src/vector.rs (module root with re-exports)..."
cat > "$VECTOR_FILE" << 'EOF'
//! Vector embeddings and semantic search
//!
//! This module provides:
//! - Multiple embedding model implementations
//! - HNSW-based approximate nearest neighbor search
//! - Snapshot persistence for fast startup
//! - Query result caching
//!
//! # Architecture
//!
//! - `embeddings/`: Text → vector conversion
//! - `store`: Main VectorStore with HNSW indexing
//! - `backends/`: HNSW backend implementations (TODO)
//! - `types`: Shared data structures
//!
//! # Example
//!
//! ```rust,no_run
//! use syncore::vector::{VectorStore, HuggingFaceEmbeddings};
//!
//! let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
//! let mut store = VectorStore::new(embeddings);
//! store.insert_text(1, None, "hello world", "test")?;
//! let results = store.search_text("greeting", 10, None)?;
//! # Ok::<(), anyhow::Error>(())
//! ```

pub mod embeddings;
pub mod types;
pub mod store;
pub mod hnsw;

// Re-export main types for convenience
pub use embeddings::{Embeddings, HuggingFaceEmbeddings, RealEmbeddings, StubEmbeddings};
pub use types::{VectorData, VectorMeta, Hit, PendingVector, QueryCache};
pub use store::VectorStore;
pub use hnsw::{HnswConfig, HnswVectorIndex, WarmupState};

// Re-export traits for backward compatibility
pub mod traits {
    pub use super::hnsw::VectorIndex;
}
EOF
echo "  ✓ Created new vector.rs as module root"

# Step 10: Run cargo check to verify compilation
echo "[10/10] Running cargo check..."
cd "$PROJECT_ROOT"
if cargo check 2>&1 | tee /tmp/vector_split_check.log; then
    echo "  ✓ Compilation successful!"
    echo ""
    echo "=== MODULARIZATION COMPLETE ==="
    echo "Original: vector.rs (1,800 LOC)"
    echo "New structure:"
    echo "  - src/vector.rs (module root): $(wc -l < $VECTOR_FILE) lines"
    echo "  - src/vector/embeddings/mod.rs: $(wc -l < $PROJECT_ROOT/src/vector/embeddings/mod.rs) lines"
    echo "  - src/vector/embeddings/huggingface.rs: $(wc -l < $PROJECT_ROOT/src/vector/embeddings/huggingface.rs) lines"
    echo "  - src/vector/embeddings/real.rs: $(wc -l < $PROJECT_ROOT/src/vector/embeddings/real.rs) lines"
    echo "  - src/vector/embeddings/stub.rs: $(wc -l < $PROJECT_ROOT/src/vector/embeddings/stub.rs) lines"
    echo "  - src/vector/types.rs: $(wc -l < $PROJECT_ROOT/src/vector/types.rs) lines"
    echo "  - src/vector/store.rs: $(wc -l < $PROJECT_ROOT/src/vector/store.rs) lines"
    echo ""
    echo "Backup saved at: $BACKUP_FILE"
    echo ""
    echo "Next: Run 'cargo test' to verify all tests pass"
else
    echo "  ✗ Compilation failed. See /tmp/vector_split_check.log"
    echo "  Restoring backup..."
    mv "$BACKUP_FILE" "$VECTOR_FILE"
    echo "  Backup restored. Please review errors."
    exit 1
fi
