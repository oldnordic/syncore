//! Project Architecture Overview Meta-Tool
//!
//! Aggregates existing PAE data to provide a human-usable "big picture" of codebase.
//! This is Meta-Tool #1 from the defined list.

use crate::project_analysis::{
    compute_risk_score, diagnostics::DiagnosticsManager, normalize_severity, DeadCodeInfo,
    FileRiskInputs, HotspotInfo, ModuleInfo, PAEResponse, ProjectAnalysisEngine, UnusedImportInfo,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap};
use std::sync::Arc;

/// Request parameters for project_architecture_overview
#[derive(Debug, Deserialize, Default)]
pub struct ArchitectureOverviewRequest {
    pub limit_hotspots: Option<usize>,
    pub limit_modules: Option<usize>,
    pub loc_threshold: Option<u32>,
}

/// Module with aggregated analysis data
#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleOverview {
    pub file_path: String,
    pub entity_count: u32,
    pub fan_in: u32,
    pub fan_out: u32,
    pub loc: Option<u32>,
    pub hotspot_score: f32,
    pub dead_entities: u32,
    pub unused_imports: u32,
    pub clippy_warning_count: u32,
    pub risk_score: f32,
}

/// Architecture overview summary statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct ArchitectureSummary {
    pub total_files_indexed: u32,
    pub total_entities: u32,
    pub total_edges: u32,
    pub modules_analyzed: u32,
    pub hotspot_count: u32,
    pub dead_entity_count: u32,
    pub unused_import_count: u32,
    pub clippy_warning_count: u32,
    pub max_loc: u32,
}

/// Architecture overview response data
#[derive(Debug, Serialize, Deserialize)]
pub struct ArchitectureOverviewData {
    pub summary: ArchitectureSummary,
    pub modules: Vec<ModuleOverview>,
    pub hotspots: Vec<HotspotInfo>,
    pub notes: ArchitectureNotes,
}

/// Processing notes and limits used
#[derive(Debug, Serialize, Deserialize)]
pub struct ArchitectureNotes {
    pub loc_threshold: u32,
    pub limit_modules: usize,
    pub limit_hotspots: usize,
}

impl ProjectAnalysisEngine {
    /// Generate comprehensive architecture overview of the project
    pub async fn architecture_overview(
        &self,
        request: ArchitectureOverviewRequest,
    ) -> Result<PAEResponse<ArchitectureOverviewData>> {
        match self.generate_architecture_overview(request).await {
            Ok(data) => Ok(PAEResponse::success(data)),
            Err(e) => Ok(PAEResponse::error(e.to_string())),
        }
    }

    async fn generate_architecture_overview(
        &self,
        request: ArchitectureOverviewRequest,
    ) -> Result<ArchitectureOverviewData> {
        // Extract request parameters with defaults
        let limit_hotspots = request.limit_hotspots.unwrap_or(10);
        let limit_modules = request.limit_modules.unwrap_or(50);
        let loc_threshold = request.loc_threshold.unwrap_or(300);

        // 2. Get module map (dependency analysis)
        let module_map_request = crate::project_analysis::deps::ModuleMapRequest {
            root: None,
            max_modules: Some(limit_modules as u32),
        };
        let module_map_response = self.module_map(module_map_request).await?;
        let modules = match module_map_response.data {
            Some(data) => data.modules,
            None => return Err(anyhow::anyhow!("Module map analysis returned no data")),
        };

        // 3. Get hotspots
        let hotspots_request = crate::project_analysis::hotspots::HotspotsRequest {
            limit: limit_hotspots as u32,
            min_fan_in: Some(1),
            min_fan_out: Some(1),
            min_entity_count: Some(1),
            min_loc: Some(1),
        };
        let hotspots_response = self.hotspots(hotspots_request).await?;
        let hotspots = match hotspots_response.data {
            Some(data) => data.hotspots,
            None => {
                // No hotspots found - this is valid for small test projects
                Vec::new()
            }
        };

        // 1. Get basic project statistics (after getting hotspots for accurate count)
        let mut summary = self.get_project_summary().await?;
        summary.hotspot_count = hotspots.len() as u32;

        // 4. Get dead code
        let dead_code_request = crate::project_analysis::dead_code::DeadCodeRequest {
            exclude_public: Some(true),
            limit: None,
        };
        let dead_code_response = self.dead_code(dead_code_request).await?;
        let dead_entities = match dead_code_response.data {
            Some(data) => data.dead_entities,
            None => return Err(anyhow::anyhow!("Dead code analysis returned no data")),
        };

        // 5. Get unused imports
        let unused_imports_request =
            crate::project_analysis::unused_imports::UnusedImportsRequest {
                file_path: None,
                limit: None,
            };
        let unused_imports_response = self.unused_imports(unused_imports_request).await?;
        let unused_imports = match unused_imports_response.data {
            Some(data) => data.unused_imports,
            None => return Err(anyhow::anyhow!("Unused imports analysis returned no data")),
        };

        // 6. Aggregate data by module
        let module_overviews = self.aggregate_module_data(
            &modules,
            &hotspots,
            &dead_entities,
            &unused_imports,
            loc_threshold,
        )?;

        // 7. Filter hotspots by limit
        let limited_hotspots = hotspots.into_iter().take(limit_hotspots).collect();

        let notes = ArchitectureNotes {
            loc_threshold,
            limit_modules,
            limit_hotspots,
        };

        Ok(ArchitectureOverviewData {
            summary,
            modules: module_overviews,
            hotspots: limited_hotspots,
            notes,
        })
    }

    /// Get basic project statistics
    async fn get_project_summary(&self) -> Result<ArchitectureSummary> {
        // Get counts for dead code and unused imports using public APIs
        let dead_code_request = crate::project_analysis::dead_code::DeadCodeRequest {
            exclude_public: Some(true),
            limit: None,
        };
        let dead_code_response = self.dead_code(dead_code_request).await?;
        let dead_entity_count = match dead_code_response.data {
            Some(data) => data.dead_entities.len() as u32,
            None => 0,
        };

        let unused_imports_request =
            crate::project_analysis::unused_imports::UnusedImportsRequest {
                file_path: None,
                limit: None,
            };
        let unused_imports_response = self.unused_imports(unused_imports_request).await?;
        let unused_import_count = match unused_imports_response.data {
            Some(data) => data.unused_imports.len() as u32,
            None => 0,
        };

        // Get basic database stats without holding lock across awaits
        {
            let conn = self.code_graph_conn();
            let conn_guard = conn.lock().unwrap();

            let total_entities: u32 = conn_guard
                .query_row("SELECT COUNT(*) FROM code_entities", [], |row| row.get(0))
                .unwrap_or(0);

            let total_edges: u32 = conn_guard
                .query_row("SELECT COUNT(*) FROM code_edges", [], |row| row.get(0))
                .unwrap_or(0);

            let total_files_indexed: u32 = conn_guard
                .query_row(
                    "SELECT COUNT(DISTINCT file_path) FROM code_entities",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(0);

            let max_loc: u32 = conn_guard.query_row(
                "SELECT MAX(CAST(line_end AS INTEGER) - CAST(line_start AS INTEGER)) FROM code_entities WHERE line_end > line_start",
                [],
                |row| row.get(0),
            ).unwrap_or(0);

            // Get total Clippy warning count
            let clippy_warning_count: u32 = conn_guard
                .query_row(
                    "SELECT COUNT(*) FROM code_diagnostics WHERE tool = 'clippy'",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(0);

            Ok(ArchitectureSummary {
                total_files_indexed,
                total_entities,
                total_edges,
                modules_analyzed: total_files_indexed, // Each file is a module for this overview
                hotspot_count: 0, // Will be updated after hotspots are calculated
                dead_entity_count,
                unused_import_count,
                clippy_warning_count,
                max_loc,
            })
        }
    }

    /// Aggregate data from multiple analyses into module-level overview
    fn aggregate_module_data(
        &self,
        modules: &[ModuleInfo],
        hotspots: &[HotspotInfo],
        dead_entities: &[DeadCodeInfo],
        unused_imports: &[UnusedImportInfo],
        loc_threshold: u32,
    ) -> Result<Vec<ModuleOverview>> {
        // Get connection for diagnostics access
        let conn = self.code_graph_conn();
        let conn_guard = conn.lock().unwrap();
        // Create lookup maps for faster aggregation
        let hotspot_map: HashMap<String, f32> = hotspots
            .iter()
            .map(|h| (h.file_path.clone(), h.score))
            .collect();

        let dead_entities_by_file: HashMap<String, u32> = {
            let mut map = HashMap::new();
            for entity in dead_entities {
                *map.entry(entity.file_path.clone()).or_insert(0) += 1;
            }
            map
        };

        let unused_imports_by_file: HashMap<String, u32> = {
            let mut map = HashMap::new();
            for import in unused_imports {
                *map.entry(import.file_path.clone()).or_insert(0) += 1;
            }
            map
        };

        // Get Clippy warning counts per file
        let diagnostics = DiagnosticsManager::new(Arc::clone(self.db_manager()));
        let clippy_warnings_by_file: HashMap<String, u32> = {
            let mut map = HashMap::new();
            if let Ok(clippy_diagnostics) = diagnostics.query_diagnostics_by_tool("clippy") {
                for diagnostic in clippy_diagnostics {
                    *map.entry(diagnostic.file_path).or_insert(0) += 1;
                }
            }
            map
        };

        // Build module overviews
        let mut module_overviews = Vec::new();
        for module in modules {
            let file_path = &module.file_path;

            // Calculate hotspot score (0.0 if not a hotspot)
            let hotspot_score = hotspot_map.get(file_path).copied().unwrap_or(0.0);

            // Count dead entities in this module
            let dead_entities = dead_entities_by_file.get(file_path).copied().unwrap_or(0);

            // Count unused imports in this module
            let unused_imports = unused_imports_by_file.get(file_path).copied().unwrap_or(0);

            // Count Clippy warnings in this module
            let clippy_warning_count = clippy_warnings_by_file.get(file_path).copied().unwrap_or(0);

            // Get all diagnostics for this module to compute risk score
            let diagnostics =
                DiagnosticsManager::get_diagnostics_for_file_with_conn(&conn_guard, file_path)
                    .unwrap_or_default();

            // Build diagnostics by severity map for risk scoring
            let mut diagnostics_by_severity = std::collections::HashMap::new();
            for diagnostic in &diagnostics {
                let normalized = normalize_severity(&diagnostic.severity);
                *diagnostics_by_severity.entry(normalized).or_insert(0) += 1;
            }

            // Calculate risk score using new utilities
            let risk_inputs = FileRiskInputs {
                file_path: file_path.clone(),
                hotspot_score,
                loc: module.loc.unwrap_or(0),
                diagnostics_by_severity,
            };

            let risk_score = compute_risk_score(&risk_inputs);

            // Only include modules that meet LOC threshold or have significant metrics
            let include = module.loc.unwrap_or(0) >= loc_threshold
                || hotspot_score > 0.0
                || dead_entities > 0
                || unused_imports > 0
                || clippy_warning_count > 0
                || module.entity_count >= 5;

            if include {
                module_overviews.push(ModuleOverview {
                    file_path: file_path.clone(),
                    entity_count: module.entity_count,
                    fan_in: module.fan_in,
                    fan_out: module.fan_out,
                    loc: module.loc,
                    hotspot_score,
                    dead_entities,
                    unused_imports,
                    clippy_warning_count,
                    risk_score,
                });
            }
        }

        // Sort by hotspot score descending, then by entity count
        module_overviews.sort_by(|a, b| {
            b.hotspot_score
                .partial_cmp(&a.hotspot_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b.entity_count.cmp(&a.entity_count))
        });

        Ok(module_overviews)
    }
}
