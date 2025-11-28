//! Project Complexity Dashboard Tool
//!
//! Aggregates multiple analysis metrics into a comprehensive complexity dashboard.

use crate::project_analysis::{
    compute_risk_score, diagnostics::DiagnosticsManager, normalize_severity, DeadCodeInfo,
    FileRiskInputs, HotspotInfo, PAEResponse, ProjectAnalysisEngine, UnusedImportInfo,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Request parameters for project_complexity_dashboard
#[derive(Debug, Deserialize, Default)]
pub struct ComplexityDashboardRequest {
    pub limit_hotspots: Option<usize>,
    pub loc_threshold: Option<u32>,
}

/// File complexity information for dashboard
#[derive(Debug, Serialize, Deserialize)]
pub struct FileComplexityInfo {
    pub file_path: String,
    pub entity_count: u32,
    pub dead_entities: u32,
    pub unused_imports: u32,
    pub clippy_warning_count: u32,
    pub fan_in: u32,
    pub fan_out: u32,
    pub loc: Option<u32>,
    pub risk_score: f32,
}

/// Distribution statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct DistributionStats {
    pub min: f32,
    pub max: f32,
    pub mean: f32,
}

/// Dashboard summary statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct ComplexitySummary {
    pub total_files: u32,
    pub total_entities: u32,
    pub total_edges: u32,
    pub total_dead_entities: u32,
    pub total_unused_imports: u32,
    pub total_clippy_warnings: u32,
    pub max_loc: u32,
    pub hotspot_count: u32,
}

/// Dashboard statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct ComplexityStats {
    pub loc_distribution: DistributionStats,
    pub fan_in_distribution: DistributionStats,
    pub fan_out_distribution: DistributionStats,
    pub dead_entity_ratio: f32,
    pub unused_import_ratio: f32,
}

/// Processing notes
#[derive(Debug, Serialize, Deserialize)]
pub struct ComplexityNotes {
    pub loc_threshold: u32,
    pub limit_hotspots: usize,
}

/// Complexity dashboard response data
#[derive(Debug, Serialize, Deserialize)]
pub struct ComplexityDashboardData {
    pub summary: ComplexitySummary,
    pub hotspots: Vec<HotspotInfo>,
    pub files: Vec<FileComplexityInfo>,
    pub stats: ComplexityStats,
    pub notes: ComplexityNotes,
}

impl ProjectAnalysisEngine {
    /// Generate comprehensive complexity dashboard
    pub async fn complexity_dashboard(
        &self,
        request: ComplexityDashboardRequest,
    ) -> Result<PAEResponse<ComplexityDashboardData>> {
        match self.generate_complexity_dashboard(request).await {
            Ok(data) => Ok(PAEResponse::success(data)),
            Err(e) => Ok(PAEResponse::error(e.to_string())),
        }
    }

    async fn generate_complexity_dashboard(
        &self,
        request: ComplexityDashboardRequest,
    ) -> Result<ComplexityDashboardData> {
        let limit_hotspots = request.limit_hotspots.unwrap_or(10);
        let loc_threshold = request.loc_threshold.unwrap_or(300);

        // 1. Get hotspots
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
            None => Vec::new(),
        };

        // 2. Get dead code
        let dead_code_request = crate::project_analysis::dead_code::DeadCodeRequest {
            exclude_public: Some(true),
            limit: None,
        };
        let dead_code_response = self.dead_code(dead_code_request).await?;
        let dead_entities = match dead_code_response.data {
            Some(data) => data.dead_entities,
            None => Vec::new(),
        };

        // 3. Get unused imports
        let unused_imports_request =
            crate::project_analysis::unused_imports::UnusedImportsRequest {
                file_path: None,
                limit: None,
            };
        let unused_imports_response = self.unused_imports(unused_imports_request).await?;
        let unused_imports = match unused_imports_response.data {
            Some(data) => data.unused_imports,
            None => Vec::new(),
        };

        // 4. Get basic project metrics and build file info in separate database access
        let (files, total_files, total_entities, total_edges) = {
            let conn = self.code_graph_conn();
            let conn_guard = conn.lock().unwrap();

            let (total_files, total_entities, total_edges) = self.get_basic_metrics(&conn_guard)?;
            let files = self.build_file_complexity_info(
                &conn_guard,
                &dead_entities,
                &unused_imports,
                &hotspots,
                loc_threshold,
            )?;

            (files, total_files, total_entities, total_edges)
        };

        // 5. Calculate summary
        let total_clippy_warnings = {
            let diagnostics = DiagnosticsManager::new(Arc::clone(self.db_manager()));
            diagnostics
                .count_diagnostics_for_tool("clippy")
                .unwrap_or(0) as u32
        };

        let summary = ComplexitySummary {
            total_files,
            total_entities,
            total_edges,
            total_dead_entities: dead_entities.len() as u32,
            total_unused_imports: unused_imports.len() as u32,
            total_clippy_warnings,
            max_loc: files.iter().filter_map(|f| f.loc).max().unwrap_or(0),
            hotspot_count: hotspots.len() as u32,
        };

        // 6. Calculate statistics
        let stats = self.calculate_complexity_stats(
            &files,
            total_entities,
            dead_entities.len(),
            unused_imports.len(),
        );

        // 7. Create notes
        let notes = ComplexityNotes {
            loc_threshold,
            limit_hotspots,
        };

        Ok(ComplexityDashboardData {
            summary,
            hotspots,
            files,
            stats,
            notes,
        })
    }

    /// Get basic project metrics
    fn get_basic_metrics(&self, conn: &rusqlite::Connection) -> Result<(u32, u32, u32)> {
        let total_files: u32 = conn.query_row(
            "SELECT COUNT(DISTINCT file_path) FROM code_entities",
            [],
            |row| row.get(0),
        )?;

        let total_entities: u32 =
            conn.query_row("SELECT COUNT(*) FROM code_entities", [], |row| row.get(0))?;

        let total_edges: u32 =
            conn.query_row("SELECT COUNT(*) FROM code_edges", [], |row| row.get(0))?;

        Ok((total_files, total_entities, total_edges))
    }

    /// Build file complexity information
    fn build_file_complexity_info(
        &self,
        conn: &rusqlite::Connection,
        dead_entities: &[DeadCodeInfo],
        unused_imports: &[UnusedImportInfo],
        hotspots: &[HotspotInfo],
        loc_threshold: u32,
    ) -> Result<Vec<FileComplexityInfo>> {
        // Get all files with their entity counts and LOC
        let mut stmt = conn.prepare(
            r#"
            SELECT 
                file_path,
                COUNT(*) as entity_count,
                MAX(line_end) as loc
            FROM code_entities
            GROUP BY file_path
            HAVING entity_count > 0 OR MAX(line_end) > 0
            ORDER BY entity_count DESC
            "#,
        )?;

        let file_rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, u32>(1)?,
                row.get::<_, Option<i64>>(2)?.map(|x| x as u32),
            ))
        })?;

        let mut files = Vec::new();
        for row_result in file_rows {
            let (file_path, entity_count, loc) = row_result?;

            // Skip files below LOC threshold
            if let Some(loc_val) = loc {
                if loc_val < loc_threshold {
                    continue;
                }
            }

            // Calculate fan-in and fan-out using existing metrics
            let fan_in = crate::project_analysis::metrics::MetricsCalculator::calculate_fan_in(
                conn, &file_path,
            )?;
            let fan_out = crate::project_analysis::metrics::MetricsCalculator::calculate_fan_out(
                conn, &file_path,
            )?;

            // Count dead entities in this file
            let dead_entities = dead_entities
                .iter()
                .filter(|entity| entity.file_path == file_path)
                .count() as u32;

            // Count unused imports in this file
            let unused_imports = unused_imports
                .iter()
                .filter(|import| import.file_path == file_path)
                .count() as u32;

            // Count Clippy warnings in this file (use connection-based method to avoid deadlock)
            let clippy_warning_count = DiagnosticsManager::count_diagnostics_for_file_with_conn(
                conn, &file_path, "clippy",
            )
            .unwrap_or(0) as u32;

            // Get all diagnostics for this file to compute risk score
            let diagnostics =
                DiagnosticsManager::get_diagnostics_for_file_with_conn(conn, &file_path)
                    .unwrap_or_default();

            // Build diagnostics by severity map for risk scoring
            let mut diagnostics_by_severity = std::collections::HashMap::new();
            for diagnostic in &diagnostics {
                let normalized = normalize_severity(&diagnostic.severity);
                *diagnostics_by_severity.entry(normalized).or_insert(0) += 1;
            }

            // Calculate risk score using new utilities
            let hotspot_score = hotspots
                .iter()
                .find(|h| h.file_path == file_path)
                .map(|h| h.score)
                .unwrap_or(0.0);

            let risk_inputs = FileRiskInputs {
                file_path: file_path.clone(),
                hotspot_score,
                loc: loc.unwrap_or(0),
                diagnostics_by_severity,
            };

            let risk_score = compute_risk_score(&risk_inputs);

            files.push(FileComplexityInfo {
                file_path,
                entity_count,
                dead_entities,
                unused_imports,
                clippy_warning_count,
                fan_in,
                fan_out,
                loc,
                risk_score,
            });
        }

        Ok(files)
    }

    /// Calculate complexity statistics
    fn calculate_complexity_stats(
        &self,
        files: &[FileComplexityInfo],
        total_entities: u32,
        dead_count: usize,
        unused_count: usize,
    ) -> ComplexityStats {
        if files.is_empty() {
            return ComplexityStats {
                loc_distribution: DistributionStats {
                    min: 0.0,
                    max: 0.0,
                    mean: 0.0,
                },
                fan_in_distribution: DistributionStats {
                    min: 0.0,
                    max: 0.0,
                    mean: 0.0,
                },
                fan_out_distribution: DistributionStats {
                    min: 0.0,
                    max: 0.0,
                    mean: 0.0,
                },
                dead_entity_ratio: 0.0,
                unused_import_ratio: 0.0,
            };
        }

        let loc_values: Vec<f32> = files
            .iter()
            .filter_map(|f| f.loc.map(|x| x as f32))
            .collect();
        let fan_in_values: Vec<f32> = files.iter().map(|f| f.fan_in as f32).collect();
        let fan_out_values: Vec<f32> = files.iter().map(|f| f.fan_out as f32).collect();

        ComplexityStats {
            loc_distribution: self.calculate_distribution(&loc_values),
            fan_in_distribution: self.calculate_distribution(&fan_in_values),
            fan_out_distribution: self.calculate_distribution(&fan_out_values),
            dead_entity_ratio: if total_entities > 0 {
                dead_count as f32 / total_entities as f32
            } else {
                0.0
            },
            unused_import_ratio: if !files.is_empty() {
                unused_count as f32 / files.len() as f32
            } else {
                0.0
            },
        }
    }

    /// Calculate distribution statistics
    fn calculate_distribution(&self, values: &[f32]) -> DistributionStats {
        if values.is_empty() {
            return DistributionStats {
                min: 0.0,
                max: 0.0,
                mean: 0.0,
            };
        }
        let min = values.iter().copied().fold(f32::INFINITY, f32::min);
        let max = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let mean = values.iter().sum::<f32>() / values.len() as f32;
        DistributionStats { min, max, mean }
    }
}
