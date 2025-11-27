//! APEX 2.7-LIVE-INDEXER: Real-time continuous indexing engine
//!
//! Provides background live indexing as files change:
//! FsEvent → ParserService → DeltaEngine → UpdateService → HNSW → LSP

use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::code_graph::update_service::{CodeGraphUpdateEvent, CodeGraphUpdateService};
use crate::fs_watcher::FsEvent;
use crate::lsp_bridge::LspBridge;
use crate::parser_service::ParserService;
use crate::vector::VectorStore;

/// Configuration for LiveIndexer
#[derive(Debug, Clone)]
pub struct LiveIndexerConfig {
    pub debounce_ms: u64,
    pub max_queue: usize,
    pub index_threads: usize,
}

impl Default for LiveIndexerConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 100,
            max_queue: 100,
            index_threads: 1,
        }
    }
}

/// Live indexer that continuously processes file system events
pub struct LiveIndexer {
    shutdown_tx: mpsc::Sender<()>,
    shutdown_rx: Arc<Mutex<Option<mpsc::Receiver<()>>>>,
    components: Arc<Mutex<Option<Components>>>,
}

struct Components {
    fs_rx: mpsc::Receiver<FsEvent>,
    parser: ParserService,
    update_service: CodeGraphUpdateService,
    vector_store: Arc<Mutex<VectorStore>>,
    lsp_bridge: LspBridge,
    config: LiveIndexerConfig,
}

impl LiveIndexer {
    /// Create a new LiveIndexer
    pub fn new(
        fs_rx: mpsc::Receiver<FsEvent>,
        parser: ParserService,
        update_service: CodeGraphUpdateService,
        vector_store: Arc<Mutex<VectorStore>>,
        lsp_bridge: LspBridge,
        config: LiveIndexerConfig,
    ) -> Result<Self> {
        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1);

        let components = Components {
            fs_rx,
            parser,
            update_service,
            vector_store,
            lsp_bridge,
            config,
        };

        Ok(Self {
            shutdown_tx,
            shutdown_rx: Arc::new(Mutex::new(Some(shutdown_rx))),
            components: Arc::new(Mutex::new(Some(components))),
        })
    }

    /// Start the live indexer background task
    pub async fn start(&self) -> Result<JoinHandle<()>> {
        let mut components_lock = self.components.lock().unwrap();
        let components = components_lock.take()
            .ok_or_else(|| anyhow::anyhow!("LiveIndexer already started"))?;

        let mut shutdown_rx_lock = self.shutdown_rx.lock().unwrap();
        let shutdown_rx = shutdown_rx_lock.take()
            .ok_or_else(|| anyhow::anyhow!("LiveIndexer already started"))?;

        let handle = tokio::spawn(event_loop(
            components.fs_rx,
            shutdown_rx,
            components.parser,
            components.update_service,
            components.vector_store,
            components.lsp_bridge,
            components.config,
        ));

        Ok(handle)
    }

    /// Shutdown the live indexer gracefully
    pub async fn shutdown(&self) -> Result<()> {
        let _ = self.shutdown_tx.send(()).await;
        Ok(())
    }
}

/// Per-file throttle state
struct FileThrottle {
    last_processed: Instant,
    pending: bool,
}

impl FileThrottle {
    fn new() -> Self {
        Self {
            last_processed: Instant::now(),
            pending: false,
        }
    }

    fn should_process(&self, debounce: Duration) -> bool {
        self.last_processed.elapsed() >= debounce
    }

    fn mark_processed(&mut self) {
        self.last_processed = Instant::now();
        self.pending = false;
    }

    fn mark_pending(&mut self) {
        self.pending = true;
    }
}

/// Background event processing loop
async fn event_loop(
    mut fs_rx: mpsc::Receiver<FsEvent>,
    mut shutdown_rx: mpsc::Receiver<()>,
    mut parser: ParserService,
    mut update_service: CodeGraphUpdateService,
    vector_store: Arc<Mutex<VectorStore>>,
    mut lsp_bridge: LspBridge,
    config: LiveIndexerConfig,
) {
    let debounce_duration = Duration::from_millis(config.debounce_ms);
    let mut throttle_map: HashMap<PathBuf, FileThrottle> = HashMap::new();

    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                break;
            }

            Some(fs_event) = fs_rx.recv() => {
                let path = fs_event.path.clone();

                let should_process = throttle_map
                    .get(&path)
                    .map(|t| t.should_process(debounce_duration))
                    .unwrap_or(true);

                if should_process {
                    process_fs_event(
                        fs_event,
                        &mut parser,
                        &mut update_service,
                        &vector_store,
                        &mut lsp_bridge,
                    ).await;

                    throttle_map
                        .entry(path)
                        .or_insert_with(FileThrottle::new)
                        .mark_processed();
                } else {
                    throttle_map
                        .entry(path)
                        .or_insert_with(FileThrottle::new)
                        .mark_pending();
                }
            }
        }
    }
}

/// Process a single file system event
async fn process_fs_event(
    fs_event: FsEvent,
    parser: &mut ParserService,
    update_service: &mut CodeGraphUpdateService,
    _vector_store: &Arc<Mutex<VectorStore>>,
    lsp_bridge: &mut LspBridge,
) {
    let parse_deltas = match parser.apply_fs_event(fs_event.clone()) {
        Ok(deltas) => deltas,
        Err(_) => return,
    };

    let update_event = CodeGraphUpdateEvent {
        fs_event: fs_event.clone(),
        parse_delta: parse_deltas.first().cloned(),
    };

    let _ = update_service.apply_update(update_event);

    // Trigger LSP notification (if file exists)
    if fs_event.path.exists() {
        if let Ok(text) = std::fs::read_to_string(&fs_event.path) {
            let _ = lsp_bridge.send_did_change(&fs_event.path, &text).await;
        }
    }
}
