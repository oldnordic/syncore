//! MVCC-lite Snapshot Isolation
//!
//! This module provides ArcSwap-based snapshot isolation for AI queries.
//! It offers zero-blocking reads with consistent cross-domain views.
//!
//! Key features:
//! - Zero-blocking reads via ArcSwap
//! - Consistent cross-domain view for AI queries  
//! - Safe, atomic swap-in on updates
//! - No long-lived locks
//! - No writers blocking readers
//! - No readers blocking writers

use arc_swap::ArcSwap;
use std::sync::Arc;

/// Read-only metadata for CodeGraph domain
#[derive(Debug, Clone)]
pub struct CodeGraphMetadata {
    /// Entity count in the code graph
    pub entity_count: usize,
    /// Last update timestamp
    pub last_updated: std::time::SystemTime,
    /// Code graph version
    pub version: u64,
}

/// Read-only metadata for VectorStore domain
#[derive(Debug, Clone)]
pub struct VectorStoreMetadata {
    /// Vector dimension
    pub dimension: usize,
    /// Number of vectors stored
    pub vector_count: usize,
    /// HNSW index readiness
    pub hnsw_ready: bool,
    /// Last update timestamp
    pub last_updated: std::time::SystemTime,
    /// Vector store version
    pub version: u64,
}

/// Read-only metadata for Memory domain
#[derive(Debug, Clone)]
pub struct MemoryMetadata {
    /// Number of key-value pairs stored
    pub entry_count: usize,
    /// Last update timestamp
    pub last_updated: std::time::SystemTime,
    /// Memory version
    pub version: u64,
}

/// A consistent, read-only snapshot view across all domains
#[derive(Debug, Clone)]
pub struct SnapshotView {
    /// CodeGraph metadata
    pub code_graph: CodeGraphMetadata,
    /// VectorStore metadata
    pub vector_meta: VectorStoreMetadata,
    /// Memory metadata
    pub memory_meta: MemoryMetadata,
}

impl SnapshotView {
    /// Create a new snapshot view from metadata
    pub fn new(
        code_graph: CodeGraphMetadata,
        vector_meta: VectorStoreMetadata,
        memory_meta: MemoryMetadata,
    ) -> Self {
        Self {
            code_graph,
            vector_meta,
            memory_meta,
        }
    }
}

/// Atomic snapshot handle using ArcSwap for zero-blocking reads
#[derive(Debug)]
pub struct SnapshotHandle {
    /// Inner ArcSwap holding the current snapshot
    inner: ArcSwap<SnapshotView>,
}

impl SnapshotHandle {
    /// Create a new snapshot handle with initial snapshot
    pub fn new(initial_snapshot: SnapshotView) -> Self {
        Self {
            inner: ArcSwap::from(Arc::new(initial_snapshot)),
        }
    }

    /// Load the current snapshot (zero-blocking read)
    pub fn load(&self) -> Arc<SnapshotView> {
        self.inner.load().clone()
    }

    /// Atomically swap in a new snapshot
    pub fn store(&self, new_snapshot: Arc<SnapshotView>) {
        self.inner.store(new_snapshot);
    }

    /// Get a guard for the current snapshot (for advanced use cases)
    pub fn guard(&self) -> arc_swap::Guard<Arc<SnapshotView>> {
        self.inner.load()
    }
}

impl Default for SnapshotView {
    fn default() -> Self {
        let now = std::time::SystemTime::now();
        Self {
            code_graph: CodeGraphMetadata {
                entity_count: 0,
                last_updated: now,
                version: 0,
            },
            vector_meta: VectorStoreMetadata {
                dimension: 384,
                vector_count: 0,
                hnsw_ready: false,
                last_updated: now,
                version: 0,
            },
            memory_meta: MemoryMetadata {
                entry_count: 0,
                last_updated: now,
                version: 0,
            },
        }
    }
}

impl Default for SnapshotHandle {
    fn default() -> Self {
        Self::new(SnapshotView::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[test]
    fn test_snapshot_view_creation() {
        let now = std::time::SystemTime::now();
        let code_meta = CodeGraphMetadata {
            entity_count: 100,
            last_updated: now,
            version: 1,
        };
        let vector_meta = VectorStoreMetadata {
            dimension: 384,
            vector_count: 1000,
            hnsw_ready: true,
            last_updated: now,
            version: 2,
        };
        let memory_meta = MemoryMetadata {
            entry_count: 500,
            last_updated: now,
            version: 3,
        };

        let snapshot =
            SnapshotView::new(code_meta.clone(), vector_meta.clone(), memory_meta.clone());

        assert_eq!(snapshot.code_graph.entity_count, 100);
        assert_eq!(snapshot.vector_meta.vector_count, 1000);
        assert_eq!(snapshot.memory_meta.entry_count, 500);
    }

    #[test]
    fn test_snapshot_handle_atomic_swap() {
        let handle = SnapshotHandle::default();

        // Load initial snapshot
        let snap1 = handle.load();
        assert_eq!(snap1.code_graph.entity_count, 0);

        // Create new snapshot
        let new_snapshot = SnapshotView::new(
            CodeGraphMetadata {
                entity_count: 200,
                last_updated: std::time::SystemTime::now(),
                version: 1,
            },
            VectorStoreMetadata {
                dimension: 384,
                vector_count: 2000,
                hnsw_ready: true,
                last_updated: std::time::SystemTime::now(),
                version: 1,
            },
            MemoryMetadata {
                entry_count: 1000,
                last_updated: std::time::SystemTime::now(),
                version: 1,
            },
        );

        // Swap in new snapshot
        handle.store(Arc::new(new_snapshot));

        // Load updated snapshot
        let snap2 = handle.load();
        assert_eq!(snap2.code_graph.entity_count, 200);
        assert_eq!(snap2.vector_meta.vector_count, 2000);
        assert_eq!(snap2.memory_meta.entry_count, 1000);

        // Old snapshot should still be valid
        assert_eq!(snap1.code_graph.entity_count, 0);
    }

    #[test]
    fn test_concurrent_snapshot_reads() {
        let handle = Arc::new(SnapshotHandle::default());
        let barrier = Arc::new(std::sync::Barrier::new(4));
        let counter = Arc::new(AtomicU64::new(0));

        let mut handles = vec![];

        // Spawn 3 reader threads
        for _ in 0..3 {
            let handle_clone = Arc::clone(&handle);
            let counter_clone = Arc::clone(&counter);
            let barrier_clone = Arc::clone(&barrier);

            let handle = std::thread::spawn(move || {
                barrier_clone.wait();

                for _ in 0..1000 {
                    let _snap = handle_clone.load();
                    counter_clone.fetch_add(1, Ordering::Relaxed);
                }
            });

            handles.push(handle);
        }

        // Spawn 1 writer thread
        let handle_writer = Arc::clone(&handle);
        let barrier_writer = Arc::clone(&barrier);

        let writer_handle = std::thread::spawn(move || {
            barrier_writer.wait();

            for i in 1..=10 {
                let new_snapshot = SnapshotView::new(
                    CodeGraphMetadata {
                        entity_count: i * 100,
                        last_updated: std::time::SystemTime::now(),
                        version: i as u64,
                    },
                    VectorStoreMetadata {
                        dimension: 384,
                        vector_count: i * 1000,
                        hnsw_ready: true,
                        last_updated: std::time::SystemTime::now(),
                        version: i as u64,
                    },
                    MemoryMetadata {
                        entry_count: i * 500,
                        last_updated: std::time::SystemTime::now(),
                        version: i as u64,
                    },
                );

                handle_writer.store(Arc::new(new_snapshot));
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        });

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }
        writer_handle.join().unwrap();

        // Verify all reads completed
        assert_eq!(counter.load(Ordering::Relaxed), 3000);
    }
}
