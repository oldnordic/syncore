//! Project Refactor Suggestions Tool
//! 
//! Provides heuristic-based refactor hints based on graph and metrics analysis.

use crate::project_analysis::{
    PAEResponse, RefactorSuggestion, RefactorKind, ProjectAnalysisEngine,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Request parameters for project_refactor_suggestions
#[derive(Debug, Deserialize)]
pub struct RefactorSuggestionsRequest {
    pub limit: u32,
    pub loc_threshold: Option<u32>,
    pub entity_threshold: Option<u32>,
    pub fan_in_threshold: Option<u32>,
    pub fan_out_threshold: Option<u32>,
}

/// Refactor suggestions response data
#[derive(Debug, Serialize, Deserialize)]
pub struct RefactorSuggestionsData {
    pub suggestions: Vec<RefactorSuggestion>,
}

// Thresholds for refactor suggestions
const DEFAULT_LOC_THRESHOLD: u32 = 500;
const DEFAULT_ENTITY_THRESHOLD: u32 = 20;
const DEFAULT_FAN_IN_THRESHOLD: u32 = 10;
const DEFAULT_FAN_OUT_THRESHOLD: u32 = 15;

impl ProjectAnalysisEngine {
    /// Generate refactor suggestions based on project analysis
    pub async fn refactor_suggestions(&self, request: RefactorSuggestionsRequest) -> Result<PAEResponse<RefactorSuggestionsData>> {
        match self.generate_refactor_suggestions(request).await {
            Ok(data) => Ok(PAEResponse::success(data)),
            Err(e) => Ok(PAEResponse::error(e.to_string())),
        }
    }

    async fn generate_refactor_suggestions(&self, request: RefactorSuggestionsRequest) -> Result<RefactorSuggestionsData> {
        let conn = self.code_graph_conn();
        let conn_guard = conn.lock().unwrap();

        let mut suggestions = Vec::new();

        // Get thresholds
        let loc_threshold = request.loc_threshold.unwrap_or(DEFAULT_LOC_THRESHOLD);
        let entity_threshold = request.entity_threshold.unwrap_or(DEFAULT_ENTITY_THRESHOLD);
        let fan_in_threshold = request.fan_in_threshold.unwrap_or(DEFAULT_FAN_IN_THRESHOLD);
        let fan_out_threshold = request.fan_out_threshold.unwrap_or(DEFAULT_FAN_OUT_THRESHOLD);

        // 1. Suggest file splits for large files
        suggestions.extend(self.suggest_file_splits(&conn_guard, loc_threshold, entity_threshold)?);

        // 2. Suggest facade extraction for high fan-in files
        suggestions.extend(self.suggest_facade_extraction(&conn_guard, fan_in_threshold)?);

        // 3. Suggest cycle reduction
        suggestions.extend(self.suggest_cycle_reduction(&conn_guard)?);

        // 4. Suggest dead code pruning
        suggestions.extend(self.suggest_dead_code_pruning(&conn_guard)?);

        // 5. Suggest dependency simplification
        suggestions.extend(self.suggest_dependency_simplification(&conn_guard, fan_out_threshold)?);

        // Sort by some priority metric and limit
        suggestions.sort_by(|a, b| {
            // Simple priority: files with more issues first
            let a_score = self.calculate_suggestion_priority(&a);
            let b_score = self.calculate_suggestion_priority(&b);
            b_score.partial_cmp(&a_score).unwrap_or(std::cmp::Ordering::Equal)
        });

        suggestions.truncate(request.limit as usize);

        Ok(RefactorSuggestionsData { suggestions })
    }
}