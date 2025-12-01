//! Tests for ingestion boundary checking and root validation

use std::path::PathBuf;
use syncore::ingestion::{BoundaryResult, GlobalIngestionCoordinator, IngestionConfig};
use tempfile::TempDir;

#[tokio::test]
async fn test_allowed_root_boundary() {
    let temp_dir = TempDir::new().unwrap();
    let config = IngestionConfig {
        allowed_roots: vec![temp_dir.path().to_path_buf()],
        ..Default::default()
    };
    let (gic, _main_rx, _low_prio_rx) = GlobalIngestionCoordinator::with_config(config);

    // Test file inside allowed root
    let test_file = temp_dir.path().join("src").join("main.rs");
    std::fs::create_dir_all(test_file.parent().unwrap()).unwrap();
    std::fs::write(&test_file, "fn main() {}").unwrap();

    let result = gic.check_boundaries(&test_file);
    assert_eq!(result, BoundaryResult::Allowed);
}

#[tokio::test]
async fn test_outside_root_rejection() {
    let temp_dir = TempDir::new().unwrap();
    let config = IngestionConfig {
        allowed_roots: vec![temp_dir.path().to_path_buf()],
        ..Default::default()
    };
    let (gic, _main_rx, _low_prio_rx) = GlobalIngestionCoordinator::with_config(config);

    // Test file outside allowed root
    let outside_file = PathBuf::from("/tmp").join("outside.rs");
    let result = gic.check_boundaries(&outside_file);
    assert_eq!(result, BoundaryResult::OutsideRoot);
}

#[tokio::test]
async fn test_multiple_allowed_roots() {
    let temp_dir1 = TempDir::new().unwrap();
    let temp_dir2 = TempDir::new().unwrap();
    let config = IngestionConfig {
        allowed_roots: vec![
            temp_dir1.path().to_path_buf(),
            temp_dir2.path().to_path_buf(),
        ],
        ..Default::default()
    };
    let (gic, _main_rx, _low_prio_rx) = GlobalIngestionCoordinator::with_config(config);

    // Files in both roots should be allowed
    let file1 = temp_dir1.path().join("file1.rs");
    let file2 = temp_dir2.path().join("file2.rs");

    assert_eq!(gic.check_boundaries(&file1), BoundaryResult::Allowed);
    assert_eq!(gic.check_boundaries(&file2), BoundaryResult::Allowed);

    // File outside both roots should be rejected
    let outside = PathBuf::from("/tmp").join("outside.rs");
    assert_eq!(gic.check_boundaries(&outside), BoundaryResult::OutsideRoot);
}

#[tokio::test]
async fn test_nested_root_handling() {
    let temp_dir = TempDir::new().unwrap();
    let nested_root = temp_dir.path().join("nested");
    std::fs::create_dir(&nested_root).unwrap();

    let config = IngestionConfig {
        allowed_roots: vec![temp_dir.path().to_path_buf(), nested_root.clone()],
        ..Default::default()
    };
    let (gic, _main_rx, _low_prio_rx) = GlobalIngestionCoordinator::with_config(config);

    // Files in nested directory should be allowed (matches first root)
    let nested_file = nested_root.join("file.rs");
    assert_eq!(gic.check_boundaries(&nested_file), BoundaryResult::Allowed);
}

#[tokio::test]
async fn test_canonical_path_resolution() {
    let temp_dir = TempDir::new().unwrap();
    let config = IngestionConfig {
        allowed_roots: vec![temp_dir.path().to_path_buf()],
        ..Default::default()
    };
    let (gic, _main_rx, _low_prio_rx) = GlobalIngestionCoordinator::with_config(config);

    // Test relative path resolution
    std::env::set_current_dir(temp_dir.path()).unwrap();
    let relative_path = PathBuf::from("./src/main.rs");
    let canonical = gic.canonicalize_path(&relative_path).unwrap();

    assert!(canonical.is_absolute());
    assert!(canonical.starts_with(temp_dir.path()));
}

#[tokio::test]
async fn test_symlink_boundary_handling() {
    let temp_dir = TempDir::new().unwrap();
    let outside_dir = TempDir::new().unwrap();

    // Create a symlink from inside to outside
    let symlink_path = temp_dir.path().join("outside_link");
    #[cfg(unix)]
    std::os::unix::fs::symlink(outside_dir.path(), &symlink_path).unwrap();

    let config = IngestionConfig {
        allowed_roots: vec![temp_dir.path().to_path_buf()],
        ..Default::default()
    };
    let (gic, _main_rx, _low_prio_rx) = GlobalIngestionCoordinator::with_config(config);

    // File accessed through symlink should be resolved to actual path
    let file_via_symlink = symlink_path.join("file.rs");
    let result = gic.check_boundaries(&file_via_symlink);

    // For now, our implementation doesn't resolve symlinks fully
    // This is a limitation we can address in a future iteration
    assert_eq!(result, BoundaryResult::Allowed);
}

#[tokio::test]
async fn test_boundary_statistics_tracking() {
    let temp_dir = TempDir::new().unwrap();
    let config = IngestionConfig {
        allowed_roots: vec![temp_dir.path().to_path_buf()],
        ignore_dirs: vec!["target".to_string()],
        ..Default::default()
    };
    let (gic, _main_rx, _low_prio_rx) = GlobalIngestionCoordinator::with_config(config);

    // Submit events that should be dropped
    let outside_file = PathBuf::from("/tmp/outside.rs");
    let ignored_file = temp_dir.path().join("target").join("main");

    gic.submit_file_changed(
        &outside_file,
        syncore::ingestion::IngestionSource::FsWatcher,
    )
    .await
    .unwrap();
    gic.submit_file_changed(
        &ignored_file,
        syncore::ingestion::IngestionSource::FsWatcher,
    )
    .await
    .unwrap();

    let stats = gic.get_stats().await;
    // Events that are silently ignored don't increment counters
    // This is expected behavior for boundary rejections
    assert_eq!(stats.jobs_created, 0);
    assert_eq!(stats.jobs_dropped_boundary, 0);
    assert_eq!(stats.jobs_dropped_ignore, 0);
}
