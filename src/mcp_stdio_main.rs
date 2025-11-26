use anyhow::Result;
use std::env;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;

use syncore::config::SyncoreConfig;
use syncore::http_stream_server::HttpStreamServer;
use syncore::mcp_server::run_mcp_stdio_server;
use syncore::router::SynCoreState;
use syncore::tools_cli::log_registered_tools;
use syncore::vector::{HuggingFaceEmbeddings, VectorStore};

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration from file (or use defaults)
    let config_path =
        env::var("SYNCORE_CONFIG").unwrap_or_else(|_| "config/syncore.toml".to_string());

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
    eprintln!(
        "Global config initialized ({} excluded dirs)",
        config.indexing.excluded_dirs.len()
    );

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
    eprintln!("[syncore] Initializing CODE domain VectorStore (BGE-small-en-v1.5 for code entities)...");
    let code_embeddings = Box::new(HuggingFaceEmbeddings::new_bge()?);
    let mut code_store = VectorStore::new(code_embeddings);
    let code_index_path = db_paths::code_vector_index_path();
    eprintln!("[syncore] CODE vector index path: {}", code_index_path);
    code_store.set_index_path(code_index_path);
    let code_store = std::sync::Arc::new(std::sync::Mutex::new(code_store));

    // Create GENERAL domain store with all-MiniLM embeddings (for general text)
    eprintln!("[syncore] Initializing GENERAL domain VectorStore (all-MiniLM-L6-v2 for documents, tasks, notes)...");
    let general_embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let mut general_store = VectorStore::new(general_embeddings);
    let general_index_path = db_paths::general_vector_index_path();
    eprintln!("[syncore] GENERAL vector index path: {}", general_index_path);
    general_store.set_index_path(general_index_path);
    let general_store = std::sync::Arc::new(std::sync::Mutex::new(general_store));

    // Initialize state with dual stores
    let mut state = SynCoreState::with_dual_stores(code_store, general_store)?;
    eprintln!("[syncore] Dual-embedding architecture initialized (CODE + GENERAL domains)");

    {
        use syncore::message_bus::MessageBus;
        let bus = MessageBus::new();
        state = state.with_message_bus(bus);
    }

    // Initialize IntelliTask with Ollama backend
    {
        use std::sync::Arc;
        use syncore::ollama::OllamaClient;
        use syncore::intellitask::IntelliTask;

        eprintln!("Initializing IntelliTask with Ollama backend...");

        match OllamaClient::new_default() {
            Ok(ollama) => {
                let intellitask = Arc::new(IntelliTask::new(ollama));
                state = state.with_intellitask(intellitask);
                eprintln!("IntelliTask initialized successfully");
            }
            Err(e) => {
                eprintln!("IntelliTask initialization failed: {}", e);
                eprintln!("IntelliTask AI features will be unavailable");
                eprintln!("Ensure Ollama is running for production");
            }
        }
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
                        // Set index path based on db_path
                        vs.set_index_path("syncore_vectors".to_string());

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
            eprintln!(
                "HTTP Streamable MCP server listening on http://{}/mcp",
                http_bind
            );
            if let Err(e) = server.start(http_bind).await {
                eprintln!("HTTP Streamable server error: {:?}", e);
            }
        });
    }

    eprintln!("Starting STDIO server (Claude Code interface)...");

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
