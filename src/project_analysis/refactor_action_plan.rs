//! Refactor Action Plan Generator
//!
//! Meta-tool that aggregates PAE metrics into prioritized actionable refactor plans.

use crate::project_analysis::{
    diagnostics::DiagnosticsManager, CycleInfo, DeadCodeInfo, HotspotInfo, ModuleInfo, PAEResponse,
    ProjectAnalysisEngine, UnusedImportInfo,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Request parameters for project_refactor_action_plan
#[derive(Debug, Deserialize)]
pub struct RefactorActionPlanRequest {}

/// High-risk hotspot entry
#[derive(Debug, Serialize, Deserialize)]
pub struct HotspotEntry {
    pub file_path: String,
    pub score: f32,
    pub fan_in: u32,
    pub fan_out: u32,
    pub entity_count: u32,
    pub loc: Option<u32>,
}

/// Entity identifier for dead code cleanup
#[derive(Debug, Serialize, Deserialize)]
pub struct EntityId {
    pub id: i64,
    pub name: String,
    pub entity_type: String,
    pub file_path: String,
    pub line_start: i32,
}

/// Unused import entry
#[derive(Debug, Serialize, Deserialize)]
pub struct UnusedImportEntry {
    pub file_path: String,
    pub import_name: String,
    pub line: Option<i32>,
    pub module: Option<String>,
}

/// Module name for cycle break candidates
#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleName {
    pub file_path: String,
}

/// Module refactor operation
#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleOp {
    pub file_path: String,
    pub operation: String, // "split" or "merge_candidate"
    pub loc: Option<u32>,
    pub entity_count: u32,
    pub reason: String,
}

/// Refactor action plan summary statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct RefactorActionSummary {
    pub total_hotspots: u32,
    pub total_dead_code: u32,
    pub total_unused_imports: u32,
    pub total_cycle_breaks: u32,
    pub total_module_ops: u32,
    pub clippy_warning_count: u32,
}

/// Refactor action plan response data
#[derive(Debug, Serialize, Deserialize)]
pub struct RefactorActionPlanData {
    pub high_risk_hotspots: Vec<HotspotEntry>,
    pub dead_code_cleanup: Vec<EntityId>,
    pub unused_imports: Vec<UnusedImportEntry>,
    pub cycle_break_candidates: Vec<ModuleName>,
    pub module_refactor_ops: Vec<ModuleOp>,
    pub summary: RefactorActionSummary,
}

impl ProjectAnalysisEngine {
    /// Generate comprehensive refactor action plan
    pub async fn refactor_action_plan(
        &self,
        _request: RefactorActionPlanRequest,
    ) -> Result<PAEResponse<RefactorActionPlanData>> {
        match self.generate_refactor_action_plan().await {
            Ok(data) => Ok(PAEResponse::success(data)),
            Err(e) => Ok(PAEResponse::error(e.to_string())),
        }
    }

    async fn generate_refactor_action_plan(&self) -> Result<RefactorActionPlanData> {
        // 1. Get hotspots
        let hotspots_request = crate::project_analysis::hotspots::HotspotsRequest {
            limit: 50,
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
            exclude_public: Some(false),
            limit: Some(100),
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
                limit: Some(10),
            };
        let unused_imports_response = self.unused_imports(unused_imports_request).await?;
        let unused_imports = match unused_imports_response.data {
            Some(data) => data.unused_imports,
            None => Vec::new(),
        };

        // 4. Get cycles
        let cycles_request = crate::project_analysis::cycles::CyclesRequest {
            max_cycles: 20,
            max_depth: 10,
        };
        let cycles_response = self.cycles(cycles_request).await?;
        let cycles = match cycles_response.data {
            Some(data) => data.cycles,
            None => Vec::new(),
        };

        // 5. Get module map
        let module_map_request = crate::project_analysis::deps::ModuleMapRequest {
            root: None,
            max_modules: Some(100),
        };
        let module_map_response = self.module_map(module_map_request).await?;
        let modules = match module_map_response.data {
            Some(data) => data.modules,
            None => Vec::new(),
        };

        // 6. Process results
        let high_risk_hotspots = self.process_high_risk_hotspots(&hotspots);
        let dead_code_cleanup = self.process_dead_code(&dead_entities);
        let unused_imports = self.process_unused_imports(&unused_imports);
        let cycle_break_candidates = self.process_cycles(&cycles);
        let module_refactor_ops = self.process_module_refactors(&modules);

        // Get total Clippy warning count
        let clippy_warning_count = {
            let diagnostics = DiagnosticsManager::new(Arc::clone(self.db_manager()));
            diagnostics
                .count_diagnostics_for_tool("clippy")
                .unwrap_or(0) as u32
        };

        let summary = RefactorActionSummary {
            total_hotspots: high_risk_hotspots.len() as u32,
            total_dead_code: dead_code_cleanup.len() as u32,
            total_unused_imports: unused_imports.len() as u32,
            total_cycle_breaks: cycle_break_candidates.len() as u32,
            total_module_ops: module_refactor_ops.len() as u32,
            clippy_warning_count,
        };

        Ok(RefactorActionPlanData {
            high_risk_hotspots,
            dead_code_cleanup,
            unused_imports,
            cycle_break_candidates,
            module_refactor_ops,
            summary,
        })
    }

    /// Process high-risk hotspots (score >= 100)
    fn process_high_risk_hotspots(&self, hotspots: &[HotspotInfo]) -> Vec<HotspotEntry> {
        hotspots
            .iter()
            .filter(|h| h.score >= 100.0)
            .map(|h| HotspotEntry {
                file_path: h.file_path.clone(),
                score: h.score,
                fan_in: h.fan_in,
                fan_out: h.fan_out,
                entity_count: h.entity_count,
                loc: h.loc,
            })
            .collect()
    }

    /// Process dead code for cleanup
    fn process_dead_code(&self, dead_entities: &[DeadCodeInfo]) -> Vec<EntityId> {
        dead_entities
            .iter()
            .map(|e| EntityId {
                id: e.id,
                name: e.name.clone(),
                entity_type: e.entity_type.clone(),
                file_path: e.file_path.clone(),
                line_start: e.line_start,
            })
            .collect()
    }

    /// Process unused imports (top 10)
    fn process_unused_imports(
        &self,
        unused_imports: &[UnusedImportInfo],
    ) -> Vec<UnusedImportEntry> {
        unused_imports
            .iter()
            .take(10)
            .map(|u| UnusedImportEntry {
                file_path: u.file_path.clone(),
                import_name: u.import_name.clone(),
                line: u.line,
                module: u.module.clone(),
            })
            .collect()
    }

    /// Process cycles for break candidates
    fn process_cycles(&self, cycles: &[CycleInfo]) -> Vec<ModuleName> {
        let mut modules = Vec::new();
        for cycle in cycles {
            for file_path in &cycle.files {
                modules.push(ModuleName {
                    file_path: file_path.clone(),
                });
            }
        }
        modules.sort_by(|a, b| a.file_path.cmp(&b.file_path));
        modules.dedup_by(|a, b| a.file_path == b.file_path);
        modules
    }

    /// Process module refactor operations
    fn process_module_refactors(&self, modules: &[ModuleInfo]) -> Vec<ModuleOp> {
        modules
            .iter()
            .filter_map(|m| {
                if let Some(loc) = m.loc {
                    if loc > 500 {
                        Some(ModuleOp {
                            file_path: m.file_path.clone(),
                            operation: "split".to_string(),
                            loc: Some(loc),
                            entity_count: m.entity_count,
                            reason: format!("Module exceeds 500 LOC (actual: {})", loc),
                        })
                    } else if loc < 100 && m.entity_count < 5 {
                        Some(ModuleOp {
                            file_path: m.file_path.clone(),
                            operation: "merge_candidate".to_string(),
                            loc: Some(loc),
                            entity_count: m.entity_count,
                            reason: format!(
                                "Small module with {} LOC and {} entities",
                                loc, m.entity_count
                            ),
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }
}
