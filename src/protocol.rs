use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum SynCoreTool {
    MemoryStore,
    MemoryQuery,
    VectorInsert,
    VectorSearch,
    LogsTail,
    TaskCreate,
    GraphLink,
    GraphQuery,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SynCoreMsg {
    pub tool: SynCoreTool,
    pub args: Vec<u8>, // MessagePack-encoded inner payload
}