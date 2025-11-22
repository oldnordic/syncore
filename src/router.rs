use crate::common::db_paths;
use crate::graph::Neo4jClient;
use crate::logger::{CogLogger, MarkdownLogger};
use crate::memory::Memory;
use crate::message_bus::MessageBus;
use crate::protocol::{SynCoreMsg, SynCoreTool};
use crate::storage::{create_read_pool, FaissPool, FaissQueue, ReadPool, WriteQueue};
use crate::tasks::Tasks;
use crate::vector::{SearchScope, VectorStore};
use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct SynCoreState {
    /// Centralized SQLite connection manager (long-lived connections for WAL mode)
    pub db_manager: Arc<crate::db::DbManager>,
    pub memory: Arc<Memory>,
    pub tasks: Arc<Tasks>,
    pub vector_store: Arc<Mutex<VectorStore>>,
    pub logger: Arc<dyn CogLogger>,
    pub message_bus: Option<Arc<MessageBus>>,
    pub write_queue: Option<Arc<WriteQueue>>,
    pub read_pool: Option<Arc<ReadPool>>,
    pub faiss_queue: Option<Arc<FaissQueue>>,
    pub faiss_pool: Option<Arc<FaissPool>>,
    pub neo4j: Option<Arc<Neo4jClient>>,
    /// HNSW index warmup status - true when index is ready for fast search
    pub hnsw_ready: Arc<AtomicBool>,
}

impl SynCoreState {
    /// Create SynCoreState with DbManager (preferred constructor for production).
    ///
    /// This constructor initializes DbManager with long-lived connections and wires
    /// all SQLite-backed components to use those connections. This eliminates the
    /// "short-lived WAL connection" persistence bug.
    ///
    /// # Example
    ///
    /// ```rust
    /// let embeddings = Box::new(RealEmbeddings::new(384)?);
    /// let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    /// let state = SynCoreState::with_db_manager(vector_store)?;
    /// ```
    pub fn with_db_manager(vector_store: Arc<Mutex<VectorStore>>) -> Result<Self> {
        // Get database paths from centralized helpers
        let main_db_path = db_paths::main_db_path();
        let code_graph_db_path = db_paths::code_graph_db_path();

        // Initialize DbManager with long-lived connections
        let db_manager = Arc::new(crate::db::DbManager::new(
            &main_db_path,
            &code_graph_db_path,
        )?);

        // Create Memory using DbManager's main connection
        let main_cache_path = format!("{}_cache", main_db_path);
        let memory = Memory::with_connection(db_manager.main_conn(), &main_cache_path)?;

        // Create Tasks using DbManager's main connection
        let tasks = Tasks::with_connection(db_manager.main_conn())?;

        let logger = Arc::new(MarkdownLogger::new("./logs"));

        Ok(Self {
            db_manager,
            memory: Arc::new(memory),
            tasks: Arc::new(tasks),
            vector_store,
            logger,
            message_bus: None,
            write_queue: None,
            read_pool: None,
            faiss_queue: None,
            faiss_pool: None,
            neo4j: None,
            hnsw_ready: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Legacy constructor - accepts pre-created components (deprecated, use with_db_manager instead).
    ///
    /// This method is kept for backward compatibility with existing code that hasn't
    /// been refactored to use DbManager yet. Components created this way may open
    /// their own short-lived connections, which can cause persistence issues with WAL mode.
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
        Self {
            db_manager,
            memory: Arc::new(memory),
            tasks: Arc::new(tasks),
            vector_store,
            logger,
            message_bus: None,
            write_queue: None,
            read_pool: None,
            faiss_queue: None,
            faiss_pool: None,
            neo4j: None,
            hnsw_ready: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Add message bus to state (builder pattern)
    pub fn with_message_bus(mut self, bus: MessageBus) -> Self {
        self.message_bus = Some(Arc::new(bus));
        self
    }

    /// Add Neo4j client to state (builder pattern)
    pub fn with_neo4j(mut self, client: Arc<Neo4jClient>) -> Self {
        self.neo4j = Some(client);
        self
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

    /// Create a minimal state with only FAISS infrastructure (for testing)
    pub fn faiss_only(path: &str) -> Self {
        // Use unique temp paths to avoid lock conflicts in tests
        let id = std::process::id();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
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

        Self {
            db_manager,
            memory: Arc::new(memory),
            tasks: Arc::new(tasks),
            vector_store: Arc::new(Mutex::new(crate::vector::VectorStore::new(Box::new(
                crate::vector::RealEmbeddings::new(384).unwrap(),
            )))),
            logger: Arc::new(MarkdownLogger::new("./logs")),
            message_bus: None,
            write_queue: None,
            read_pool: None,
            faiss_queue: Some(FaissQueue::new(128)),
            faiss_pool: Some(FaissPool::new(path, 8)),
            neo4j: None,
            hnsw_ready: Arc::new(AtomicBool::new(false)),
        }
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
        "sequential.cycle" => SynCoreTool::SequentialCycle,
        "parser.analyze" => SynCoreTool::ParserAnalyze,
        "parser.search" => SynCoreTool::ParserSearch,
        "code.explain" => SynCoreTool::CodeExplain,
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
            let mut store = state.vector_store.lock().unwrap();
            store.insert_text(id, task_id, &text, &kind)?;
            let response = serde_json::json!({"success": true, "id": id, "task_id": task_id});
            rmp_serde::to_vec(&response).map_err(|e| anyhow::anyhow!("Serialization error: {}", e))
        }
        SynCoreTool::VectorSearch => {
            let (query, k, scope): (String, usize, SearchScope) = rmp_serde::from_slice(&msg.args)?;
            let store = state.vector_store.lock().unwrap();
            let results = store.search(&query, k, scope)?;
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
            let links = state
                .tasks
                .with_db(|db| crate::tasks::get_task_links(db, task_id, &direction))?;
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
        SynCoreTool::SequentialCycle => {
            let max_cycles: Option<usize> = rmp_serde::from_slice(&msg.args)?;
            let sequential_core = crate::sequential::SequentialCore::new(
                state.tasks.clone(),
                state.vector_store.clone(),
                state.memory.clone(),
                Arc::new(Mutex::new(crate::sequential::DemoLanguageModel::new())),
                state.logger.clone(),
            );
            let results = sequential_core.run_batch_cycles(max_cycles.unwrap_or(1))?;
            let response = serde_json::json!({"success": true, "cycles_processed": results.len()});
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
                .args(&[
                    "--json",
                    "-C",
                    &context_lines.to_string(),
                    &pattern,
                    &search_path,
                ])
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
        SynCoreTool::CodeExplain => {
            use crate::code_explainer::{CodeExplainer, ExplainRequest};

            // Deserialize request
            let request: ExplainRequest = rmp_serde::from_slice(&msg.args)
                .map_err(|e| anyhow::anyhow!("Failed to deserialize request: {}", e))?;

            // Create explainer (with custom model if specified)
            let explainer = if let Some(ref model) = request.model {
                CodeExplainer::new_with_model(model)?
            } else {
                CodeExplainer::new()?
            };

            // Get explanation
            let response = explainer.explain(&request)?;

            // Serialize response
            rmp_serde::to_vec(&response).map_err(|e| anyhow::anyhow!("Serialization error: {}", e))
        }
        SynCoreTool::CodeIndexDirectory => {
            use crate::code_directory_indexer::{DirectoryIndexRequest, DirectoryIndexer};

            // Deserialize request
            let request: DirectoryIndexRequest = rmp_serde::from_slice(&msg.args)
                .map_err(|e| anyhow::anyhow!("Failed to deserialize request: {}", e))?;

            // Create indexer with state's vector store using unified path
            let db_path = db_paths::code_graph_db_path();
            let mut indexer = DirectoryIndexer::new(&db_path, state.vector_store.clone())?;

            // Index directory
            let response = indexer.index_directory(&request)?;

            // Serialize response
            rmp_serde::to_vec(&response).map_err(|e| anyhow::anyhow!("Serialization error: {}", e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector::HuggingFaceEmbeddings;
    use serde_json::Value;
    use tempfile::NamedTempFile;

    #[test]
    fn test_memory_store() -> Result<()> {
        let temp_db = NamedTempFile::new()?;
        let db_path = temp_db.path().to_str().unwrap();

        let memory = Memory::new(db_path)?;
        let tasks = Tasks::new(&format!("{}_tasks", db_path))?;
        let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
        let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

        let state = SynCoreState::new(memory, tasks, vector_store);

        let args = rmp_serde::to_vec(&("test_key".to_string(), "test_value".to_string()))?;
        let result = route_tool("memory.store", &args, &state)?;

        let response: Value = rmp_serde::from_slice(&result)?;
        assert_eq!(response["success"], true);

        Ok(())
    }

    #[test]
    fn test_memory_query() -> Result<()> {
        let temp_db = NamedTempFile::new()?;
        let db_path = temp_db.path().to_str().unwrap();

        let memory = Memory::new(db_path)?;
        memory.store("test_key", "test_value")?;

        let tasks = Tasks::new(&format!("{}_tasks", db_path))?;
        let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
        let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

        let state = SynCoreState::new(memory, tasks, vector_store);

        let args = rmp_serde::to_vec(&"test_key".to_string())?;
        let result = route_tool("memory.query", &args, &state)?;

        let response: Value = rmp_serde::from_slice(&result)?;
        assert_eq!(response["value"], "test_value");

        Ok(())
    }

    #[test]
    fn test_unknown_tool() -> Result<()> {
        let temp_db = NamedTempFile::new()?;
        let db_path = temp_db.path().to_str().unwrap();

        let memory = Memory::new(db_path)?;
        let tasks = Tasks::new(&format!("{}_tasks", db_path))?;
        let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
        let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

        let state = SynCoreState::new(memory, tasks, vector_store);

        let args = rmp_serde::to_vec(&"test".to_string())?;
        let result = route_tool("unknown.tool", &args, &state);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown tool"));

        Ok(())
    }
}
