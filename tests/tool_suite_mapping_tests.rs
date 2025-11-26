//! Tool Suite Mapping Tests - APEX v1.4
//!
//! Validates the tool_suite_mapping.json file and ensures
//! all legacy tools are properly mapped to suite commands.

use serde::Deserialize;
use std::collections::HashSet;

#[derive(Debug, Deserialize)]
struct ToolMapping {
    legacy_name: String,
    suite_name: String,
    suite_command: String,
    status: String,
    deprecated: bool,
}

#[derive(Debug, Deserialize)]
struct MappingStatistics {
    total_legacy_tools: usize,
    exact_parity: usize,
    missing: usize,
    suites_count: usize,
}

#[derive(Debug, Deserialize)]
struct ToolSuiteMapping {
    version: String,
    description: String,
    suites: Vec<String>,
    mappings: Vec<ToolMapping>,
    statistics: MappingStatistics,
}

fn load_mapping() -> ToolSuiteMapping {
    let json_content = include_str!("tool_suite_mapping.json");
    serde_json::from_str(json_content).expect("Failed to parse tool_suite_mapping.json")
}

#[test]
fn test_mapping_file_is_well_formed() {
    let mapping = load_mapping();

    assert_eq!(mapping.version, "1.4.0");
    assert!(!mapping.suites.is_empty());
    assert!(!mapping.mappings.is_empty());
}

#[test]
fn test_all_suites_are_valid() {
    let mapping = load_mapping();
    let valid_suites: HashSet<&str> = [
        "memory_suite",
        "code_suite",
        "graph_suite",
        "mapping_suite",
        "debug_suite",
    ]
    .into_iter()
    .collect();

    // All declared suites must be in the valid set
    for suite in &mapping.suites {
        assert!(
            valid_suites.contains(suite.as_str()),
            "Invalid suite declared: {}",
            suite
        );
    }

    // All mappings must reference valid suites
    for m in &mapping.mappings {
        assert!(
            valid_suites.contains(m.suite_name.as_str()),
            "Mapping {} references invalid suite: {}",
            m.legacy_name,
            m.suite_name
        );
    }
}

#[test]
fn test_no_duplicate_legacy_tools() {
    let mapping = load_mapping();
    let mut seen: HashSet<&str> = HashSet::new();

    for m in &mapping.mappings {
        assert!(
            seen.insert(&m.legacy_name),
            "Duplicate legacy tool in mapping: {}",
            m.legacy_name
        );
    }
}

#[test]
fn test_status_values_are_valid() {
    let mapping = load_mapping();
    let valid_statuses = ["exact_parity", "partial_parity", "missing"];

    for m in &mapping.mappings {
        assert!(
            valid_statuses.contains(&m.status.as_str()),
            "Invalid status '{}' for tool {}",
            m.status,
            m.legacy_name
        );
    }
}

#[test]
fn test_statistics_match_mappings() {
    let mapping = load_mapping();

    let total = mapping.mappings.len();
    let exact_parity = mapping
        .mappings
        .iter()
        .filter(|m| m.status == "exact_parity")
        .count();
    let missing = mapping
        .mappings
        .iter()
        .filter(|m| m.status == "missing")
        .count();

    assert_eq!(
        mapping.statistics.total_legacy_tools, total,
        "Statistics total_legacy_tools mismatch"
    );
    assert_eq!(
        mapping.statistics.exact_parity, exact_parity,
        "Statistics exact_parity mismatch"
    );
    assert_eq!(
        mapping.statistics.missing, missing,
        "Statistics missing mismatch"
    );
    assert_eq!(
        mapping.statistics.suites_count,
        mapping.suites.len(),
        "Statistics suites_count mismatch"
    );
}

#[test]
fn test_deprecated_tools_have_exact_parity() {
    let mapping = load_mapping();

    for m in &mapping.mappings {
        if m.deprecated {
            assert_eq!(
                m.status, "exact_parity",
                "Deprecated tool {} should have exact_parity status, has {}",
                m.legacy_name, m.status
            );
        }
    }
}

#[test]
fn test_exact_parity_tools_are_deprecated() {
    let mapping = load_mapping();

    for m in &mapping.mappings {
        if m.status == "exact_parity" {
            assert!(
                m.deprecated,
                "Tool {} with exact_parity should be deprecated",
                m.legacy_name
            );
        }
    }
}

#[test]
fn test_suite_command_not_empty() {
    let mapping = load_mapping();

    for m in &mapping.mappings {
        assert!(
            !m.suite_command.is_empty(),
            "Tool {} has empty suite_command",
            m.legacy_name
        );
    }
}

#[test]
fn test_known_legacy_tools_are_mapped() {
    let mapping = load_mapping();
    let mapped_tools: HashSet<&str> = mapping
        .mappings
        .iter()
        .map(|m| m.legacy_name.as_str())
        .collect();

    // Critical tools that MUST be mapped
    let required_tools = [
        "memory_store",
        "memory_query",
        "vector_insert",
        "vector_search",
        "code_index",
        "code_search",
        "graph_query",
        "graph_insert",
        "graph_relate",
        "mapping_record",
        "mapping_get",
        "mapping_search",
        "mapping_deps",
        "logs_tail",
        "project_hotspots",
        "project_dead_code",
    ];

    for tool in required_tools {
        assert!(
            mapped_tools.contains(tool),
            "Required tool {} is not in mapping",
            tool
        );
    }
}

/// Count tools by suite for reporting
#[test]
fn test_suite_distribution() {
    let mapping = load_mapping();

    let mut suite_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for m in &mapping.mappings {
        *suite_counts.entry(&m.suite_name).or_insert(0) += 1;
    }

    // Each suite should have at least one tool
    for suite in &mapping.suites {
        assert!(
            suite_counts.get(suite.as_str()).copied().unwrap_or(0) > 0,
            "Suite {} has no mapped tools",
            suite
        );
    }
}

/// Ensure migration progress is trackable
#[test]
fn test_migration_progress() {
    let mapping = load_mapping();

    let exact_parity = mapping.statistics.exact_parity;
    let total = mapping.statistics.total_legacy_tools;

    let progress = (exact_parity as f64 / total as f64) * 100.0;

    // Currently at ~26% parity (17/65)
    // This test will fail when we complete migration and need to update
    assert!(
        progress >= 25.0,
        "Migration progress too low: {:.1}%",
        progress
    );

    println!(
        "Migration progress: {:.1}% ({}/{})",
        progress, exact_parity, total
    );
}
