//! Code Drift Detection Module
//!
//! Detects various forms of code drift using existing Syncore infrastructure:
//! - Semantic drift: Similar functions with different implementations
//! - Architectural drift: Hotspots and structural violations
//! - Temporal aging: Stale files and changing patterns
//! - Pattern violations: Deviations from established patterns
//! - Cross-repo inconsistencies: Divergence between syncore/odincode
//! - Function divergence: Same name, different behavior

use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;
use crate::router::SynCoreState;

/// Detect semantic drift between similar functions
pub async fn detect_semantic_drift(state: std::sync::Arc<SynCoreState>, params: Value) -> Result<Value> {
    let query = params.get("query").and_then(|v| v.as_str()).unwrap_or("function");
    let threshold = params.get("similarity_threshold").and_then(|v| v.as_f64()).unwrap_or(0.8);

    // Use code_graph_fusion_query to find similar functions
    let fusion_result = state.mcp_delegate("code_graph_fusion_query", json!({
        "query": query,
        "mode_hint": "attention",
        "top_k": 20,
        "scope": "project"
    })).await?;

    // Analyze semantic drift by comparing body_snippet and signatures
    let mut drift_entities = Vec::new();
    if let Some(results) = fusion_result.get("results").and_then(|v| v.as_array()) {
        for result in results {
            if let (Some(name), Some(path), Some(signature)) = (
                result.get("name").and_then(|v| v.as_str()),
                result.get("path").and_then(|v| v.as_str()),
                result.get("signature").and_then(|v| v.as_str())
            ) {
                // Check for similar signatures with different implementations
                let similarity = calculate_signature_similarity(signature, &result);
                if similarity > threshold {
                    drift_entities.push(json!({
                        "name": name,
                        "path": path,
                        "signature": signature,
                        "similarity": similarity,
                        "drift_detected": similarity < 1.0
                    }));
                }
            }
        }
    }

    Ok(json!({
        "drift_type": "semantic",
        "query": query,
        "threshold": threshold,
        "duplicates": drift_entities,
        "drift_score": calculate_drift_score(&drift_entities),
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Detect architectural drift using hotspots and structural analysis
pub async fn detect_architectural_drift(state: std::sync::Arc<SynCoreState>, params: Value) -> Result<Value> {
    let fan_in_threshold = params.get("fan_in_threshold").and_then(|v| v.as_u64()).unwrap_or(10);
    let fan_out_threshold = params.get("fan_out_threshold").and_then(|v| v.as_u64()).unwrap_or(15);
    let loc_threshold = params.get("loc_threshold").and_then(|v| v.as_u64()).unwrap_or(500);

    // Use debug_suite to find hotspots
    let hotspots_result = state.mcp_delegate("project_hotspots", json!({
        "min_fan_in": fan_in_threshold,
        "min_fan_out": fan_out_threshold,
        "min_loc": loc_threshold
    })).await?;

    // Analyze architectural violations
    let mut violations = Vec::new();
    if let Some(hotspots) = hotspots_result.get("hotspots").and_then(|v| v.as_array()) {
        for hotspot in hotspots {
            let fan_in = hotspot.get("fan_in").and_then(|v| v.as_u64()).unwrap_or(0);
            let fan_out = hotspot.get("fan_out").and_then(|v| v.as_u64()).unwrap_or(0);

            if fan_in > fan_in_threshold || fan_out > fan_out_threshold {
                violations.push(json!({
                    "path": hotspot.get("path"),
                    "name": hotspot.get("name"),
                    "fan_in": fan_in,
                    "fan_out": fan_out,
                    "violation_type": if fan_in > fan_in_threshold { "high_coupling" } else { "high_responsibility" },
                    "severity": "warning"
                }));
            }
        }
    }

    Ok(json!({
        "drift_type": "architectural",
        "hotspots": hotspots_result,
        "violations": violations,
        "thresholds": {
            "fan_in": fan_in_threshold,
            "fan_out": fan_out_threshold,
            "loc": loc_threshold
        },
        "drift_score": violations.len() as f64 / 10.0,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Detect temporal aging using SQLiteGraph metadata
pub async fn detect_temporal_aging(state: std::sync::Arc<SynCoreState>, params: Value) -> Result<Value> {
    let max_age_days = params.get("max_age_days").and_then(|v| v.as_u64()).unwrap_or(30);
    let min_change_count = params.get("min_change_count").and_then(|v| v.as_u64()).unwrap_or(5);

    // Get recent entities with metadata from SQLiteGraph
    let entities_result = state.mcp_delegate("code_graph_fusion_query", json!({
        "query": "entity",
        "mode_hint": "simple",
        "top_k": 100,
        "scope": "project"
    })).await?;

    let cutoff_date = chrono::Utc::now() - chrono::Duration::days(max_age_days as i64);
    let mut stale_files = Vec::new();
    let mut aging_metrics = HashMap::new();

    if let Some(results) = entities_result.get("results").and_then(|v| v.as_array()) {
        for result in results {
            if let (Some(path), Some(last_modified), Some(change_count)) = (
                result.get("path").and_then(|v| v.as_str()),
                result.get("last_modified_at").and_then(|v| v.as_str()),
                result.get("change_count").and_then(|v| v.as_u64())
            ) {
                if let Ok(last_modified_date) = chrono::DateTime::parse_from_rfc3339(last_modified) {
                    if last_modified_date.naive_utc() < cutoff_date.naive_utc() && change_count >= min_change_count {
                        stale_files.push(json!({
                            "path": path,
                            "last_modified": last_modified,
                            "change_count": change_count,
                            "age_days": (chrono::Utc::now() - last_modified_date.with_timezone(&chrono::Utc)).num_days()
                        }));
                    }
                }

                // Collect aging metrics
                aging_metrics.insert("total_entities", results.len() as i64);
                aging_metrics.insert("stale_count", stale_files.len() as i64);
                aging_metrics.insert("max_age_days", max_age_days as i64);
            }
        }
    }

    Ok(json!({
        "drift_type": "temporal",
        "stale_files": stale_files,
        "aging_metrics": aging_metrics,
        "thresholds": {
            "max_age_days": max_age_days,
            "min_change_count": min_change_count
        },
        "drift_score": (stale_files.len() as f64) / 50.0,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Detect pattern violations in code structure and conventions
pub async fn detect_pattern_violations(state: std::sync::Arc<SynCoreState>, params: Value) -> Result<Value> {
    let patterns = params.get("patterns")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
        .unwrap_or_else(|| vec!["error_handling", "validation", "logging"]);

    let mut violations = Vec::new();
    let mut compliance_scores = HashMap::new();

    // Check each pattern type
    for pattern in &patterns {
        let pattern_score = analyze_pattern_compliance(state.clone(), pattern).await?;
        compliance_scores.insert(pattern.to_string(), pattern_score.clone());

        if let Some(violation_score) = pattern_score.get("violation_score").and_then(|v| v.as_f64()) {
            if violation_score > 0.3 {
                violations.push(json!({
                    "pattern": pattern,
                    "violation_score": violation_score,
                    "severity": if violation_score > 0.7 { "error" } else { "warning" },
                    "details": pattern_score.get("details")
                }));
            }
        }
    }

    Ok(json!({
        "drift_type": "pattern",
        "violations": violations,
        "pattern_compliance": compliance_scores,
        "patterns_checked": patterns,
        "drift_score": violations.len() as f64 / patterns.len() as f64,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Detect cross-repo divergence between syncore and odincode
pub async fn detect_cross_repo_divergence(state: std::sync::Arc<SynCoreState>, params: Value) -> Result<Value> {
    let repo_a = params.get("repo_a").and_then(|v| v.as_str()).unwrap_or("syncore");
    let repo_b = params.get("repo_b").and_then(|v| v.as_str()).unwrap_or("odincode");
    let similarity_threshold = params.get("similarity_threshold").and_then(|v| v.as_f64()).unwrap_or(0.9);

    // Query entities from both repos (assuming they share the same SQLiteGraph)
    let entities_a = state.mcp_delegate("code_graph_fusion_query", json!({
        "query": repo_a,
        "mode_hint": "simple",
        "top_k": 50,
        "scope": "project"
    })).await?;

    let entities_b = state.mcp_delegate("code_graph_fusion_query", json!({
        "query": repo_b,
        "mode_hint": "simple",
        "top_k": 50,
        "scope": "project"
    })).await?;

    let mut divergent_entities = Vec::new();

    // Compare entities between repos
    if let (Some(results_a), Some(results_b)) = (
        entities_a.get("results").and_then(|v| v.as_array()),
        entities_b.get("results").and_then(|v| v.as_array())
    ) {
        for entity_a in results_a {
            for entity_b in results_b {
                if let (Some(name_a), Some(name_b)) = (
                    entity_a.get("name").and_then(|v| v.as_str()),
                    entity_b.get("name").and_then(|v| v.as_str())
                ) {
                    if name_a == name_b {
                        let similarity = calculate_entity_similarity(entity_a, entity_b);
                        if similarity < similarity_threshold {
                            divergent_entities.push(json!({
                                "entity_name": name_a,
                                "repo_a_path": entity_a.get("path"),
                                "repo_b_path": entity_b.get("path"),
                                "similarity": similarity,
                                "divergence_type": "implementation_different"
                            }));
                        }
                    }
                }
            }
        }
    }

    let consistency_score = 1.0 - (divergent_entities.len() as f64 / 100.0);

    Ok(json!({
        "drift_type": "cross_repo",
        "repo_a": repo_a,
        "repo_b": repo_b,
        "divergent_entities": divergent_entities,
        "consistency_score": consistency_score,
        "similarity_threshold": similarity_threshold,
        "drift_score": divergent_entities.len() as f64 / 20.0,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Detect function divergence (same name, different behavior)
pub async fn detect_function_divergence(state: std::sync::Arc<SynCoreState>, params: Value) -> Result<Value> {
    let function_name = params.get("function_name").and_then(|v| v.as_str()).unwrap_or("process");
    let compare_signatures = params.get("compare_signatures").and_then(|v| v.as_bool()).unwrap_or(true);
    let compare_bodies = params.get("compare_bodies").and_then(|v| v.as_bool()).unwrap_or(true);

    // Search for functions with the same name
    let functions_result = state.mcp_delegate("code_graph_fusion_query", json!({
        "query": format!("function:{}", function_name),
        "mode_hint": "simple",
        "top_k": 20,
        "scope": "project"
    })).await?;

    let mut divergent_functions = Vec::new();
    let mut signature_matches = Vec::new();
    let mut body_similarities = Vec::new();

    if let Some(functions) = functions_result.get("results").and_then(|v| v.as_array()) {
        for (i, func_a) in functions.iter().enumerate() {
            for (j, func_b) in functions.iter().enumerate().skip(i + 1) {
                if let (Some(name_a), Some(name_b)) = (
                    func_a.get("name").and_then(|v| v.as_str()),
                    func_b.get("name").and_then(|v| v.as_str())
                ) {
                    if name_a == name_b && name_a.contains(function_name) {
                        let signature_match = if compare_signatures {
                            compare_function_signatures(func_a, func_b)
                        } else {
                            true
                        };

                        let body_sim = if compare_bodies {
                            calculate_body_similarity(func_a, func_b)
                        } else {
                            1.0
                        };

                        if !signature_match || body_sim < 0.8 {
                            divergent_functions.push(json!({
                                "function_name": name_a,
                                "path_a": func_a.get("path"),
                                "path_b": func_b.get("path"),
                                "signature_match": signature_match,
                                "body_similarity": body_sim,
                                "divergence_detected": true
                            }));
                        }

                        signature_matches.push(signature_match);
                        body_similarities.push(body_sim);
                    }
                }
            }
        }
    }

    Ok(json!({
        "drift_type": "function_divergence",
        "function_name": function_name,
        "divergent_functions": divergent_functions,
        "signature_matches": signature_matches,
        "body_similarity": body_similarities,
        "comparison_settings": {
            "compare_signatures": compare_signatures,
            "compare_bodies": compare_bodies
        },
        "drift_score": divergent_functions.len() as f64 / 10.0,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

// Helper functions

fn calculate_signature_similarity(signature: &str, result: &Value) -> f64 {
    // Simple similarity calculation based on signature components
    let result_signature = result.get("signature").and_then(|v| v.as_str()).unwrap_or("");

    if signature == result_signature {
        1.0
    } else {
        // Basic similarity based on parameter count and return type
        let sig_params = signature.matches(':').count();
        let res_params = result_signature.matches(':').count();

        1.0 - ((sig_params as f64 - res_params as f64).abs() / sig_params.max(1) as f64)
    }
}

fn calculate_drift_score(entities: &[Value]) -> f64 {
    if entities.is_empty() {
        return 0.0;
    }

    let drift_count = entities.iter()
        .filter(|e| e.get("drift_detected").and_then(|v| v.as_bool()).unwrap_or(false))
        .count();

    drift_count as f64 / entities.len() as f64
}

async fn analyze_pattern_compliance(state: std::sync::Arc<SynCoreState>, pattern: &str) -> Result<Value> {
    // Use existing tools to analyze pattern compliance
    let query = format!("pattern:{}", pattern);
    let result = state.mcp_delegate("code_graph_fusion_query", json!({
        "query": query,
        "mode_hint": "simple",
        "top_k": 10
    })).await?;

    let violation_score = if result.get("results")
        .and_then(|v| v.as_array())
        .map(|arr| arr.len())
        .unwrap_or(0) > 0 {
        0.5 // Simple heuristic
    } else {
        0.1
    };

    Ok(json!({
        "pattern": pattern,
        "violation_score": violation_score,
        "details": result
    }))
}

fn calculate_entity_similarity(entity_a: &Value, entity_b: &Value) -> f64 {
    // Calculate similarity based on multiple factors
    let name_match = if entity_a.get("name") == entity_b.get("name") { 0.4 } else { 0.0 };
    let type_match = if entity_a.get("label") == entity_b.get("label") { 0.3 } else { 0.0 };

    let signature_a = entity_a.get("signature").and_then(|v| v.as_str()).unwrap_or("");
    let signature_b = entity_b.get("signature").and_then(|v| v.as_str()).unwrap_or("");
    let sig_similarity = calculate_signature_similarity(signature_a, entity_b);

    name_match + type_match + (sig_similarity * 0.3)
}

fn compare_function_signatures(func_a: &Value, func_b: &Value) -> bool {
    func_a.get("signature") == func_b.get("signature")
}

fn calculate_body_similarity(func_a: &Value, func_b: &Value) -> f64 {
    let body_a = func_a.get("body_snippet").and_then(|v| v.as_str()).unwrap_or("");
    let body_b = func_b.get("body_snippet").and_then(|v| v.as_str()).unwrap_or("");

    if body_a == body_b {
        return 1.0;
    }

    // Simple similarity based on common lines
    let lines_a: std::collections::HashSet<_> = body_a.lines().collect();
    let lines_b: std::collections::HashSet<_> = body_b.lines().collect();

    let common = lines_a.intersection(&lines_b).count();
    let total = lines_a.union(&lines_b).count();

    if total > 0 {
        common as f64 / total as f64
    } else {
        0.0
    }
}