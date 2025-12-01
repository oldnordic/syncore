//! Project Problem Map Analysis
//!
//! Identifies and categorizes problems including critical hotspots,
//! brittle paths, cross-module issues, and risk distribution.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::ProjectReasoningHelpers;
use crate::project_analysis::{
    compute_risk_score, diagnostics_severity::NormalizedSeverity, FileRiskInputs, HotspotInfo,
    ProjectAnalysisEngine,
};

/// Risk distribution across the project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskDistribution {
    pub high_risk_files: u32,   // risk_score > 25
    pub medium_risk_files: u32, // risk_score 10-25
    pub low_risk_files: u32,    // risk_score < 10
    pub average_risk_score: f32,
}

/// Complete problem map summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemMapSummary {
    pub critical_hotspots: Vec<String>,
    pub brittle_paths: Vec<String>,
    pub cross_module_issues: Vec<String>,
    pub risk_distribution: RiskDistribution,
}

impl ProjectAnalysisEngine {
    /// Build problem map for the project
    pub async fn build_problem_map(&self) -> Result<ProblemMapSummary> {
        // Get hotspots, dead code, unused imports, and diagnostics
        let hotspots_data = self.get_hotspots().await?;
        let _dead_code_data = self.get_dead_code().await?;
        let _unused_imports_data = self.get_unused_imports().await?;
        let diagnostics = self.get_all_diagnostics().await?;

        // Identify critical hotspots
        let critical_hotspots = self.identify_critical_hotspots(&hotspots_data.hotspots)?;

        // Identify brittle paths
        let brittle_paths =
            self.identify_brittle_paths(&hotspots_data.hotspots, &diagnostics).await?;

        // Identify cross-module issues
        let cross_module_issues =
            self.identify_cross_module_issues(&hotspots_data.hotspots).await?;

        // Compute risk distribution
        let risk_distribution =
            self.compute_risk_distribution(&hotspots_data.hotspots, &diagnostics)?;

        Ok(ProblemMapSummary {
            critical_hotspots,
            brittle_paths,
            cross_module_issues,
            risk_distribution,
        })
    }

    /// Identify critical hotspots (high score, high fan-in/out)
    fn identify_critical_hotspots(&self, hotspots: &[HotspotInfo]) -> Result<Vec<String>> {
        let mut critical = Vec::new();

        for hotspot in hotspots {
            // Critical if: hotspot_score >= 100 OR fan_in >= 30 OR fan_out >= 30
            if hotspot.score >= 100.0 || hotspot.fan_in >= 30 || hotspot.fan_out >= 30 {
                critical.push(format!(
                    "{}: score={:.1}, fan_in={}, fan_out={}, loc={:?}",
                    hotspot.file_path, hotspot.score, hotspot.fan_in, hotspot.fan_out, hotspot.loc
                ));
            }
        }

        Ok(critical)
    }

    /// Identify brittle paths (high risk + clippy errors)
    async fn identify_brittle_paths(
        &self,
        hotspots: &[HotspotInfo],
        diagnostics: &[crate::project_analysis::CodeDiagnostic],
    ) -> Result<Vec<String>> {
        let mut brittle = Vec::new();

        // Group diagnostics by file and count clippy errors
        let mut clippy_errors_by_file: HashMap<String, u32> = HashMap::new();
        for diagnostic in diagnostics {
            if diagnostic.tool == "clippy"
                && (diagnostic.severity == "error" || diagnostic.severity == "warning")
            {
                *clippy_errors_by_file.entry(diagnostic.file_path.clone()).or_insert(0) += 1;
            }
        }

        // Check each hotspot for brittleness
        for hotspot in hotspots {
            let clippy_errors = clippy_errors_by_file.get(&hotspot.file_path).unwrap_or(&0);

            // Brittle if: risk_score > 30 AND clippy errors > 0
            if hotspot.score > 30.0 && *clippy_errors > 0 {
                brittle.push(format!(
                    "{}: risk_score={:.1}, clippy_errors={}, line_start={}",
                    hotspot.file_path,
                    hotspot.score,
                    clippy_errors,
                    0 // We don't have line info for hotspots
                ));
            }
        }

        Ok(brittle)
    }

    /// Identify cross-module issues (high fan_out or risk crossing boundaries)
    async fn identify_cross_module_issues(&self, hotspots: &[HotspotInfo]) -> Result<Vec<String>> {
        let mut issues = Vec::new();

        for hotspot in hotspots {
            // Cross-module issue if: fan_out > 30 OR risk_score > 25
            if hotspot.fan_out > 30 || hotspot.score > 25.0 {
                issues.push(format!(
                    "{}: fan_out={}, risk_score={:.1}, entity_count={}",
                    hotspot.file_path, hotspot.fan_out, hotspot.score, hotspot.entity_count
                ));
            }
        }

        Ok(issues)
    }

    /// Compute risk distribution across files
    fn compute_risk_distribution(
        &self,
        hotspots: &[HotspotInfo],
        diagnostics: &[crate::project_analysis::CodeDiagnostic],
    ) -> Result<RiskDistribution> {
        if hotspots.is_empty() {
            return Ok(RiskDistribution {
                high_risk_files: 0,
                medium_risk_files: 0,
                low_risk_files: 0,
                average_risk_score: 0.0,
            });
        }

        // Group diagnostics by file and severity
        let mut diagnostics_by_file: HashMap<String, HashMap<NormalizedSeverity, u32>> =
            HashMap::new();

        for diagnostic in diagnostics {
            let file_diagnostics =
                diagnostics_by_file.entry(diagnostic.file_path.clone()).or_default();

            let severity = crate::project_analysis::diagnostics_severity::normalize_severity(
                &diagnostic.severity,
            );
            *file_diagnostics.entry(severity).or_insert(0) += 1;
        }

        // Compute risk scores and categorize
        let mut high_risk = 0u32;
        let mut medium_risk = 0u32;
        let mut low_risk = 0u32;
        let mut total_risk = 0.0f32;

        for hotspot in hotspots {
            let file_diagnostics =
                diagnostics_by_file.get(&hotspot.file_path).cloned().unwrap_or_default();

            let risk_inputs = FileRiskInputs {
                file_path: hotspot.file_path.clone(),
                hotspot_score: hotspot.score,
                loc: hotspot.loc.unwrap_or(0),
                diagnostics_by_severity: file_diagnostics,
            };

            let risk_score = compute_risk_score(&risk_inputs);
            total_risk += risk_score;

            if risk_score > 25.0 {
                high_risk += 1;
            } else if risk_score >= 10.0 {
                medium_risk += 1;
            } else {
                low_risk += 1;
            }
        }

        let average_risk_score = total_risk / hotspots.len() as f32;

        Ok(RiskDistribution {
            high_risk_files: high_risk,
            medium_risk_files: medium_risk,
            low_risk_files: low_risk,
            average_risk_score,
        })
    }
}
