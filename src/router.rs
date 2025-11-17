use anyhow::Result;
use crate::protocol::{SynCoreTool, SynCoreMsg};
use crate::memory::Memory;
use crate::tasks::Tasks;
use crate::vector::{VectorStore, SearchScope};
use crate::logger::{MarkdownLogger, CogLogger};
use crate::message_bus::MessageBus;
use crate::storage::{WriteQueue, ReadPool, create_read_pool, FaissQueue, FaissPool};
use crate::graph::Neo4jClient;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct SynCoreState {
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
}

impl SynCoreState {
    pub fn new(memory: Memory, tasks: Tasks, vector_store: Arc<Mutex<VectorStore>>) -> Self {
        let logger = Arc::new(MarkdownLogger::new("./logs"));
        Self {
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

        Self {
            memory: Arc::new(crate::memory::Memory::new(&mem_path).unwrap()),
            tasks: Arc::new(crate::tasks::Tasks::new(&task_path).unwrap()),
            vector_store: Arc::new(Mutex::new(crate::vector::VectorStore::new(
                Box::new(crate::vector::RealEmbeddings::new(384).unwrap()),
            ))),
            logger: Arc::new(MarkdownLogger::new("./logs")),
            message_bus: None,
            write_queue: None,
            read_pool: None,
            faiss_queue: Some(FaissQueue::new(128)),
            faiss_pool: Some(FaissPool::new(path, 8)),
            neo4j: None,
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
            let (id, task_id, text, kind): (i64, Option<i64>, String, String) = rmp_serde::from_slice(&msg.args)?;
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
            let links = state.tasks.with_db(|db| {
                crate::tasks::get_task_links(db, task_id, &direction)
            })?;
            let response = serde_json::json!({"task_id": task_id, "direction": direction, "links": links});
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
            let (pattern, directory, context_lines): (String, Option<String>, Option<usize>) = rmp_serde::from_slice(&msg.args)?;
            let search_path = directory.unwrap_or(".".to_string());
            let context_lines = context_lines.unwrap_or(3);

            use std::process::Command;
            let output = Command::new("rg")
                .args(&["--json", "-C", &context_lines.to_string(), &pattern, &search_path])
                .output()?;

            if output.status.success() {
                let results = String::from_utf8_lossy(&output.stdout);
                let response = serde_json::json!({"results": results});
                rmp_serde::to_vec(&response).map_err(|e| anyhow::anyhow!("Serialization error: {}", e))
            } else {
                let error = String::from_utf8_lossy(&output.stderr);
                let response = serde_json::json!({"error": error});
                rmp_serde::to_vec(&response).map_err(|e| anyhow::anyhow!("Serialization error: {}", e))
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
            use crate::code_directory_indexer::{DirectoryIndexer, DirectoryIndexRequest};

            // Deserialize request
            let request: DirectoryIndexRequest = rmp_serde::from_slice(&msg.args)
                .map_err(|e| anyhow::anyhow!("Failed to deserialize request: {}", e))?;

            // Create indexer with state's vector store
            let db_path = "syncore_code_graph.db";
            let mut indexer = DirectoryIndexer::new(db_path, state.vector_store.clone())?;

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
    use tempfile::NamedTempFile;
    use crate::vector::HuggingFaceEmbeddings;
    use serde_json::Value;

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
