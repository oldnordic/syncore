//! Project Hotspots Analysis Tool
//!
//! Identifies code hotspots based on fan-in, fan-out, LOC, and entity count.

use crate::project_analysis::{HotspotInfo, PAEResponse, ProjectAnalysisEngine};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Request parameters for project_hotspots
#[derive(Debug, Deserialize)]
pub struct HotspotsRequest {
    pub limit: u32,
    pub min_fan_in: Option<u32>,
    pub min_fan_out: Option<u32>,
    pub min_entity_count: Option<u32>,
    pub min_loc: Option<u32>,
}

/// Hotspots analysis response data
#[derive(Debug, Serialize, Deserialize)]
pub struct HotspotsData {
    pub hotspots: Vec<HotspotInfo>,
}

impl ProjectAnalysisEngine {
    /// Identify code hotspots in the project
    pub async fn hotspots(&self, request: HotspotsRequest) -> Result<PAEResponse<HotspotsData>> {
        match self.generate_hotspots(request).await {
            Ok(data) => Ok(PAEResponse::success(data)),
            Err(e) => Ok(PAEResponse::error(e.to_string())),
        }
    }

    async fn generate_hotspots(&self, request: HotspotsRequest) -> Result<HotspotsData> {
        let conn = self.code_graph_conn();
        let conn_guard = conn.lock().unwrap();

        let hotspots = self.calculate_hotspots(
            &conn_guard,
            request.limit,
            request.min_fan_in,
            request.min_fan_out,
            request.min_entity_count,
            request.min_loc,
        )?;

        Ok(HotspotsData { hotspots })
    }

    fn calculate_hotspots(
        &self,
        conn: &rusqlite::Connection,
        limit: u32,
        min_fan_in: Option<u32>,
        min_fan_out: Option<u32>,
        min_entity_count: Option<u32>,
        min_loc: Option<u32>,
    ) -> Result<Vec<HotspotInfo>> {
        // STEP A: Exclude build artifacts from hotspot analysis
        let mut query = r#"
            SELECT
                file_path,
                COUNT(DISTINCT ce.id) as entity_count,
                COUNT(DISTINCT CASE WHEN ce_in.dst_entity_id IS NOT NULL THEN ce_in.dst_entity_id END) as fan_in,
                COUNT(DISTINCT CASE WHEN ce_out.src_entity_id IS NOT NULL THEN ce_out.src_entity_id END) as fan_out,
                MAX(ce.line_end) as max_line
            FROM code_entities ce
            LEFT JOIN code_edges ce_in ON ce.id = ce_in.dst_entity_id
            LEFT JOIN code_edges ce_out ON ce.id = ce_out.src_entity_id
            WHERE file_path NOT LIKE '%/target/%'
              AND file_path NOT LIKE '%/node_modules/%'
              AND file_path NOT LIKE '%/.git/%'
              AND file_path NOT LIKE '%/__pycache__/%'
              AND file_path NOT LIKE '%/vendor/%'
              AND file_path NOT LIKE '%/.venv/%'
              AND file_path NOT LIKE '%/dist/%'
              AND file_path NOT LIKE '%/build/%'
        "#.to_string();

        let mut where_clauses = Vec::new();
        let mut params = Vec::new();
        let mut param_idx = 1;

        if let Some(min_fi) = min_fan_in {
            where_clauses.push(format!("fan_in >= ?{}", param_idx));
            params.push(min_fi as i64);
            param_idx += 1;
        }

        if let Some(min_fo) = min_fan_out {
            where_clauses.push(format!("fan_out >= ?{}", param_idx));
            params.push(min_fo as i64);
            param_idx += 1;
        }

        if let Some(min_ec) = min_entity_count {
            where_clauses.push(format!("entity_count >= ?{}", param_idx));
            params.push(min_ec as i64);
            param_idx += 1;
        }

        if let Some(min_lines) = min_loc {
            where_clauses.push(format!("max_line >= ?{}", param_idx));
            params.push(min_lines as i64);
            param_idx += 1;
        }

        query.push_str(" GROUP BY file_path");

        if !where_clauses.is_empty() {
            query.push_str(" HAVING ");
            query.push_str(&where_clauses.join(" AND "));
        }

        // Calculate hotspot score and order by it
        query.push_str(&format!(
            " ORDER BY (fan_in * 0.4 + fan_out * 0.3 + entity_count * 0.2 + COALESCE(max_line, 0) * 0.1) DESC LIMIT {}",
            limit
        ));

        let mut stmt = conn.prepare(&query)?;

        let mut param_refs: Vec<&dyn rusqlite::ToSql> = Vec::new();
        for param in &params {
            param_refs.push(param);
        }

        let hotspots = stmt.query_map(&param_refs[..], |row| {
            let file_path: String = row.get(0)?;
            let entity_count: i64 = row.get(1)?;
            let fan_in: i64 = row.get(2)?;
            let fan_out: i64 = row.get(3)?;
            let max_line: Option<i64> = row.get(4)?;

            // Calculate hotspot score (weighted combination)
            let loc = max_line.unwrap_or(0) as f32;
            let score =
                fan_in as f32 * 0.4 + fan_out as f32 * 0.3 + entity_count as f32 * 0.2 + loc * 0.1;

            Ok(HotspotInfo {
                file_path,
                fan_in: fan_in as u32,
                fan_out: fan_out as u32,
                entity_count: entity_count as u32,
                loc: if max_line.is_some() && max_line.unwrap() > 0 {
                    Some(max_line.unwrap() as u32)
                } else {
                    None
                },
                score,
            })
        })?;

        let mut result = Vec::new();
        for hotspot in hotspots {
            result.push(hotspot?);
        }
        Ok(result)
    }
}
