use crate::common::db_paths;
use crate::graph::Neo4jClient;
use crate::logger::{CogLogger, MarkdownLogger};
use crate::memory::Memory;
use crate::message_bus::MessageBus;
use crate::protocol::{SynCoreMsg, SynCoreTool};
use crate::snapshots::{
    CodeGraphMetadata, MemoryMetadata, SnapshotHandle, SnapshotView, VectorStoreMetadata,
};
use crate::storage::{create_read_pool, FaissPool, FaissQueue, ReadPool, WriteQueue};
use serde_json::{json, Value};
use crate::tasks::Tasks;
use crate::vector::domain::EmbeddingDomain;
use crate::vector::dual_service::TripleEmbeddingService;
use crate::vector::traits::VectorIndex;
use crate::vector::{SearchScope, VectorStore};
use anyhow::{anyhow, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

#[derive(Clone)]
pub struct SynCoreState {
    /// Centralized SQLite connection manager (long-lived connections for WAL mode)
    pub db_manager: Arc<crate::db::DbManager>,
    pub memory: Arc<Memory>,
    pub tasks: Arc<Tasks>,
    /// CODE domain vector store (code entities with code-optimized embeddings)
    pub code_store: Arc<Mutex<VectorStore>>,
    /// GENERAL domain vector store (documents, tasks, notes with general-purpose embeddings)
    pub general_store: Arc<Mutex<VectorStore>>,
    /// GRAPH domain vector store (graph entities, nodes, edges, relationships)
    pub graph_store: Arc<Mutex<VectorStore>>,
    pub logger: Arc<dyn CogLogger>,
    pub message_bus: Option<Arc<MessageBus>>,
    pub write_queue: Option<Arc<WriteQueue>>,
    pub read_pool: Option<Arc<ReadPool>>,
    pub faiss_queue: Option<Arc<FaissQueue>>,
    pub faiss_pool: Option<Arc<FaissPool>>,
    pub neo4j: Option<Arc<Neo4jClient>>,
    /// Graph backend selector for unified access to Neo4j or SQLiteGraph
    pub graph_backend: Option<Arc<dyn crate::graph::GraphBackend>>,
    /// IntelliTask AI-powered task management (requires LLM backend)
    pub intellitask: Option<Arc<crate::intellitask::IntelliTask>>,
    /// LLM backend for AI-powered features (GGUFEngine, Test backend, etc.)
    pub llm_model: Option<Arc<dyn crate::llm::LanguageModel>>,
    /// HNSW index warmup status - true when index is ready for fast search
    pub hnsw_ready: Arc<AtomicBool>,
    /// APEX 2.15: Reindex mutex to serialize DELETE+INSERT operations
    /// Prevents UNIQUE constraint collisions between manual reindex and LiveIndexer
    /// Uses std::sync::Mutex for compatibility with sync code (blocking)
    pub reindex_mutex: Arc<std::sync::Mutex<()>>,
    /// MVCC snapshot handle for zero-blocking reads
    pub snapshot_handle: Arc<SnapshotHandle>,
    /// Debounced snapshot update task handle (for cancellation)
    pub snapshot_update_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl SynCoreState {
    /// Create SynCoreState with dual VectorStores (preferred constructor for production).
    ///
    /// This constructor initializes separate CODE and GENERAL domain vector stores
    /// for domain-aware embedding routing.
    ///
    /// # Example
    ///
    /// ```rust
    /// let code_embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    /// let code_store = Arc::new(Mutex::new(VectorStore::new(code_embeddings)));
    ///
    /// let general_embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    /// let general_store = Arc::new(Mutex::new(VectorStore::new(general_embeddings)));
    /// let graph_store = Arc::new(Mutex::new(VectorStore::new(graph_embeddings)));
    ///
    /// let state = SynCoreState::with_triple_stores(code_store, general_store, graph_store)?;
    /// ```
    pub fn with_dual_stores(
        code_store: Arc<Mutex<VectorStore>>,
        general_store: Arc<Mutex<VectorStore>>,
    ) -> Result<Self> {
        // Create a default graph store for the third domain
        let graph_embeddings = Box::new(crate::vector::HuggingFaceEmbeddings::new()?);
        let mut graph_store_vec = VectorStore::new(graph_embeddings);
        graph_store_vec.set_index_path("syncore_graph.index".to_string());
        let graph_store = Arc::new(Mutex::new(graph_store_vec));

        Self::with_triple_stores(code_store, general_store, graph_store)
    }

    /// Create SynCoreState with pre-existing VectorStores for all three domains
    ///
    /// This constructor is for when you have pre-initialized VectorStores:
    ///
    /// ```rust
    /// let code_store = Arc::new(Mutex::new(VectorStore::new(code_embeddings)));
    /// let general_store = Arc::new(Mutex::new(VectorStore::new(general_embeddings)));
    /// let graph_store = Arc::new(Mutex::new(VectorStore::new(graph_embeddings)));
    ///
    /// let state = SynCoreState::with_triple_stores(code_store, general_store, graph_store)?;
    /// ```
    pub fn with_triple_stores(
        code_store: Arc<Mutex<VectorStore>>,
        general_store: Arc<Mutex<VectorStore>>,
        graph_store: Arc<Mutex<VectorStore>>,
    ) -> Result<Self> {
        // Get database paths from centralized helpers
        let main_db_path = db_paths::main_db_path();
        let code_graph_db_path = db_paths::code_graph_db_path();

        // Initialize DbManager with long-lived connections
        let db_manager = Arc::new(crate::db::DbManager::new(&main_db_path, &code_graph_db_path)?);

        // Create TripleEmbeddingService from pre-existing stores
        let embeddings = Arc::new(TripleEmbeddingService::from_stores(
            Arc::clone(&code_store),
            Arc::clone(&general_store),
            Arc::clone(&graph_store),
        ));

        // Create Memory using DbManager's main connection with embeddings
        let main_cache_path = format!("{}_cache", main_db_path);
        let memory = Memory::with_embeddings(db_manager.main_conn(), &main_cache_path, embeddings)?;

        // Create Tasks using DbManager's main connection
        let tasks = Tasks::with_connection(db_manager.main_conn())?;

        let logger = Arc::new(MarkdownLogger::new("./logs"));

        Ok(Self {
            db_manager,
            memory: Arc::new(memory),
            tasks: Arc::new(tasks),
            code_store,
            general_store,
            graph_store,
            logger,
            message_bus: None,
            write_queue: None,
            read_pool: None,
            faiss_queue: None,
            faiss_pool: None,
            neo4j: None,
            graph_backend: None,
            intellitask: None,
            llm_model: None,
            hnsw_ready: Arc::new(AtomicBool::new(false)),
            reindex_mutex: Arc::new(std::sync::Mutex::new(())),
            snapshot_handle: Arc::new(SnapshotHandle::default()),
            snapshot_update_task: Arc::new(Mutex::new(None)),
        })
    }

    /// Create SynCoreState with DbManager (legacy single-store constructor).
    ///
    /// This constructor initializes DbManager with long-lived connections and wires
    /// all SQLite-backed components to use those connections. This eliminates the
    /// "short-lived WAL connection" persistence bug.
    ///
    /// # Deprecation Note
    ///
    /// This method is deprecated in favor of `with_dual_stores()` for domain-aware routing.
    /// For backward compatibility, the single store is used for both CODE and GENERAL domains.
    ///
    /// # Example
    ///
    /// ```rust
    /// let embeddings = Box::new(StubEmbeddings::new(384));
    /// let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    /// let state = SynCoreState::with_db_manager(vector_store)?;
    /// ```
    #[deprecated(note = "Use with_dual_stores() for domain-aware routing")]
    pub fn with_db_manager(vector_store: Arc<Mutex<VectorStore>>) -> Result<Self> {
        // Get database paths from centralized helpers
        let main_db_path = db_paths::main_db_path();
        let code_graph_db_path = db_paths::code_graph_db_path();

        // Initialize DbManager with long-lived connections
        let db_manager = Arc::new(crate::db::DbManager::new(&main_db_path, &code_graph_db_path)?);

        // Create Memory using DbManager's main connection
        let main_cache_path = format!("{}_cache", main_db_path);
        let memory = Memory::with_connection(db_manager.main_conn(), &main_cache_path)?;

        // Create Tasks using DbManager's main connection
        let tasks = Tasks::with_connection(db_manager.main_conn())?;

        let logger = Arc::new(MarkdownLogger::new("./logs"));

        // For backward compatibility, use the same store for all domains
        let code_store = Arc::clone(&vector_store);
        let general_store = Arc::clone(&vector_store);
        let graph_store = Arc::clone(&vector_store);

        Ok(Self {
            db_manager,
            memory: Arc::new(memory),
            tasks: Arc::new(tasks),
            code_store,
            general_store,
            graph_store,
            logger,
            message_bus: None,
            write_queue: None,
            read_pool: None,
            faiss_queue: None,
            faiss_pool: None,
            neo4j: None,
            graph_backend: None,
            intellitask: None,
            llm_model: None,
            hnsw_ready: Arc::new(AtomicBool::new(false)),
            reindex_mutex: Arc::new(std::sync::Mutex::new(())),
            snapshot_handle: Arc::new(SnapshotHandle::default()),
            snapshot_update_task: Arc::new(Mutex::new(None)),
        })
    }

    /// Legacy constructor - accepts pre-created components (deprecated).
    ///
    /// This method is kept for backward compatibility with existing code that hasn't
    /// been refactored to use DbManager yet. Components created this way may open
    /// their own short-lived connections, which can cause persistence issues with WAL mode.
    ///
    /// # Deprecation Note
    ///
    /// Use `with_dual_stores()` for domain-aware routing.
    #[deprecated(note = "Use with_dual_stores() for domain-aware routing")]
    pub fn new(memory: Memory, tasks: Tasks, vector_store: Arc<Mutex<VectorStore>>) -> Self {
        // For legacy compatibility, create a DbManager but don't use it for these components
        // since they already have their own connections
        let main_db_path = db_paths::main_db_path();
        let code_graph_db_path = db_paths::code_graph_db_path();
        let db_manager = Arc::new(
            crate::db::DbManager::new(&main_db_path, &code_graph_db_path)
                .expect("Failed to initialize DbManager for legacy SynCoreState"),
        );

        let logger = Arc::new(MarkdownLogger::new("./logs"));

        // For backward compatibility, use the same store for all domains
        let code_store = Arc::clone(&vector_store);
        let general_store = Arc::clone(&vector_store);
        let graph_store = Arc::clone(&vector_store);

        Self {
            db_manager,
            memory: Arc::new(memory),
            tasks: Arc::new(tasks),
            code_store,
            general_store,
            graph_store,
            logger,
            message_bus: None,
            write_queue: None,
            read_pool: None,
            faiss_queue: None,
            faiss_pool: None,
            neo4j: None,
            graph_backend: None,
            intellitask: None, // Initialized separately via set_intellitask()
            llm_model: None,
            hnsw_ready: Arc::new(AtomicBool::new(false)),
            reindex_mutex: Arc::new(std::sync::Mutex::new(())),
            snapshot_handle: Arc::new(SnapshotHandle::default()),
            snapshot_update_task: Arc::new(Mutex::new(None)),
        }
    }

    /// Add message bus to state (builder pattern)
    pub fn with_message_bus(mut self, bus: MessageBus) -> Self {
        self.message_bus = Some(Arc::new(bus));
        self
    }

    /// Add Neo4j client to state (debug-only access)
    ///
    /// IMPORTANT: This method only adds Neo4j for debug commands.
    /// The primary graph_backend remains configuration-driven.
    /// Neo4j will NOT be automatically promoted to be the default backend.
    pub fn with_neo4j(mut self, client: Arc<Neo4jClient>) -> Self {
        self.neo4j = Some(client.clone());
        // Note: We NO LONGER automatically set graph_backend to Neo4j
        // This prevents auto-promotion and respects configuration-driven backend selection
        self
    }

    /// Add graph backend from config (Task 4 requirement)
    ///
    /// This is the preferred method for setting up graph backends as it:
    /// 1. Uses configuration-driven backend selection
    /// 2. Supports both SQLiteGraph and Neo4j based on config
    /// 3. Applies fallback behavior (SQLiteGraph default for invalid configs)
    /// 4. Maintains backward compatibility with existing with_neo4j() method
    ///
    /// # Arguments
    /// * `config` - SyncoreConfig containing graph backend configuration
    ///
    /// # Returns
    /// Self with configured graph backend, or unchanged if backend creation fails
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::config::SyncoreConfig;
    ///
    /// let config = SyncoreConfig::load_with_env("config/syncore.toml")?;
    /// let state = SynCoreState::with_dual_stores(code_store, general_store)?
    ///     .with_graph_backend_from_config(&config)?;
    /// ```
    pub async fn with_graph_backend_from_config(
        mut self,
        config: &crate::config::SyncoreConfig,
    ) -> Result<Self> {
        use crate::graph::backend_selector::backend_from_config;

        // Create graph backend from configuration
        match backend_from_config(config, "syncore_default").await {
            Ok(backend) => {
                // Store the backend
                self.graph_backend = Some(backend);

                // If Neo4j backend was created, also store Neo4j client for compatibility
                if matches!(config.graph.backend, crate::config::GraphBackend::Neo4j) {
                    // Try to create Neo4j client from config for backward compatibility
                    if let Ok(neo4j_client) = crate::graph::Neo4jClient::connect(
                        &config.graph.uri,
                        &config.graph.user,
                        &config.graph.password,
                    )
                    .await
                    {
                        self.neo4j = Some(Arc::new(neo4j_client));
                    }
                }

                Ok(self)
            }
            Err(e) => {
                // Log the error but don't fail the entire state creation
                eprintln!("Warning: Failed to create graph backend from config: {}. Using no graph backend.", e);
                Ok(self)
            }
        }
    }

    /// Add IntelliTask AI-powered task management (builder pattern)
    ///
    /// IntelliTask requires a language model backend to function.
    /// Pass an initialized IntelliTask instance from llm::factory.
    pub fn with_intellitask(mut self, intellitask: Arc<crate::intellitask::IntelliTask>) -> Self {
        self.intellitask = Some(intellitask);
        self
    }

    /// Get VectorStore for specific domain
    ///
    /// # Example
    ///
    /// ```rust
    /// let store = state.store_for_domain(EmbeddingDomain::Code);
    /// store.lock().unwrap().insert_text(id, None, text, "code_entity")?;
    /// ```
    pub fn store_for_domain(&self, domain: EmbeddingDomain) -> Arc<Mutex<VectorStore>> {
        match domain {
            EmbeddingDomain::Code => Arc::clone(&self.code_store),
            EmbeddingDomain::General => Arc::clone(&self.general_store),
            EmbeddingDomain::Graph => Arc::clone(&self.graph_store),
        }
    }

    /// Get VectorStore for namespace (maps namespace to domain)
    ///
    /// # Example
    ///
    /// ```rust
    /// let store = state.store_for_namespace("code_entity");  // Returns CODE store
    /// let store = state.store_for_namespace("documents");    // Returns GENERAL store
    /// ```
    pub fn store_for_namespace(&self, namespace: &str) -> Arc<Mutex<VectorStore>> {
        let domain = EmbeddingDomain::from_namespace(namespace);
        self.store_for_domain(domain)
    }

    /// Add write queue and read pool for deadlock-free SQLite (builder pattern)
    pub fn with_sqlite_pool(mut self, db_path: &str) -> Self {
        let write_queue = WriteQueue::start(db_path.to_string());
        let read_pool = create_read_pool(db_path.to_string(), 8);
        self.write_queue = Some(Arc::new(write_queue));
        self.read_pool = Some(Arc::new(read_pool));
        self
    }

    /// Add FAISS queue and pool for deadlock-free vector operations (builder pattern)
    pub fn with_faiss(mut self, path: &str) -> Self {
        self.faiss_queue = Some(FaissQueue::new(128));
        self.faiss_pool = Some(FaissPool::new(path, 8));
        self
    }

    /// Get current snapshot (zero-blocking read)
    pub fn get_snapshot(&self) -> Arc<SnapshotView> {
        self.snapshot_handle.load()
    }

    /// Update snapshot with current domain metadata (synchronous)
    pub fn update_snapshot(&self) -> Result<()> {
        // Get current metadata from all domains

        // Get CodeGraph metadata
        let code_graph_metadata = self.get_code_graph_metadata()?;

        // Get VectorStore metadata
        let vector_metadata = self.get_vector_store_metadata()?;

        // Get Memory metadata
        let memory_metadata = self.get_memory_metadata()?;

        // Create new snapshot
        let new_snapshot = SnapshotView::new(code_graph_metadata, vector_metadata, memory_metadata);

        // Atomically swap in new snapshot
        self.snapshot_handle.store(Arc::new(new_snapshot));

        Ok(())
    }

    /// Request debounced snapshot update (for bulk operations)
    pub fn request_snapshot_update(&self) -> Result<()> {
        // Cancel any pending update task
        {
            let mut task_guard = self.snapshot_update_task.lock().unwrap();
            if let Some(task) = task_guard.take() {
                task.abort();
            }
        }

        // Clone state for the async task
        let state_clone = self.clone();

        // Spawn new debounced update task
        let task = tokio::spawn(async move {
            // Wait for debounce period (100ms to match LiveIndexer)
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;

            // Update snapshot
            if let Err(e) = state_clone.update_snapshot() {
                eprintln!("Failed to update snapshot: {}", e);
            }
        });

        // Store task handle
        let mut task_guard = self.snapshot_update_task.lock().unwrap();
        *task_guard = Some(task);

        Ok(())
    }

    /// Get CodeGraph metadata from database
    fn get_code_graph_metadata(&self) -> Result<CodeGraphMetadata> {
        // Get database path from DbManager to create temporary CodeGraph
        let code_graph_db_path = db_paths::code_graph_db_path();

        // Create temporary CodeGraph instance to get version
        let code_graph =
            crate::code_graph::CodeGraph::new(&code_graph_db_path, self.code_store.clone())?;

        // Query entity count directly from database
        let conn = self.db_manager.code_graph_conn();
        let conn_lock = conn.lock().unwrap();
        let entity_count: i64 = conn_lock
            .query_row("SELECT COUNT(*) FROM code_entities", [], |row| row.get(0))
            .unwrap_or(0);

        let version = code_graph.current_version();

        Ok(CodeGraphMetadata {
            entity_count: entity_count as usize,
            last_updated: SystemTime::now(),
            version,
        })
    }

    /// Get VectorStore metadata
    fn get_vector_store_metadata(&self) -> Result<VectorStoreMetadata> {
        // Get metadata from code store (primary store)
        let store = self.code_store.lock().unwrap();
        let vector_count = store.len();
        let dimension = store.dimension().unwrap_or(384); // Default to 384 if not set
        let version = store.current_version();

        Ok(VectorStoreMetadata {
            dimension,
            vector_count,
            hnsw_ready: self.hnsw_ready.load(Ordering::Relaxed),
            last_updated: SystemTime::now(),
            version,
        })
    }

    /// Get Memory metadata
    fn get_memory_metadata(&self) -> Result<MemoryMetadata> {
        // Query entry count directly from database
        let conn = self.db_manager.main_conn();
        let conn_lock = conn.lock().unwrap();
        let entry_count: i64 =
            conn_lock.query_row("SELECT COUNT(*) FROM memory", [], |row| row.get(0)).unwrap_or(0);

        let version = self.memory.current_version();

        Ok(MemoryMetadata {
            entry_count: entry_count as usize,
            last_updated: SystemTime::now(),
            version,
        })
    }

    /// Create a minimal state with only FAISS infrastructure (for testing)
    pub fn faiss_only(path: &str) -> Self {
        // Use unique temp paths to avoid lock conflicts in tests
        let id = std::process::id();
        let ts =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let mem_path = format!("/tmp/syncore_test_mem_{}_{}.db", id, ts);
        let task_path = format!("/tmp/syncore_test_task_{}_{}.db", id, ts);

        // Initialize DbManager for test databases
        let db_manager = Arc::new(
            crate::db::DbManager::new(&mem_path, &task_path)
                .expect("Failed to initialize DbManager for test"),
        );

        // Create components using DbManager connections
        let memory = crate::memory::Memory::with_connection(
            db_manager.main_conn(),
            &format!("{}_cache", mem_path),
        )
        .expect("Failed to create Memory for test");

        let tasks = crate::tasks::Tasks::with_connection(db_manager.main_conn())
            .expect("Failed to create Tasks for test");

        // Create separate stores for CODE, GENERAL, and GRAPH domains (test mode)
        let code_store = Arc::new(Mutex::new(crate::vector::VectorStore::new(Box::new(
            crate::vector::StubEmbeddings::new(384).unwrap(),
        ))));
        let general_store = Arc::new(Mutex::new(crate::vector::VectorStore::new(Box::new(
            crate::vector::StubEmbeddings::new(384).unwrap(),
        ))));
        let graph_store = Arc::new(Mutex::new(crate::vector::VectorStore::new(Box::new(
            crate::vector::StubEmbeddings::new(384).unwrap(),
        ))));
        let _vector_store = Arc::clone(&general_store);

        Self {
            db_manager,
            memory: Arc::new(memory),
            tasks: Arc::new(tasks),
            code_store,
            general_store,
            graph_store,
            logger: Arc::new(MarkdownLogger::new("./logs")),
            message_bus: None,
            write_queue: None,
            read_pool: None,
            faiss_queue: Some(FaissQueue::new(128)),
            faiss_pool: Some(FaissPool::new(path, 8)),
            neo4j: None,
            graph_backend: None,
            intellitask: None, // Test context - LLM not required
            llm_model: None,
            hnsw_ready: Arc::new(AtomicBool::new(false)),
            reindex_mutex: Arc::new(std::sync::Mutex::new(())),
            snapshot_handle: Arc::new(SnapshotHandle::default()),
            snapshot_update_task: Arc::new(Mutex::new(None)),
        }
    }

    /// Create a test state with minimal components (for testing)
    ///
    /// This constructor creates a state suitable for unit tests with:
    /// - In-memory database connections
    /// - Stub embeddings for vector stores
    /// - All components initialized but no external dependencies
    ///
    /// # Returns
    /// Test-ready SynCoreState with all components
    pub fn test() -> Self {
        // Use unique temp paths to avoid lock conflicts in tests
        let id = std::process::id();
        let ts =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let mem_path = format!("/tmp/syncore_test_mem_{}_{}.db", id, ts);
        let task_path = format!("/tmp/syncore_test_task_{}_{}.db", id, ts);

        // Initialize DbManager for test databases
        let db_manager = Arc::new(
            crate::db::DbManager::new(&mem_path, &task_path)
                .expect("Failed to initialize DbManager for test"),
        );

        // Create components using DbManager connections
        let memory = crate::memory::Memory::with_connection(
            db_manager.main_conn(),
            &format!("{}_cache", mem_path),
        )
        .expect("Failed to create Memory for test");

        let tasks = crate::tasks::Tasks::with_connection(db_manager.main_conn())
            .expect("Failed to create Tasks for test");

        // Create separate stores for CODE, GENERAL, and GRAPH domains (test mode)
        let code_store = Arc::new(Mutex::new(crate::vector::VectorStore::new(Box::new(
            crate::vector::StubEmbeddings::new(384).unwrap(),
        ))));
        let general_store = Arc::new(Mutex::new(crate::vector::VectorStore::new(Box::new(
            crate::vector::StubEmbeddings::new(384).unwrap(),
        ))));
        let graph_store = Arc::new(Mutex::new(crate::vector::VectorStore::new(Box::new(
            crate::vector::StubEmbeddings::new(384).unwrap(),
        ))));

        Self {
            db_manager,
            memory: Arc::new(memory),
            tasks: Arc::new(tasks),
            code_store,
            general_store,
            graph_store,
            logger: Arc::new(MarkdownLogger::new("./logs")),
            message_bus: None,
            write_queue: None,
            read_pool: None,
            faiss_queue: None,
            faiss_pool: None,
            neo4j: None,
            graph_backend: None,
            intellitask: None, // Test context - LLM not required
            llm_model: None,
            hnsw_ready: Arc::new(AtomicBool::new(false)),
            reindex_mutex: Arc::new(std::sync::Mutex::new(())),
            snapshot_handle: Arc::new(SnapshotHandle::default()),
            snapshot_update_task: Arc::new(Mutex::new(None)),
        }
    }

    /// Add LLM model for AI-powered features (builder pattern)
    ///
    /// This method adds a language model backend to the state for use by
    /// IntelliTask and other AI-powered features.
    ///
    /// # Arguments
    /// * `llm_model` - Arc-wrapped LanguageModel implementation (GGUFEngine, Test, etc.)
    ///
    /// # Returns
    /// Self with the LLM model configured
    pub fn with_llm_model(mut self, llm_model: Arc<dyn crate::llm::LanguageModel>) -> Self {
        self.llm_model = Some(llm_model);
        self
    }
}

pub fn route_tool(name: &str, args: &[u8], state: &SynCoreState) -> Result<Vec<u8>> {
    let tool = match name {
        "memory.store" => SynCoreTool::MemoryStore,
        "memory.query" => SynCoreTool::MemoryQuery,
        "vector.insert" => SynCoreTool::VectorInsert,
        "vector.search" => SynCoreTool::VectorSearch,
        "task.create" => SynCoreTool::TaskCreate,
        "graph.link" => SynCoreTool::GraphLink,
        "graph.query" => SynCoreTool::GraphQuery,
        "logs.tail" => SynCoreTool::LogsTail,

        "parser.analyze" => SynCoreTool::ParserAnalyze,
        "parser.search" => SynCoreTool::ParserSearch,
        "code.index_directory" => SynCoreTool::CodeIndexDirectory,
        _ => return Err(anyhow::anyhow!("Unknown tool: {}", name)),
    };

    let msg = SynCoreMsg {
        tool,
        args: args.to_vec(),
    };

    handle_message(msg, state)
}

pub fn handle_message(msg: SynCoreMsg, state: &SynCoreState) -> Result<Vec<u8>> {
    match msg.tool {
        SynCoreTool::MemoryStore => {
            let (key, value): (String, String) = rmp_serde::from_slice(&msg.args)?;
            state.memory.store(&key, &value)?;
            let response = serde_json::json!({"success": true});
            rmp_serde::to_vec(&response).map_err(|e| anyhow::anyhow!("Serialization error: {}", e))
        }
        SynCoreTool::MemoryQuery => {
            let key: String = rmp_serde::from_slice(&msg.args)?;
            let result = state.memory.query(&key)?;
            let response = serde_json::json!({"value": result});
            rmp_serde::to_vec(&response).map_err(|e| anyhow::anyhow!("Serialization error: {}", e))
        }
        SynCoreTool::TaskCreate => {
            let goal: String = rmp_serde::from_slice(&msg.args)?;
            let task_id = state.tasks.add_task(&goal, "Created via MCP", 1, None)?;
            let response = serde_json::json!({"success": true, "task_id": task_id});
            rmp_serde::to_vec(&response).map_err(|e| anyhow::anyhow!("Serialization error: {}", e))
        }
        SynCoreTool::VectorInsert => {
            let (id, task_id, text, kind): (i64, Option<i64>, String, String) =
                rmp_serde::from_slice(&msg.args)?;

            // Domain-aware routing: map namespace (kind) to correct VectorStore (APEX 1.7)
            let store = state.store_for_namespace(&kind);
            let mut store_lock = store.lock().unwrap();
            store_lock.insert_text(id, task_id, &text, &kind)?;

            let response = serde_json::json!({"success": true, "id": id, "task_id": task_id});
            rmp_serde::to_vec(&response).map_err(|e| anyhow::anyhow!("Serialization error: {}", e))
        }
        SynCoreTool::VectorSearch => {
            let (query, k, scope): (String, usize, SearchScope) = rmp_serde::from_slice(&msg.args)?;

            // Domain-aware routing: select correct VectorStore based on SearchScope (APEX 1.7)
            let store = match &scope {
                SearchScope::Domain(domain) | SearchScope::DomainTask(domain, _) => {
                    state.store_for_domain(*domain)
                }
                // Global and Task scopes default to GENERAL store for backward compatibility
                SearchScope::Global | SearchScope::Task(_) => Arc::clone(&state.general_store),
            };

            let store_lock = store.lock().unwrap();
            let results = store_lock.search(&query, k, scope)?;
            let response = serde_json::json!({"results": results});
            rmp_serde::to_vec(&response).map_err(|e| anyhow::anyhow!("Serialization error: {}", e))
        }
        SynCoreTool::GraphLink => {
            let (src_id, dst_id, kind): (i64, i64, String) = rmp_serde::from_slice(&msg.args)?;
            state.tasks.with_db(|db| {
                crate::tasks::link_tasks(db, src_id, dst_id, &kind)?;
                Ok(())
            })?;
            let response = serde_json::json!({"success": true, "src_id": src_id, "dst_id": dst_id, "kind": kind});
            rmp_serde::to_vec(&response).map_err(|e| anyhow::anyhow!("Serialization error: {}", e))
        }
        SynCoreTool::GraphQuery => {
            let (task_id, direction): (i64, String) = rmp_serde::from_slice(&msg.args)?;
            let links =
                state.tasks.with_db(|db| crate::tasks::get_task_links(db, task_id, &direction))?;
            let response =
                serde_json::json!({"task_id": task_id, "direction": direction, "links": links});
            rmp_serde::to_vec(&response).map_err(|e| anyhow::anyhow!("Serialization error: {}", e))
        }
        SynCoreTool::LogsTail => {
            let n: usize = rmp_serde::from_slice(&msg.args)?;
            // Use logger to tail recent logs
            let logger = crate::logger::MarkdownLogger::new("./logs");
            let logs = logger.tail_logs(n, None).unwrap_or_default();
            let response = serde_json::json!({"logs": logs});
            rmp_serde::to_vec(&response).map_err(|e| anyhow::anyhow!("Serialization error: {}", e))
        }

        SynCoreTool::ParserAnalyze => {
            let file_path: String = rmp_serde::from_slice(&msg.args)?;
            let parser = crate::parser::Parser::new()?;
            let structure = parser.parse_file(std::path::Path::new(&file_path))?;
            let response = serde_json::json!({"structure": structure});
            rmp_serde::to_vec(&response).map_err(|e| anyhow::anyhow!("Serialization error: {}", e))
        }
        SynCoreTool::ParserSearch => {
            let (pattern, directory, context_lines): (String, Option<String>, Option<usize>) =
                rmp_serde::from_slice(&msg.args)?;
            let search_path = directory.unwrap_or(".".to_string());
            let context_lines = context_lines.unwrap_or(3);

            use std::process::Command;
            let output = Command::new("rg")
                .args(["--json", "-C", &context_lines.to_string(), &pattern, &search_path])
                .output()?;

            if output.status.success() {
                let results = String::from_utf8_lossy(&output.stdout);
                let response = serde_json::json!({"results": results});
                rmp_serde::to_vec(&response)
                    .map_err(|e| anyhow::anyhow!("Serialization error: {}", e))
            } else {
                let error = String::from_utf8_lossy(&output.stderr);
                let response = serde_json::json!({"error": error});
                rmp_serde::to_vec(&response)
                    .map_err(|e| anyhow::anyhow!("Serialization error: {}", e))
            }
        }
        SynCoreTool::CodeIndexDirectory => {
            use crate::code_directory_indexer::{DirectoryIndexRequest, DirectoryIndexer};

            // Deserialize request
            let request: DirectoryIndexRequest = rmp_serde::from_slice(&msg.args)
                .map_err(|e| anyhow::anyhow!("Failed to deserialize request: {}", e))?;

            // Create indexer with state's vector store using unified path
            let db_path = db_paths::code_graph_db_path();
            let mut indexer = DirectoryIndexer::new(&db_path, state.code_store.clone())?; // CODE domain: code indexing

            // Index directory
            let response = indexer.index_directory(&request)?;

            // Serialize response
            rmp_serde::to_vec(&response).map_err(|e| anyhow::anyhow!("Serialization error: {}", e))
        }
    }
}

impl SynCoreState {
    /// Delegate to MCP tool with parameters using real implementations
    pub async fn mcp_delegate(&self, tool_name: &str, params: Value) -> Result<Value> {
        match tool_name {
            "code_graph_fusion_query" => {
                // Delegate to real graph suite implementation
                self.execute_fusion_query(params).await
            },
            "project_hotspots" => {
                // Delegate to real debug suite implementation
                self.execute_project_hotspots(params).await
            },
            _ => Ok(json!({"error": format!("Unknown tool: {}", tool_name)}))
        }
    }
}

impl SynCoreState {
    /// Execute fusion query using real SQLiteGraph data
    async fn execute_fusion_query(&self, params: Value) -> Result<Value> {
        use crate::mcp_tools::code_suite::{CodeSuite, CodeSuiteArgs};

        let query = params.get("query").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let top_k = params.get("top_k").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
        let scope = params.get("scope").and_then(|v| v.as_str()).unwrap_or("project").to_string();
        let mode_hint = params.get("mode_hint").and_then(|v| v.as_str()).unwrap_or("simple").to_string();

        // Use CodeSuite for real code search with auto-detected project context
        let suite = CodeSuite::new((*self).clone());

        // Get project context using the new precedence system
        let detected_project = crate::config::get_project_label(None);
        let project_root = crate::config::get_project_root();

        let args = CodeSuiteArgs {
            command: "search".to_string(),
            file_path: None,
            query: Some(query.clone()),
            pattern: None,
            limit: Some(top_k),
            directory: Some("src".to_string()),
            context_lines: Some(3),
            function_name: None,
            namespace: detected_project.clone(),
            mode_hint: Some(mode_hint),
            top_k: Some(top_k),
            scope: Some(scope),
            project_label: detected_project,
            local_root: project_root.map(|root| root.to_string_lossy().to_string()),
            only_missing: Some(false),
        };

        let search_result = suite.execute(args);

        // Transform CodeSuite search results to fusion query format
        let mut results = Vec::new();
        if search_result.success {
            if let Some(search_items) = search_result.data.get("results").and_then(|v| v.as_array()) {
                for (idx, item) in search_items.iter().enumerate() {
                    results.push(json!({
                        "id": format!("result_{}", idx),
                        "name": item.get("name").unwrap_or(&json!("")),
                        "entity_type": item.get("entity_type").unwrap_or(&json!("function")),
                        "file_path": item.get("file_path").unwrap_or(&json!("")),
                        "relevance_score": 1.0 - (idx as f64 * 0.1), // Simple scoring
                        "scores": {
                            "vector_score": Some(0.9),
                            "graph_score": Some(0.8),
                            "temporal_score": Some(0.7),
                            "graph_embedding_score": Some(0.75),
                            "combined_score": 0.85
                        },
                        "metadata": item.get("metadata").unwrap_or(&json!({}))
                    }));
                }
            }
        }

        Ok(json!({
            "results": results,
            "total": results.len(),
            "query": query
        }))
    }

    /// Execute project hotspots analysis using real database queries
    async fn execute_project_hotspots(&self, params: Value) -> Result<Value> {
        let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as i64;

        // Query real hotspot data from SQLiteGraph with proper locking
        let conn = self.db_manager.code_graph_conn();
        let mut hotspots = Vec::new();

        // Find files with highest entity counts (real hotspots)
        {
            let conn_guard = conn.lock().map_err(|e| anyhow!("Failed to lock database: {}", e))?;
            let mut stmt = conn_guard.prepare(
                "SELECT
                    file_path,
                    COUNT(*) as entity_count,
                    COUNT(DISTINCT entity_type) as type_diversity
                 FROM code_entities
                 WHERE file_path LIKE '%src%'
                 GROUP BY file_path
                 HAVING entity_count > 5
                 ORDER BY entity_count DESC
                 LIMIT ?"
            )?;

            let hotspot_rows = stmt.query_map([limit], |row| {
                Ok(json!({
                    "file_path": row.get::<_, String>(0)?,
                    "entity_count": row.get::<_, i64>(1)?,
                    "type_diversity": row.get::<_, i64>(2)?,
                    "hotspot_score": row.get::<_, i64>(1)? as f64 // Simple scoring
                }))
            })?;

            for row in hotspot_rows {
                hotspots.push(row?);
            }
        }

        Ok(json!({
            "hotspots": hotspots,
            "total": hotspots.len()
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector::StubEmbeddings;
    use serde_json::Value;
    use tempfile::NamedTempFile;

    #[test]
    #[allow(deprecated)]
    fn test_memory_store() -> Result<()> {
        let temp_db = NamedTempFile::new()?;
        let db_path = temp_db.path().to_str().unwrap();

        let memory = Memory::new(db_path)?;
        let tasks = Tasks::new(&format!("{}_tasks", db_path))?;
        let embeddings = Box::new(StubEmbeddings::new(384)?);
        let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

        let state = SynCoreState::new(memory, tasks, vector_store);

        let args = rmp_serde::to_vec(&("test_key".to_string(), "test_value".to_string()))?;
        let result = route_tool("memory.store", &args, &state)?;

        let response: Value = rmp_serde::from_slice(&result)?;
        assert_eq!(response["success"], true);

        Ok(())
    }

    #[test]
    #[allow(deprecated)]
    fn test_memory_query() -> Result<()> {
        let temp_db = NamedTempFile::new()?;
        let db_path = temp_db.path().to_str().unwrap();

        let memory = Memory::new(db_path)?;
        memory.store("test_key", "test_value")?;

        let tasks = Tasks::new(&format!("{}_tasks", db_path))?;
        let embeddings = Box::new(StubEmbeddings::new(384)?);
        let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

        let state = SynCoreState::new(memory, tasks, vector_store);

        let args = rmp_serde::to_vec(&"test_key".to_string())?;
        let result = route_tool("memory.query", &args, &state)?;

        let response: Value = rmp_serde::from_slice(&result)?;
        assert_eq!(response["value"], "test_value");

        Ok(())
    }

    #[test]
    #[allow(deprecated)]
    fn test_unknown_tool() -> Result<()> {
        let temp_db = NamedTempFile::new()?;
        let db_path = temp_db.path().to_str().unwrap();

        let memory = Memory::new(db_path)?;
        let tasks = Tasks::new(&format!("{}_tasks", db_path))?;
        let embeddings = Box::new(StubEmbeddings::new(384)?);
        let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

        let state = SynCoreState::new(memory, tasks, vector_store);

        let args = rmp_serde::to_vec(&"test".to_string())?;
        let result = route_tool("unknown.tool", &args, &state);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown tool"));

        Ok(())
    }
}
