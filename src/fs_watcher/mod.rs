//! APEX 2.2-FW: Filesystem Watcher Module
//!
//! Minimal, deterministic filewatcher using notify crate with inotify backend.
//! Provides debounced file change events via Tokio mpsc channel.

use anyhow::Result;
use notify::{recommended_watcher, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc as std_mpsc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

// ============================================================================
// Public Types
// ============================================================================

/// Filesystem event kind (simplified from notify)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsEventKind {
    Created,
    Modified,
    Removed,
    Renamed(PathBuf), // New path after rename
}

/// Filesystem event with path and kind
#[derive(Debug, Clone)]
pub struct FsEvent {
    pub path: PathBuf,
    pub kind: FsEventKind,
}

/// Handle to running filesystem watcher
pub struct FsWatcherHandle {
    pub rx: mpsc::Receiver<FsEvent>,
    _watcher: RecommendedWatcher,
    _worker: JoinHandle<()>,
}

/// Errors from filesystem watcher
#[derive(Debug, thiserror::Error)]
pub enum FsWatcherError {
    #[error("Failed to create watcher: {0}")]
    WatcherCreation(#[from] notify::Error),

    #[error("Failed to watch path: {0}")]
    WatchPath(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// ============================================================================
// Public API
// ============================================================================

/// Start filesystem watcher on given root directory
///
/// Returns handle with receiver channel for FsEvents.
/// Debounces rapid writes with ~50ms window.
/// Only emits events for paths inside watched root.
pub fn start_fs_watcher(root: PathBuf) -> Result<FsWatcherHandle, FsWatcherError> {
    // Create channels
    let (tx, rx) = mpsc::channel::<FsEvent>(100);
    let (notify_tx, notify_rx) = std_mpsc::channel();

    // Create watcher
    let mut watcher = recommended_watcher(notify_tx)?;

    // Watch root recursively
    watcher
        .watch(&root, RecursiveMode::Recursive)
        .map_err(|e| FsWatcherError::WatchPath(format!("{}: {}", root.display(), e)))?;

    // Spawn worker task to process events
    let worker = tokio::spawn(async move {
        process_events(notify_rx, tx, root).await;
    });

    Ok(FsWatcherHandle {
        rx,
        _watcher: watcher,
        _worker: worker,
    })
}

// ============================================================================
// Internal Implementation
// ============================================================================

/// Process events from notify watcher with debouncing
async fn process_events(
    notify_rx: std_mpsc::Receiver<Result<Event, notify::Error>>,
    tx: mpsc::Sender<FsEvent>,
    root: PathBuf,
) {
    // Simple debounce: collect events for 50ms windows
    let debounce_duration = Duration::from_millis(50);
    let mut last_events: std::collections::HashMap<PathBuf, FsEventKind> =
        std::collections::HashMap::new();

    loop {
        // Collect events for debounce window
        let start = std::time::Instant::now();

        while start.elapsed() < debounce_duration {
            match notify_rx.recv_timeout(debounce_duration - start.elapsed()) {
                Ok(Ok(event)) => {
                    if let Some(fs_event) = convert_event(event, &root) {
                        // Update last event for this path (debounce duplicates)
                        last_events.insert(fs_event.path.clone(), fs_event.kind);
                    }
                }
                Ok(Err(e)) => {
                    eprintln!("Watcher error: {}", e);
                }
                Err(std_mpsc::RecvTimeoutError::Timeout) => {
                    break;
                }
                Err(std_mpsc::RecvTimeoutError::Disconnected) => {
                    return; // Watcher dropped
                }
            }
        }

        // Emit debounced events
        for (path, kind) in last_events.drain() {
            let fs_event = FsEvent { path, kind };
            if tx.send(fs_event).await.is_err() {
                return; // Receiver dropped
            }
        }

        // Small sleep to avoid busy loop
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// Convert notify Event to FsEvent (only for paths inside root)
fn convert_event(event: Event, root: &PathBuf) -> Option<FsEvent> {
    let path = event.paths.first()?.clone();

    // Only emit events for paths inside root
    if !path.starts_with(root) {
        return None;
    }

    let kind = match event.kind {
        EventKind::Create(_) => FsEventKind::Created,
        EventKind::Modify(_) => FsEventKind::Modified,
        EventKind::Remove(_) => FsEventKind::Removed,
        EventKind::Access(_) => return None, // Ignore access events
        EventKind::Any | EventKind::Other => FsEventKind::Modified, // Treat as modify
    };

    Some(FsEvent { path, kind })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fs_event_kind_equality() {
        assert_eq!(FsEventKind::Created, FsEventKind::Created);
        assert_eq!(FsEventKind::Modified, FsEventKind::Modified);
        assert_eq!(FsEventKind::Removed, FsEventKind::Removed);

        let path1 = PathBuf::from("/tmp/test");
        let path2 = PathBuf::from("/tmp/test");
        assert_eq!(FsEventKind::Renamed(path1), FsEventKind::Renamed(path2));
    }

    #[test]
    fn test_convert_event_filters_outside_root() {
        let root = PathBuf::from("/watched");
        let outside_path = PathBuf::from("/other/file.txt");

        let event = Event {
            kind: EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Any,
            )),
            paths: vec![outside_path],
            attrs: Default::default(),
        };

        let result = convert_event(event, &root);
        assert!(result.is_none(), "Should filter out paths outside root");
    }
}
