//! Project File Report Tool
//!
//! Provides detailed analysis of a single source file including entities,
//! relationships, imports, and complexity metrics.

use crate::project_analysis::{
    EntityInfo, FileMetrics, PAEResponse, ProjectAnalysisEngine, RelationshipInfo,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Request parameters for project_file_report
#[derive(Debug, Deserialize)]
pub struct FileReportRequest {
    pub file_path: String,
}

/// File report response data
#[derive(Debug, Serialize, Deserialize)]
pub struct FileReportData {
    pub file_path: String,
    pub loc: Option<u32>,
    pub entities: Vec<EntityInfo>,
    pub calls_out: Vec<RelationshipInfo>,
    pub calls_in: Vec<RelationshipInfo>,
    pub imports: Vec<ImportInfo>,
    pub uses: Vec<UseInfo>,
    pub metrics: FileMetrics,
}

/// Import information
#[derive(Debug, Serialize, Deserialize)]
pub struct ImportInfo {
    pub module: String,
    pub file_path: String,
    pub resolved_target: Option<String>,
    pub line: Option<i32>,
}

/// Use information
#[derive(Debug, Serialize, Deserialize)]
pub struct UseInfo {
    pub from_entity: String,
    pub to_entity: String,
    pub relation_type: String,
}

impl ProjectAnalysisEngine {
    /// Generate a comprehensive report for a single file
    pub async fn file_report(
        &self,
        request: FileReportRequest,
    ) -> Result<PAEResponse<FileReportData>> {
        match self.generate_file_report(&request.file_path).await {
            Ok(data) => Ok(PAEResponse::success(data)),
            Err(e) => Ok(PAEResponse::error(e.to_string())),
        }
    }

    async fn generate_file_report(&self, file_path: &str) -> Result<FileReportData> {
        let conn = self.code_graph_conn();
        let conn_guard = conn.lock().unwrap();

        // Get entities in the file
        let entities = self.get_file_entities(&conn_guard, file_path)?;

        // Get outgoing relationships (calls, imports, etc.)
        let calls_out = self.get_outgoing_relationships(&conn_guard, file_path)?;

        // Get incoming relationships
        let calls_in = self.get_incoming_relationships(&conn_guard, file_path)?;

        // Get imports
        let imports = self.get_file_imports(&conn_guard, file_path)?;

        // Get uses
        let uses = self.get_file_uses(&conn_guard, file_path)?;

        // Calculate metrics
        let metrics = self.calculate_file_metrics(&entities, &calls_in, &calls_out);

        // Estimate LOC if not available
        let loc = self.estimate_file_loc(&entities);

        Ok(FileReportData {
            file_path: file_path.to_string(),
            loc,
            entities,
            calls_out,
            calls_in,
            imports,
            uses,
            metrics,
        })
    }
}
