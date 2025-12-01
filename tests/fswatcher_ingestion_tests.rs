//! Tests for FsWatcher → GIC integration

use std::path::PathBuf;
use syncore::fs_watcher::FsEvent;
use syncore::ingestion::{GlobalIngestionCoordinator, IngestionConfig};
use tempfile::TempDir;

#[tokio::test]
async fn test_fswatcher_gic_event_emission() {
    let temp_dir = TempDir::new().unwrap();
    let config = IngestionConfig {
        allowed_roots: vec![temp_dir.path().to_path_buf()],
        ..Default::default()
    };
    let (gic, _main_rx, _low_prio_rx) = GlobalIngestionCoordinator::with_config(config);

    // Test 1: create file → Created event → 1 enqueued job
    let test_file = temp_dir.path().join("created.rs");
    std::fs::write(&test_file, "fn main() {}").unwrap();

    gic.handle_fs_event(FsEvent::Created(test_file.clone())).await.unwrap();

    let stats = gic.get_stats().await;
    assert_eq!(stats.jobs_created, 1);
    assert_eq!(stats.jobs_deduped, 0);

    // Test 2: modify file → Modified event → dedup ensures NOT >1 job
    gic.handle_fs_event(FsEvent::Modified(test_file.clone())).await.unwrap();
    gic.handle_fs_event(FsEvent::Modified(test_file.clone())).await.unwrap();

    let stats = gic.get_stats().await;
    assert_eq!(stats.jobs_created, 3); // Created + 2 Modified
    assert_eq!(stats.jobs_deduped, 1); // Second Modified should be deduped

    // Test 3: remove file → Removed event → DeleteFile job
    gic.handle_fs_event(FsEvent::Removed(test_file.clone())).await.unwrap();

    let stats = gic.get_stats().await;
    assert_eq!(stats.jobs_created, 4); // +1 Removed
                                       // Note: DeleteFile has different kind, so no dedup with Modified
}

#[tokio::test]
async fn test_ignore_directory_enforcement() {
    let temp_dir = TempDir::new().unwrap();
    let config = IngestionConfig {
        allowed_roots: vec![temp_dir.path().to_path_buf()],
        ignore_dirs: vec!["target".to_string(), ".git".to_string()],
        ignore_globs: vec!["*.hnsw.*".to_string(), "*.index.*".to_string()],
        ..Default::default()
    };
    let (gic, _main_rx, _low_prio_rx) = GlobalIngestionCoordinator::with_config(config);

    // Test 1: events under target/ → ignored
    let target_dir = temp_dir.path().join("target");
    std::fs::create_dir_all(&target_dir).unwrap();
    let target_file = target_dir.join("debug.rs");
    std::fs::write(&target_file, "debug code").unwrap();

    gic.handle_fs_event(FsEvent::Modified(target_file.clone())).await.unwrap();

    let stats = gic.get_stats().await;
    println!("After target file event: jobs_dropped_ignore={}", stats.jobs_dropped_ignore);
    assert_eq!(stats.jobs_dropped_ignore, 1);

    // Test 2: events under .git/ → ignored
    let git_dir = temp_dir.path().join(".git");
    std::fs::create_dir_all(&git_dir).unwrap();
    let git_file = git_dir.join("config");
    std::fs::write(&git_file, "git config").unwrap();
    gic.handle_fs_event(FsEvent::Modified(git_file.clone())).await.unwrap();

    let stats = gic.get_stats().await;
    println!("After git file event: jobs_dropped_ignore={}", stats.jobs_dropped_ignore);
    assert_eq!(stats.jobs_dropped_ignore, 2);

    // Test 3: events for generated files (*.hnsw.*, *.index.*) → ignored
    let hnsw_file = temp_dir.path().join("test.hnsw.bin");
    gic.handle_fs_event(FsEvent::Modified(hnsw_file.clone())).await.unwrap();

    let index_file = temp_dir.path().join("test.index.meta");
    gic.handle_fs_event(FsEvent::Modified(index_file.clone())).await.unwrap();

    let stats = gic.get_stats().await;
    assert_eq!(stats.jobs_dropped_ignore, 4);
}

#[tokio::test]
async fn test_boundary_enforcement() {
    let temp_dir = TempDir::new().unwrap();
    let config = IngestionConfig {
        allowed_roots: vec![temp_dir.path().to_path_buf()],
        ..Default::default()
    };
    let (gic, _main_rx, _low_prio_rx) = GlobalIngestionCoordinator::with_config(config);

    // Test: events outside project root MUST be ignored
    let outside_file = PathBuf::from("/tmp/outside.rs");
    gic.handle_fs_event(FsEvent::Modified(outside_file.clone())).await.unwrap();

    let stats = gic.get_stats().await;
    assert_eq!(stats.jobs_dropped_boundary, 1);
}
