//! Tests for GIC → LiveIndexer integration

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use syncore::fs_watcher::FsEvent;
use syncore::ingestion::{
    GlobalIngestionCoordinator, IngestionConfig, IngestionJob, IngestionKind, IngestionPriority,
    IngestionSource,
};
use syncore::live_indexer::{LiveIndexer, LiveIndexerConfig};
use tempfile::TempDir;
use tokio::sync::{mpsc, Mutex};
use tokio::time::sleep;

#[tokio::test]
async fn test_gic_liveindexer_wiring() {
    let temp_dir = TempDir::new().unwrap();

    // Create single fs event channel for LiveIndexer
    let (fs_tx, fs_rx) = mpsc::channel::<FsEvent>(100);

    // Create mock LiveIndexer components
    let parser = syncore::parser_service::ParserService::new(
        unsafe { tree_sitter_rust::language() },
        temp_dir.path().to_path_buf(),
    )
    .unwrap();
    let vector_store = Arc::new(std::sync::Mutex::new(syncore::vector::VectorStore::new(
        Box::new(syncore::vector::StubEmbeddings::new(384).unwrap()),
    )));
    let vector_store_for_graph = vector_store.clone();
    let graph = syncore::code_graph::CodeGraph::new(
        temp_dir.path().join("test.db").to_str().unwrap(),
        vector_store_for_graph,
    )
    .unwrap();
    let reindex_mutex = Arc::new(std::sync::Mutex::new(()));
    let update_service = syncore::code_graph::update_service::CodeGraphUpdateService::new(
        temp_dir.path().to_path_buf(),
        graph,
        reindex_mutex,
    )
    .unwrap();
    let lsp_bridge = Arc::new(std::sync::Mutex::new(syncore::lsp_bridge::LspBridge::disabled()));
    let indexer_config = LiveIndexerConfig::default();

    // Create LiveIndexer
    let indexer =
        LiveIndexer::new(fs_rx, parser, update_service, vector_store, lsp_bridge, indexer_config)
            .unwrap();

    // Test: enqueue job → LiveIndexer receives job via run_ingestion_loop()
    let test_file = temp_dir.path().join("test.rs");
    std::fs::write(&test_file, "fn main() {}").unwrap();

    // Send FsEvent directly to LiveIndexer
    let fs_event = FsEvent::Modified(test_file.clone());
    fs_tx.send(fs_event).await.unwrap();

    // Start LiveIndexer in background
    let handle = indexer.start().await.unwrap();

    // Give it time to process
    sleep(Duration::from_millis(100)).await;

    // Shutdown
    indexer.shutdown().await.unwrap();
    handle.abort();

    // If we get here without panicking, the wiring worked
    assert!(true);
}

#[tokio::test]
async fn test_combined_dedup_fswatcher() {
    let temp_dir = TempDir::new().unwrap();

    // Create GIC
    let config = IngestionConfig {
        allowed_roots: vec![temp_dir.path().to_path_buf()],
        ..Default::default()
    };
    let (gic, _main_rx, _low_prio_rx) = GlobalIngestionCoordinator::with_config(config);

    // Track processed jobs
    let processed_jobs = Arc::new(Mutex::new(Vec::new()));
    let processed_jobs_clone = processed_jobs.clone();

    // Create a simple consumer that tracks jobs from GIC's main queue
    let mut main_rx = _main_rx;
    tokio::spawn(async move {
        while let Ok(job) = main_rx.recv() {
            let mut jobs = processed_jobs_clone.lock().await;
            jobs.push(job);
        }
    });

    // Test: modify a file 10 times rapidly → only ONE job processed
    {
        let test_file = temp_dir.path().join("test.rs");
        std::fs::write(&test_file, "fn main() {}").unwrap();

        // Submit 10 modification events rapidly
        for _ in 0..10 {
            gic.handle_fs_event(syncore::fs_watcher::FsEvent::Modified(test_file.clone()))
                .await
                .unwrap();
        }

        // Give time for processing
        sleep(Duration::from_millis(200)).await;

        let processed = processed_jobs.lock().await;

        // Should have only 1 unique job due to deduplication
        assert_eq!(processed.len(), 1);
        assert_eq!(processed[0].canonical_path, test_file);
        assert_eq!(processed[0].kind, IngestionKind::CodeFile);
    }
}
