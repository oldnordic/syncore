//! Self-Consistency Checker Module
//!
//! Advanced cognitive constraint system that evaluates reasoning plans for:
//! - Repeated failed sequences
//! - Conflicting patterns
//! - Graph inconsistencies
//! - Namespace mismatches
//! - Suspicious tool ordering
//! - Missing required steps
//! - Potential loops

use super::context_bundle::ContextBundle;
use super::continuity_engine::ReasoningContinuity;
use super::intent_classifier::QueryIntent;
use super::pattern_engine::ReasoningPattern;
use super::reasoning_ledger::ReasoningEpisode;

// Re-export types for convenience
pub use super::self_consistency_types::{
    SelfConsistencyIssue, SelfConsistencyIssueKind, SelfConsistencyResult, SelfConsistencySeverity,
    SuggestedPlan,
};

/// Evaluate self-consistency of a reasoning plan
pub fn evaluate_self_consistency(
    _query: &str,
    intent: &QueryIntent,
    selected_mode: &str,
    planned_tools: &[String],
    context_bundle: &ContextBundle,
    continuity: &ReasoningContinuity,
    recommended_patterns: &[ReasoningPattern],
    ledger_episodes: &[ReasoningEpisode],
) -> SelfConsistencyResult {
    let mut issues = Vec::new();

    // Check 1: Repeated failed sequence
    detect_repeated_failed_sequence(planned_tools, ledger_episodes, &mut issues);

    // Check 2: Conflicting pattern
    detect_conflicting_pattern(
        intent,
        selected_mode,
        planned_tools,
        recommended_patterns,
        &mut issues,
    );

    // Check 3: Graph inconsistency
    detect_graph_inconsistency(planned_tools, context_bundle, &mut issues);

    // Check 4: Namespace mismatch
    detect_namespace_mismatch(context_bundle, &mut issues);

    // Check 5: Tool order suspicious
    detect_tool_order_suspicious(
        intent,
        selected_mode,
        planned_tools,
        recommended_patterns,
        &mut issues,
    );

    // Check 6: Missing required step
    detect_missing_required_step(planned_tools, recommended_patterns, &mut issues);

    // Check 7: Potential loop
    detect_potential_loop(planned_tools, continuity, &mut issues);

    // Compute score based on issues
    let score = compute_score(&issues);

    // Generate suggested plan if needed
    let suggested_plan = generate_suggested_plan(recommended_patterns, &issues);

    SelfConsistencyResult {
        score,
        issues,
        suggested_plan,
    }
}

/// Detect if planned tools match failed sequences in history
fn detect_repeated_failed_sequence(
    planned_tools: &[String],
    episodes: &[ReasoningEpisode],
    issues: &mut Vec<SelfConsistencyIssue>,
) {
    let mut failure_count = 0;

    for episode in episodes {
        if episode.outcome == "failure" && tools_match_sequence(planned_tools, &episode.tool_calls)
        {
            failure_count += 1;
        }
    }

    if failure_count >= 2 {
        issues.push(SelfConsistencyIssue {
            kind: SelfConsistencyIssueKind::RepeatedFailedSequence,
            description: format!(
                "This tool sequence has failed {} times in history",
                failure_count
            ),
            severity: if failure_count >= 3 {
                SelfConsistencySeverity::Error
            } else {
                SelfConsistencySeverity::Warning
            },
        });
    }
}

/// Detect conflicting patterns with better success rates
fn detect_conflicting_pattern(
    intent: &QueryIntent,
    selected_mode: &str,
    planned_tools: &[String],
    recommended_patterns: &[ReasoningPattern],
    issues: &mut Vec<SelfConsistencyIssue>,
) {
    for pattern in recommended_patterns {
        // Check if pattern matches intent and mode
        if intent_matches(&pattern.intent_type, intent) && pattern.selected_mode == selected_mode {
            // Check if pattern suggests different tools with higher success
            if !tools_match_sequence(planned_tools, &pattern.tool_sequence)
                && pattern.success_rate > 0.7
            {
                issues.push(SelfConsistencyIssue {
                    kind: SelfConsistencyIssueKind::ConflictingPattern,
                    description: format!(
                        "Recommended pattern has {:.1}% success rate with different tool sequence",
                        pattern.success_rate * 100.0
                    ),
                    severity: SelfConsistencySeverity::Warning,
                });
            }
        }
    }
}

/// Detect graph tool usage without graph entities
fn detect_graph_inconsistency(
    planned_tools: &[String],
    context_bundle: &ContextBundle,
    issues: &mut Vec<SelfConsistencyIssue>,
) {
    let graph_tools = [
        "code_graph_fusion_query",
        "raggraph_query",
        "raggraph_multihop",
        "graph_query",
    ];

    let has_graph_tools = planned_tools
        .iter()
        .any(|t| graph_tools.iter().any(|gt| t.contains(gt)));

    let has_graph_entities =
        !context_bundle.raggraph_entities.is_empty() || !context_bundle.memory_graph.is_empty();

    if has_graph_tools && !has_graph_entities {
        issues.push(SelfConsistencyIssue {
            kind: SelfConsistencyIssueKind::GraphInconsistency,
            description: "Plan uses graph tools but context has no graph entities".to_string(),
            severity: SelfConsistencySeverity::Warning,
        });
    }
}

/// Detect namespace mismatches in context
fn detect_namespace_mismatch(
    context_bundle: &ContextBundle,
    issues: &mut Vec<SelfConsistencyIssue>,
) {
    // Extract namespaces from entities (via file paths)
    let mut namespaces = std::collections::HashSet::new();

    for entity in &context_bundle.raggraph_entities {
        if let Some(ns) = extract_namespace(&entity.file_path) {
            namespaces.insert(ns);
        }
    }

    // Check graph relations for namespace consistency
    for relation in &context_bundle.memory_graph {
        if let Some(props) = &relation.properties {
            if let Some(ns) = props.get("namespace").and_then(|v| v.as_str()) {
                if !namespaces.is_empty() && !namespaces.contains(ns) {
                    issues.push(SelfConsistencyIssue {
                        kind: SelfConsistencyIssueKind::NamespaceMismatch,
                        description: format!(
                            "Graph relations reference different namespace: {}",
                            ns
                        ),
                        severity: SelfConsistencySeverity::Info,
                    });
                    return; // Only report once
                }
            }
        }
    }
}

/// Detect suspicious tool ordering
fn detect_tool_order_suspicious(
    intent: &QueryIntent,
    selected_mode: &str,
    planned_tools: &[String],
    recommended_patterns: &[ReasoningPattern],
    issues: &mut Vec<SelfConsistencyIssue>,
) {
    for pattern in recommended_patterns {
        if intent_matches(&pattern.intent_type, intent)
            && pattern.selected_mode == selected_mode
            && pattern.success_rate > 0.8
        {
            // Check if planned tools are in reversed or incorrect order
            if has_suspicious_ordering(planned_tools, &pattern.tool_sequence) {
                issues.push(SelfConsistencyIssue {
                    kind: SelfConsistencyIssueKind::ToolOrderSuspicious,
                    description: "Tool ordering differs from successful historical patterns"
                        .to_string(),
                    severity: SelfConsistencySeverity::Warning,
                });
                return;
            }
        }
    }
}

/// Detect missing required steps
fn detect_missing_required_step(
    planned_tools: &[String],
    recommended_patterns: &[ReasoningPattern],
    issues: &mut Vec<SelfConsistencyIssue>,
) {
    // Check if patterns show common prerequisites
    for pattern in recommended_patterns {
        if pattern.success_rate > 0.8 {
            // Check if pattern has indexing before graph operations
            if pattern.tool_sequence.len() > 1 {
                let first_tool = &pattern.tool_sequence[0];
                if first_tool.contains("code_index") || first_tool.contains("document_index") {
                    // Check if planned tools skip this step
                    if !planned_tools.iter().any(|t| t.contains("index"))
                        && planned_tools
                            .iter()
                            .any(|t| t.contains("graph") || t.contains("query"))
                    {
                        issues.push(SelfConsistencyIssue {
                            kind: SelfConsistencyIssueKind::MissingRequiredStep,
                            description: "Missing indexing step before graph/query operations"
                                .to_string(),
                            severity: SelfConsistencySeverity::Warning,
                        });
                        return;
                    }
                }
            }
        }
    }
}

/// Detect potential loops in continuity
fn detect_potential_loop(
    planned_tools: &[String],
    continuity: &ReasoningContinuity,
    issues: &mut Vec<SelfConsistencyIssue>,
) {
    if continuity.episodes.len() < 4 {
        return;
    }

    // Check for alternating pattern in last 4 episodes
    let recent = &continuity.episodes[continuity.episodes.len().saturating_sub(4)..];

    if recent.len() >= 4 {
        let tools: Vec<_> = recent.iter().map(|e| &e.tool_calls).collect();

        // Detect A-B-A-B pattern
        if tools[0] == tools[2] && tools[1] == tools[3] && tools[0] != tools[1] {
            // Check if current plan continues the pattern
            if !planned_tools.is_empty() && planned_tools[0] == tools[0][0] {
                issues.push(SelfConsistencyIssue {
                    kind: SelfConsistencyIssueKind::PotentialLoop,
                    description: "Detected repeating tool alternation pattern".to_string(),
                    severity: SelfConsistencySeverity::Warning,
                });
            }
        }
    }
}

/// Compute overall consistency score
fn compute_score(issues: &[SelfConsistencyIssue]) -> f32 {
    let mut penalty: f32 = 0.0;

    for issue in issues {
        penalty += match issue.severity {
            SelfConsistencySeverity::Info => 0.05,
            SelfConsistencySeverity::Warning => 0.15,
            SelfConsistencySeverity::Error => 0.30,
        };
    }

    (1.0_f32 - penalty).max(0.0_f32)
}

/// Generate suggested plan based on patterns and issues
fn generate_suggested_plan(
    recommended_patterns: &[ReasoningPattern],
    _issues: &[SelfConsistencyIssue],
) -> Option<SuggestedPlan> {
    // Find highest success rate pattern
    let best_pattern = recommended_patterns
        .iter()
        .max_by(|a, b| a.success_rate.partial_cmp(&b.success_rate).unwrap())?;

    if best_pattern.success_rate > 0.8 {
        Some(SuggestedPlan {
            recommended_tool_sequence: best_pattern.tool_sequence.clone(),
            recommended_mode: Some(best_pattern.selected_mode.clone()),
            notes: Some(format!(
                "Based on pattern with {:.1}% success rate",
                best_pattern.success_rate * 100.0
            )),
        })
    } else {
        None
    }
}

// Helper functions

fn tools_match_sequence(a: &[String], b: &[String]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).all(|(x, y)| x == y)
}

fn intent_matches(a: &QueryIntent, b: &QueryIntent) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

fn extract_namespace(file_path: &str) -> Option<String> {
    // Extract namespace from file path (e.g., "src/core/alpha.rs" -> "core::alpha")
    let path = std::path::Path::new(file_path);
    let components: Vec<_> = path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .filter(|s| *s != "src" && !s.ends_with(".rs"))
        .collect();

    if components.is_empty() {
        None
    } else {
        Some(components.join("::"))
    }
}

fn has_suspicious_ordering(planned: &[String], pattern: &[String]) -> bool {
    if planned.len() != pattern.len() {
        return false;
    }

    // Check if same tools but different order
    let mut planned_sorted = planned.to_vec();
    let mut pattern_sorted = pattern.to_vec();
    planned_sorted.sort();
    pattern_sorted.sort();

    planned_sorted == pattern_sorted && planned != pattern
}
