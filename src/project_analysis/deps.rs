//! Project Module Map Tool
//!
//! Provides module-level dependency mapping and analysis.

use crate::project_analysis::{ModuleEdge, ModuleInfo, PAEResponse, ProjectAnalysisEngine};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Request parameters for project_module_map
#[derive(Debug, Deserialize)]
pub struct ModuleMapRequest {
    pub root: Option<String>,
    pub max_modules: Option<u32>,
}

/// Module map response data
#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleMapData {
    pub modules: Vec<ModuleInfo>,
    pub edges: Vec<ModuleEdge>,
}

impl ProjectAnalysisEngine {
    /// Generate a module-level map of the project
    pub async fn module_map(
        &self,
        request: ModuleMapRequest,
    ) -> Result<PAEResponse<ModuleMapData>> {
        match self
            .generate_module_map(request.root, request.max_modules)
            .await
        {
            Ok(data) => Ok(PAEResponse::success(data)),
            Err(e) => Ok(PAEResponse::error(e.to_string())),
        }
    }

    async fn generate_module_map(
        &self,
        root: Option<String>,
        max_modules: Option<u32>,
    ) -> Result<ModuleMapData> {
        let conn = self.code_graph_conn();
        let conn_guard = conn.lock().unwrap();

        // Get all files with entity counts
        let modules = self.get_module_info(&conn_guard, root.as_deref(), max_modules)?;

        // Get relationships between modules
        let edges = self.get_module_edges(&conn_guard, &modules)?;

        Ok(ModuleMapData { modules, edges })
    }

    fn get_module_info(
        &self,
        conn: &rusqlite::Connection,
        root: Option<&str>,
        max_modules: Option<u32>,
    ) -> Result<Vec<ModuleInfo>> {
        let mut query = r#"
            SELECT 
                file_path,
                COUNT(*) as entity_count,
                COUNT(DISTINCT CASE WHEN ce_in.src_entity_id IS NOT NULL THEN ce_in.src_entity_id END) as fan_in,
                COUNT(DISTINCT CASE WHEN ce_out.dst_entity_id IS NOT NULL THEN ce_out.dst_entity_id END) as fan_out
            FROM code_entities ce
            LEFT JOIN code_edges ce_in ON ce.id = ce_in.dst_entity_id
            LEFT JOIN code_edges ce_out ON ce.id = ce_out.src_entity_id
        "#.to_string();

        let mut params = Vec::new();
        let param_idx = 1;

        if let Some(root_path) = root {
            query.push_str(&format!(" WHERE ce.file_path LIKE ?{}", param_idx));
            params.push(format!("{}%", root_path));
        }

        query.push_str(" GROUP BY ce.file_path ORDER BY entity_count DESC");

        if let Some(limit) = max_modules {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        let mut stmt = conn.prepare(&query)?;

        let mut param_refs: Vec<&dyn rusqlite::ToSql> = Vec::new();
        for param in &params {
            param_refs.push(param);
        }

        let modules = stmt.query_map(&param_refs[..], |row| {
            let file_path: String = row.get(0)?;
            let entity_count: u32 = row.get(1)?;
            let fan_in: u32 = row.get(2)?;
            let fan_out: u32 = row.get(3)?;

            // Generate ID from file path (use relative path if possible)
            let id = if let Some(stripped) = file_path.strip_prefix("./") {
                stripped.to_string()
            } else {
                file_path.clone()
            };

            Ok(ModuleInfo {
                id,
                file_path,
                entity_count,
                fan_in,
                fan_out,
                loc: None, // Could be calculated in future
            })
        })?;

        let mut result = Vec::new();
        for module in modules {
            result.push(module?);
        }
        Ok(result)
    }

    fn get_module_edges(
        &self,
        conn: &rusqlite::Connection,
        modules: &[ModuleInfo],
    ) -> Result<Vec<ModuleEdge>> {
        if modules.is_empty() {
            return Ok(Vec::new());
        }

        // Create a set of file paths for fast lookup
        let _module_files: std::collections::HashSet<&str> =
            modules.iter().map(|m| m.file_path.as_str()).collect();

        // Build placeholders for IN clause
        let placeholders = (0..modules.len())
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");

        let query = format!(
            r#"
            SELECT DISTINCT
                e1.file_path as from_file,
                e2.file_path as to_file,
                ce.edge_type
            FROM code_edges ce
            JOIN code_entities e1 ON ce.src_entity_id = e1.id
            JOIN code_entities e2 ON ce.dst_entity_id = e2.id
            WHERE e1.file_path IN ({})
            AND e2.file_path IN ({})
            AND e1.file_path != e2.file_path
            ORDER BY e1.file_path, e2.file_path
            "#,
            placeholders, placeholders
        );

        let mut stmt = conn.prepare(&query)?;

        // Create parameter references (each file path appears twice)
        let mut params: Vec<&dyn rusqlite::ToSql> = Vec::new();
        for module in modules {
            params.push(&module.file_path);
        }
        for module in modules {
            params.push(&module.file_path);
        }

        let edges = stmt.query_map(&params[..], |row| {
            Ok(ModuleEdge {
                from_file: row.get(0)?,
                to_file: row.get(1)?,
                relationship_type: row.get(2)?,
            })
        })?;

        let mut result = Vec::new();
        for edge in edges {
            result.push(edge?);
        }
        Ok(result)
    }
}
