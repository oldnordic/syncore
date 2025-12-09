//! Code Drift Detection MCP Tools
//!
//! Provides MCP tool interfaces for code drift detection:
//! - drift.semantic: Detect semantic drift between similar functions
//! - drift.architecture: Detect architectural drift using hotspots
//! - drift.aging: Detect temporal aging of files
//! - drift.patterns: Detect pattern violations
//! - drift.crossrepo: Detect cross-repo inconsistencies

use anyhow::Result;
use serde_json::{json, Value};
use crate::code_drift::*;
use crate::router::SynCoreState;
use crate::mcp_server::types::CodeDriftSuiteRequest;
use rmcp::tool;

/// Arguments for code drift suite commands
#[derive(Debug, Clone)]
pub struct CodeDriftSuiteArgs {
    pub command: String,
    pub query: Option<String>,
    pub similarity_threshold: Option<f64>,
    pub fan_in_threshold: Option<u64>,
    pub fan_out_threshold: Option<u64>,
    pub loc_threshold: Option<u64>,
    pub max_age_days: Option<u64>,
    pub min_change_count: Option<u64>,
    pub pattern_types: Option<Vec<String>>,
    pub severity: Option<String>,
    pub baseline_repo: Option<String>,
    pub comparison_repo: Option<String>,
    pub function_name: Option<String>,
    pub compare_signatures: Option<bool>,
    pub compare_bodies: Option<bool>,
    pub include_semantic: Option<bool>,
    pub include_architectural: Option<bool>,
    pub include_temporal: Option<bool>,
    pub include_patterns: Option<bool>,
    pub include_crossrepo: Option<bool>,
    /// Maximum items to return (for pagination)
    pub max_items: Option<usize>,
    /// Cursor for pagination (0-based index)
    pub cursor: Option<String>,
}

impl From<CodeDriftSuiteRequest> for CodeDriftSuiteArgs {
    fn from(request: CodeDriftSuiteRequest) -> Self {
        Self {
            command: request.command,
            query: request.query,
            similarity_threshold: request.similarity_threshold,
            fan_in_threshold: request.fan_in_threshold,
            fan_out_threshold: request.fan_out_threshold,
            loc_threshold: request.loc_threshold,
            max_age_days: request.max_age_days,
            min_change_count: request.min_change_count,
            pattern_types: request.pattern_types,
            severity: request.severity,
            baseline_repo: request.baseline_repo,
            comparison_repo: request.comparison_repo,
            function_name: request.function_name,
            compare_signatures: request.compare_signatures,
            compare_bodies: request.compare_bodies,
            include_semantic: request.include_semantic,
            include_architectural: request.include_architectural,
            include_temporal: request.include_temporal,
            include_patterns: request.include_patterns,
            include_crossrepo: request.include_crossrepo,
            max_items: request.max_items,
            cursor: request.cursor,
        }
    }
}

impl Default for CodeDriftSuiteArgs {
    fn default() -> Self {
        Self {
            command: "help".to_string(),
            query: None,
            similarity_threshold: None,
            fan_in_threshold: None,
            fan_out_threshold: None,
            loc_threshold: None,
            max_age_days: None,
            min_change_count: None,
            pattern_types: None,
            severity: None,
            baseline_repo: None,
            comparison_repo: None,
            function_name: None,
            compare_signatures: None,
            compare_bodies: None,
            include_semantic: None,
            include_architectural: None,
            include_temporal: None,
            include_patterns: None,
            include_crossrepo: None,
            max_items: None,
            cursor: None,
        }
    }
}

/// Helper function to apply pagination to a JSON array
fn apply_pagination_to_array(
    items: Vec<Value>,
    max_items: Option<usize>,
    cursor: Option<String>,
) -> serde_json::Value {
    // Parse cursor (default to 0)
    let start_idx = cursor
        .and_then(|c| c.parse::<usize>().ok())
        .unwrap_or(0);

    // Determine limit (default to 50)
    let limit = max_items.unwrap_or(50);

    // Validate bounds
    if start_idx >= items.len() {
        // Cursor beyond end - return empty result
        return json!({
            "items": [],
            "next_cursor": serde_json::Value::Null,
            "total_items": items.len(),
            "has_more": false
        });
    }

    // Slice the array
    let end_idx = std::cmp::min(start_idx + limit, items.len());
    let page_items: Vec<Value> = items[start_idx..end_idx].to_vec();

    // Calculate next cursor and has_more flag
    let (next_cursor, has_more) = if end_idx < items.len() {
        (Some(end_idx.to_string()), true)
    } else {
        (None, false)
    };

    json!({
        "items": page_items,
        "next_cursor": next_cursor.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null),
        "total_items": items.len(),
        "has_more": has_more,
        "page_info": {
            "start_index": start_idx,
            "end_index": end_idx,
            "items_returned": page_items.len()
        }
    })
}

/// Code Drift Suite dispatcher
pub struct CodeDriftSuite {
    state: std::sync::Arc<SynCoreState>,
}

impl CodeDriftSuite {
    pub fn new(state: SynCoreState) -> Self {
        Self { state: std::sync::Arc::new(state) }
    }

    /// Execute a code drift suite command
    pub async fn execute(&self, args: CodeDriftSuiteArgs) -> Result<Value> {
        match args.command.as_str() {
            "semantic" => {
                handle_drift_semantic(
                    self.state.clone(),
                    args.query,
                    args.similarity_threshold,
                    args.max_items,
                    args.cursor,
                ).await
            },
            "architecture" => {
                handle_drift_architecture(
                    self.state.clone(),
                    args.fan_in_threshold,
                    args.fan_out_threshold,
                    args.loc_threshold,
                ).await
            },
            "aging" => {
                handle_drift_aging(
                    self.state.clone(),
                    args.max_age_days,
                    args.min_change_count,
                ).await
            },
            "patterns" => {
                handle_drift_patterns(
                    self.state.clone(),
                    args.pattern_types,
                    args.severity,
                ).await
            },
            "crossrepo" => {
                handle_drift_crossrepo(
                    self.state.clone(),
                    args.baseline_repo,
                    args.comparison_repo,
                    args.similarity_threshold,
                ).await
            },
            "comprehensive" => {
                handle_drift_comprehensive(
                    self.state.clone(),
                    args.include_semantic,
                    args.include_architectural,
                    args.include_temporal,
                    args.include_patterns,
                    args.include_crossrepo,
                ).await
            },
            "functions" => {
                if let Some(function_name) = args.function_name {
                    handle_drift_functions(
                        self.state.clone(),
                        function_name,
                        args.compare_signatures,
                        args.compare_bodies,
                    ).await
                } else {
                    Ok(json!({
                        "tool": "drift.functions",
                        "success": false,
                        "error": "function_name is required for functions command"
                    }))
                }
            },
            "help" => Ok(json!({
                "tool": "drift.help",
                "success": true,
                "commands": [
                    "semantic - Detect semantic drift between similar functions",
                    "architecture - Detect architectural drift using hotspots analysis",
                    "aging - Detect temporal aging of files and entities",
                    "patterns - Detect violations of established code patterns",
                    "crossrepo - Detect inconsistencies between repositories",
                    "comprehensive - Run comprehensive drift analysis across all dimensions",
                    "functions - Detect function divergence (same name, different behavior)",
                    "help - Show this help message"
                ]
            })),
            _ => Ok(json!({
                "tool": "drift.unknown",
                "success": false,
                "error": format!("Unknown command: {}", args.command)
            }))
        }
    }
}

/// Handle semantic drift detection
pub async fn handle_drift_semantic(
    state: std::sync::Arc<SynCoreState>,
    query: Option<String>,
    similarity_threshold: Option<f64>,
    max_items: Option<usize>,
    cursor: Option<String>,
) -> Result<Value> {
    let params = json!({
        "query": query.unwrap_or_else(|| "function".to_string()),
        "similarity_threshold": similarity_threshold.unwrap_or(0.8)
    });

    let drift_result = detect_semantic_drift(state, params).await?;

    // Check if pagination is requested
    let is_paginated = max_items.is_some() || cursor.is_some();

    if is_paginated {
        // Extract items from drift result
        if let Some(duplicates) = drift_result.get("duplicates").and_then(|v| v.as_array()) {
            let items = duplicates.clone();
            let paged_result = apply_pagination_to_array(items, max_items, cursor);

            Ok(json!({
                "tool": "drift.semantic",
                "success": true,
                "paged_result": paged_result
            }))
        } else {
            // No duplicates array found - return empty paged result
            Ok(json!({
                "tool": "drift.semantic",
                "success": true,
                "paged_result": apply_pagination_to_array(vec![], max_items, cursor)
            }))
        }
    } else {
        // Legacy mode - return original response
        Ok(json!({
            "tool": "drift.semantic",
            "success": true,
            "drift_report": drift_result
        }))
    }
}

/// Handle architectural drift detection
#[tool(description = "Detect architectural drift using hotspots analysis")]
pub async fn handle_drift_architecture(
    state: std::sync::Arc<SynCoreState>,
     fan_in_threshold: Option<u64>,
     fan_out_threshold: Option<u64>,
     loc_threshold: Option<u64>,
) -> Result<Value> {
    let params = json!({
        "fan_in_threshold": fan_in_threshold.unwrap_or(10),
        "fan_out_threshold": fan_out_threshold.unwrap_or(15),
        "loc_threshold": loc_threshold.unwrap_or(500)
    });

    let drift_result = detect_architectural_drift(state, params).await?;

    Ok(json!({
        "tool": "drift.architecture",
        "success": true,
        "drift_report": drift_result
    }))
}

/// Handle temporal aging detection
#[tool(description = "Detect temporal aging of files and entities")]
pub async fn handle_drift_aging(
    state: std::sync::Arc<SynCoreState>,
     max_age_days: Option<u64>,
     min_change_count: Option<u64>,
) -> Result<Value> {
    let params = json!({
        "max_age_days": max_age_days.unwrap_or(30),
        "min_change_count": min_change_count.unwrap_or(5)
    });

    let drift_result = detect_temporal_aging(state, params).await?;

    Ok(json!({
        "tool": "drift.aging",
        "success": true,
        "drift_report": drift_result
    }))
}

/// Handle pattern violation detection
#[tool(description = "Detect violations of established code patterns")]
pub async fn handle_drift_patterns(
    state: std::sync::Arc<SynCoreState>,
     pattern_types: Option<Vec<String>>,
     severity: Option<String>,
) -> Result<Value> {
    let patterns = pattern_types.unwrap_or_else(|| vec![
        "error_handling".to_string(),
        "validation".to_string(),
        "logging".to_string()
    ]);

    let params = json!({
        "patterns": patterns,
        "severity": severity.unwrap_or_else(|| "warning".to_string())
    });

    let drift_result = detect_pattern_violations(state, params).await?;

    Ok(json!({
        "tool": "drift.patterns",
        "success": true,
        "drift_report": drift_result
    }))
}

/// Handle cross-repo divergence detection
#[tool(description = "Detect inconsistencies between repositories")]
pub async fn handle_drift_crossrepo(
    state: std::sync::Arc<SynCoreState>,
     baseline_repo: Option<String>,
     comparison_repo: Option<String>,
     similarity_threshold: Option<f64>,
) -> Result<Value> {
    let params = json!({
        "repo_a": baseline_repo.unwrap_or_else(|| "syncore".to_string()),
        "repo_b": comparison_repo.unwrap_or_else(|| "odincode".to_string()),
        "similarity_threshold": similarity_threshold.unwrap_or(0.9)
    });

    let drift_result = detect_cross_repo_divergence(state, params).await?;

    Ok(json!({
        "tool": "drift.crossrepo",
        "success": true,
        "drift_report": drift_result
    }))
}

/// Handle comprehensive drift analysis
#[tool(description = "Run comprehensive drift analysis across all dimensions")]
pub async fn handle_drift_comprehensive(
    state: std::sync::Arc<SynCoreState>,
     include_semantic: Option<bool>,
     include_architectural: Option<bool>,
     include_temporal: Option<bool>,
     include_patterns: Option<bool>,
     include_crossrepo: Option<bool>,
) -> Result<Value> {
    let mut drift_summary = json!({});
    let mut total_drift_score = 0.0;
    let mut analysis_count = 0;

    // Semantic drift
    if include_semantic.unwrap_or(true) {
        let semantic_result = detect_semantic_drift(state.clone(), json!({})).await?;
        drift_summary["semantic"] = semantic_result.clone();
        total_drift_score += semantic_result.get("drift_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        analysis_count += 1;
    }

    // Architectural drift
    if include_architectural.unwrap_or(true) {
        let arch_result = detect_architectural_drift(state.clone(), json!({})).await?;
        drift_summary["architectural"] = arch_result.clone();
        total_drift_score += arch_result.get("drift_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        analysis_count += 1;
    }

    // Temporal aging
    if include_temporal.unwrap_or(true) {
        let temp_result = detect_temporal_aging(state.clone(), json!({})).await?;
        drift_summary["temporal"] = temp_result.clone();
        total_drift_score += temp_result.get("drift_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        analysis_count += 1;
    }

    // Pattern violations
    if include_patterns.unwrap_or(true) {
        let pattern_result = detect_pattern_violations(state.clone(), json!({})).await?;
        drift_summary["patterns"] = pattern_result.clone();
        total_drift_score += pattern_result.get("drift_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        analysis_count += 1;
    }

    // Cross-repo divergence
    if include_crossrepo.unwrap_or(false) {
        let cross_result = detect_cross_repo_divergence(state.clone(), json!({})).await?;
        drift_summary["cross_repo"] = cross_result.clone();
        total_drift_score += cross_result.get("drift_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        analysis_count += 1;
    }

    let overall_drift_score = if analysis_count > 0 {
        total_drift_score / analysis_count as f64
    } else {
        0.0
    };

    let recommendations = generate_drift_recommendations(&drift_summary, overall_drift_score);

    Ok(json!({
        "tool": "drift.comprehensive",
        "success": true,
        "drift_summary": drift_summary,
        "overall_drift_score": overall_drift_score,
        "analysis_count": analysis_count,
        "recommendations": recommendations,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Handle function divergence detection
#[tool(description = "Detect function divergence (same name, different behavior)")]
pub async fn handle_drift_functions(
    state: std::sync::Arc<SynCoreState>,
     function_name: String,
     compare_signatures: Option<bool>,
     compare_bodies: Option<bool>,
) -> Result<Value> {
    let params = json!({
        "function_name": function_name,
        "compare_signatures": compare_signatures.unwrap_or(true),
        "compare_bodies": compare_bodies.unwrap_or(true)
    });

    let drift_result = detect_function_divergence(state, params).await?;

    Ok(json!({
        "tool": "drift.functions",
        "success": true,
        "drift_report": drift_result
    }))
}

/// Generate recommendations based on drift analysis
fn generate_drift_recommendations(drift_summary: &Value, overall_score: f64) -> Vec<Value> {
    let mut recommendations = Vec::new();

    if overall_score > 0.7 {
        recommendations.push(json!({
            "priority": "high",
            "category": "overall",
            "message": "High drift detected across multiple dimensions. Consider refactoring.",
            "action": "Schedule comprehensive code review and refactoring session."
        }));
    }

    // Check semantic drift
    if let Some(semantic) = drift_summary.get("semantic") {
        if let Some(drift_score) = semantic.get("drift_score").and_then(|v| v.as_f64()) {
            if drift_score > 0.5 {
                recommendations.push(json!({
                    "priority": "medium",
                    "category": "semantic",
                    "message": "Semantic drift detected between similar functions.",
                    "action": "Standardize function signatures and implementations."
                }));
            }
        }
    }

    // Check architectural drift
    if let Some(architectural) = drift_summary.get("architectural") {
        if let Some(violations) = architectural.get("violations").and_then(|v| v.as_array()) {
            if !violations.is_empty() {
                recommendations.push(json!({
                    "priority": "high",
                    "category": "architectural",
                    "message": format!("{} architectural violations detected.", violations.len()),
                    "action": "Reduce coupling and apply single responsibility principle."
                }));
            }
        }
    }

    // Check temporal aging
    if let Some(temporal) = drift_summary.get("temporal") {
        if let Some(stale_files) = temporal.get("stale_files").and_then(|v| v.as_array()) {
            if !stale_files.is_empty() {
                recommendations.push(json!({
                    "priority": "low",
                    "category": "temporal",
                    "message": format!("{} stale files detected.", stale_files.len()),
                    "action": "Review and update stale code or consider deprecation."
                }));
            }
        }
    }

    // Check pattern violations
    if let Some(patterns) = drift_summary.get("patterns") {
        if let Some(violations) = patterns.get("violations").and_then(|v| v.as_array()) {
            if !violations.is_empty() {
                recommendations.push(json!({
                    "priority": "medium",
                    "category": "patterns",
                    "message": format!("{} pattern violations detected.", violations.len()),
                    "action": "Enforce coding standards and pattern consistency."
                }));
            }
        }
    }

    if recommendations.is_empty() {
        recommendations.push(json!({
            "priority": "info",
            "category": "overall",
            "message": "No significant drift detected.",
            "action": "Continue monitoring code quality metrics."
        }));
    }

    recommendations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_drift_recommendations() {
        let drift_summary = json!({
            "semantic": {"drift_score": 0.6},
            "architectural": {"violations": [1, 2, 3]},
            "temporal": {"stale_files": []},
            "patterns": {"violations": []}
        });

        let recommendations = generate_drift_recommendations(&drift_summary, 0.5);
        assert!(!recommendations.is_empty());

        // Should have semantic, architectural, and overall recommendations
        let categories: Vec<_> = recommendations.iter()
            .filter_map(|r| r.get("category").and_then(|v| v.as_str()))
            .collect();

        assert!(categories.contains(&"semantic"));
        assert!(categories.contains(&"architectural"));
    }
}