//! Project Blueprint Generation
//!
//! Creates prioritized action blueprints by combining improvement roadmap
//! and refactor action plan into immediate, medium, and long-term tasks.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::ProjectReasoningHelpers;
use crate::project_analysis::{
    compute_risk_score, diagnostics_severity::NormalizedSeverity, DeadCodeInfo, FileRiskInputs,
    HotspotInfo, ProjectAnalysisEngine, UnusedImportInfo,
};

/// Complete project blueprint with prioritized actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectBlueprint {
    pub immediate_fixes: Vec<String>,
    pub medium_tasks: Vec<String>,
    pub long_term_refactors: Vec<String>,
}

impl ProjectAnalysisEngine {
    /// Build project blueprint from PAE data
    pub async fn build_blueprint(&self) -> Result<ProjectBlueprint> {
        // Get data from various PAE tools
        let hotspots_data = self.get_hotspots().await?;
        let dead_code_data = self.get_dead_code().await?;
        let unused_imports_data = self.get_unused_imports().await?;
        let diagnostics = self.get_all_diagnostics().await?;
        let improvement_roadmap = self.get_improvement_roadmap().await?;
        let refactor_action_plan = self.get_refactor_action_plan().await?;

        // Categorize into three buckets
        let immediate_fixes = self.categorize_immediate_fixes(
            &dead_code_data.dead_entities,
            &unused_imports_data.unused_imports,
            &diagnostics,
        )?;

        let medium_tasks = self.categorize_medium_tasks(
            &hotspots_data.hotspots,
            &diagnostics,
            &improvement_roadmap,
        )?;

        let long_term_refactors =
            self.categorize_long_term_refactors(&hotspots_data.hotspots, &refactor_action_plan)?;

        Ok(ProjectBlueprint {
            immediate_fixes,
            medium_tasks,
            long_term_refactors,
        })
    }

    /// Categorize immediate fixes: dead code, unused imports, clippy errors
    fn categorize_immediate_fixes(
        &self,
        dead_code: &[DeadCodeInfo],
        unused_imports: &[UnusedImportInfo],
        diagnostics: &[crate::project_analysis::CodeDiagnostic],
    ) -> Result<Vec<String>> {
        let mut immediate = Vec::new();

        // Dead code fixes
        for dead in dead_code.iter().take(20) {
            // Limit to prevent overwhelming output
            immediate.push(format!(
                "Remove dead code: {} ({}) at {}:{}",
                dead.name, dead.entity_type, dead.file_path, dead.line_start
            ));
        }

        // Unused import fixes
        for unused in unused_imports.iter().take(20) {
            immediate.push(format!(
                "Remove unused import: {} in {}{}",
                unused.import_name,
                unused.file_path,
                unused.line.map(|l| format!(":{}", l)).unwrap_or_default()
            ));
        }

        // Clippy error fixes
        for diagnostic in diagnostics {
            if diagnostic.tool == "clippy" && diagnostic.severity == "error" {
                immediate.push(format!(
                    "Fix clippy error: {} at {}:{} - {}",
                    diagnostic.diagnostic_type,
                    diagnostic.file_path,
                    diagnostic.line_start,
                    diagnostic.message
                ));
            }
        }

        Ok(immediate)
    }

    /// Categorize medium tasks: medium risk, moderate hotspots, smaller files
    fn categorize_medium_tasks(
        &self,
        hotspots: &[HotspotInfo],
        diagnostics: &[crate::project_analysis::CodeDiagnostic],
        _improvement_roadmap: &crate::project_analysis::improvement_roadmap::ImprovementRoadmapData,
    ) -> Result<Vec<String>> {
        let mut medium = Vec::new();

        // Group diagnostics by file and severity
        let mut diagnostics_by_file: std::collections::HashMap<
            String,
            std::collections::HashMap<NormalizedSeverity, u32>,
        > = std::collections::HashMap::new();

        for diagnostic in diagnostics {
            let file_diagnostics =
                diagnostics_by_file.entry(diagnostic.file_path.clone()).or_default();

            let severity = crate::project_analysis::diagnostics_severity::normalize_severity(
                &diagnostic.severity,
            );
            *file_diagnostics.entry(severity).or_insert(0) += 1;
        }

        // Medium risk hotspots
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

            // Medium criteria: risk_score 5-15, hotspot_score < 100, LOC < 500
            if (5.0..=15.0).contains(&risk_score)
                && hotspot.score < 100.0
                && hotspot.loc.unwrap_or(0) < 500
            {
                medium.push(format!(
                    "Medium priority: {} (risk_score={:.1}, hotspot_score={:.1}, loc={:?})",
                    hotspot.file_path, risk_score, hotspot.score, hotspot.loc
                ));
            }
        }

        Ok(medium)
    }

    /// Categorize long-term refactors: high hotspots, large files, high fan-in
    fn categorize_long_term_refactors(
        &self,
        hotspots: &[HotspotInfo],
        _refactor_action_plan: &crate::project_analysis::refactor_action_plan::RefactorActionPlanData,
    ) -> Result<Vec<String>> {
        let mut long_term = Vec::new();

        for hotspot in hotspots {
            // Long-term criteria: hotspot_score >= 100, LOC >= 1000, fan_in >= 30
            if hotspot.score >= 100.0 || hotspot.loc.unwrap_or(0) >= 1000 || hotspot.fan_in >= 30 {
                long_term.push(format!(
                    "Long-term refactor: {} (hotspot_score={:.1}, loc={:?}, fan_in={}, fan_out={})",
                    hotspot.file_path, hotspot.score, hotspot.loc, hotspot.fan_in, hotspot.fan_out
                ));
            }
        }

        Ok(long_term)
    }

    /// Helper: Get improvement roadmap
    async fn get_improvement_roadmap(
        &self,
    ) -> Result<crate::project_analysis::improvement_roadmap::ImprovementRoadmapData> {
        use crate::project_analysis::improvement_roadmap::ImprovementRoadmapRequest;

        let request = ImprovementRoadmapRequest {
            limit_per_category: Some(20),
            high_priority_only: Some(false),
            hotspot_loc_threshold: Some(100),
            project_label: None,
        };

        let response = self.improvement_roadmap(request).await?;
        Ok(response.data.unwrap())
    }

    /// Helper: Get refactor action plan
    async fn get_refactor_action_plan(
        &self,
    ) -> Result<crate::project_analysis::refactor_action_plan::RefactorActionPlanData> {
        use crate::project_analysis::refactor_action_plan::RefactorActionPlanRequest;

        let request = RefactorActionPlanRequest {};

        let response = self.refactor_action_plan(request).await?;
        Ok(response.data.unwrap())
    }
}
