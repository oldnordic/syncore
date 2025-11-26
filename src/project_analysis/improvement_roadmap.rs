//! Project Improvement Roadmap Meta-Tool
//!
//! Combines multiple PAE analysis results into a comprehensive, prioritized improvement plan.
//! This is Meta-Tool #3 from the defined list.

use crate::project_analysis::{
    compute_risk_score, cycles::CyclesData, dead_code::DeadCodeData,
    diagnostics::DiagnosticsManager,
    unused_imports::UnusedImportsData, FileRiskInputs, HotspotInfo, PAEResponse,
    ProjectAnalysisEngine, RefactorKind, RefactorSuggestion,
};

/// Helper function to calculate risk score for a file
fn calculate_file_risk_score(
    file_path: &str,
    hotspot_score: f32,
    loc: u32,
    diagnostics_by_severity: &std::collections::HashMap<
        crate::project_analysis::NormalizedSeverity,
        u32,
    >,
) -> f32 {
    // Calculate risk score using new utilities
    let risk_inputs = FileRiskInputs {
        file_path: file_path.to_string(),
        hotspot_score,
        loc,
        diagnostics_by_severity: diagnostics_by_severity.clone(),
    };

    compute_risk_score(&risk_inputs)
}
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Request parameters for project_improvement_roadmap
#[derive(Debug, Deserialize)]
pub struct ImprovementRoadmapRequest {
    /// Maximum number of items per category
    pub limit_per_category: Option<u32>,
    /// Include only high-priority items
    pub high_priority_only: Option<bool>,
    /// Minimum LOC threshold for hotspots to include
    pub hotspot_loc_threshold: Option<u32>,
    /// Project label for scoping
    pub project_label: Option<String>,
}

/// Priority level for improvement items
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Critical, // Security risks, blocking issues
    High,     // Performance, maintainability
    Medium,   // Code quality, cleanup
    Low,      // Nice to have improvements
}

/// Improvement item with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementItem {
    /// Unique identifier for this item
    pub id: String,
    /// Type of improvement
    pub improvement_type: ImprovementType,
    /// Priority level
    pub priority: Priority,
    /// File path where improvement is needed
    pub file_path: String,
    /// Line number (if applicable)
    pub line_number: Option<u32>,
    /// Description of the improvement
    pub description: String,
    /// Estimated effort (1-5 scale)
    pub effort: u32,
    /// Impact on codebase (1-5 scale)
    pub impact: u32,
    /// Unified risk score for the file
    pub risk_score: f32,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Types of improvements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImprovementType {
    RemoveDeadCode,
    RemoveUnusedImport,
    RefactorComplex,
    BreakCycle,
    ReduceComplexity,
    ExtractModule,
    ImproveNaming,
    AddDocumentation,
    SplitFile,
    ExtractFacade,
    ReduceCycle,
    PruneDeadCode,
    SimplifyDependency,
}

/// Comprehensive improvement roadmap
#[derive(Debug, Serialize, Deserialize)]
pub struct ImprovementRoadmapData {
    /// Summary statistics
    pub summary: RoadmapSummary,
    /// Prioritized improvement items
    pub improvements: Vec<ImprovementItem>,
    /// Breakdown by category
    pub by_category: CategoryBreakdown,
    /// Effort vs impact matrix
    pub effort_impact_matrix: EffortImpactMatrix,
}

/// Summary statistics for the roadmap
#[derive(Debug, Serialize, Deserialize)]
pub struct RoadmapSummary {
    /// Total improvements identified
    pub total_improvements: u32,
    /// Count by priority
    pub by_priority: HashMap<String, u32>,
    /// Count by improvement type
    pub by_type: HashMap<String, u32>,
    /// Estimated total effort (person-hours)
    pub estimated_total_effort: f32,
    /// Files requiring attention
    pub files_affected: u32,
    /// Total Clippy warnings
    pub clippy_warning_count: u32,
}

/// Breakdown by improvement category
#[derive(Debug, Serialize, Deserialize)]
pub struct CategoryBreakdown {
    /// Dead code removal items
    pub dead_code: Vec<ImprovementItem>,
    /// Unused import cleanup
    pub unused_imports: Vec<ImprovementItem>,
    /// Refactoring suggestions
    pub refactor_suggestions: Vec<ImprovementItem>,
    /// Circular dependency fixes
    pub cycle_fixes: Vec<ImprovementItem>,
    /// Complexity reductions
    pub complexity_reductions: Vec<ImprovementItem>,
}

/// Effort vs impact analysis
#[derive(Debug, Serialize, Deserialize)]
pub struct EffortImpactMatrix {
    /// Quick wins (low effort, high impact)
    pub quick_wins: Vec<ImprovementItem>,
    /// Major projects (high effort, high impact)
    pub major_projects: Vec<ImprovementItem>,
    /// Fill-ins (low effort, low impact)
    pub fill_ins: Vec<ImprovementItem>,
    /// Reconsider (high effort, low impact)
    pub reconsider: Vec<ImprovementItem>,
}

impl ProjectAnalysisEngine {
    /// Generate comprehensive improvement roadmap
    pub async fn improvement_roadmap(
        &self,
        request: ImprovementRoadmapRequest,
    ) -> Result<PAEResponse<ImprovementRoadmapData>> {
        match self.generate_improvement_roadmap(request).await {
            Ok(data) => Ok(PAEResponse::success(data)),
            Err(e) => Ok(PAEResponse::error(e.to_string())),
        }
    }

    async fn generate_improvement_roadmap(
        &self,
        request: ImprovementRoadmapRequest,
    ) -> Result<ImprovementRoadmapData> {
        let limit = request.limit_per_category.unwrap_or(20);
        let high_priority_only = request.high_priority_only.unwrap_or(false);
        let hotspot_loc_threshold = request.hotspot_loc_threshold.unwrap_or(100);

        // Collect data from all analysis tools
        let dead_code_data = self.get_dead_code_analysis(limit).await?;
        let unused_imports_data = self.get_unused_imports_analysis(limit).await?;
        let refactor_suggestions = self.get_refactor_suggestions(limit).await?;
        let cycles_data = self.get_cycles_analysis(limit).await?;
        let hotspots_data = self
            .get_hotspots_analysis(hotspot_loc_threshold, limit)
            .await?;

        // Convert to improvement items
        let mut improvements = Vec::new();

        // Process dead code
        for (i, dead_entity) in dead_code_data.dead_entities.iter().enumerate() {
            if high_priority_only && dead_entity.entity_type != "function" {
                continue;
            }

            // Calculate risk score for this file
            let hotspot_score = hotspots_data
                .iter()
                .find(|h| h.file_path == dead_entity.file_path)
                .map(|h| h.score)
                .unwrap_or(0.0);

            let loc = hotspots_data
                .iter()
                .find(|h| h.file_path == dead_entity.file_path)
                .and_then(|h| h.loc)
                .unwrap_or(0);

            // Create empty diagnostics map for now (would need actual diagnostics data)
            let diagnostics_by_severity = std::collections::HashMap::new();

            let risk_score = calculate_file_risk_score(
                &dead_entity.file_path,
                hotspot_score,
                loc,
                &diagnostics_by_severity,
            );

            improvements.push(ImprovementItem {
                id: format!("dead_code_{}", i),
                improvement_type: ImprovementType::RemoveDeadCode,
                priority: if dead_entity.entity_type == "function" {
                    Priority::High
                } else {
                    Priority::Medium
                },
                file_path: dead_entity.file_path.clone(),
                line_number: Some(dead_entity.line_start as u32),
                description: format!(
                    "Remove unused {} '{}'",
                    dead_entity.entity_type, dead_entity.name
                ),
                effort: 1, // Usually easy to remove
                impact: 2, // Cleanup benefit
                risk_score,
                metadata: HashMap::from([
                    (
                        "entity_type".to_string(),
                        serde_json::Value::String(dead_entity.entity_type.clone()),
                    ),
                    (
                        "visibility".to_string(),
                        serde_json::Value::String(
                            dead_entity.visibility.clone().unwrap_or_default(),
                        ),
                    ),
                ]),
            });
        }

        // Process unused imports
        for (i, unused_import) in unused_imports_data.unused_imports.iter().enumerate() {
            // Calculate risk score for this file
            let hotspot_score = hotspots_data
                .iter()
                .find(|h| h.file_path == unused_import.file_path)
                .map(|h| h.score)
                .unwrap_or(0.0);

            let loc = hotspots_data
                .iter()
                .find(|h| h.file_path == unused_import.file_path)
                .and_then(|h| h.loc)
                .unwrap_or(0);

            // Create empty diagnostics map for now (would need actual diagnostics data)
            let diagnostics_by_severity = std::collections::HashMap::new();

            let risk_score = calculate_file_risk_score(
                &unused_import.file_path,
                hotspot_score,
                loc,
                &diagnostics_by_severity,
            );

            improvements.push(ImprovementItem {
                id: format!("unused_import_{}", i),
                improvement_type: ImprovementType::RemoveUnusedImport,
                priority: Priority::Low, // Usually low impact
                file_path: unused_import.file_path.clone(),
                line_number: unused_import.line.map(|x| x as u32),
                description: format!("Remove unused import '{}'", unused_import.import_name),
                effort: 1, // Very easy
                impact: 1, // Minor cleanup
                risk_score,
                metadata: HashMap::from([
                    (
                        "import_name".to_string(),
                        serde_json::Value::String(unused_import.import_name.clone()),
                    ),
                    (
                        "module".to_string(),
                        serde_json::Value::String(unused_import.module.clone().unwrap_or_default()),
                    ),
                ]),
            });
        }

        // Process refactor suggestions
        for (i, suggestion) in refactor_suggestions.iter().enumerate() {
            let priority = match suggestion.kind {
                RefactorKind::SplitFile => Priority::High,
                RefactorKind::ExtractFacade => Priority::Medium,
                RefactorKind::ReduceCycle => Priority::High,
                RefactorKind::PruneDeadCode => Priority::Medium,
                RefactorKind::SimplifyDependency => Priority::Medium,
            };

            // Calculate risk score for this file
            let hotspot_score = hotspots_data
                .iter()
                .find(|h| h.file_path == suggestion.file_path.clone().unwrap_or_default())
                .map(|h| h.score)
                .unwrap_or(0.0);

            let loc = hotspots_data
                .iter()
                .find(|h| h.file_path == suggestion.file_path.clone().unwrap_or_default())
                .and_then(|h| h.loc)
                .unwrap_or(0);

            // Create empty diagnostics map for now (would need actual diagnostics data)
            let diagnostics_by_severity = std::collections::HashMap::new();

            let risk_score = calculate_file_risk_score(
                &suggestion.file_path.clone().unwrap_or_default(),
                hotspot_score,
                loc,
                &diagnostics_by_severity,
            );

            improvements.push(ImprovementItem {
                id: format!("refactor_{}", i),
                improvement_type: match suggestion.kind {
                    RefactorKind::SplitFile => ImprovementType::SplitFile,
                    RefactorKind::ExtractFacade => ImprovementType::ExtractFacade,
                    RefactorKind::ReduceCycle => ImprovementType::ReduceCycle,
                    RefactorKind::PruneDeadCode => ImprovementType::PruneDeadCode,
                    RefactorKind::SimplifyDependency => ImprovementType::SimplifyDependency,
                },
                priority,
                file_path: suggestion.file_path.clone().unwrap_or_default(),
                line_number: None,
                description: suggestion.description.clone(),
                effort: 3,
                impact: 3,
                risk_score,
                metadata: HashMap::from([
                    (
                        "refactor_kind".to_string(),
                        serde_json::Value::String(format!("{:?}", suggestion.kind)),
                    ),
                    (
                        "metrics".to_string(),
                        serde_json::Value::Object(suggestion.metrics.clone().into_iter().collect()),
                    ),
                ]),
            });
        }

        // Process cycles
        for (i, cycle) in cycles_data.cycles.iter().enumerate() {
            // Calculate risk score for the first file in the cycle
            let first_file = cycle
                .files
                .first()
                .unwrap_or(&"unknown".to_string())
                .clone();

            let hotspot_score = hotspots_data
                .iter()
                .find(|h| h.file_path == first_file)
                .map(|h| h.score)
                .unwrap_or(0.0);

            let loc = hotspots_data
                .iter()
                .find(|h| h.file_path == first_file)
                .and_then(|h| h.loc)
                .unwrap_or(0);

            // Create empty diagnostics map for now (would need actual diagnostics data)
            let diagnostics_by_severity = std::collections::HashMap::new();

            let risk_score = calculate_file_risk_score(
                &first_file,
                hotspot_score,
                loc,
                &diagnostics_by_severity,
            );

            improvements.push(ImprovementItem {
                id: format!("cycle_{}", i),
                improvement_type: ImprovementType::BreakCycle,
                priority: Priority::Critical, // Circular dependencies are serious
                file_path: first_file,
                line_number: None,
                description: format!("Break circular dependency in {} files", cycle.files.len()),
                effort: 4, // Usually complex
                impact: 5, // High impact on maintainability
                risk_score,
                metadata: HashMap::from([
                    (
                        "cycle_length".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(cycle.files.len())),
                    ),
                    (
                        "files".to_string(),
                        serde_json::Value::Array(
                            cycle
                                .files
                                .iter()
                                .map(|f| serde_json::Value::String(f.clone()))
                                .collect(),
                        ),
                    ),
                ]),
            });
        }

        // Process hotspots for complexity reduction
        for (i, hotspot) in hotspots_data.iter().enumerate() {
            if let Some(loc) = hotspot.loc {
                if loc < hotspot_loc_threshold {
                    continue;
                }
            } else {
                continue;
            }

            // Calculate risk score for this hotspot
            // Create empty diagnostics map for now (would need actual diagnostics data)
            let diagnostics_by_severity = std::collections::HashMap::new();

            let risk_score = calculate_file_risk_score(
                &hotspot.file_path,
                hotspot.score,
                hotspot.loc.unwrap_or(0),
                &diagnostics_by_severity,
            );

            improvements.push(ImprovementItem {
                id: format!("hotspot_{}", i),
                improvement_type: ImprovementType::ReduceComplexity,
                priority: if hotspot.fan_out > 20 {
                    Priority::High
                } else {
                    Priority::Medium
                },
                file_path: hotspot.file_path.clone(),
                line_number: None,
                description: format!(
                    "Reduce complexity in {} (LOC: {}, fan-in: {}, fan-out: {})",
                    hotspot.file_path,
                    hotspot.loc.unwrap_or(0),
                    hotspot.fan_in,
                    hotspot.fan_out
                ),
                effort: 3,
                impact: 4,
                risk_score,
                metadata: HashMap::from([
                    (
                        "loc".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(
                            hotspot.loc.unwrap_or(0),
                        )),
                    ),
                    (
                        "fan_in".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(hotspot.fan_in)),
                    ),
                    (
                        "fan_out".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(hotspot.fan_out)),
                    ),
                    (
                        "entity_count".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(hotspot.entity_count)),
                    ),
                ]),
            });
        }

        // Sort by priority (descending), then by impact/effort ratio
        improvements.sort_by(|a, b| {
            let ratio_a = a.impact as f32 / a.effort as f32;
            let ratio_b = b.impact as f32 / b.effort as f32;
            b.priority.cmp(&a.priority).then_with(|| {
                ratio_b
                    .partial_cmp(&ratio_a)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });

        // Create category breakdown
        let by_category = CategoryBreakdown {
            dead_code: improvements
                .iter()
                .filter(|i| matches!(i.improvement_type, ImprovementType::RemoveDeadCode))
                .cloned()
                .collect(),
            unused_imports: improvements
                .iter()
                .filter(|i| matches!(i.improvement_type, ImprovementType::RemoveUnusedImport))
                .cloned()
                .collect(),
            refactor_suggestions: improvements
                .iter()
                .filter(|i| {
                    matches!(
                        i.improvement_type,
                        ImprovementType::RefactorComplex
                            | ImprovementType::ExtractModule
                            | ImprovementType::ImproveNaming
                            | ImprovementType::AddDocumentation
                    )
                })
                .cloned()
                .collect(),
            cycle_fixes: improvements
                .iter()
                .filter(|i| matches!(i.improvement_type, ImprovementType::BreakCycle))
                .cloned()
                .collect(),
            complexity_reductions: improvements
                .iter()
                .filter(|i| matches!(i.improvement_type, ImprovementType::ReduceComplexity))
                .cloned()
                .collect(),
        };

        // Create effort-impact matrix
        let effort_impact_matrix = EffortImpactMatrix {
            quick_wins: improvements
                .iter()
                .filter(|i| i.effort <= 2 && i.impact >= 4)
                .cloned()
                .collect(),
            major_projects: improvements
                .iter()
                .filter(|i| i.effort >= 4 && i.impact >= 4)
                .cloned()
                .collect(),
            fill_ins: improvements
                .iter()
                .filter(|i| i.effort <= 2 && i.impact <= 2)
                .cloned()
                .collect(),
            reconsider: improvements
                .iter()
                .filter(|i| i.effort >= 4 && i.impact <= 2)
                .cloned()
                .collect(),
        };

        // Generate summary
        let mut by_priority = HashMap::new();
        let mut by_type = HashMap::new();
        let mut total_effort = 0.0;
        let mut affected_files = std::collections::HashSet::new();

        for improvement in &improvements {
            *by_priority
                .entry(format!("{:?}", improvement.priority))
                .or_insert(0) += 1;
            *by_type
                .entry(format!("{:?}", improvement.improvement_type))
                .or_insert(0) += 1;
            total_effort += improvement.effort as f32 * 0.5; // Assume 0.5 hours per effort point
            affected_files.insert(&improvement.file_path);
        }

        // Get total Clippy warning count
        let clippy_warning_count = {
            let diagnostics = DiagnosticsManager::new(Arc::clone(self.db_manager()));
            diagnostics
                .count_diagnostics_for_tool("clippy")
                .unwrap_or(0) as u32
        };

        let summary = RoadmapSummary {
            total_improvements: improvements.len() as u32,
            by_priority,
            by_type,
            estimated_total_effort: total_effort,
            files_affected: affected_files.len() as u32,
            clippy_warning_count,
        };

        Ok(ImprovementRoadmapData {
            summary,
            improvements,
            by_category,
            effort_impact_matrix,
        })
    }

    // Helper methods to get data from other analysis tools
    async fn get_dead_code_analysis(&self, limit: u32) -> Result<DeadCodeData> {
        let request = crate::project_analysis::dead_code::DeadCodeRequest {
            exclude_public: Some(true),
            limit: Some(limit),
        };

        match self.dead_code(request).await {
            Ok(response) if response.ok && response.data.is_some() => Ok(response.data.unwrap()),
            _ => Ok(DeadCodeData {
                dead_entities: Vec::new(),
            }),
        }
    }

    async fn get_unused_imports_analysis(&self, limit: u32) -> Result<UnusedImportsData> {
        let request = crate::project_analysis::unused_imports::UnusedImportsRequest {
            file_path: None,
            limit: Some(limit),
        };

        match self.unused_imports(request).await {
            Ok(response) if response.ok && response.data.is_some() => Ok(response.data.unwrap()),
            _ => Ok(UnusedImportsData {
                unused_imports: Vec::new(),
            }),
        }
    }

    async fn get_refactor_suggestions(&self, limit: u32) -> Result<Vec<RefactorSuggestion>> {
        let request = crate::project_analysis::refactor::RefactorSuggestionsRequest {
            limit,
            loc_threshold: None,
            entity_threshold: None,
            fan_in_threshold: None,
            fan_out_threshold: None,
        };

        match self.refactor_suggestions(request).await {
            Ok(response) if response.ok && response.data.is_some() => {
                Ok(response.data.unwrap().suggestions)
            }
            _ => Ok(Vec::new()),
        }
    }

    async fn get_cycles_analysis(&self, limit: u32) -> Result<CyclesData> {
        let request = crate::project_analysis::cycles::CyclesRequest {
            max_cycles: limit,
            max_depth: 10,
        };

        match self.cycles(request).await {
            Ok(response) if response.ok && response.data.is_some() => Ok(response.data.unwrap()),
            _ => Ok(CyclesData { cycles: Vec::new() }),
        }
    }

    async fn get_hotspots_analysis(
        &self,
        loc_threshold: u32,
        limit: u32,
    ) -> Result<Vec<HotspotInfo>> {
        let request = crate::project_analysis::hotspots::HotspotsRequest {
            limit,
            min_fan_in: None,
            min_fan_out: None,
            min_entity_count: None,
            min_loc: Some(loc_threshold),
        };

        match self.hotspots(request).await {
            Ok(response) if response.ok && response.data.is_some() => {
                Ok(response.data.unwrap().hotspots)
            }
            _ => Ok(Vec::new()),
        }
    }
}
