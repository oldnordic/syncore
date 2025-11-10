use crate::protocol::{SynCoreMsg, SynCoreTool};
use crate::memory::Memory;
use crate::vector::{VectorStore, MockEmbeddings, SearchScope};
use crate::logger::MarkdownLogger;
use crate::taskmaster::TaskMaster;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone)]
pub struct SynCoreState {
    pub memory: Memory,
    pub vector_store: Arc<Mutex<VectorStore>>,
    pub logger: Arc<MarkdownLogger>,
    pub taskmaster: Arc<Mutex<TaskMaster>>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct VectorSearchArgs {
    query: String,
    #[serde(default = "default_k")]
    k: usize,
    #[serde(default = "default_scope")]
    scope: ScopeWire,
}

fn default_k() -> usize { 8 }

fn default_scope() -> ScopeWire { ScopeWire::Global }

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
enum ScopeWire {
    Global,
    Task { task_id: u64 },
}

#[derive(Serialize)]
struct VectorSearchResult<'a> {
    hits: &'a [(u64, f32)], // (step_id, score)
}

// Concurrency limits
static VECTOR_SEARCH_LIMIT: Semaphore = Semaphore::const_new(4);
static MEMORY_STORE_LIMIT: Semaphore = Semaphore::const_new(8);
static TASK_CREATE_LIMIT: Semaphore = Semaphore::const_new(4);

// Metrics counters
pub static RPC_INFLIGHT: AtomicU64 = AtomicU64::new(0);
pub static VEC_POINTS_TOTAL: AtomicU64 = AtomicU64::new(0);
pub static TASKS_OPEN_TOTAL: AtomicU64 = AtomicU64::new(0);
pub static VEC_SNAPSHOTS_TOTAL: AtomicU64 = AtomicU64::new(0);

// Deadline wrapper for tool execution
pub async fn with_deadline<F, T>(f: F) -> anyhow::Result<T>
where 
    F: std::future::Future<Output = anyhow::Result<T>>,
{
    use tokio::time::{timeout, Duration};
    Ok(timeout(Duration::from_secs(2), f).await??)
}

impl SynCoreState {
    pub fn new(memory: Memory, db_path: &str) -> anyhow::Result<Self> {
        let embeddings = Box::new(MockEmbeddings::new(384));
        let vector_store = VectorStore::new(embeddings);
        let logger = MarkdownLogger::new("logs");
        let taskmaster = TaskMaster::new(db_path)?;
        
        Ok(Self {
            memory,
            vector_store: Arc::new(Mutex::new(vector_store)),
            logger: Arc::new(logger),
            taskmaster: Arc::new(Mutex::new(taskmaster)),
        })
    }
}

pub async fn route(msg: SynCoreMsg, state: &SynCoreState) -> Vec<u8> {
    RPC_INFLIGHT.fetch_add(1, Ordering::Relaxed);
    let _guard = scopeguard::guard((), |_| {
        RPC_INFLIGHT.fetch_sub(1, Ordering::Relaxed);
    });
    
    let result = match msg.tool {
        SynCoreTool::MemoryStore => {
            let _permit = match MEMORY_STORE_LIMIT.try_acquire() {
                Ok(p) => p,
                Err(_) => {
                    return rmp_serde::to_vec(&serde_json::json!({
                        "error": {"code": "busy", "retry_after_ms": 250}
                    })).unwrap();
                }
            };
            
            let result = with_deadline(async {
                let (k, v): (String, String) = rmp_serde::from_slice(&msg.args)?;
                state.memory.store(&k, &v);
                anyhow::Ok("ok")
            }).await;
            
            drop(_permit);
            result.map(|v| rmp_serde::to_vec(&v).unwrap())
                .unwrap_or_else(|e| rmp_serde::to_vec(&serde_json::json!({
                    "error": {"code": "deadline_exceeded", "message": e.to_string()}
                })).unwrap())
        }
        SynCoreTool::MemoryQuery => {
            let result = with_deadline(async {
                let k: String = rmp_serde::from_slice(&msg.args)?;
                let v = state.memory.query(&k);
                anyhow::Ok(v)
            }).await;
            
            result.map(|v| rmp_serde::to_vec(&v).unwrap())
                .unwrap_or_else(|e| rmp_serde::to_vec(&serde_json::json!({
                    "error": {"code": "deadline_exceeded", "message": e.to_string()}
                })).unwrap())
        }
        SynCoreTool::VectorInsert => {
            let result = with_deadline(async {
                let (step_id, task_id, text): (u64, u64, String) = rmp_serde::from_slice(&msg.args)?;
                let mut store = state.vector_store.lock().unwrap();
                let point_id = store.insert(step_id, task_id, &text)?;
                VEC_POINTS_TOTAL.fetch_add(1, Ordering::Relaxed);
                anyhow::Ok(point_id)
            }).await;
            
            result.map(|id| rmp_serde::to_vec(&id).unwrap())
                .unwrap_or_else(|e| rmp_serde::to_vec(&serde_json::json!({
                    "error": {"code": "deadline_exceeded", "message": e.to_string()}
                })).unwrap())
        }
        SynCoreTool::VectorSearch => {
            let _permit = match VECTOR_SEARCH_LIMIT.try_acquire() {
                Ok(p) => p,
                Err(_) => {
                    return rmp_serde::to_vec(&serde_json::json!({
                        "error": {"code": "busy", "retry_after_ms": 250}
                    })).unwrap();
                }
            };
            
            let result = with_deadline(async {
                let (query, k, scope): (String, usize, SearchScope) = rmp_serde::from_slice(&msg.args)?;
                let store = state.vector_store.lock().unwrap();
                let results = store.search(&query, k, scope)?;
                anyhow::Ok(results)
            }).await;
            
            drop(_permit);
            result.map(|hits| {
                let response = VectorSearchResult { hits: &hits };
                rmp_serde::to_vec(&response).unwrap()
            }).unwrap_or_else(|e| rmp_serde::to_vec(&serde_json::json!({
                "error": {"code": "deadline_exceeded", "message": e.to_string()}
            })).unwrap())
        }
        SynCoreTool::LogsTail => {
            let result = with_deadline(async {
                let n: usize = rmp_serde::from_slice(&msg.args)?;
                let response = format!("Requested last {} log entries", n);
                anyhow::Ok(response)
            }).await;
            
            result.map(|resp| rmp_serde::to_vec(&resp).unwrap())
                .unwrap_or_else(|e| rmp_serde::to_vec(&serde_json::json!({
                    "error": {"code": "deadline_exceeded", "message": e.to_string()}
                })).unwrap())
        }
        SynCoreTool::TaskCreate => {
            let _permit = match TASK_CREATE_LIMIT.try_acquire() {
                Ok(p) => p,
                Err(_) => {
                    return rmp_serde::to_vec(&serde_json::json!({
                        "error": {"code": "busy", "retry_after_ms": 250}
                    })).unwrap();
                }
            };
            
            let result = with_deadline(async {
                let goal: String = rmp_serde::from_slice(&msg.args)?;
                let tm = state.taskmaster.lock().unwrap();
                let task_id = tm.add_task(goal, 5)?;
                TASKS_OPEN_TOTAL.fetch_add(1, Ordering::Relaxed);
                anyhow::Ok(task_id)
            }).await;
            
            drop(_permit);
            result.map(|task_id| {
                let response = serde_json::json!({
                    "task_id": task_id.to_string(),
                    "success": true
                });
                rmp_serde::to_vec(&response).unwrap()
            }).unwrap_or_else(|e| rmp_serde::to_vec(&serde_json::json!({
                "error": {"code": "deadline_exceeded", "message": e.to_string()}
            })).unwrap())
        }
        _ => rmp_serde::to_vec(&serde_json::json!({
            "error": {"code": "unimplemented", "message": "Tool not implemented"}
        })).unwrap(),
    };
    
    result
}

pub async fn handle_vector_search(args_mp: &[u8], state: &SynCoreState) -> anyhow::Result<Vec<(u64, f32)>> {
    // Accept either raw MessagePack OR JSON (some clients hand JSON straight through)
    let args: VectorSearchArgs = match rmp_serde::from_slice(args_mp) {
        Ok(v) => v,
        Err(_) => serde_json::from_slice(args_mp)?,
    };

    let scope = match args.scope {
        ScopeWire::Global => SearchScope::Global,
        ScopeWire::Task { task_id } => SearchScope::Task(task_id),
    };

    let store = state.vector_store.lock().unwrap();
    let hits = store.search(&args.query, args.k, scope)?;
    Ok(hits)
}