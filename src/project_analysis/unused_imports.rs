//! Project Unused Imports Detection Tool
//! 
//! Identifies imports that are never used in their containing files.

use crate::project_analysis::{
    PAEResponse, UnusedImportInfo, ProjectAnalysisEngine,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Request parameters for project_unused_imports
#[derive(Debug, Deserialize)]
pub struct UnusedImportsRequest {
    pub file_path: Option<String>,
    pub limit: Option<u32>,
}

/// Unused imports analysis response data
#[derive(Debug, Serialize, Deserialize)]
pub struct UnusedImportsData {
    pub unused_imports: Vec<UnusedImportInfo>,
}

impl ProjectAnalysisEngine {
    /// Identify unused imports in the project
    pub async fn unused_imports(&self, request: UnusedImportsRequest) -> Result<PAEResponse<UnusedImportsData>> {
        match self.find_unused_imports(request.file_path, request.limit).await {
            Ok(data) => Ok(PAEResponse::success(data)),
            Err(e) => Ok(PAEResponse::error(e.to_string())),
        }
    }

    async fn find_unused_imports(
        &self,
        file_path: Option<String>,
        limit: Option<u32>,
    ) -> Result<UnusedImportsData> {
        let conn = self.code_graph_conn();
        let conn_guard = conn.lock().unwrap();

        let unused_imports = self.query_unused_imports(&conn_guard, file_path.as_deref(), limit)?;

        Ok(UnusedImportsData { unused_imports })
    }

    fn query_unused_imports(
        &self,
        conn: &rusqlite::Connection,
        file_path: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<UnusedImportInfo>> {
        let mut query = r#"
            SELECT 
                ce.file_path,
                ce.name,
                ce.line_start,
                ce.signature
            FROM code_entities ce
            WHERE ce.entity_type = 'import'
            AND NOT EXISTS (
                SELECT 1 FROM code_edges ce_use
                JOIN code_entities ce_used ON ce_use.dst_entity_id = ce_used.id
                WHERE ce_use.src_entity_id IN (
                    SELECT id FROM code_entities 
                    WHERE file_path = ce.file_path 
                    AND entity_type != 'import'
                )
                AND (
                    ce_used.name = ce.name 
                    OR ce_used.name LIKE '%' || ce.name || '%'
                    OR ce.signature LIKE '%' || ce.name || '%'
                )
            )
        "#.to_string();

        let mut params = Vec::new();
        let mut param_idx = 1;

        if let Some(fp) = file_path {
            query.push_str(&format!(" AND ce.file_path = ?{}", param_idx));
            params.push(fp.to_string());
            param_idx += 1;
        }

        // Exclude test files
        query.push_str(&format!(" AND ce.file_path NOT LIKE ?{}", param_idx));
        params.push("%/tests/%".to_string());
        param_idx += 1;

        query.push_str(&format!(" AND ce.file_path NOT LIKE ?{}", param_idx));
        params.push("%_test.rs".to_string());
        param_idx += 1;

        query.push_str(" ORDER BY ce.file_path, ce.line_start");

        if let Some(limit_val) = limit {
            query.push_str(&format!(" LIMIT {}", limit_val));
        }

        let mut stmt = conn.prepare(&query)?;

        let mut param_refs: Vec<&dyn rusqlite::ToSql> = Vec::new();
        for param in &params {
            param_refs.push(param);
        }

        let imports = stmt.query_map(&param_refs[..], |row| {
            let file_path: String = row.get(0)?;
            let name: String = row.get(1)?;
            let line: Option<i32> = row.get(2)?;
            let signature: Option<String> = row.get(3)?;

            // Extract module from signature if available
            let module = signature.as_ref().and_then(|sig| {
                // Try to extract module from patterns like "use std::collections::HashMap;"
                if let Some(start) = sig.find("use ") {
                    let after_use = &sig[start + 4..];
                    if let Some(end) = after_use.find(';') {
                        Some(after_use[..end].to_string())
                    } else {
                        Some(after_use.to_string())
                    }
                } else {
                    None
                }
            });

            Ok(UnusedImportInfo {
                file_path,
                import_name: name,
                line,
                module,
            })
        })?;

        let mut result = Vec::new();
        for import in imports {
            result.push(import?);
        }
        Ok(result)
    }
}