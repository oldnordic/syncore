use anyhow::Result;
use std::env;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use syncore::config::SyncoreConfig;
use syncore::http_stream_server::HttpStreamServer;
use syncore::mcp_server::run_mcp_stdio_server;
use syncore::router::SynCoreState;
use syncore::tools_cli::log_registered_tools;
use syncore::vector::{HuggingFaceEmbeddings, VectorStore};
// APEX 2.12 - Live indexing subsystems
use syncore::code_graph::update_service::CodeGraphUpdateService;
use syncore::embedding_refresh::{EmbeddingRefreshConfig, EmbeddingRefreshDaemon};
use syncore::fs_watcher::start_fs_watcher;
use syncore::ingestion::GlobalIngestionCoordinator;
use syncore::live_indexer::{LiveIndexer, LiveIndexerConfig};
use syncore::lsp_bridge::LspBridge;
use syncore::parser_service::ParserService;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration from file (optional - if not found, uses defaults)
    // Config file should be next to the binary: ~/.config/syncore/syncore.toml
    let config_path = env::var("SYNCORE_CONFIG").unwrap_or_else(|_| {
        if let Ok(exe_path) = env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let config = exe_dir.join("syncore.toml");
                if config.exists() {
                    return config.to_string_lossy().to_string();
                }
            }
        }
        // No config found - will use defaults with DBs next to binary
        "syncore.toml".to_string()
    });

    let config = if std::path::Path::new(&config_path).exists() {
        match SyncoreConfig::load(&config_path) {
            Ok(c) => {
                eprintln!("Loaded config from: {}", config_path);
                c
            }
            Err(e) => {
                eprintln!("Warning: Failed to load config from {}: {}", config_path, e);
                eprintln!("Using default configuration");
                SyncoreConfig::default()
            }
        }
    } else {
        eprintln!("Config file not found at {}, using defaults", config_path);
        SyncoreConfig::default()
    };

    // Initialize global config for path filtering and other config-aware tools
    SyncoreConfig::init_global(config.clone());
    eprintln!("Global config initialized ({} excluded dirs)", config.indexing.excluded_dirs.len());

    // Get configuration from environment (overrides config file)
    let db_path = env::var("DB_PATH").unwrap_or_else(|_| config.paths.db_path.clone());
    let http_port = env::var("HTTP_PORT").unwrap_or_else(|_| config.http.port.to_string());
    let http_bind: SocketAddr = format!("127.0.0.1:{}", http_port).parse()?;

    eprintln!("Starting SynCore MCP servers...");
    eprintln!("Database path: {}", db_path);
    eprintln!("HTTP Streaming server: {}", http_bind);

    // Log all registered MCP tools
    log_registered_tools().await;

    // Initialize dual VectorStores for domain-aware embedding routing (APEX 1.7)
    // CODE domain: code entities with code-optimized embeddings
    // GENERAL domain: documents, tasks, notes with general-purpose embeddings
    use syncore::common::db_paths;

    // Create CODE domain store with BGE embeddings (optimized for code)
    eprintln!(
        "[syncore] Initializing CODE domain VectorStore (BGE-small-en-v1.5 for code entities)..."
    );
    let code_embeddings = Box::new(HuggingFaceEmbeddings::new_bge()?);
    let mut code_store = VectorStore::new(code_embeddings);
    let code_index_path = db_paths::code_vector_index_path();
    eprintln!("[syncore] CODE vector index path: {}", code_index_path);
    code_store.set_index_path(code_index_path);

    // BUGFIX #3: Load snapshot from disk to restore embeddings state
    // Without this, search_code() operates on empty vector store → poisoned locks
    if let Err(e) = code_store.load_snapshot() {
        eprintln!("[syncore] Warning: Failed to load CODE vector snapshot: {}", e);
        eprintln!("[syncore] Will start with empty CODE vector store (bootstrap will rebuild)");
    } else {
        eprintln!(
            "[syncore] Successfully loaded CODE vector snapshot ({} vectors)",
            code_store.len()
        );
    }

    let code_store = std::sync::Arc::new(std::sync::Mutex::new(code_store));

    // Create GENERAL domain store with all-MiniLM embeddings (for general text)
    eprintln!("[syncore] Initializing GENERAL domain VectorStore (all-MiniLM-L6-v2 for documents, tasks, notes)...");
    let general_embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let mut general_store = VectorStore::new(general_embeddings);
    let general_index_path = db_paths::general_vector_index_path();
    eprintln!("[syncore] GENERAL vector index path: {}", general_index_path);
    general_store.set_index_path(general_index_path);

    // BUGFIX #3: Load snapshot from disk to restore embeddings state
    if let Err(e) = general_store.load_snapshot() {
        eprintln!("[syncore] Warning: Failed to load GENERAL vector snapshot: {}", e);
        eprintln!("[syncore] Will start with empty GENERAL vector store");
    } else {
        eprintln!(
            "[syncore] Successfully loaded GENERAL vector snapshot ({} vectors)",
            general_store.len()
        );
    }

    let general_store = std::sync::Arc::new(std::sync::Mutex::new(general_store));

    // Initialize state with dual stores
    let mut state = SynCoreState::with_dual_stores(code_store, general_store)?;
    eprintln!("[syncore] Dual-embedding architecture initialized (CODE + GENERAL domains)");

    {
        use syncore::message_bus::MessageBus;
        let bus = MessageBus::new();
        state = state.with_message_bus(bus);
    }

    // Configure graph backend from configuration (Phase G4 requirement)
    {
        eprintln!("Loading graph backend configuration...");
        let config = SyncoreConfig::load_with_env("config/syncore.toml").unwrap_or_else(|e| {
            eprintln!("Warning: Failed to load config, using defaults: {}", e);
            let mut config = SyncoreConfig::default();
            config.apply_env_overrides();
            config
        });

        // Apply configuration-driven backend selection
        state = match state.clone().with_graph_backend_from_config(&config).await {
            Ok(updated_state) => {
                eprintln!("Graph backend configured: {:?}", config.graph.backend);
                updated_state
            }
            Err(e) => {
                eprintln!("Warning: Failed to configure graph backend: {}", e);
                // Continue without graph backend - raggraph methods will create fallback SQLiteGraph
                state
            }
        };
    }

    // Initialize IntelliTask with Candle GGUFEngine backend
    {
        use std::sync::Arc;
        use syncore::intellitask::IntelliTask;
        use syncore::llm::LanguageModel;
        use syncore::models::gguf_engine::GGUFEngine;

        eprintln!("Initializing IntelliTask with REAL GGUFEngine backend...");

        // Use LlmFactory to get real model (fixed to load real GGUF instead of test)
        use syncore::llm::factory::{LlmBackend, LlmConfig, LlmFactory};

        let config = LlmConfig {
            backend: LlmBackend::GGUFEngine,
            model: std::env::var("SYNC_LLM_MODEL_PATH")
                .unwrap_or_else(|_| "qwen2.5-0.5b".to_string()),
            url: "".to_string(),
            timeout_seconds: 30,
        };

        let llm_model: Arc<dyn LanguageModel> = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                match LlmFactory::from_config(&config).await {
                    Ok(model_arc) => Ok::<Arc<dyn LanguageModel>, anyhow::Error>(model_arc),
                    Err(e) => {
                        eprintln!("❌ Failed to create LLM from config: {}", e);
                        // Legitimate fallback only if factory completely fails
                        Ok::<Arc<dyn LanguageModel>, anyhow::Error>(
                            Arc::new(GGUFEngine::new_test()) as Arc<dyn LanguageModel>,
                        )
                    }
                }
            })
        })?;

        if llm_model.backend_name() != "gguf_engine"
            || (llm_model.backend_name() == "gguf_engine"
                && llm_model
                    .complete(&syncore::llm::Prompt::new("test", "test"))
                    .map(|c| c.text.starts_with("GGUFEngine response to:"))
                    .unwrap_or(true))
        {
            eprintln!("⚠️  Using test GGUFEngine backend as fallback");
        } else {
            eprintln!("✅ Successfully loaded REAL GGUFEngine backend via factory");
        }

        // Store LLM model in state for MCP handlers
        state = state.with_llm_model(llm_model.clone());

        let intellitask = Arc::new(IntelliTask::new(llm_model));
        state = state.with_intellitask(intellitask);
        eprintln!("IntelliTask initialized successfully with Candle backend");
    }

    // Connect to Neo4j if available
    {
        use std::sync::Arc;
        use syncore::graph::neo4j_client::Neo4jClient;

        let neo4j_uri =
            env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string());
        let neo4j_user = env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
        let neo4j_pass = env::var("NEO4J_PASS").unwrap_or_else(|_| "testpassword123".to_string());

        eprintln!("Connecting to Neo4j at {}...", neo4j_uri);
        match Neo4jClient::connect(&neo4j_uri, &neo4j_user, &neo4j_pass).await {
            Ok(neo) => {
                state = state.with_neo4j(Arc::new(neo));
                eprintln!("Neo4j connected successfully");
            }
            Err(e) => {
                eprintln!("Neo4j connection failed (graph tools disabled): {}", e);
            }
        }
    }

    eprintln!("State initialized...");

    // =====================================================================
    // APEX 2.12 - LIVE INDEXING PIPELINE INITIALIZATION
    // =====================================================================
    // Wire subsystems: FsWatcher → ParserService → DeltaEngine →
    //                  UpdateService → HNSW → LiveIndexer → EmbeddingRefreshDaemon

    eprintln!("[SynCore] Initializing live indexing pipeline...");

    // Get project root (current working directory for file watching)
    let project_root = std::env::current_dir()?;

    // APEX 2.15: Run bootstrap check BEFORE starting subsystems
    syncore::bootstrap::run_startup_bootstrap_for_tests(&config).await?;
    eprintln!("[SynCore] Bootstrap check complete");

    // Step 1: Create CodeGraph for UpdateService (matches test order)
    let code_graph_db_path = &config.paths.code_graph_db;
    let code_graph =
        syncore::code_graph::CodeGraph::new(code_graph_db_path, state.code_store.clone())?;
    eprintln!("[SynCore] CodeGraph created at {:?}", code_graph_db_path);

    // Step 2: Create CodeGraphUpdateService (wraps CodeGraph + DeltaEngine)
    // APEX 2.15: Pass reindex mutex to serialize DELETE+INSERT operations
    let update_service = CodeGraphUpdateService::new(code_graph, state.reindex_mutex.clone())?;
    eprintln!("[SynCore] CodeGraphUpdateService created (DeltaEngine initialized)");

    // Step 3: Create ParserService (Rust only for now)
    let language = tree_sitter_rust::language();
    let parser = ParserService::new(language, project_root.clone())?;
    eprintln!("[SynCore] ParserService created (Rust language)");

    // Step 4: Create LspBridge (disabled for now - no LSP server)
    let lsp_bridge = LspBridge::disabled();
    eprintln!("[SynCore] LspBridge disabled (no LSP server)");

    // Step 5: Create Global Ingestion Coordinator (GIC)
    let (gic, main_rx, low_prio_rx) = GlobalIngestionCoordinator::new();
    eprintln!("[SynCore] Global Ingestion Coordinator created");

    // Step 6: Start FsWatcher and connect to GIC
    let watcher_handle = start_fs_watcher(project_root.clone())?;
    let fs_rx = watcher_handle.rx;
    eprintln!("[SynCore] FsWatcher started for LiveIndexer");

    // Connect FsWatcher to GIC in a background task
    let gic_clone = gic.clone();
    let fs_rx_for_gic = fs_rx.clone();
    tokio::spawn(async move {
        use crossbeam::channel::TryRecvError;
        loop {
            match fs_rx_for_gic.try_recv() {
                Ok(fs_event) => {
                    if let Err(e) = gic_clone.handle_fs_event(fs_event).await {
                        eprintln!("[GIC] Error handling fs_event: {}", e);
                    }
                }
                Err(TryRecvError::Empty) => {
                    // No events, sleep briefly
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
                Err(TryRecvError::Disconnected) => {
                    eprintln!("[GIC] FsWatcher disconnected, stopping event handler");
                    break;
                }
            }
        }
    });

    // Step 7: Create and start LiveIndexer
    let indexer_config = LiveIndexerConfig {
        debounce_ms: 100,
        max_queue: 100,
        index_threads: 1,
    };

    // Bridge crossbeam receiver to tokio receiver for LiveIndexer
    let (tokio_tx, tokio_rx) = tokio::sync::mpsc::channel::<FsEvent>(100);
    let fs_rx_clone = fs_rx.clone();
    tokio::spawn(async move {
        use crossbeam::channel::TryRecvError;
        use std::time::Duration;
        loop {
            match fs_rx_clone.try_recv() {
                Ok(fs_event) => {
                    if tokio_tx.send(fs_event).await.is_err() {
                        break;
                    }
                }
                Err(TryRecvError::Empty) => {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                Err(TryRecvError::Disconnected) => {
                    break;
                }
            }
        }
    });

    // Clone values for priority consumer
    let parser_clone = parser.clone();
    let update_service_clone = update_service.clone();
    let lsp_bridge_arc = Arc::new(Mutex::new(lsp_bridge));
    let lsp_bridge_clone = lsp_bridge_arc.clone();
    let code_store_clone = state.code_store.clone();

    // LiveIndexer consumes from tokio_rx
    let indexer = LiveIndexer::new(
        tokio_rx,
        parser_clone.clone(),
        update_service_clone.clone(),
        code_store_clone.clone(),
        lsp_bridge_clone.clone(),
        indexer_config,
    )?;

    let indexer_handle = indexer.start().await?;
    eprintln!("[SynCore] LiveIndexer started (background task spawned)");

    // Phase 8: Create Priority-aware Ingestion Consumer
    // Consumes from main_rx and low_prio_rx with fair priority routing
    use syncore::fs_watcher::FsEvent;
    use syncore::ingestion::PriorityIngestionConsumer;

    let priority_consumer = PriorityIngestionConsumer::new(
        main_rx,
        low_prio_rx,
        parser_clone,
        update_service_clone,
        code_store_clone,
        lsp_bridge_clone,
    );

    let _priority_handle = priority_consumer.start().await?;
    eprintln!("[SynCore] Phase 8: Priority-aware ingestion consumer started");

    // Step 8: Start EmbeddingRefreshDaemon (dual-domain: CODE + GENERAL)
    let refresh_config = EmbeddingRefreshConfig::default();
    let (daemon, _daemon_tx) = EmbeddingRefreshDaemon::spawn(
        state.code_store.clone(),
        state.general_store.clone(),
        refresh_config,
    )?;
    eprintln!("[SynCore] EmbeddingRefreshDaemon spawned (dual-domain: CODE + GENERAL)");

    // Step 9: Store handles to keep subsystems alive
    // Handles are kept in scope until program exits for graceful shutdown
    // Note: watcher_handle already moved (via fs_rx extraction), just keep other handles
    let _live_indexing_handles = (indexer_handle, daemon, _daemon_tx);

    eprintln!("[SynCore] ✓ Live indexing pipeline fully wired and operational");
    // =====================================================================

    // SNAPSHOT-FIRST STARTUP: Try loading snapshot before rebuild
    // This allows O(1) warmup (~20-50ms) vs O(n) rebuild (minutes)
    {
        use syncore::code_graph::CodeGraph;

        let warmup_state = state.clone();
        let hnsw_ready = state.hnsw_ready.clone();

        tokio::spawn(async move {
            eprintln!("[SynCore] Starting HNSW warmup for CODE domain (snapshot-first pattern)...");

            // Clone references needed for spawn_blocking
            let blocking_state = warmup_state.clone();
            // Use CODE domain store for warmup (code entities only)
            let vector_store_for_warmup = warmup_state.code_store.clone();

            // Mark as WarmingUp state
            if let Ok(vs) = vector_store_for_warmup.lock() {
                vs.warmup_controller().mark_warming_up();
            }

            // Phase 1: Try snapshot load (fast path)
            let snapshot_loaded = {
                let vs_for_snapshot = vector_store_for_warmup.clone();
                tokio::task::spawn_blocking(move || {
                    if let Ok(mut vs) = vs_for_snapshot.lock() {
                        // Index path already set by main thread - don't override!
                        // Try to load snapshot - this will mark Hot if successful
                        if vs.load_snapshot().is_ok() && vs.warmup_controller().is_hot() {
                            return true;
                        }
                    }
                    false
                })
                .await
                .unwrap_or(false)
            };

            if snapshot_loaded {
                // Snapshot loaded successfully, HNSW is hot - done!
                hnsw_ready.store(true, Ordering::SeqCst);
                eprintln!("[SynCore] Snapshot loaded successfully - warmup complete!");
                return;
            }

            // Phase 2: Snapshot not available - rebuild from SQLite (slow path)
            eprintln!("[SynCore] No valid snapshot, rebuilding HNSW from SQLite...");

            let result = tokio::task::spawn_blocking(move || {
                match CodeGraph::with_connection(
                    blocking_state.db_manager.code_graph_conn(),
                    blocking_state.code_store.clone(),
                ) {
                    Ok(code_graph) => code_graph.rebuild_hnsw_from_entities(),
                    Err(e) => Err(e),
                }
            })
            .await;

            // Handle rebuild result
            let vector_store_for_flush = warmup_state.code_store.clone();
            match result {
                Ok(Ok(count)) => {
                    // Mark HNSW as ready
                    hnsw_ready.store(true, Ordering::SeqCst);

                    if let Ok(mut vs) = vector_store_for_flush.lock() {
                        vs.set_hnsw_ready(true);
                        vs.warmup_controller().mark_hot();

                        // Flush pending vectors
                        match vs.flush_pending_vectors() {
                            Ok(flushed) if flushed > 0 => {
                                eprintln!(
                                    "[SynCore] HNSW ready. Rebuilt {} vectors, flushed {} pending.",
                                    count, flushed
                                );
                            }
                            Ok(_) => {
                                if count > 0 {
                                    eprintln!("[SynCore] HNSW ready. Rebuilt {} vectors.", count);
                                } else {
                                    eprintln!("[SynCore] HNSW ready. Index empty (no indexed entities yet).");
                                }
                            }
                            Err(e) => {
                                eprintln!("[SynCore] HNSW ready. Rebuilt {} vectors. Warning: flush pending failed: {:?}", count, e);
                            }
                        }
                    }
                }
                Ok(Err(e)) => {
                    eprintln!("[SynCore] HNSW rebuild failed: {:?}", e);
                    // Mark as ready anyway so brute-force fallback works
                    hnsw_ready.store(true, Ordering::SeqCst);
                    if let Ok(vs) = vector_store_for_flush.lock() {
                        vs.set_hnsw_ready(true);
                        // Don't mark Hot - stay in WarmingUp for fallback behavior
                    }
                }
                Err(e) => {
                    eprintln!("[SynCore] HNSW warmup task panicked: {:?}", e);
                    hnsw_ready.store(true, Ordering::SeqCst);
                    if let Ok(vs) = vector_store_for_flush.lock() {
                        vs.set_hnsw_ready(true);
                    }
                }
            }
        });
    }

    // Spawn HTTP Streamable server (for non-STDIO clients like Codex, other AI tools)
    // Uses rmcp's StreamableHttpService for proper MCP protocol compliance
    // Endpoint: http://{http_bind}/mcp (handles both POST and GET per MCP 2025-03-26 spec)
    {
        let http_state = state.clone();
        tokio::spawn(async move {
            let server = HttpStreamServer::new(http_state);
            eprintln!("HTTP Streamable MCP server listening on http://{}/mcp", http_bind);
            if let Err(e) = server.start(http_bind).await {
                eprintln!("HTTP Streamable server error: {:?}", e);
            }
        });
    }

    eprintln!("Starting STDIO server (Claude Code interface)...");

    // Enable backwards compatibility mode for stdio by default
    // This allows clients that don't send notifications/initialized to work
    std::env::set_var("MCP_BACKWARDS_COMPATIBLE", "true");
    eprintln!("[SynCore] MCP_BACKWARDS_COMPATIBLE mode enabled for stdio transport");

    // Run rmcp-based stdio server (blocks until shutdown)
    // If STDIO closes, HTTP server continues running
    match run_mcp_stdio_server(state).await {
        Ok(_) => eprintln!("STDIO server shut down normally"),
        Err(e) => eprintln!("STDIO server error: {:?}", e),
    }

    // Keep the process alive for HTTP clients even if STDIO is closed
    // Wait for ctrl-c signal to shut down
    eprintln!("STDIO closed, HTTP Streamable MCP server still running at http://{}/mcp. Press Ctrl-C to stop.", http_bind);
    tokio::signal::ctrl_c().await?;

    eprintln!("MCP servers shut down");
    Ok(())
}
