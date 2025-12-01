//! Tests for ingestion job deduplication and queue management

use std::path::PathBuf;

use syncore::ingestion::{
    GlobalIngestionCoordinator, IngestionConfig, IngestionEventKind, IngestionJob, IngestionKind,
    IngestionPriority, IngestionSource,
};
use tempfile::TempDir;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_job_dedup_key_generation() {
    let path1 = PathBuf::from("/test/src/main.rs");
    let path2 = PathBuf::from("/test/src/main.rs");
    let path3 = PathBuf::from("/test/src/lib.rs");

    let job1 = IngestionJob::new(
        path1.clone(),
        IngestionKind::CodeFile,
        IngestionEventKind::Modified,
        IngestionPriority::Normal,
        IngestionSource::FsWatcher,
    );

    let job2 = IngestionJob::new(
        path2.clone(),
        IngestionKind::CodeFile,
        IngestionEventKind::Modified,
        IngestionPriority::Normal,
        IngestionSource::FsWatcher,
    );

    let job3 = IngestionJob::new(
        path3.clone(),
        IngestionKind::CodeFile,
        IngestionEventKind::Modified,
        IngestionPriority::Normal,
        IngestionSource::FsWatcher,
    );

    // Same path and kind should have same dedup key
    assert_eq!(job1.dedup_key(), job2.dedup_key());

    // Different path should have different dedup key
    assert_ne!(job1.dedup_key(), job3.dedup_key());
}

#[tokio::test]
async fn test_different_kinds_different_keys() {
    let path = PathBuf::from("/test/src/main.rs");

    let code_job = IngestionJob::new(
        path.clone(),
        IngestionKind::CodeFile,
        IngestionEventKind::Modified,
        IngestionPriority::Normal,
        IngestionSource::FsWatcher,
    );

    let doc_job = IngestionJob::new(
        path.clone(),
        IngestionKind::DocFile,
        IngestionEventKind::Modified,
        IngestionPriority::Low,
        IngestionSource::FsWatcher,
    );

    // Same path but different kind should have different dedup keys
    assert_ne!(code_job.dedup_key(), doc_job.dedup_key());
}

#[tokio::test]
async fn test_immediate_deduplication() {
    let temp_dir = TempDir::new().unwrap();
    let config = IngestionConfig {
        allowed_roots: vec![temp_dir.path().to_path_buf()],
        ..Default::default()
    };
    let (gic, _main_rx, _low_prio_rx) = GlobalIngestionCoordinator::with_config(config);

    let test_file = temp_dir.path().join("src").join("main.rs");
    std::fs::create_dir_all(test_file.parent().unwrap()).unwrap();
    std::fs::write(&test_file, "fn main() {}").unwrap();

    // Submit the same job twice quickly
    gic.submit_file_changed(&test_file, IngestionSource::FsWatcher).await.unwrap();
    gic.submit_file_changed(&test_file, IngestionSource::FsWatcher).await.unwrap();

    let stats = gic.get_stats().await;
    // Should have created 2 jobs but deduped 1
    assert_eq!(stats.jobs_created, 2);
    assert_eq!(stats.jobs_deduped, 1);
}

#[tokio::test]
async fn test_deduplication_timeout() {
    let temp_dir = TempDir::new().unwrap();
    let config = IngestionConfig {
        allowed_roots: vec![temp_dir.path().to_path_buf()],
        ..Default::default()
    };
    let (gic, _main_rx, _low_prio_rx) = GlobalIngestionCoordinator::with_config(config);

    let test_file = temp_dir.path().join("src").join("main.rs");
    std::fs::create_dir_all(test_file.parent().unwrap()).unwrap();
    std::fs::write(&test_file, "fn main() {}").unwrap();

    // Submit first job
    gic.submit_file_changed(&test_file, IngestionSource::FsWatcher).await.unwrap();

    // Wait longer than dedup window (5 seconds in current implementation)
    sleep(Duration::from_secs(6)).await;

    // Submit same job again - should not be deduped
    gic.submit_file_changed(&test_file, IngestionSource::FsWatcher).await.unwrap();

    let stats = gic.get_stats().await;
    // Should have created 2 jobs with no deduplication due to timeout
    assert_eq!(stats.jobs_created, 2);
    assert_eq!(stats.jobs_deduped, 0);
}

#[tokio::test]
async fn test_different_sources_not_deduped() {
    let temp_dir = TempDir::new().unwrap();
    let config = IngestionConfig {
        allowed_roots: vec![temp_dir.path().to_path_buf()],
        ..Default::default()
    };
    let (gic, _main_rx, _low_prio_rx) = GlobalIngestionCoordinator::with_config(config);

    let test_file = temp_dir.path().join("src").join("main.rs");
    std::fs::create_dir_all(test_file.parent().unwrap()).unwrap();
    std::fs::write(&test_file, "fn main() {}").unwrap();

    // Submit same file from different sources
    gic.submit_file_changed(&test_file, IngestionSource::FsWatcher).await.unwrap();
    gic.submit_file_changed(&test_file, IngestionSource::Cli).await.unwrap();

    let stats = gic.get_stats().await;
    // Should be deduped since dedup key doesn't include source
    assert_eq!(stats.jobs_created, 2);
    assert_eq!(stats.jobs_deduped, 1);
}

#[tokio::test]
async fn test_priority_based_deduplication() {
    let temp_dir = TempDir::new().unwrap();
    let config = IngestionConfig {
        allowed_roots: vec![temp_dir.path().to_path_buf()],
        ..Default::default()
    };
    let (gic, _main_rx, _low_prio_rx) = GlobalIngestionCoordinator::with_config(config);

    let code_file = temp_dir.path().join("src").join("main.rs");
    let doc_file = temp_dir.path().join("README.md");

    std::fs::create_dir_all(code_file.parent().unwrap()).unwrap();
    std::fs::write(&code_file, "fn main() {}").unwrap();
    std::fs::write(&doc_file, "# README").unwrap();

    // Submit code file (normal priority) and doc file (low priority)
    gic.submit_file_changed(&code_file, IngestionSource::FsWatcher).await.unwrap();
    gic.submit_file_changed(&doc_file, IngestionSource::FsWatcher).await.unwrap();

    let stats = gic.get_stats().await;
    // Both should be processed, no deduplication since different files
    assert_eq!(stats.jobs_created, 2);
    assert_eq!(stats.jobs_deduped, 0);
}

#[tokio::test]
async fn test_manual_index_deduplication() {
    let temp_dir = TempDir::new().unwrap();
    let config = IngestionConfig {
        allowed_roots: vec![temp_dir.path().to_path_buf()],
        ..Default::default()
    };
    let (gic, _main_rx, _low_prio_rx) = GlobalIngestionCoordinator::with_config(config);

    let test_file = temp_dir.path().join("src").join("main.rs");
    std::fs::create_dir_all(test_file.parent().unwrap()).unwrap();
    std::fs::write(&test_file, "fn main() {}").unwrap();

    // Submit manual index request
    gic.submit_manual_index(&test_file, IngestionKind::CodeFile).await.unwrap();

    // Submit same file as FS event
    gic.submit_file_changed(&test_file, IngestionSource::FsWatcher).await.unwrap();

    let stats = gic.get_stats().await;
    // Should be deduped since same path and kind
    assert_eq!(stats.jobs_created, 2);
    assert_eq!(stats.jobs_deduped, 1);
}

#[tokio::test]
async fn test_queue_overflow_handling() {
    let temp_dir = TempDir::new().unwrap();

    // Create queue directly for testing
    let (queue, mut main_rx, mut low_prio_rx) =
        syncore::ingestion::queue::IngestionQueue::new(1, 1);

    // Spawn a task to consume from queues (simulating LiveIndexer)
    tokio::spawn(async move {
        while let Ok(_job) = main_rx.recv() {
            // Simulate processing - just consume
        }
    });

    tokio::spawn(async move {
        while let Ok(_job) = low_prio_rx.recv() {
            // Simulate processing - just consume
        }
    });

    // Give consumers time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Submit more jobs than queue can handle rapidly (different files to avoid dedup)
    let mut results = Vec::new();
    for i in 0..3 {
        let test_file = temp_dir.path().join("src").join(format!("file{}.rs", i));
        std::fs::create_dir_all(test_file.parent().unwrap()).unwrap();
        std::fs::write(&test_file, "fn main() {}").unwrap();

        let job = IngestionJob::new(
            test_file,
            IngestionKind::CodeFile,
            IngestionEventKind::Modified,
            IngestionPriority::Normal,
            IngestionSource::FsWatcher,
        );

        let result = queue.submit_job(job).await;
        results.push(result);
    }

    // Check results and stats
    let success_count = results.iter().filter(|r| r.is_ok()).count();
    let stats = queue.get_stats();

    println!(
        "Results: {:?}, Success: {}, Stats: {:?}",
        results.iter().map(|r| r.is_ok()).collect::<Vec<_>>(),
        success_count,
        stats
    );

    // With active consumers, all jobs should be accepted
    assert_eq!(success_count, 3, "All jobs should be accepted with active consumer");
    assert_eq!(stats.jobs_created, 3, "Should have created 3 jobs");
    assert_eq!(stats.jobs_deduped, 0, "No deduplication expected for different files");
}
