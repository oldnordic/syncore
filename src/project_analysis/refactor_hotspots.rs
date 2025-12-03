//! Refactor Hotspots Detection
//!
//! SQL queries for detecting refactor opportunities in the codebase.

use crate::project_analysis::{ProjectAnalysisEngine, RefactorKind, RefactorSuggestion};
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;

impl ProjectAnalysisEngine {
    /// Suggest file splits for large files
    pub fn suggest_file_splits(
        &self,
        conn: &rusqlite::Connection,
        loc_threshold: u32,
        entity_threshold: u32,
    ) -> Result<Vec<RefactorSuggestion>> {
        let mut stmt = conn.prepare(
            "
            SELECT 
                file_path,
                COUNT(*) as entity_count,
                MAX(line_end) as max_line
            FROM code_entities
            WHERE file_path NOT LIKE '%/tests/%'
            AND file_path NOT LIKE '%_test.rs'
            GROUP BY file_path
            HAVING entity_count >= ?1 OR max_line >= ?2
            ORDER BY entity_count DESC, max_line DESC
            ",
        )?;

        let suggestions =
            stmt.query_map([entity_threshold as i64, loc_threshold as i64], |row| {
                let file_path: String = row.get(0)?;
                let entity_count: i64 = row.get(1)?;
                let max_line: i64 = row.get(2)?;

                let mut metrics = HashMap::new();
                metrics.insert(
                    "entity_count".to_string(),
                    Value::Number(serde_json::Number::from(entity_count)),
                );
                metrics
                    .insert("loc".to_string(), Value::Number(serde_json::Number::from(max_line)));

                Ok(RefactorSuggestion {
                    kind: RefactorKind::SplitFile,
                    description: format!(
                        "Split {} ({} entities, ~{} LOC) into smaller, focused modules",
                        file_path, entity_count, max_line
                    ),
                    file_path: Some(file_path),
                    related_files: None,
                    metrics,
                })
            })?;

        let mut result = Vec::new();
        for suggestion in suggestions {
            result.push(suggestion?);
        }
        Ok(result)
    }

    /// Suggest facade extraction for high fan-in files
    pub fn suggest_facade_extraction(
        &self,
        conn: &rusqlite::Connection,
        fan_in_threshold: u32,
    ) -> Result<Vec<RefactorSuggestion>> {
        let mut stmt = conn.prepare(
            "
            SELECT 
                e1.file_path,
                COUNT(DISTINCT ce_in.dst_entity_id) as fan_in,
                COUNT(DISTINCT ce_out.src_entity_id) as fan_out
            FROM code_entities e1
            LEFT JOIN code_edges ce_in ON e1.id = ce_in.dst_entity_id
            LEFT JOIN code_edges ce_out ON e1.id = ce_out.src_entity_id
            WHERE e1.file_path NOT LIKE '%/tests/%'
            AND e1.file_path NOT LIKE '%_test.rs'
            GROUP BY e1.file_path
            HAVING fan_in >= ?1 AND fan_in > fan_out * 2
            ORDER BY fan_in DESC
            ",
        )?;

        let suggestions = stmt.query_map([fan_in_threshold as i64], |row| {
            let file_path: String = row.get(0)?;
            let fan_in: i64 = row.get(1)?;
            let fan_out: i64 = row.get(2)?;

            let mut metrics = HashMap::new();
            metrics.insert("fan_in".to_string(), Value::Number(serde_json::Number::from(fan_in)));
            metrics.insert("fan_out".to_string(), Value::Number(serde_json::Number::from(fan_out)));

            Ok(RefactorSuggestion {
                kind: RefactorKind::ExtractFacade,
                description: format!(
                    "Extract facade for {} (high fan-in: {}, low fan-out: {})",
                    file_path, fan_in, fan_out
                ),
                file_path: Some(file_path),
                related_files: None,
                metrics,
            })
        })?;

        let mut result = Vec::new();
        for suggestion in suggestions {
            result.push(suggestion?);
        }
        Ok(result)
    }

    /// Suggest cycle reduction
    pub fn suggest_cycle_reduction(
        &self,
        conn: &rusqlite::Connection,
    ) -> Result<Vec<RefactorSuggestion>> {
        let mut stmt = conn.prepare(
            "
            SELECT DISTINCT
                e1.file_path as file1,
                e2.file_path as file2,
                COUNT(*) as cycle_strength
            FROM code_edges ce1
            JOIN code_entities e1 ON ce1.src_entity_id = e1.id
            JOIN code_entities e2 ON ce1.dst_entity_id = e2.id
            JOIN code_edges ce2 ON e2.id = ce2.src_entity_id AND e1.id = ce2.dst_entity_id
            WHERE e1.file_path < e2.file_path
            AND e1.file_path NOT LIKE '%/tests/%'
            AND e2.file_path NOT LIKE '%/tests/%'
            GROUP BY e1.file_path, e2.file_path
            HAVING cycle_strength >= 2
            ORDER BY cycle_strength DESC
            ",
        )?;

        let suggestions = stmt.query_map([], |row| {
            let file1: String = row.get(0)?;
            let file2: String = row.get(1)?;
            let cycle_strength: i64 = row.get(2)?;

            let mut metrics = HashMap::new();
            metrics.insert(
                "cycle_strength".to_string(),
                Value::Number(serde_json::Number::from(cycle_strength)),
            );
            metrics.insert("cycle_length".to_string(), Value::Number(serde_json::Number::from(2)));

            Ok(RefactorSuggestion {
                kind: RefactorKind::ReduceCycle,
                description: format!(
                    "Reduce circular dependency between {} and {} (strength: {})",
                    file1, file2, cycle_strength
                ),
                file_path: Some(file1.clone()),
                related_files: Some(vec![file2]),
                metrics,
            })
        })?;

        let mut result = Vec::new();
        for suggestion in suggestions {
            result.push(suggestion?);
        }
        Ok(result)
    }

    /// Suggest dead code pruning
    pub fn suggest_dead_code_pruning(
        &self,
        conn: &rusqlite::Connection,
    ) -> Result<Vec<RefactorSuggestion>> {
        let mut stmt = conn.prepare(
            "
            SELECT 
                file_path,
                COUNT(*) as dead_count
            FROM code_entities ce
            LEFT JOIN code_edges ce_in ON ce.id = ce_in.dst_entity_id
            WHERE ce_in.dst_entity_id IS NULL
            AND ce.entity_type NOT IN ('import', 'module')
            AND ce.file_path NOT LIKE '%/tests/%'
            AND ce.file_path NOT LIKE '%_test.rs'
            GROUP BY file_path
            HAVING dead_count >= 3
            ORDER BY dead_count DESC
            ",
        )?;

        let suggestions = stmt.query_map([], |row| {
            let file_path: String = row.get(0)?;
            let dead_count: i64 = row.get(1)?;

            let mut metrics = HashMap::new();
            metrics.insert(
                "dead_entities".to_string(),
                Value::Number(serde_json::Number::from(dead_count)),
            );

            Ok(RefactorSuggestion {
                kind: RefactorKind::PruneDeadCode,
                description: format!("Remove {} unused entities from {}", dead_count, file_path),
                file_path: Some(file_path),
                related_files: None,
                metrics,
            })
        })?;

        let mut result = Vec::new();
        for suggestion in suggestions {
            result.push(suggestion?);
        }
        Ok(result)
    }

    /// Suggest dependency simplification
    pub fn suggest_dependency_simplification(
        &self,
        conn: &rusqlite::Connection,
        fan_out_threshold: u32,
    ) -> Result<Vec<RefactorSuggestion>> {
        let mut stmt = conn.prepare(
            "
            SELECT 
                e1.file_path,
                COUNT(DISTINCT e2.file_path) as distinct_deps,
                COUNT(*) as total_deps
            FROM code_edges ce
            JOIN code_entities e1 ON ce.src_entity_id = e1.id
            JOIN code_entities e2 ON ce.dst_entity_id = e2.id
            WHERE e1.file_path != e2.file_path
            AND e1.file_path NOT LIKE '%/tests/%'
            AND e2.file_path NOT LIKE '%/tests/%'
            GROUP BY e1.file_path
            HAVING distinct_deps >= ?1
            ORDER BY distinct_deps DESC
            ",
        )?;

        let suggestions = stmt.query_map([fan_out_threshold as i64], |row| {
            let file_path: String = row.get(0)?;
            let distinct_deps: i64 = row.get(1)?;
            let total_deps: i64 = row.get(2)?;

            let mut metrics = HashMap::new();
            metrics.insert(
                "distinct_dependencies".to_string(),
                Value::Number(serde_json::Number::from(distinct_deps)),
            );
            metrics.insert(
                "total_dependencies".to_string(),
                Value::Number(serde_json::Number::from(total_deps)),
            );

            Ok(RefactorSuggestion {
                kind: RefactorKind::SimplifyDependency,
                description: format!(
                    "Simplify dependencies for {} (depends on {} different files)",
                    file_path, distinct_deps
                ),
                file_path: Some(file_path),
                related_files: None,
                metrics,
            })
        })?;

        let mut result = Vec::new();
        for suggestion in suggestions {
            result.push(suggestion?);
        }
        Ok(result)
    }
}
