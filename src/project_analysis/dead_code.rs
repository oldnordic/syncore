//! Project Dead Code Detection Tool
//! 
//! Identifies entities that appear to be unused (no incoming relationships).

use crate::project_analysis::{
    PAEResponse, DeadCodeInfo, ProjectAnalysisEngine,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Request parameters for project_dead_code
#[derive(Debug, Deserialize)]
pub struct DeadCodeRequest {
    pub exclude_public: Option<bool>,
    pub limit: Option<u32>,
}

/// Dead code analysis response data
#[derive(Debug, Serialize, Deserialize)]
pub struct DeadCodeData {
    pub dead_entities: Vec<DeadCodeInfo>,
}

impl ProjectAnalysisEngine {
    /// Identify potentially dead code entities
    pub async fn dead_code(&self, request: DeadCodeRequest) -> Result<PAEResponse<DeadCodeData>> {
        match self.find_dead_code(request.exclude_public.unwrap_or(true), request.limit).await {
            Ok(data) => Ok(PAEResponse::success(data)),
            Err(e) => Ok(PAEResponse::error(e.to_string())),
        }
    }

    async fn find_dead_code(
        &self,
        exclude_public: bool,
        limit: Option<u32>,
    ) -> Result<DeadCodeData> {
        let conn = self.code_graph_conn();
        let conn_guard = conn.lock().unwrap();

        let dead_entities = self.query_dead_entities(&conn_guard, exclude_public, limit)?;

        Ok(DeadCodeData { dead_entities })
    }

    fn query_dead_entities(
        &self,
        conn: &rusqlite::Connection,
        exclude_public: bool,
        limit: Option<u32>,
    ) -> Result<Vec<DeadCodeInfo>> {
        let mut query = r#"
            SELECT 
                ce.id,
                ce.name,
                ce.entity_type,
                ce.file_path,
                ce.line_start,
                ce.signature
            FROM code_entities ce
            LEFT JOIN code_edges ce_in ON ce.id = ce_in.dst_entity_id
            WHERE ce_in.dst_entity_id IS NULL
            AND ce.entity_type NOT IN ('import', 'module')
        "#.to_string();

        let mut params = Vec::new();
        let mut param_idx = 1;

        // Exclude test files
        query.push_str(&format!(" AND ce.file_path NOT LIKE ?{}", param_idx));
        params.push("%/tests/%".to_string());
        param_idx += 1;

        query.push_str(&format!(" AND ce.file_path NOT LIKE ?{}", param_idx));
        params.push("%_test.rs".to_string());
        param_idx += 1;

        // Exclude public entities if requested
        if exclude_public {
            query.push_str(&format!(" AND (ce.signature NOT LIKE 'pub %' AND ce.signature NOT LIKE 'pub(crate)%')"));
        }

        query.push_str(" ORDER BY ce.file_path, ce.line_start");

        if let Some(limit_val) = limit {
            query.push_str(&format!(" LIMIT {}", limit_val));
        }

        let mut stmt = conn.prepare(&query)?;

        let mut param_refs: Vec<&dyn rusqlite::ToSql> = Vec::new();
        for param in &params {
            param_refs.push(param);
        }

        let entities = stmt.query_map(&param_refs[..], |row| {
            let signature: Option<String> = row.get(5)?;
            
            // Try to extract visibility from signature
            let visibility = if let Some(sig) = &signature {
                if sig.starts_with("pub") {
                    Some("public".to_string())
                } else {
                    Some("private".to_string())
                }
            } else {
                None
            };

            Ok(DeadCodeInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                entity_type: row.get(2)?,
                file_path: row.get(3)?,
                visibility,
                line_start: row.get(4)?,
            })
        })?;

        let mut result = Vec::new();
        for entity in entities {
            result.push(entity?);
        }
        Ok(result)
    }
}