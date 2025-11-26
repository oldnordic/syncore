//! Project File Report Summary
//!
//! Summary calculation and formatting for file analysis.

use crate::project_analysis::{EntityInfo, FileMetrics, ProjectAnalysisEngine, RelationshipInfo};

impl ProjectAnalysisEngine {
    /// Calculate file-level metrics
    pub fn calculate_file_metrics(
        &self,
        entities: &[EntityInfo],
        calls_in: &[RelationshipInfo],
        calls_out: &[RelationshipInfo],
    ) -> FileMetrics {
        FileMetrics {
            fan_in: calls_in.len() as u32,
            fan_out: calls_out.len() as u32,
            entity_count: entities.len() as u32,
        }
    }
}
