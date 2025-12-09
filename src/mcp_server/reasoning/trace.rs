//! Reasoning trace structures and utilities
//!
//! Provides deterministic, machine-readable traces of reasoning execution
//! for introspection and cognitive analysis.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// A trace entry representing a single stage in the reasoning pipeline
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReasoningTraceStage {
    /// Stage identifier (e.g., "backend_selection", "vector_search")
    pub stage: String,
    /// Whether the stage completed successfully
    pub ok: bool,
    /// Short, human-readable explanation of what happened
    pub detail: String,
    /// Timestamp when this stage completed (milliseconds since epoch)
    pub timestamp_ms: u128,
}

impl ReasoningTraceStage {
    /// Create a new reasoning trace stage
    pub fn new(stage: impl Into<String>, ok: bool, detail: impl Into<String>) -> Self {
        Self {
            stage: stage.into(),
            ok,
            detail: detail.into(),
            timestamp_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
        }
    }

    /// Create a successful stage entry
    pub fn success(stage: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::new(stage, true, detail)
    }

    /// Create a failed stage entry
    pub fn failure(stage: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::new(stage, false, detail)
    }
}

/// Complete reasoning trace for a single request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReasoningTrace {
    /// Ordered list of stages that were executed
    pub stages: Vec<ReasoningTraceStage>,
    /// Brief summary of the entire reasoning execution
    pub summary: String,
    /// Backend that was used (SQLiteGraph, Neo4j)
    pub backend: String,
    /// Breakdown of timing per stage in milliseconds
    pub timing_breakdown: HashMap<String, u128>,
    /// SHA256 hash of the original request parameters
    pub parameters_hash: String,
}

impl ReasoningTrace {
    /// Calculate SHA256 hash of request parameters
    pub fn calculate_parameters_hash(parameters: &serde_json::Value) -> String {
        let param_str = serde_json::to_string(parameters).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(param_str.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Generate a summary based on stages and overall success
    pub fn generate_summary(stages: &[ReasoningTraceStage]) -> String {
        if stages.is_empty() {
            return "No stages executed".to_string();
        }

        let failed_stages: Vec<&ReasoningTraceStage> = stages.iter().filter(|s| !s.ok).collect();

        if failed_stages.is_empty() {
            format!(
                "Successfully completed {} stages: {}",
                stages.len(),
                stages.iter().map(|s| s.stage.as_str()).collect::<Vec<_>>().join(", ")
            )
        } else {
            format!(
                "Failed at stage '{}' after completing {} stages. Error: {}",
                failed_stages[0].stage,
                stages.len(),
                failed_stages[0].detail
            )
        }
    }

    /// Extract timing breakdown from metadata and stages
    pub fn extract_timing_breakdown(
        metadata: &crate::mcp_server::reasoning::metadata::ReasoningMetadata,
    ) -> HashMap<String, u128> {
        let mut breakdown = HashMap::new();

        // Add known timing fields from metadata
        if let Some(vector_ms) = metadata.vector_search_ms {
            breakdown.insert("vector_search".to_string(), vector_ms);
        }
        if let Some(graph_ms) = metadata.graph_traversal_ms {
            breakdown.insert("graph_traversal".to_string(), graph_ms);
        }
        if let Some(fusion_ms) = metadata.fusion_ms {
            breakdown.insert("fusion".to_string(), fusion_ms);
        }

        breakdown
    }
}

/// Builder for constructing ReasoningTrace instances
#[derive(Debug, Clone)]
pub struct ReasoningTraceBuilder {
    _request_start_time_ms: u128,
    stages: Vec<ReasoningTraceStage>,
    backend: Option<String>,
    parameters_hash: Option<String>,
}

impl ReasoningTraceBuilder {
    /// Create a new trace builder with request start time
    pub fn new(parameters: &serde_json::Value) -> Self {
        Self {
            _request_start_time_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            stages: Vec::new(),
            backend: None,
            parameters_hash: Some(ReasoningTrace::calculate_parameters_hash(parameters)),
        }
    }

    /// Add a stage to the trace
    pub fn add_stage(&mut self, stage: ReasoningTraceStage) -> &mut Self {
        self.stages.push(stage);
        self
    }

    /// Add a successful stage
    pub fn add_success(
        &mut self,
        stage: impl Into<String>,
        detail: impl Into<String>,
    ) -> &mut Self {
        self.add_stage(ReasoningTraceStage::success(stage, detail))
    }

    /// Add a failed stage
    pub fn add_failure(
        &mut self,
        stage: impl Into<String>,
        detail: impl Into<String>,
    ) -> &mut Self {
        self.add_stage(ReasoningTraceStage::failure(stage, detail))
    }

    /// Set the backend used
    pub fn set_backend(&mut self, backend: impl Into<String>) -> &mut Self {
        self.backend = Some(backend.into());
        self
    }

    /// Finalize the trace builder into a complete ReasoningTrace
    pub fn finalize(
        self,
        metadata: &crate::mcp_server::reasoning::metadata::ReasoningMetadata,
    ) -> ReasoningTrace {
        let summary = ReasoningTrace::generate_summary(&self.stages);
        let backend = self.backend.unwrap_or_else(|| metadata.backend_used.clone());
        let timing_breakdown = ReasoningTrace::extract_timing_breakdown(metadata);

        ReasoningTrace {
            stages: self.stages,
            summary,
            backend,
            timing_breakdown,
            parameters_hash: self.parameters_hash.unwrap_or_default(),
        }
    }

    /// Get the current stages (for testing/debugging)
    pub fn get_stages(&self) -> &[ReasoningTraceStage] {
        &self.stages
    }

    /// Check if trace has any failed stages
    pub fn has_failures(&self) -> bool {
        self.stages.iter().any(|s| !s.ok)
    }

    /// Get the first failed stage, if any
    pub fn first_failure(&self) -> Option<&ReasoningTraceStage> {
        self.stages.iter().find(|s| !s.ok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_reasoning_trace_stage_creation() {
        let stage = ReasoningTraceStage::new("test_stage", true, "test completed");
        assert_eq!(stage.stage, "test_stage");
        assert!(stage.ok);
        assert_eq!(stage.detail, "test completed");
        assert!(stage.timestamp_ms > 0);
    }

    #[test]
    fn test_success_and_failure_helpers() {
        let success = ReasoningTraceStage::success("vector_search", "found 5 results");
        assert!(success.ok);
        assert_eq!(success.stage, "vector_search");

        let failure = ReasoningTraceStage::failure("validation", "invalid parameters");
        assert!(!failure.ok);
        assert_eq!(failure.stage, "validation");
    }

    #[test]
    fn test_parameters_hash_consistency() {
        let params1 = json!({"query": "test", "top_k": 5});
        let params2 = json!({"query": "test", "top_k": 5});
        let params3 = json!({"query": "different", "top_k": 5});

        let hash1 = ReasoningTrace::calculate_parameters_hash(&params1);
        let hash2 = ReasoningTrace::calculate_parameters_hash(&params2);
        let hash3 = ReasoningTrace::calculate_parameters_hash(&params3);

        assert_eq!(hash1, hash2, "Identical parameters should produce identical hashes");
        assert_ne!(hash1, hash3, "Different parameters should produce different hashes");
        assert_eq!(hash1.len(), 64, "SHA256 hash should be 64 characters");
    }

    #[test]
    fn test_summary_generation() {
        let all_success = vec![
            ReasoningTraceStage::success("parsing", "parsed successfully"),
            ReasoningTraceStage::success("vector_search", "found results"),
        ];
        let summary = ReasoningTrace::generate_summary(&all_success);
        assert!(summary.contains("Successfully"));
        assert!(summary.contains("parsing"));
        assert!(summary.contains("vector_search"));

        let with_failure = vec![
            ReasoningTraceStage::success("parsing", "parsed successfully"),
            ReasoningTraceStage::failure("vector_search", "index unavailable"),
        ];
        let summary = ReasoningTrace::generate_summary(&with_failure);
        assert!(summary.contains("Failed"));
        assert!(summary.contains("vector_search"));
        assert!(summary.contains("index unavailable"));
    }

    #[test]
    fn test_trace_builder() {
        let params = json!({"query": "test", "k": 3});
        let mut builder = ReasoningTraceBuilder::new(&params);

        builder
            .add_success("parsing", "request parsed")
            .add_success("backend_selection", "SQLiteGraph selected")
            .set_backend("SQLiteGraph");

        let metadata = crate::mcp_server::reasoning::metadata::ReasoningMetadata {
            request_id: "test".to_string(),
            backend_used: "SQLiteGraph".to_string(),
            start_time_ms: 1000,
            end_time_ms: 2000,
            vector_search_ms: Some(500),
            graph_traversal_ms: Some(300),
            fusion_ms: None,
            parameters: params,
            debug_flags: vec!["parsing:ok".to_string()],
        };

        let trace = builder.finalize(&metadata);

        assert_eq!(trace.stages.len(), 2);
        assert_eq!(trace.backend, "SQLiteGraph");
        assert!(trace.summary.contains("Successfully"));
        assert!(trace.parameters_hash.len() == 64);
        assert_eq!(trace.timing_breakdown.get("vector_search"), Some(&500));
        assert_eq!(trace.timing_breakdown.get("graph_traversal"), Some(&300));
    }

    #[test]
    fn test_trace_builder_failure_tracking() {
        let params = json!({"query": "test"});
        let mut builder = ReasoningTraceBuilder::new(&params);

        builder.add_success("parsing", "parsed").add_failure("validation", "invalid query");

        assert!(builder.has_failures());
        let failure = builder.first_failure().unwrap();
        assert_eq!(failure.stage, "validation");
        assert!(!failure.ok);
    }
}
