//! Core types for the Global Ingestion Coordinator

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;

/// Ingestion event kinds that can be processed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IngestionEventKind {
    /// File was created
    Created,
    /// File was modified
    Modified,
    /// File was deleted
    Deleted,
    /// File was renamed (contains new path)
    Renamed(PathBuf),
}

impl IngestionEventKind {
    /// Get numeric representation for deduplication key
    pub fn as_u8(&self) -> u8 {
        match self {
            IngestionEventKind::Created => 0,
            IngestionEventKind::Modified => 1,
            IngestionEventKind::Deleted => 2,
            IngestionEventKind::Renamed(_) => 3,
        }
    }
}

/// Source of an ingestion event
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IngestionSource {
    /// File system watcher
    FsWatcher,
    /// CLI/MCP tool invocation
    Cli,
    /// MCP tool invocation
    Mcp,
    /// Internal reindexing job
    Internal,
}

/// Priority levels for ingestion jobs
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum IngestionPriority {
    /// Low priority (docs, background tasks)
    Low = 0,
    /// Normal priority (code files)
    Normal = 1,
    /// High priority (user-initiated changes)
    High = 2,
}

/// Types of ingestion jobs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IngestionKind {
    /// Source code file
    CodeFile,
    /// Documentation file
    DocFile,
    /// Mapping/graph node file
    MappingNode,
    /// File deletion
    DeleteFile,
}

/// An ingestion job to be processed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionJob {
    /// Canonical (resolved, normalized) path
    pub canonical_path: PathBuf,
    /// Type of ingestion job
    pub kind: IngestionKind,
    /// Event kind
    pub event_kind: IngestionEventKind,
    /// Priority level
    pub priority: IngestionPriority,
    /// Source of the job
    pub source: IngestionSource,
    /// When the job was created
    pub ts_created: SystemTime,
    /// Optional content hash for deduplication
    pub content_hash: Option<String>,
}

impl IngestionJob {
    /// Create a new ingestion job
    pub fn new(
        canonical_path: PathBuf,
        kind: IngestionKind,
        event_kind: IngestionEventKind,
        priority: IngestionPriority,
        source: IngestionSource,
    ) -> Self {
        Self {
            canonical_path,
            kind,
            event_kind,
            priority,
            source,
            ts_created: SystemTime::now(),
            content_hash: None,
        }
    }

    /// Create a deduplication key for this job
    pub fn dedup_key(&self) -> String {
        format!(
            "{}:{}:{}",
            self.canonical_path.display(),
            self.kind.clone() as u8,
            self.event_kind.as_u8()
        )
    }

    /// Check if this job should be processed immediately
    pub fn is_high_priority(&self) -> bool {
        matches!(self.priority, IngestionPriority::High)
    }
}

/// Configuration for the Global Ingestion Coordinator
#[derive(Debug, Clone)]
pub struct IngestionConfig {
    /// Allowed root directories for ingestion
    pub allowed_roots: Vec<PathBuf>,
    /// Directories to always ignore
    pub ignore_dirs: Vec<String>,
    /// Glob patterns to ignore
    pub ignore_globs: Vec<String>,
    /// Maximum concurrent ingestion jobs
    pub max_concurrent_jobs: usize,
    /// Queue sizes
    pub main_queue_size: usize,
    pub low_priority_queue_size: usize,
}

impl Default for IngestionConfig {
    fn default() -> Self {
        Self {
            allowed_roots: vec![std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))],
            ignore_dirs: vec![
                ".git".to_string(),
                "target".to_string(),
                "node_modules".to_string(),
                ".idea".to_string(),
                ".vscode".to_string(),
                ".cache".to_string(),
                "dist".to_string(),
                "build".to_string(),
            ],
            ignore_globs: vec![
                "*.hnsw.*".to_string(),
                "*.index.*".to_string(),
                "*.log".to_string(),
                "*.tmp".to_string(),
                "*.db".to_string(),
                "*.sqlite".to_string(),
                "*.sqlite3".to_string(),
            ],
            max_concurrent_jobs: 4,
            main_queue_size: 1000,
            low_priority_queue_size: 500,
        }
    }
}

/// Result of boundary checking
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundaryResult {
    /// Path is allowed for ingestion
    Allowed,
    /// Path is outside allowed roots
    OutsideRoot,
    /// Path matches ignore patterns
    Ignored,
    /// Path is a generated/snapshot file
    GeneratedFile,
}

/// Statistics about ingestion processing
#[derive(Debug, Clone, Default)]
pub struct IngestionStats {
    /// Total jobs created
    pub jobs_created: u64,
    /// Jobs dropped by boundary checks
    pub jobs_dropped_boundary: u64,
    /// Jobs dropped by ignore patterns
    pub jobs_dropped_ignore: u64,
    /// Jobs deduplicated
    pub jobs_deduped: u64,
    /// Jobs processed successfully
    pub jobs_processed: u64,
    /// Jobs that failed
    pub jobs_failed: u64,
    /// Phase 8: Jobs processed from main queue
    pub main_queue_processed: u64,
    /// Phase 8: Jobs processed from low priority queue
    pub low_priority_queue_processed: u64,
    /// Current queue depths
    pub main_queue_depth: usize,
    pub low_priority_queue_depth: usize,
}
