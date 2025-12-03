//! Code Smell & Anti-Pattern Detection
//!
//! Implements deterministic, LLM-free detection of code smells and anti-patterns
//! using existing PAE infrastructure and code graph data.

use crate::project_analysis::{
    diagnostics_severity::NormalizedSeverity,
    risk_score::{compute_risk_score, FileRiskInputs},
    ProjectAnalysisEngine,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// File-level code smell information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCodeSmell {
    pub file_path: String,
    pub smell_type: String,
    pub loc: Option<u32>,
    pub fan_in: Option<u32>,
    pub fan_out: Option<u32>,
    pub entity_count: Option<u32>,
    pub dead_entity_count: Option<u32>,
    pub unused_import_count: Option<u32>,
    pub risk_score: Option<f32>,
    pub notes: String,
}

/// Entity-level code smell information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityCodeSmell {
    pub file_path: String,
    pub name: String,
    pub entity_type: String,
    pub smell_type: String,
    pub line_start: i32,
    pub line_end: i32,
    pub function_loc: Option<u32>,
    pub parameter_count: Option<u32>,
    pub notes: String,
}

impl ProjectAnalysisEngine {
    /// Detect file-level code smells
    pub fn detect_file_smells(&self, limit: usize) -> Result<Vec<FileCodeSmell>> {
        let conn = self.code_graph_conn();
        let conn_guard = conn.lock().unwrap();

        let mut smells = Vec::new();

        // Detect GOD_FILE smells
        smells.extend(self.detect_god_files(&conn_guard, limit)?);

        // Detect HOTSPOT_GOD_FILE smells
        smells.extend(self.detect_hotspot_god_files(&conn_guard, limit)?);

        // Detect DEAD_CODE_CLUSTER smells
        smells.extend(self.detect_dead_code_clusters(&conn_guard, limit)?);

        // Detect IMPORT_JUNGLE smells
        smells.extend(self.detect_import_jungles(&conn_guard, limit)?);

        // Sort by file path and limit results
        smells.sort_by(|a, b| a.file_path.cmp(&b.file_path));
        smells.truncate(limit);

        Ok(smells)
    }

    /// Detect entity-level code smells
    pub fn detect_entity_smells(&self, limit: usize) -> Result<Vec<EntityCodeSmell>> {
        let conn = self.code_graph_conn();
        let conn_guard = conn.lock().unwrap();

        let mut smells = Vec::new();

        // Detect LONG_FUNCTION smells
        smells.extend(self.detect_long_functions(&conn_guard, limit)?);

        // Detect LONG_PARAMETER_LIST smells
        smells.extend(self.detect_long_parameter_lists(&conn_guard, limit)?);

        // Sort by file path and line number, then limit
        smells.sort_by(|a, b| {
            a.file_path.cmp(&b.file_path).then_with(|| a.line_start.cmp(&b.line_start))
        });
        smells.truncate(limit);

        Ok(smells)
    }

    /// Detect GOD_FILE: loc >= 800, fan_in >= 40, entity_count >= 40
    fn detect_god_files(
        &self,
        conn: &rusqlite::Connection,
        limit: usize,
    ) -> Result<Vec<FileCodeSmell>> {
        let query = "
            WITH file_metrics AS (
                SELECT 
                    ce.file_path,
                    MAX(ce.line_end) - MIN(ce.line_start) + 1 as loc,
                    COUNT(DISTINCT ce.id) as entity_count,
                    COUNT(DISTINCT ce_in.src_entity_id) as fan_in
                FROM code_entities ce
                LEFT JOIN code_edges ce_in ON ce.id = ce_in.dst_entity_id
                WHERE ce.entity_type NOT IN ('import')
                GROUP BY ce.file_path
                HAVING loc >= 800 AND entity_count >= 40 AND fan_in >= 40
            )
            SELECT file_path, loc, entity_count, fan_in
            FROM file_metrics
            ORDER BY loc DESC
            LIMIT ?
        ";

        let mut stmt = conn.prepare(query)?;
        let rows = stmt.query_map([limit as i64], |row| {
            Ok(FileCodeSmell {
                file_path: row.get(0)?,
                smell_type: "GOD_FILE".to_string(),
                loc: Some(row.get(1)?),
                fan_in: Some(row.get(3)?),
                fan_out: None,
                entity_count: Some(row.get(2)?),
                dead_entity_count: None,
                unused_import_count: None,
                risk_score: None,
                notes: format!(
                    "File with {} LOC, {} entities, {} fan-in",
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?
                ),
            })
        })?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// Detect HOTSPOT_GOD_FILE: risk_score >= 30.0, loc >= 1000
    fn detect_hotspot_god_files(
        &self,
        conn: &rusqlite::Connection,
        limit: usize,
    ) -> Result<Vec<FileCodeSmell>> {
        // First get file LOC and diagnostic counts
        let file_metrics_query = r#"
            SELECT 
                ce.file_path,
                MAX(ce.line_end) - MIN(ce.line_start) + 1 as loc,
                COUNT(DISTINCT ce.id) as entity_count
            FROM code_entities ce
            WHERE ce.entity_type NOT IN ('import')
            GROUP BY ce.file_path
            HAVING loc >= 1000
        "#;

        let mut stmt = conn.prepare(file_metrics_query)?;
        let rows = stmt.query_map([], |row| {
            let file_path: String = row.get(0)?;
            let loc: u32 = row.get(1)?;
            let entity_count: u32 = row.get(2)?;

            Ok((file_path, loc, entity_count))
        })?;

        let mut smells = Vec::new();
        for row_result in rows {
            let (file_path, loc, entity_count) = row_result?;

            // Get diagnostics for risk score calculation
            let diagnostics = self.get_file_diagnostics(conn, &file_path)?;

            // Compute risk score (hotspot_score is 0.0 since we don't have hotspot data)
            let risk_inputs = FileRiskInputs {
                file_path: file_path.clone(),
                hotspot_score: 0.0,
                loc,
                diagnostics_by_severity: diagnostics,
            };

            let risk_score = compute_risk_score(&risk_inputs);

            if risk_score >= 30.0 {
                smells.push(FileCodeSmell {
                    file_path,
                    smell_type: "HOTSPOT_GOD_FILE".to_string(),
                    loc: Some(loc),
                    fan_in: None,
                    fan_out: None,
                    entity_count: Some(entity_count),
                    dead_entity_count: None,
                    unused_import_count: None,
                    risk_score: Some(risk_score),
                    notes: format!("Large file with {} LOC and risk score {:.1}", loc, risk_score),
                });
            }
        }

        // Sort by risk score descending and limit
        smells.sort_by(|a, b| b.risk_score.partial_cmp(&a.risk_score).unwrap());
        smells.truncate(limit);

        Ok(smells)
    }

    /// Detect DEAD_CODE_CLUSTER: dead_entity_count >= 10, dead_ratio >= 0.20
    fn detect_dead_code_clusters(
        &self,
        conn: &rusqlite::Connection,
        limit: usize,
    ) -> Result<Vec<FileCodeSmell>> {
        let query = r#"
            WITH file_dead_stats AS (
                SELECT 
                    ce.file_path,
                    COUNT(DISTINCT ce.id) as total_entities,
                    COUNT(DISTINCT dead.id) as dead_entities
                FROM code_entities ce
                LEFT JOIN (
                    SELECT ce_dead.id, ce_dead.file_path
                    FROM code_entities ce_dead
                    LEFT JOIN code_edges ce_in ON ce_dead.id = ce_in.dst_entity_id
                    WHERE ce_in.dst_entity_id IS NULL
                    AND ce_dead.entity_type NOT IN ('import', 'module')
                    AND ce_dead.file_path NOT LIKE '%/tests/%'
                    AND ce_dead.file_path NOT LIKE '%_test.rs'
                ) dead ON ce.id = dead.id AND ce.file_path = dead.file_path
                WHERE ce.entity_type NOT IN ('import')
                AND ce.file_path NOT LIKE '%/tests/%'
                AND ce.file_path NOT LIKE '%_test.rs'
                GROUP BY ce.file_path
                HAVING dead_entities >= 10 AND (CAST(dead_entities AS FLOAT) / total_entities) >= 0.20
            )
            SELECT file_path, total_entities, dead_entities
            FROM file_dead_stats
            ORDER BY dead_entities DESC
            LIMIT ?
        "#;

        let mut stmt = conn.prepare(query)?;
        let rows = stmt.query_map([limit as i64], |row| {
            let file_path: String = row.get(0)?;
            let total_entities: i64 = row.get(1)?;
            let dead_entities: i64 = row.get(2)?;
            let dead_ratio = dead_entities as f32 / total_entities as f32;

            Ok(FileCodeSmell {
                file_path,
                smell_type: "DEAD_CODE_CLUSTER".to_string(),
                loc: None,
                fan_in: None,
                fan_out: None,
                entity_count: Some(total_entities as u32),
                dead_entity_count: Some(dead_entities as u32),
                unused_import_count: None,
                risk_score: None,
                notes: format!(
                    "File with {}/{} dead entities ({:.1}%)",
                    dead_entities,
                    total_entities,
                    dead_ratio * 100.0
                ),
            })
        })?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// Detect IMPORT_JUNGLE: unused_import_count >= 10
    fn detect_import_jungles(
        &self,
        conn: &rusqlite::Connection,
        limit: usize,
    ) -> Result<Vec<FileCodeSmell>> {
        let query = r#"
            WITH file_unused_imports AS (
                SELECT 
                    ce.file_path,
                    COUNT(DISTINCT ce.id) as unused_import_count
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
                AND ce.file_path NOT LIKE '%/tests/%'
                AND ce.file_path NOT LIKE '%_test.rs'
                GROUP BY ce.file_path
                HAVING unused_import_count >= 10
            )
            SELECT file_path, unused_import_count
            FROM file_unused_imports
            ORDER BY unused_import_count DESC
            LIMIT ?
        "#;

        let mut stmt = conn.prepare(query)?;
        let rows = stmt.query_map([limit as i64], |row| {
            Ok(FileCodeSmell {
                file_path: row.get(0)?,
                smell_type: "IMPORT_JUNGLE".to_string(),
                loc: None,
                fan_in: None,
                fan_out: None,
                entity_count: None,
                dead_entity_count: None,
                unused_import_count: Some(row.get(1)?),
                risk_score: None,
                notes: format!("File with {} unused imports", row.get::<_, i64>(1)?),
            })
        })?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// Detect LONG_FUNCTION: function_loc >= 40
    fn detect_long_functions(
        &self,
        conn: &rusqlite::Connection,
        limit: usize,
    ) -> Result<Vec<EntityCodeSmell>> {
        let query = r#"
            SELECT 
                ce.file_path,
                ce.name,
                ce.entity_type,
                ce.line_start,
                ce.line_end,
                (ce.line_end - ce.line_start + 1) as function_loc
            FROM code_entities ce
            WHERE ce.entity_type = 'function'
            AND (ce.line_end - ce.line_start + 1) >= 40
            AND ce.file_path NOT LIKE '%/tests/%'
            AND ce.file_path NOT LIKE '%_test.rs'
            ORDER BY function_loc DESC
            LIMIT ?
        "#;

        let mut stmt = conn.prepare(query)?;
        let rows = stmt.query_map([limit as i64], |row| {
            let function_loc: i64 = row.get(5)?;

            Ok(EntityCodeSmell {
                file_path: row.get(0)?,
                name: row.get(1)?,
                entity_type: row.get(2)?,
                smell_type: "LONG_FUNCTION".to_string(),
                line_start: row.get(3)?,
                line_end: row.get(4)?,
                function_loc: Some(function_loc as u32),
                parameter_count: None,
                notes: format!("Function with {} LOC", function_loc),
            })
        })?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// Detect LONG_PARAMETER_LIST: parameter_count >= 5
    fn detect_long_parameter_lists(
        &self,
        conn: &rusqlite::Connection,
        limit: usize,
    ) -> Result<Vec<EntityCodeSmell>> {
        let query = r#"
            SELECT 
                ce.file_path,
                ce.name,
                ce.entity_type,
                ce.line_start,
                ce.line_end,
                ce.signature
            FROM code_entities ce
            WHERE ce.entity_type = 'function'
            AND ce.signature IS NOT NULL
            AND ce.file_path NOT LIKE '%/tests/%'
            AND ce.file_path NOT LIKE '%_test.rs'
            ORDER BY ce.file_path, ce.line_start
            LIMIT ?
        "#;

        let mut stmt = conn.prepare(query)?;
        let rows = stmt.query_map([limit as i64 * 10], |row| {
            // Get more rows to filter
            let file_path: String = row.get(0)?;
            let name: String = row.get(1)?;
            let entity_type: String = row.get(2)?;
            let line_start: i32 = row.get(3)?;
            let line_end: i32 = row.get(4)?;
            let signature: String = row.get(5)?;

            // Simple parameter count heuristic
            let param_count = if let Some(paren_start) = signature.find('(') {
                if let Some(paren_end) = signature[paren_start..].find(')') {
                    let params_str = &signature[paren_start + 1..paren_start + paren_end];
                    if params_str.trim().is_empty() {
                        0
                    } else {
                        params_str.split(',').count()
                    }
                } else {
                    0
                }
            } else {
                0
            };

            Ok((file_path, name, entity_type, line_start, line_end, param_count))
        })?;

        let mut result = Vec::new();
        for row_result in rows {
            let (file_path, name, entity_type, line_start, line_end, param_count) = row_result?;

            if param_count >= 5 {
                result.push(EntityCodeSmell {
                    file_path,
                    name,
                    entity_type,
                    smell_type: "LONG_PARAMETER_LIST".to_string(),
                    line_start,
                    line_end,
                    function_loc: None,
                    parameter_count: Some(param_count as u32),
                    notes: format!("Function with {} parameters", param_count),
                });
            }
        }

        // Sort and limit
        result.sort_by(|a, b| {
            a.file_path.cmp(&b.file_path).then_with(|| a.line_start.cmp(&b.line_start))
        });
        result.truncate(limit);

        Ok(result)
    }

    /// Helper to get diagnostics for a file
    fn get_file_diagnostics(
        &self,
        conn: &rusqlite::Connection,
        file_path: &str,
    ) -> Result<HashMap<NormalizedSeverity, u32>> {
        let query = r#"
            SELECT severity, COUNT(*) as count
            FROM code_diagnostics
            WHERE file_path = ?
            GROUP BY severity
        "#;

        let mut stmt = conn.prepare(query)?;
        let rows = stmt.query_map([file_path], |row| {
            let severity: String = row.get(0)?;
            let count: i64 = row.get(1)?;

            let normalized = match severity.as_str() {
                "error" => NormalizedSeverity::Error,
                "warning" => NormalizedSeverity::Warning,
                "info" | "note" => NormalizedSeverity::Info,
                _ => NormalizedSeverity::Unknown,
            };

            Ok((normalized, count as u32))
        })?;

        let mut diagnostics = HashMap::new();
        for row_result in rows {
            let (severity, count) = row_result?;
            diagnostics.insert(severity, count);
        }

        Ok(diagnostics)
    }
}
