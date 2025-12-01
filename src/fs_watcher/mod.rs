//! APEX 2.2-FW: Filesystem Watcher Module
//!
//! Minimal, deterministic filewatcher using notify crate with inotify backend.
//! Provides debounced file change events via Tokio mpsc channel.

use anyhow::Result;
use crossbeam::channel::{self, Receiver, Sender};
use notify::{recommended_watcher, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc as std_mpsc;
use std::time::Duration;
use tokio::task::JoinHandle;

// ============================================================================
// Public Types
// ============================================================================

/// Filesystem event (exact specification required)
#[derive(Debug, Clone, PartialEq)]
pub enum FsEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Removed(PathBuf),
}

impl FsEvent {
    /// Get the path from the event
    pub fn path(&self) -> &PathBuf {
        match self {
            FsEvent::Created(path) => path,
            FsEvent::Modified(path) => path,
            FsEvent::Removed(path) => path,
        }
    }

    /// Check if the file exists (for non-Removed events)
    pub fn path_exists(&self) -> bool {
        match self {
            FsEvent::Created(_) | FsEvent::Modified(_) => self.path().exists(),
            FsEvent::Removed(_) => false,
        }
    }
}

/// Handle to running filesystem watcher
pub struct FsWatcherHandle {
    pub rx: Receiver<FsEvent>,
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
/// Returns handle with Crossbeam receiver channel for FsEvents.
/// Debounces rapid writes with ~50ms window.
/// Only emits events for paths inside watched root.
pub fn start_fs_watcher(root: PathBuf) -> Result<FsWatcherHandle, FsWatcherError> {
    // Create Crossbeam channels for lock-free operation
    let (tx, rx) = channel::bounded::<FsEvent>(100);
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
    tx: Sender<FsEvent>,
    root: PathBuf,
) {
    // Simple debounce: collect events for 50ms windows
    let debounce_duration = Duration::from_millis(50);
    let mut last_events: std::collections::HashMap<PathBuf, FsEvent> =
        std::collections::HashMap::new();

    loop {
        // Collect events for debounce window
        let start = std::time::Instant::now();

        while start.elapsed() < debounce_duration {
            match notify_rx.recv_timeout(debounce_duration - start.elapsed()) {
                Ok(Ok(event)) => {
                    if let Some(fs_event) = convert_event(event, &root) {
                        // Update last event for this path (debounce duplicates)
                        last_events.insert(fs_event.path().clone(), fs_event);
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

        // Emit debounced events using Crossbeam sender
        for (_, fs_event) in last_events.drain() {
            if tx.send(fs_event).is_err() {
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

    let fs_event = match event.kind {
        EventKind::Create(_) => FsEvent::Created(path),
        EventKind::Modify(_) => FsEvent::Modified(path),
        EventKind::Remove(_) => FsEvent::Removed(path),
        EventKind::Access(_) => return None, // Ignore access events
        EventKind::Any | EventKind::Other => FsEvent::Modified(path), // Treat as modify
    };

    Some(fs_event)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fs_event_equality() {
        let path1 = PathBuf::from("/tmp/test");
        let path2 = PathBuf::from("/tmp/test");

        assert_eq!(
            FsEvent::Created(path1.clone()),
            FsEvent::Created(path2.clone())
        );
        assert_eq!(
            FsEvent::Modified(path1.clone()),
            FsEvent::Modified(path2.clone())
        );
        assert_eq!(FsEvent::Removed(path1), FsEvent::Removed(path2));
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
