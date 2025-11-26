//! Project Behavior Analysis
//!
//! Analyzes behavioral characteristics including stability scores,
//! volatility factors, and key execution flows.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::ProjectReasoningHelpers;
use crate::project_analysis::{
    compute_risk_score, diagnostics_severity::NormalizedSeverity, FileRiskInputs, HotspotInfo,
    ProjectAnalysisEngine,
};

/// Summary of a key execution flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowSummary {
    pub flow_path: Vec<String>, // Entity names in order
    pub combined_hotspot_score: f32,
    pub file_path: String,
}

/// Complete behavior summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorSummary {
    pub stability_score: f32,
    pub volatility_factors: Vec<String>,
    pub key_flows: Vec<FlowSummary>,
}

impl ProjectAnalysisEngine {
    /// Build behavior summary for the project
    pub async fn build_behavior(&self) -> Result<BehaviorSummary> {
        // Get hotspots data
        let hotspots_data = self.get_hotspots().await?;

        // Get diagnostics data
        let diagnostics = self.get_all_diagnostics().await?;

        // Compute overall stability score
        let stability_score =
            self.compute_stability_score(&hotspots_data.hotspots, &diagnostics)?;

        // Identify volatility factors
        let volatility_factors =
            self.identify_volatility_factors(&hotspots_data.hotspots, &diagnostics)?;

        // Identify key flows
        let key_flows = self.identify_key_flows(&hotspots_data.hotspots).await?;

        Ok(BehaviorSummary {
            stability_score,
            volatility_factors,
            key_flows,
        })
    }

    /// Compute stability score from risk scores, fan-in/fan-out, and diagnostics
    fn compute_stability_score(
        &self,
        hotspots: &[HotspotInfo],
        diagnostics: &[crate::project_analysis::CodeDiagnostic],
    ) -> Result<f32> {
        if hotspots.is_empty() {
            return Ok(1.0); // Perfect stability for empty project
        }

        // Group diagnostics by file and severity
        let mut diagnostics_by_file: HashMap<String, HashMap<NormalizedSeverity, u32>> =
            HashMap::new();

        for diagnostic in diagnostics {
            let file_diagnostics = diagnostics_by_file
                .entry(diagnostic.file_path.clone())
                .or_insert_with(HashMap::new);

            let severity = crate::project_analysis::diagnostics_severity::normalize_severity(
                &diagnostic.severity,
            );
            *file_diagnostics.entry(severity).or_insert(0) += 1;
        }

        // Compute risk scores for all hotspot files
        let mut total_risk = 0.0f32;
        let mut file_count = 0u32;

        for hotspot in hotspots {
            let file_diagnostics = diagnostics_by_file
                .get(&hotspot.file_path)
                .cloned()
                .unwrap_or_default();

            let risk_inputs = FileRiskInputs {
                file_path: hotspot.file_path.clone(),
                hotspot_score: hotspot.score,
                loc: hotspot.loc.unwrap_or(0),
                diagnostics_by_severity: file_diagnostics,
            };

            total_risk += compute_risk_score(&risk_inputs);
            file_count += 1;
        }

        // Convert average risk to stability score (inverse relationship)
        let average_risk = total_risk / file_count as f32;
        let stability_score = 1.0 / (1.0 + average_risk / 10.0); // Normalize to 0-1 range

        Ok(stability_score)
    }

    /// Identify volatility factors from hotspots and diagnostics
    fn identify_volatility_factors(
        &self,
        hotspots: &[HotspotInfo],
        diagnostics: &[crate::project_analysis::CodeDiagnostic],
    ) -> Result<Vec<String>> {
        let mut factors = Vec::new();

        // High fan-in/out indicates volatility
        for hotspot in hotspots {
            if hotspot.fan_in > 50 {
                factors.push(format!(
                    "High fan-in: {} has {} incoming dependencies",
                    hotspot.file_path, hotspot.fan_in
                ));
            }

            if hotspot.fan_out > 50 {
                factors.push(format!(
                    "High fan-out: {} has {} outgoing dependencies",
                    hotspot.file_path, hotspot.fan_out
                ));
            }
        }

        // Count clippy warnings by file
        let mut clippy_counts: HashMap<String, u32> = HashMap::new();
        for diagnostic in diagnostics {
            if diagnostic.tool == "clippy" {
                *clippy_counts
                    .entry(diagnostic.file_path.clone())
                    .or_insert(0) += 1;
            }
        }

        // High clippy warning count indicates volatility
        for (file_path, count) in clippy_counts {
            if count > 10 {
                factors.push(format!(
                    "High clippy warning count: {} has {} warnings",
                    file_path, count
                ));
            }
        }

        Ok(factors)
    }

    /// Identify top 3 key flows based on hotspot scores
    async fn identify_key_flows(&self, hotspots: &[HotspotInfo]) -> Result<Vec<FlowSummary>> {
        // Sort hotspots by score (descending)
        let mut sorted_hotspots = hotspots.to_vec();
        sorted_hotspots.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Take top 3 as key flows
        let mut key_flows = Vec::new();
        for hotspot in sorted_hotspots.iter().take(3) {
            // Create a simple flow summary (entity -> file)
            key_flows.push(FlowSummary {
                flow_path: vec![hotspot.file_path.clone()],
                combined_hotspot_score: hotspot.score,
                file_path: hotspot.file_path.clone(),
            });
        }

        Ok(key_flows)
    }
}
