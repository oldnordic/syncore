//! Project-Level Reasoning Engine
//!
//! Provides deterministic, LLM-free reasoning about entire projects
//! across languages, producing architecture maps, behavior maps,
//! reasoning traces, problem maps, and action blueprints.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::project_analysis::ProjectAnalysisEngine;

// Re-export all modules
pub mod behavior;
pub mod blueprint;
pub mod problem_map;
pub mod topology;

// Re-export main types for convenience
pub use behavior::{BehaviorSummary, FlowSummary};
pub use blueprint::ProjectBlueprint;
pub use problem_map::{ProblemMapSummary, RiskDistribution};
pub use topology::{EdgeStats, ModuleSummary, TopologySummary};

/// Common helper functions for project reasoning modules
trait ProjectReasoningHelpers {
    async fn get_hotspots(&self) -> Result<crate::project_analysis::hotspots::HotspotsData>;
    async fn get_dead_code(&self) -> Result<crate::project_analysis::dead_code::DeadCodeData>;
    async fn get_unused_imports(
        &self,
    ) -> Result<crate::project_analysis::unused_imports::UnusedImportsData>;
    async fn get_all_diagnostics(&self) -> Result<Vec<crate::project_analysis::CodeDiagnostic>>;
}

impl ProjectReasoningHelpers for ProjectAnalysisEngine {
    async fn get_hotspots(&self) -> Result<crate::project_analysis::hotspots::HotspotsData> {
        use crate::project_analysis::hotspots::HotspotsRequest;

        let request = HotspotsRequest {
            limit: 100,
            min_fan_in: None,
            min_fan_out: None,
            min_entity_count: None,
            min_loc: None,
        };

        let response = self.hotspots(request).await?;
        Ok(response.data.unwrap())
    }

    async fn get_dead_code(&self) -> Result<crate::project_analysis::dead_code::DeadCodeData> {
        use crate::project_analysis::dead_code::DeadCodeRequest;

        let request = DeadCodeRequest {
            exclude_public: Some(false),
            limit: Some(100),
        };

        let response = self.dead_code(request).await?;
        Ok(response.data.unwrap())
    }

    async fn get_unused_imports(
        &self,
    ) -> Result<crate::project_analysis::unused_imports::UnusedImportsData> {
        use crate::project_analysis::unused_imports::UnusedImportsRequest;

        let request = UnusedImportsRequest {
            file_path: None,
            limit: Some(100),
        };

        let response = self.unused_imports(request).await?;
        Ok(response.data.unwrap())
    }

    async fn get_all_diagnostics(&self) -> Result<Vec<crate::project_analysis::CodeDiagnostic>> {
        let diagnostics_manager = crate::project_analysis::diagnostics::DiagnosticsManager::new(
            self.db_manager().clone(),
        );

        // Get all diagnostics (empty file path means all files)
        let diagnostics = diagnostics_manager.query_diagnostics_by_file("")?;

        Ok(diagnostics)
    }
}

/// Complete project reasoning overview
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectReasoningOverview {
    pub topology: TopologySummary,
    pub behavior: BehaviorSummary,
    pub problem_map: ProblemMapSummary,
    pub blueprint: ProjectBlueprint,
}

impl ProjectReasoningOverview {
    /// Build complete project reasoning overview
    pub async fn build(_engine: &ProjectAnalysisEngine) -> Result<Self> {
        // Build all components
        let topology = _engine.build_topology().await?;
        let behavior = _engine.build_behavior().await?;
        let problem_map = _engine.build_problem_map().await?;
        let blueprint = _engine.build_blueprint().await?;

        Ok(ProjectReasoningOverview {
            topology,
            behavior,
            problem_map,
            blueprint,
        })
    }
}
