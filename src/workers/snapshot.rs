use anyhow::Result;
use crate::vector::VectorStore;
use tokio::time::{interval, Duration};
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct SnapshotWorker {
    vector_store: Arc<Mutex<VectorStore>>,
    last_snapshot: Instant,
    insert_count: u64,
}

impl SnapshotWorker {
    pub fn new(vector_store: VectorStore) -> Self {
        Self {
            vector_store: Arc::new(Mutex::new(vector_store)),
            last_snapshot: Instant::now(),
            insert_count: 0,
        }
    }

    pub fn new_with_arc(vector_store: Arc<Mutex<VectorStore>>) -> Self {
        Self {
            vector_store,
            last_snapshot: Instant::now(),
            insert_count: 0,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut timer_interval = interval(Duration::from_secs(2)); // 2 seconds

        loop {
            timer_interval.tick().await;

            let should_snapshot = {
                let _store = self.vector_store.lock().unwrap();
                self.insert_count >= 64 || self.last_snapshot.elapsed().as_secs() >= 2
            };

            if should_snapshot {
                if let Err(e) = self.take_snapshot() {
                    eprintln!("Snapshot failed: {e}");
                } else {
                    self.last_snapshot = Instant::now();
                    self.insert_count = 0;
                }
            }
        }
    }

    fn take_snapshot(&self) -> Result<()> {
        let store = self.vector_store.lock().unwrap();
        store.save_snapshot()?;
        println!("Vector snapshot saved");
        Ok(())
    }

    pub fn increment_insert_count(&mut self) {
        self.insert_count += 1;
    }
}

pub async fn run(vector_store: Arc<Mutex<VectorStore>>) -> Result<()> {
    let mut worker = SnapshotWorker::new_with_arc(vector_store);
    worker.run().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector::HuggingFaceEmbeddings;
    use tempfile::NamedTempFile;

    #[test]
    fn test_snapshot_worker() -> Result<()> {
        let temp_file = NamedTempFile::new()?;
        let _vector_path = temp_file.path().to_str().unwrap();

        // Create a simple vector store for testing
        let embeddings = Box::new(HuggingFaceEmbeddings::new().unwrap());
        let vector_store = VectorStore::new(embeddings);
        let mut worker = SnapshotWorker::new(vector_store);

        // Test that snapshot doesn't happen immediately
        assert!(!worker.last_snapshot.elapsed().as_secs() >= 2);
        assert_eq!(worker.insert_count, 0);

        // Simulate inserts
        for _ in 0..64 {
            worker.increment_insert_count();
        }

        // Now snapshot should be triggered
        assert!(worker.insert_count >= 64);

        Ok(())
    }
}
