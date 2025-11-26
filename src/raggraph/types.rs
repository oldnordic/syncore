//! Core RagGraph types


pub type NodeId = i64;

/// RagGraph node with diffusion score
#[derive(Debug, Clone)]
pub struct RagGraphNode {
    pub id: NodeId,
    pub embedding: Vec<f32>,
    pub diffusion_score: f32,
}

/// RagGraph edge with weight
#[derive(Debug, Clone)]
pub struct RagGraphEdge {
    pub source: NodeId,
    pub target: NodeId,
    pub weight: f32,
}

/// Result from RagGraph query
#[derive(Debug, Clone)]
pub struct RagGraphResult {
    pub top_nodes: Vec<NodeId>,
    pub context_embedding: Vec<f32>,
    pub reasoning_path: Vec<String>,
}
