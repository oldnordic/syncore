//! Global Ingestion Coordinator (GIC)
//!
//! Central coordinator for all file ingestion events in SynCore.
//! Provides unified boundary checking, deduplication, and queue management
//! to prevent ingestion loops and ensure deterministic behavior.

pub mod coordinator;
pub mod priority_consumer;
pub mod queue;
pub mod types;

pub use coordinator::GlobalIngestionCoordinator;
pub use priority_consumer::PriorityIngestionConsumer;
pub use queue::{IngestionQueue, IngestionQueueKind};
pub use types::*;
