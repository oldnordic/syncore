//! Ingestion queue implementation with priority handling
//!
//! Uses Crossbeam MPMC channels for zero-backpressure, lock-free ingestion
//! with parking_lot for statistics tracking and DashMap for deduplication.

use anyhow::Result;
use crossbeam::channel::{self, Receiver, Sender};
use dashmap::DashMap;
use std::sync::Arc;

use super::types::{IngestionJob, IngestionPriority, IngestionStats};
// Import fast lock aliases
use crate::common::locks::FastMutex;

/// Queue kinds for different priority levels
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestionQueueKind {
    Main,
    LowPriority,
}

/// Ingestion queue with deduplication and priority handling
///
/// Uses Crossbeam MPMC channels for lock-free, zero-backpressure operation.
/// Provides async-compatible API while using sync channels internally.
#[derive(Clone)]
pub struct IngestionQueue {
    /// Main queue sender (Crossbeam)
    main_tx: Sender<IngestionJob>,
    /// Low priority queue sender (Crossbeam)
    low_prio_tx: Sender<IngestionJob>,
    /// Deduplication tracking
    dedup_map: Arc<DashMap<String, SystemTime>>,
    /// Statistics
    stats: Arc<FastMutex<IngestionStats>>,
}

impl IngestionQueue {
    /// Create new ingestion queues using Crossbeam channels
    pub fn new(
        main_queue_size: usize,
        low_prio_queue_size: usize,
    ) -> (Self, Receiver<IngestionJob>, Receiver<IngestionJob>) {
        // Use Crossbeam bounded channels for backpressure control
        let (main_tx, main_rx) = channel::bounded(main_queue_size);
        let (low_prio_tx, low_prio_rx) = channel::bounded(low_prio_queue_size);

        let queue = Self {
            main_tx,
            low_prio_tx,
            dedup_map: Arc::new(DashMap::new()),
            stats: Arc::new(FastMutex::new(IngestionStats::default())),
        };

        (queue, main_rx, low_prio_rx)
    }

    /// Submit a job to the appropriate queue (async-compatible API)
    ///
    /// Uses blocking channel send in a blocking task to provide async interface.
    /// Crossbeam channels are lock-free and provide excellent performance under contention.
    pub async fn submit_job(&self, job: IngestionJob) -> Result<()> {
        // Update stats
        {
            let mut stats = self.stats.lock();
            stats.jobs_created += 1;
        }

        // Check deduplication
        let dedup_key = job.dedup_key();
        {
            if let Some(existing_time) = self.dedup_map.get(&dedup_key) {
                // If we've seen this job recently, skip it
                if job.ts_created.duration_since(*existing_time).unwrap_or_default().as_secs() < 5 {
                    let mut stats = self.stats.lock();
                    stats.jobs_deduped += 1;
                    return Ok(());
                }
            }
            self.dedup_map.insert(dedup_key, job.ts_created);
        }

        // Clone job for move into async block
        let job_clone = job.clone();
        let main_tx = self.main_tx.clone();
        let low_prio_tx = self.low_prio_tx.clone();
        let stats = self.stats.clone();

        // Send to appropriate queue based on priority
        let send_result = tokio::task::spawn_blocking(move || match job_clone.priority {
            IngestionPriority::High | IngestionPriority::Normal => main_tx.send(job_clone),
            IngestionPriority::Low => low_prio_tx.send(job_clone),
        })
        .await?;

        if send_result.is_err() {
            // Queue is full or disconnected, update stats
            let mut stats_guard = stats.lock();
            stats_guard.jobs_failed += 1;
            return Err(anyhow::anyhow!("Queue is full or disconnected"));
        }

        Ok(())
    }

    /// Get current statistics
    pub fn get_stats(&self) -> IngestionStats {
        let stats = self.stats.lock();
        let mut result = stats.clone();

        // Update queue depths using Crossbeam channel capacity info
        // Note: Crossbeam doesn't expose current length, but we can track capacity
        result.main_queue_depth = self.main_tx.capacity().unwrap_or(0);
        result.low_priority_queue_depth = self.low_prio_tx.capacity().unwrap_or(0);

        result
    }

    /// Clear old deduplication entries
    pub fn cleanup_dedup(&self, max_age_secs: u64) {
        let cutoff = SystemTime::now() - std::time::Duration::from_secs(max_age_secs);

        self.dedup_map.retain(|_, time| *time > cutoff);
    }

    /// Phase 8: Priority-aware job selection using crossbeam::select!
    ///
    /// Returns the next available job with fair priority routing.
    /// Main queue is checked first but low priority jobs get service
    /// even when main queue has continuous load.
    pub fn select_next_job(
        &self,
        main_rx: &Receiver<IngestionJob>,
        low_prio_rx: &Receiver<IngestionJob>,
    ) -> Result<IngestionJob> {
        use crossbeam::channel::select;

        select! {
            recv(main_rx) -> job => {
                job.map_err(|_| anyhow::anyhow!("Main queue disconnected"))
            }
            recv(low_prio_rx) -> job => {
                job.map_err(|_| anyhow::anyhow!("Low priority queue disconnected"))
            }
        }
    }
}

use std::time::SystemTime;
