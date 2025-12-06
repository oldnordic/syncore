//! Cross-tool reasoning consistency contracts
//!
//! Provides deterministic validation of consistency across reasoning tools
//! (raggraph_query, raggraph_multihop, code_graph_fusion_query).
//!
//! This module ensures that for comparable inputs, all tools produce
//! mutually consistent metadata, traces, evaluations, and reflections
//! according to explicit, deterministic consistency rules.

use serde::{Deserialize, Serialize};

/// Snapshot of reasoning data from a single tool
///
/// Extracts key consistency-relevant fields from UnifiedReasoningResponse
/// for cross-tool comparison and validation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolReasoningSnapshot {
    /// Tool name identifier
    pub tool_name: String,
    /// Backend used according to metadata
    pub metadata_backend: String,
    /// Backend used according to trace (if available)
    pub trace_backend: Option<String>,
    /// Evaluation score (0-100)
    pub evaluation_score: u8,
    /// Evaluation confidence (0.0-1.0)
    pub evaluation_confidence: f32,
    /// Reflection category (if available)
    pub reflection_category: Option<String>,
}

/// Consistency violation detected between tool snapshots
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConsistencyViolation {
    /// Violation code identifier
    pub code: String,
    /// Human-readable explanation
    pub detail: String,
}

/// Consistency validation report
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConsistencyReport {
    /// Overall consistency status
    pub is_consistent: bool,
    /// List of detected violations
    pub violations: Vec<ConsistencyViolation>,
}

/// Build tool snapshot from unified reasoning response
///
/// Extracts consistency-relevant fields from a UnifiedReasoningResponse
/// without modifying the original response.
///
/// # Arguments
/// * `tool_name` - Name of the reasoning tool
/// * `response` - UnifiedReasoningResponse to extract from
///
/// # Returns
/// ToolReasoningSnapshot with extracted consistency data
pub fn build_tool_snapshot_from_unified_response(
    tool_name: &str,
    response: &crate::mcp_server::reasoning::UnifiedReasoningResponse,
) -> ToolReasoningSnapshot {
    let metadata_backend = response.metadata
        .as_ref()
        .map(|m| m.backend_used.clone())
        .unwrap_or_else(|| "unknown".to_string());

    let trace_backend = response.trace
        .as_ref()
        .map(|t| t.backend.clone());

    let evaluation_score = response.evaluation
        .as_ref()
        .map(|e| e.score)
        .unwrap_or(0);

    let evaluation_confidence = response.evaluation
        .as_ref()
        .map(|e| e.confidence)
        .unwrap_or(0.0);

    let reflection_category = response.reflection
        .as_ref()
        .map(|r| r.category.clone());

    ToolReasoningSnapshot {
        tool_name: tool_name.to_string(),
        metadata_backend,
        trace_backend,
        evaluation_score,
        evaluation_confidence,
        reflection_category,
    }
}

/// Validate consistency across tool snapshots
///
/// Implements deterministic consistency rules:
/// - Backend consistency across tools
/// - Score divergence limits
/// - Reflection category alignment
/// - Trace-backend alignment with metadata
///
/// # Arguments
/// * `snapshots` - Collection of tool snapshots to validate
///
/// # Returns
/// ConsistencyReport with detected violations
pub fn validate_snapshots_consistency(
    snapshots: &[ToolReasoningSnapshot],
) -> ConsistencyReport {
    let mut violations = Vec::new();

    if snapshots.is_empty() {
        return ConsistencyReport {
            is_consistent: true,
            violations,
        };
    }

    // Rule 1: Backend consistency across metadata
    let metadata_backends: Vec<String> = snapshots.iter()
        .map(|s| s.metadata_backend.clone())
        .collect();

    let unique_metadata_backends: std::collections::HashSet<&String> = metadata_backends.iter().collect();
    if unique_metadata_backends.len() > 1 {
        violations.push(ConsistencyViolation {
            code: "backend_mismatch".to_string(),
            detail: format!(
                "Metadata backends are inconsistent across tools: {:?}",
                metadata_backends
            ),
        });
    }

    // Rule 2: Trace backend alignment with metadata backend
    for snapshot in snapshots {
        if let Some(ref trace_backend) = snapshot.trace_backend {
            if trace_backend != &snapshot.metadata_backend {
                violations.push(ConsistencyViolation {
                    code: "trace_backend_mismatch".to_string(),
                    detail: format!(
                        "Tool '{}' has trace backend '{}' that differs from metadata backend '{}'",
                        snapshot.tool_name, trace_backend, snapshot.metadata_backend
                    ),
                });
            }
        }
    }

    // Rule 3: Score divergence limits
    let scores: Vec<u8> = snapshots.iter().map(|s| s.evaluation_score).collect();
    if scores.len() > 1 {
        let max_score = scores.iter().max().copied().unwrap_or(0);
        let min_score = scores.iter().min().copied().unwrap_or(100);
        if max_score > min_score + 20 {
            violations.push(ConsistencyViolation {
                code: "score_divergence".to_string(),
                detail: format!(
                    "Evaluation scores diverge beyond allowed range: min={}, max={}, difference={}",
                    min_score, max_score, max_score.saturating_sub(min_score)
                ),
            });
        }
    }

    // Rule 4: Reflection category conflicts
    let categories: Vec<Option<&String>> = snapshots.iter()
        .map(|s| s.reflection_category.as_ref())
        .collect();

    // Check for extreme category conflicts (stable vs anomalous)
    let has_stable = categories.iter().any(|c| matches!(c, Some(cat) if **cat == "stable"));
    let has_anomalous = categories.iter().any(|c| matches!(c, Some(cat) if **cat == "anomalous"));

    if has_stable && has_anomalous {
        let category_strings: Vec<String> = categories.iter()
            .filter_map(|c| c.map(|s| s.clone()))
            .collect();

        violations.push(ConsistencyViolation {
            code: "category_conflict".to_string(),
            detail: format!(
                "Conflicting reflection categories detected: stable and anomalous tools {:?}",
                category_strings
            ),
        });
    }

    // Rule 5: Reflection category alignment with evaluation scores
    for snapshot in snapshots {
        if let Some(ref category) = snapshot.reflection_category {
            let score = snapshot.evaluation_score;

            // High score should have stable reflection
            if score >= 90 && category.as_str() != "stable" {
                violations.push(ConsistencyViolation {
                    code: "reflection_score_mismatch".to_string(),
                    detail: format!(
                        "Tool '{}' has high evaluation score ({}) but reflection category '{}'",
                        snapshot.tool_name, score, category
                    ),
                });
            }

            // Low score should not have stable reflection
            if score < 70 && category.as_str() == "stable" {
                violations.push(ConsistencyViolation {
                    code: "reflection_score_mismatch".to_string(),
                    detail: format!(
                        "Tool '{}' has low evaluation score ({}) but stable reflection category",
                        snapshot.tool_name, score
                    ),
                });
            }
        }
    }

    let is_consistent = violations.is_empty();

    ConsistencyReport {
        is_consistent,
        violations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_snapshot(tool_name: &str, backend: &str, score: u8) -> ToolReasoningSnapshot {
        ToolReasoningSnapshot {
            tool_name: tool_name.to_string(),
            metadata_backend: backend.to_string(),
            trace_backend: Some(backend.to_string()),
            evaluation_score: score,
            evaluation_confidence: 0.8,
            reflection_category: if score >= 90 { Some("stable".to_string()) } else if score >= 70 { Some("degraded".to_string()) } else { Some("anomalous".to_string()) },
        }
    }

    #[test]
    fn test_validate_consistent_snapshots() {
        let snapshots = vec![
            create_test_snapshot("tool1", "SQLiteGraph", 85),
            create_test_snapshot("tool2", "SQLiteGraph", 88),
            create_test_snapshot("tool3", "SQLiteGraph", 90),
        ];

        let report = validate_snapshots_consistency(&snapshots);

        assert!(report.is_consistent);
        assert!(report.violations.is_empty());
    }

    #[test]
    fn test_validate_backend_mismatch() {
        let snapshots = vec![
            create_test_snapshot("tool1", "SQLiteGraph", 85),
            create_test_snapshot("tool2", "Neo4j", 88),
            create_test_snapshot("tool3", "SQLiteGraph", 90),
        ];

        let report = validate_snapshots_consistency(&snapshots);

        assert!(!report.is_consistent);
        assert!(report.violations.iter().any(|v| v.code == "backend_mismatch"));
    }

    #[test]
    fn test_validate_score_divergence() {
        let snapshots = vec![
            create_test_snapshot("tool1", "SQLiteGraph", 60),
            create_test_snapshot("tool2", "SQLiteGraph", 85),
        ];

        let report = validate_snapshots_consistency(&snapshots);

        assert!(!report.is_consistent);
        assert!(report.violations.iter().any(|v| v.code == "score_divergence"));
    }

    #[test]
    fn test_empty_snapshots_are_consistent() {
        let snapshots: Vec<ToolReasoningSnapshot> = vec![];
        let report = validate_snapshots_consistency(&snapshots);

        assert!(report.is_consistent);
        assert!(report.violations.is_empty());
    }
}