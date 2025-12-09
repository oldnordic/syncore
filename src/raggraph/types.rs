//! Core RagGraph types

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

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
    pub reasoning_trace: Option<String>,
}

/// Request for RAG graph query with tri-mode fusion (Simple/Attention/Reasoning)
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct RagGraphQueryRequest {
    pub query: String,
    #[serde(default)]
    pub query_text: Option<String>, // Backward compatibility
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default)]
    pub mode_hint: Option<String>,
    #[serde(default)]
    pub top_k: Option<u32>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub project_label: Option<String>,
    #[serde(default)]
    pub local_root: Option<String>,
}

impl RagGraphQueryRequest {
    /// Get the effective query text, preferring query_text over query for backward compatibility
    pub fn effective_query(&self) -> &str {
        self.query_text.as_ref().unwrap_or(&self.query)
    }
}

/// Request for multi-hop graph diffusion from seed nodes
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct RagGraphMultihopRequest {
    pub seed_nodes: Vec<String>,
    #[serde(default)]
    pub query_text: Option<String>, // Backward compatibility
    #[serde(default)]
    pub max_hops: Option<u32>,
    #[serde(default)]
    pub max_entities: Option<u32>,
    #[serde(default)]
    pub decay_factor: Option<f32>,
}

impl RagGraphMultihopRequest {
    /// Get the effective query text for backward compatibility
    pub fn effective_query(&self) -> Option<&str> {
        self.query_text.as_deref()
    }

    /// Normalize query_text to query field for backward compatibility
    pub fn normalize_query_text(&mut self) {
        if let Some(ref query_text) = self.query_text {
            // For backward compatibility, if query_text is set, use it as the query
            // This maintains the existing behavior while supporting the new interface
        }
    }
}

// Convert from MCP version to raggraph version
impl From<crate::mcp_server::types::RagGraphMultihopRequest> for RagGraphMultihopRequest {
    fn from(mcp_req: crate::mcp_server::types::RagGraphMultihopRequest) -> Self {
        Self {
            seed_nodes: mcp_req
                .seed_nodes
                .iter()
                .map(|&id| id.to_string())
                .collect(),
            query_text: Some(mcp_req.effective_query().to_string()),
            max_hops: mcp_req.max_hops.map(|h| h as u32),
            max_entities: mcp_req.max_entities.map(|e| e as u32),
            decay_factor: mcp_req.decay_factor,
        }
    }
}
