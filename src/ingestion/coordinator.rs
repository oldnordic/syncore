//! Global Ingestion Coordinator implementation

use anyhow::Result;
use crossbeam::channel::Receiver;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::queue::IngestionQueue;
use super::types::*;
use crate::fs_watcher::FsEvent;

/// Global Ingestion Coordinator - single gate for all file ingestion
#[derive(Clone)]
pub struct GlobalIngestionCoordinator {
    /// Configuration
    config: IngestionConfig,
    /// Ingestion queues
    queue: Arc<IngestionQueue>,
    /// Statistics
    stats: Arc<Mutex<IngestionStats>>,
}

impl GlobalIngestionCoordinator {
    /// Create new GIC with default configuration
    pub fn new() -> (Self, Receiver<IngestionJob>, Receiver<IngestionJob>) {
        Self::with_config(IngestionConfig::default())
    }

    /// Create new GIC with custom configuration
    pub fn with_config(
        config: IngestionConfig,
    ) -> (Self, Receiver<IngestionJob>, Receiver<IngestionJob>) {
        let (queue, main_rx, low_prio_rx) =
            IngestionQueue::new(config.main_queue_size, config.low_priority_queue_size);

        let gic = Self {
            config,
            queue: Arc::new(queue),
            stats: Arc::new(Mutex::new(IngestionStats::default())),
        };

        (gic, main_rx, low_prio_rx)
    }

    /// Submit a file discovery event
    pub async fn submit_file_discovered(&self, path: &Path, source: IngestionSource) -> Result<()> {
        self.submit_event(path, IngestionEventKind::Created, source)
            .await
    }

    /// Submit a file change event
    pub async fn submit_file_changed(&self, path: &Path, source: IngestionSource) -> Result<()> {
        self.submit_event(path, IngestionEventKind::Modified, source)
            .await
    }

    /// Submit a file deletion event
    pub async fn submit_file_deleted(&self, path: &Path, source: IngestionSource) -> Result<()> {
        self.submit_event(path, IngestionEventKind::Deleted, source)
            .await
    }

    /// Submit a manual index request
    pub async fn submit_manual_index(&self, path: &Path, mode: IngestionKind) -> Result<()> {
        let canonical_path = self.canonicalize_path(path)?;

        // Check boundaries
        match self.check_boundaries(&canonical_path) {
            BoundaryResult::Allowed => {}
            BoundaryResult::OutsideRoot => {
                self.update_boundary_stats("outside_root").await;
                return Err(anyhow::anyhow!(
                    "Path {} is outside allowed roots",
                    path.display()
                ));
            }
            BoundaryResult::Ignored => {
                self.update_boundary_stats("ignored").await;
                return Err(anyhow::anyhow!("Path {} is ignored", path.display()));
            }
            BoundaryResult::GeneratedFile => {
                self.update_boundary_stats("generated").await;
                return Err(anyhow::anyhow!(
                    "Path {} is a generated file",
                    path.display()
                ));
            }
        }

        // Determine priority based on kind
        let priority = match mode {
            IngestionKind::CodeFile => IngestionPriority::Normal,
            IngestionKind::DocFile => IngestionPriority::Low,
            IngestionKind::MappingNode => IngestionPriority::Normal,
            IngestionKind::DeleteFile => IngestionPriority::High,
        };

        let job = IngestionJob::new(
            canonical_path,
            mode,
            IngestionEventKind::Modified,
            priority,
            IngestionSource::Cli,
        );

        self.queue.submit_job(job).await
    }

    /// Submit a generic event
    async fn submit_event(
        &self,
        path: &Path,
        event_kind: IngestionEventKind,
        source: IngestionSource,
    ) -> Result<()> {
        let canonical_path = self.canonicalize_path(path)?;

        // Check boundaries
        match self.check_boundaries(&canonical_path) {
            BoundaryResult::Allowed => {}
            BoundaryResult::OutsideRoot => {
                self.update_boundary_stats("outside_root").await;
                return Ok(()); // Silently drop outside-root events
            }
            BoundaryResult::Ignored => {
                self.update_boundary_stats("ignored").await;
                return Ok(()); // Silently drop ignored events
            }
            BoundaryResult::GeneratedFile => {
                self.update_boundary_stats("generated").await;
                return Ok(()); // Silently drop generated file events
            }
        }

        // Determine ingestion kind and priority
        let (kind, priority) = self.classify_file(&canonical_path);

        let job = IngestionJob::new(canonical_path, kind, event_kind, priority, source);

        self.queue.submit_job(job).await
    }

    /// Canonicalize and normalize a path
    pub fn canonicalize_path(&self, path: &Path) -> Result<PathBuf> {
        let canonical = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()?.join(path)
        };

        // Normalize path (remove . and ..)
        Ok(canonical.canonicalize().unwrap_or(canonical))
    }

    /// Check if a path is within allowed boundaries
    pub fn check_boundaries(&self, path: &Path) -> BoundaryResult {
        // Check if path is within allowed roots
        let within_root = self
            .config
            .allowed_roots
            .iter()
            .any(|root| path.starts_with(root));

        if !within_root {
            return BoundaryResult::OutsideRoot;
        }

        // Check for generated files first (highest priority)
        if self.is_generated_file(path) {
            return BoundaryResult::GeneratedFile;
        }

        // Check ignore directories (exact path component matching)
        let path_str = path.to_string_lossy();
        for ignore_dir in &self.config.ignore_dirs {
            // Check if any path component exactly matches the ignore directory
            for component in path.components() {
                if component.as_os_str().to_string_lossy() == **ignore_dir {
                    return BoundaryResult::Ignored;
                }
            }
        }

        // Check ignore globs, but allow syncore.db as special case
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if filename == "syncore.db" {
            return BoundaryResult::Allowed;
        }

        for ignore_glob in &self.config.ignore_globs {
            if self.matches_glob(&path_str, ignore_glob) {
                return BoundaryResult::Ignored;
            }
        }

        BoundaryResult::Allowed
    }

    /// Classify file type and determine priority
    fn classify_file(&self, path: &Path) -> (IngestionKind, IngestionPriority) {
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

        match extension {
            // Code files
            "rs" | "js" | "ts" | "py" | "go" | "java" | "c" | "cpp" | "h" | "hpp" => {
                (IngestionKind::CodeFile, IngestionPriority::Normal)
            }
            // Documentation files
            "md" | "txt" | "rst" | "adoc" => (IngestionKind::DocFile, IngestionPriority::Low),
            // Configuration files (mapping relevant)
            "toml" | "json" | "yaml" | "yml" | "xml" => {
                (IngestionKind::MappingNode, IngestionPriority::Normal)
            }
            // Default to code file for unknown types
            _ => (IngestionKind::CodeFile, IngestionPriority::Normal),
        }
    }

    /// Check if path matches a glob pattern
    fn matches_glob(&self, path: &str, pattern: &str) -> bool {
        // Extract just the filename for glob matching
        let filename = path.split('/').last().unwrap_or(path);

        // Simple glob matching - can be enhanced with proper glob crate
        if pattern.contains('*') {
            let pattern_parts: Vec<&str> = pattern.split('*').collect();
            if pattern_parts.len() == 2 {
                let prefix = pattern_parts[0];
                let suffix = pattern_parts[1];
                return filename.starts_with(prefix) && filename.ends_with(suffix);
            }
        }
        filename.contains(pattern)
    }

    /// Check if file is a generated/snapshot file
    fn is_generated_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Check for snapshot files
        if path_str.contains(".hnsw.")
            || path_str.contains(".index.")
            || path_str.ends_with(".meta")
            || path_str.ends_with(".vectors")
        {
            return true;
        }

        // Check for database files (except our main syncore.db)
        if let Some(file_name) = path.file_name() {
            let file_name = file_name.to_string_lossy();
            if (file_name.ends_with(".db")
                || file_name.ends_with(".sqlite")
                || file_name.ends_with(".sqlite3"))
                && !file_name.contains("syncore")
            {
                return true;
            }
        }

        false
    }

    /// Update boundary check statistics
    async fn update_boundary_stats(&self, reason: &str) {
        let mut stats = self.stats.lock().await;
        match reason {
            "outside_root" => stats.jobs_dropped_boundary += 1,
            "ignored" => stats.jobs_dropped_ignore += 1,
            "generated" => stats.jobs_dropped_ignore += 1,
            _ => {}
        }
    }

    /// Get current statistics
    pub async fn get_stats(&self) -> IngestionStats {
        let mut queue_stats = self.queue.get_stats();
        let boundary_stats = self.stats.lock().await;

        // Merge boundary stats into queue stats
        queue_stats.jobs_dropped_boundary += boundary_stats.jobs_dropped_boundary;
        queue_stats.jobs_dropped_ignore += boundary_stats.jobs_dropped_ignore;

        queue_stats
    }

    /// Handle filesystem event from FsWatcher
    pub async fn handle_fs_event(&self, event: FsEvent) -> Result<()> {
        match event {
            FsEvent::Created(path) => {
                self.submit_file_discovered(&path, IngestionSource::FsWatcher)
                    .await
            }
            FsEvent::Modified(path) => {
                self.submit_file_changed(&path, IngestionSource::FsWatcher)
                    .await
            }
            FsEvent::Removed(path) => {
                self.submit_file_deleted(&path, IngestionSource::FsWatcher)
                    .await
            }
        }
    }

    /// Get configuration reference
    pub fn config(&self) -> &IngestionConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_boundary_check_allowed() {
        let temp_dir = TempDir::new().unwrap();
        let config = IngestionConfig {
            allowed_roots: vec![temp_dir.path().to_path_buf()],
            ..Default::default()
        };
        let gic = GlobalIngestionCoordinator::with_config(config);

        let test_file = temp_dir.path().join("src").join("main.rs");
        let result = gic.0.check_boundaries(&test_file);
        assert_eq!(result, BoundaryResult::Allowed);
    }

    #[test]
    fn test_boundary_check_outside_root() {
        let temp_dir = TempDir::new().unwrap();
        let config = IngestionConfig {
            allowed_roots: vec![temp_dir.path().to_path_buf()],
            ..Default::default()
        };
        let gic = GlobalIngestionCoordinator::with_config(config);

        let outside_file = PathBuf::from("/tmp/outside.rs");
        let result = gic.0.check_boundaries(&outside_file);
        assert_eq!(result, BoundaryResult::OutsideRoot);
    }

    #[test]
    fn test_boundary_check_ignored() {
        let temp_dir = TempDir::new().unwrap();
        let config = IngestionConfig {
            allowed_roots: vec![temp_dir.path().to_path_buf()],
            ignore_dirs: vec!["target".to_string()],
            ..Default::default()
        };
        let gic = GlobalIngestionCoordinator::with_config(config);

        let ignored_file = temp_dir.path().join("target").join("debug").join("main");
        let result = gic.0.check_boundaries(&ignored_file);
        assert_eq!(result, BoundaryResult::Ignored);
    }

    #[test]
    fn test_classify_code_file() {
        let gic = GlobalIngestionCoordinator::new();
        let rust_file = PathBuf::from("src/main.rs");
        let (kind, priority) = gic.0.classify_file(&rust_file);
        assert_eq!(kind, IngestionKind::CodeFile);
        assert_eq!(priority, IngestionPriority::Normal);
    }

    #[test]
    fn test_classify_doc_file() {
        let gic = GlobalIngestionCoordinator::new();
        let doc_file = PathBuf::from("README.md");
        let (kind, priority) = gic.0.classify_file(&doc_file);
        assert_eq!(kind, IngestionKind::DocFile);
        assert_eq!(priority, IngestionPriority::Low);
    }

    #[test]
    fn test_is_generated_file() {
        let gic = GlobalIngestionCoordinator::new();

        assert!(gic.0.is_generated_file(&PathBuf::from("data.hnsw.data")));
        assert!(gic.0.is_generated_file(&PathBuf::from("index.vectors")));
        assert!(gic.0.is_generated_file(&PathBuf::from("other.db")));
        assert!(!gic.0.is_generated_file(&PathBuf::from("syncore.db")));
        assert!(!gic.0.is_generated_file(&PathBuf::from("src/main.rs")));
    }
}
