//! Project Topology Analysis
//!
//! Analyzes the structural topology of a project including modules,
//! edges, cross-language links, and architecture warnings.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::project_analysis::{
    architecture_overview::ArchitectureOverviewData, complexity_dashboard::ComplexityDashboardData,
    ProjectAnalysisEngine, UnifiedDependencySummary,
};

/// Module summary for topology analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleSummary {
    pub file_path: String,
    pub language: String,
    pub entity_count: u32,
    pub incoming_edges: u32,
    pub outgoing_edges: u32,
    pub loc: Option<u32>,
}

/// Edge statistics for the project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeStats {
    pub calls: u32,
    pub imports: u32,
    pub contains: u32,
    pub implements: u32,
}

/// Complete topology summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologySummary {
    pub modules: Vec<ModuleSummary>,
    pub cross_language_links: u32,
    pub edge_counts: EdgeStats,
    pub architecture_warnings: Vec<String>,
}

impl ProjectAnalysisEngine {
    /// Build topology summary for project
    pub async fn build_topology(&self) -> Result<TopologySummary> {
        // Get unified dependencies (this should work)
        let unified_deps = self.build_unified_dependency_summary(None, None)?;

        // Build simple module summaries from unified deps
        let modules: Vec<ModuleSummary> = unified_deps
            .modules
            .iter()
            .map(|m| ModuleSummary {
                file_path: m.file_path.clone(),
                language: m.language.clone(),
                entity_count: m.entity_count,
                incoming_edges: m.incoming_edges,
                outgoing_edges: m.outgoing_edges,
                loc: None, // Placeholder
            })
            .collect();

        // Count edge types from unified deps
        let mut edge_counts = EdgeStats {
            calls: 0,
            imports: 0,
            contains: 0,
            implements: 0,
        };

        for dep in &unified_deps.dependencies {
            for edge_type in &dep.edge_types {
                match edge_type.as_str() {
                    "CALLS" => edge_counts.calls += dep.edge_count,
                    "IMPORTS" => edge_counts.imports += dep.edge_count,
                    "CONTAINS" => edge_counts.contains += dep.edge_count,
                    "IMPLEMENTS" => edge_counts.implements += dep.edge_count,
                    _ => {}
                }
            }
        }

        // Count cross-language links (simplified)
        let cross_language_links = 0; // Placeholder

        // Generate simple architecture warnings
        let mut architecture_warnings = Vec::new();
        if modules.is_empty() {
            architecture_warnings.push("No modules found in project".to_string());
        }

        Ok(TopologySummary {
            modules,
            cross_language_links,
            edge_counts,
            architecture_warnings,
        })
    }
}
