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
        // Get unified dependencies
        let unified_deps = self.build_unified_dependency_summary(None, None)?;

        // Get complexity dashboard for LOC data
        let complexity = self.get_complexity_dashboard().await?;

        // Build comprehensive module summaries using integrated helper
        let modules = self.build_module_summaries(&unified_deps, &complexity)?;

        // Count edge types using integrated helper
        let edge_counts = self.count_edge_types(&unified_deps)?;

        // Count cross-language links using integrated helper
        let cross_language_links = self.count_cross_language_links(&unified_deps)?;

        // Get architecture overview for warnings
        let arch_overview = self.get_architecture_overview().await?;

        // Generate comprehensive architecture warnings using integrated helper
        let architecture_warnings =
            self.generate_architecture_warnings(&modules, &arch_overview)?;

        Ok(TopologySummary {
            modules,
            cross_language_links,
            edge_counts,
            architecture_warnings,
        })
    }

    /// Build module summaries from unified dependencies
    fn build_module_summaries(
        &self,
        unified_deps: &UnifiedDependencySummary,
        complexity: &ComplexityDashboardData,
    ) -> Result<Vec<ModuleSummary>> {
        let mut modules = Vec::new();

        for unified_module in &unified_deps.modules {
            // Get LOC from complexity data if available
            let loc = complexity
                .files
                .iter()
                .find(|f| f.file_path == unified_module.file_path)
                .and_then(|f| f.loc);

            modules.push(ModuleSummary {
                file_path: unified_module.file_path.clone(),
                language: unified_module.language.clone(),
                entity_count: unified_module.entity_count,
                incoming_edges: unified_module.incoming_edges,
                outgoing_edges: unified_module.outgoing_edges,
                loc,
            });
        }

        Ok(modules)
    }

    /// Count different types of edges
    fn count_edge_types(&self, unified_deps: &UnifiedDependencySummary) -> Result<EdgeStats> {
        let mut edge_counts = EdgeStats {
            calls: 0,
            imports: 0,
            contains: 0,
            implements: 0,
        };

        for dependency in &unified_deps.dependencies {
            for edge_type in &dependency.edge_types {
                match edge_type.as_str() {
                    "CALLS" => edge_counts.calls += dependency.edge_count,
                    "IMPORTS" => edge_counts.imports += dependency.edge_count,
                    "CONTAINS" => edge_counts.contains += dependency.edge_count,
                    "IMPLEMENTS" => edge_counts.implements += dependency.edge_count,
                    _ => {} // Ignore other edge types
                }
            }
        }

        Ok(edge_counts)
    }

    /// Count cross-language links
    fn count_cross_language_links(&self, unified_deps: &UnifiedDependencySummary) -> Result<u32> {
        let mut cross_language_count = 0u32;

        for dependency in &unified_deps.dependencies {
            if dependency.from_language != dependency.to_language {
                cross_language_count += dependency.edge_count;
            }
        }

        Ok(cross_language_count)
    }

    /// Generate architecture warnings based on heuristics
    fn generate_architecture_warnings(
        &self,
        modules: &[ModuleSummary],
        _arch_overview: &ArchitectureOverviewData,
    ) -> Result<Vec<String>> {
        let mut warnings = Vec::new();

        for module in modules {
            // Warning: modules with > 20 incoming edges
            if module.incoming_edges > 20 {
                warnings.push(format!(
                    "High incoming edge count: {} has {} incoming edges",
                    module.file_path, module.incoming_edges
                ));
            }

            // Warning: modules with > 1000 LOC
            if let Some(loc) = module.loc {
                if loc > 1000 {
                    warnings.push(format!(
                        "Large module: {} has {} LOC",
                        module.file_path, loc
                    ));
                }
            }
        }

        Ok(warnings)
    }

    /// Helper: Get unified dependency summary
    fn get_unified_dependency_summary(&self) -> Result<UnifiedDependencySummary> {
        // Use existing unified dependency API
        self.build_unified_dependency_summary(None, None)
    }

    /// Helper: Get architecture overview
    async fn get_architecture_overview(&self) -> Result<ArchitectureOverviewData> {
        use crate::project_analysis::architecture_overview::ArchitectureOverviewRequest;

        let request = ArchitectureOverviewRequest {
            limit_hotspots: Some(100),
            limit_modules: Some(100),
            loc_threshold: Some(1000),
        };

        let response = self.architecture_overview(request).await?;
        Ok(response.data.unwrap())
    }

    /// Helper: Get complexity dashboard
    async fn get_complexity_dashboard(&self) -> Result<ComplexityDashboardData> {
        use crate::project_analysis::complexity_dashboard::ComplexityDashboardRequest;

        let request = ComplexityDashboardRequest {
            limit_hotspots: Some(100),
            loc_threshold: Some(1000),
        };

        let response = self.complexity_dashboard(request).await?;
        Ok(response.data.unwrap())
    }
}
