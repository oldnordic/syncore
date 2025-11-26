// SPEC: SYNCORE-CONFIG-AND-TOOLS-FIX-01 (APEX v1.2)
// STEP B: Stale-data purge/cleanup tools
//
// Provides tools to remove stale data from code graph tables
// for paths that are now excluded (e.g., target/, node_modules/).

use crate::macro_tools::path_filter::get_excluded_dirs;
use crate::project_analysis::{PAEResponse, ProjectAnalysisEngine};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Request parameters for project_cleanup_excluded
#[derive(Debug, Deserialize)]
pub struct CleanupExcludedRequest {
    /// If true, only report what would be deleted without actually deleting
    #[serde(default)]
    pub dry_run: bool,
    /// Override excluded directories (uses config if not provided)
    pub excluded_dirs: Option<Vec<String>>,
}

/// Response data for cleanup operation
#[derive(Debug, Serialize, Deserialize)]
pub struct CleanupExcludedData {
    /// Number of entities deleted
    pub entities_deleted: u64,
    /// Number of edges deleted
    pub edges_deleted: u64,
    /// Number of files affected
    pub files_affected: u64,
    /// Sample of deleted file paths (first 10)
    pub sample_paths: Vec<String>,
    /// Was this a dry run?
    pub dry_run: bool,
    /// Excluded directories used
    pub excluded_dirs: Vec<String>,
}

impl ProjectAnalysisEngine {
    /// Clean up indexed data for paths that match excluded directories
    pub async fn cleanup_excluded(
        &self,
        request: CleanupExcludedRequest,
    ) -> Result<PAEResponse<CleanupExcludedData>> {
        match self.perform_cleanup(request).await {
            Ok(data) => Ok(PAEResponse::success(data)),
            Err(e) => Ok(PAEResponse::error(e.to_string())),
        }
    }

    async fn perform_cleanup(
        &self,
        request: CleanupExcludedRequest,
    ) -> Result<CleanupExcludedData> {
        // Get excluded directories from request or config
        let excluded_dirs = request.excluded_dirs.unwrap_or_else(get_excluded_dirs);

        // Build LIKE patterns for each excluded directory
        let like_patterns: Vec<String> = excluded_dirs
            .iter()
            .map(|dir| format!("%/{}/%", dir))
            .collect();

        // Also match patterns at the start of path
        let like_patterns_start: Vec<String> = excluded_dirs
            .iter()
            .map(|dir| format!("{}/%", dir))
            .collect();

        // Perform SQLite cleanup (scoped to drop guard before async)
        let (entities_deleted, edges_deleted, files_affected, sample_paths) = {
            let conn = self.code_graph_conn();
            let conn_guard = conn.lock().unwrap();

            // First, find affected files
            let mut affected_files: Vec<String> = Vec::new();
            {
                let mut query = String::from("SELECT DISTINCT file_path FROM code_entities WHERE ");
                let mut conditions: Vec<String> = Vec::new();

                for (i, _pattern) in like_patterns.iter().enumerate() {
                    conditions.push(format!("file_path LIKE ?{}", i + 1));
                }
                for (i, _pattern) in like_patterns_start.iter().enumerate() {
                    conditions.push(format!("file_path LIKE ?{}", like_patterns.len() + i + 1));
                }

                if conditions.is_empty() {
                    return Ok(CleanupExcludedData {
                        entities_deleted: 0,
                        edges_deleted: 0,
                        files_affected: 0,
                        sample_paths: vec![],
                        dry_run: request.dry_run,
                        excluded_dirs,
                    });
                }

                query.push_str(&conditions.join(" OR "));
                query.push_str(" ORDER BY file_path LIMIT 1000");

                let mut stmt = conn_guard.prepare(&query)?;

                // Build params
                let all_patterns: Vec<&str> = like_patterns
                    .iter()
                    .chain(like_patterns_start.iter())
                    .map(|s| s.as_str())
                    .collect();

                let param_refs: Vec<&dyn rusqlite::ToSql> = all_patterns
                    .iter()
                    .map(|s| s as &dyn rusqlite::ToSql)
                    .collect();

                let rows = stmt.query_map(&param_refs[..], |row| row.get::<_, String>(0))?;

                for row in rows {
                    affected_files.push(row?);
                }
            }

            let files_affected = affected_files.len() as u64;
            let sample_paths: Vec<String> = affected_files.iter().take(10).cloned().collect();

            if request.dry_run {
                // Count entities that would be deleted
                let entities_count = self.count_entities_for_patterns(
                    &conn_guard,
                    &like_patterns,
                    &like_patterns_start,
                )?;
                let edges_count = self.count_edges_for_patterns(
                    &conn_guard,
                    &like_patterns,
                    &like_patterns_start,
                )?;

                return Ok(CleanupExcludedData {
                    entities_deleted: entities_count,
                    edges_deleted: edges_count,
                    files_affected,
                    sample_paths,
                    dry_run: true,
                    excluded_dirs,
                });
            }

            // Actually delete the data
            let edges_deleted =
                self.delete_edges_for_patterns(&conn_guard, &like_patterns, &like_patterns_start)?;
            let entities_deleted = self.delete_entities_for_patterns(
                &conn_guard,
                &like_patterns,
                &like_patterns_start,
            )?;

            (
                entities_deleted,
                edges_deleted,
                files_affected,
                sample_paths,
            )
        };
        // Guard dropped here

        // Also clean up Neo4j if available
        if let Some(neo4j) = self.neo4j() {
            for dir in &excluded_dirs {
                let cypher = format!(
                    "MATCH (n) WHERE n.file_path CONTAINS '/{}/' DETACH DELETE n",
                    dir
                );
                let _ = neo4j.execute_query(&cypher, vec![]).await;
            }
        }

        Ok(CleanupExcludedData {
            entities_deleted,
            edges_deleted,
            files_affected,
            sample_paths,
            dry_run: false,
            excluded_dirs,
        })
    }

    fn count_entities_for_patterns(
        &self,
        conn: &rusqlite::Connection,
        like_patterns: &[String],
        like_patterns_start: &[String],
    ) -> Result<u64> {
        if like_patterns.is_empty() && like_patterns_start.is_empty() {
            return Ok(0);
        }

        let mut query = String::from("SELECT COUNT(*) FROM code_entities WHERE ");
        let mut conditions: Vec<String> = Vec::new();

        for (i, _) in like_patterns.iter().enumerate() {
            conditions.push(format!("file_path LIKE ?{}", i + 1));
        }
        for (i, _) in like_patterns_start.iter().enumerate() {
            conditions.push(format!("file_path LIKE ?{}", like_patterns.len() + i + 1));
        }

        query.push_str(&conditions.join(" OR "));

        let mut stmt = conn.prepare(&query)?;
        let all_patterns: Vec<&str> = like_patterns
            .iter()
            .chain(like_patterns_start.iter())
            .map(|s| s.as_str())
            .collect();
        let param_refs: Vec<&dyn rusqlite::ToSql> = all_patterns
            .iter()
            .map(|s| s as &dyn rusqlite::ToSql)
            .collect();

        let count: i64 = stmt.query_row(&param_refs[..], |row| row.get(0))?;
        Ok(count as u64)
    }

    fn count_edges_for_patterns(
        &self,
        conn: &rusqlite::Connection,
        like_patterns: &[String],
        like_patterns_start: &[String],
    ) -> Result<u64> {
        if like_patterns.is_empty() && like_patterns_start.is_empty() {
            return Ok(0);
        }

        // Count edges where either source or destination entity is in excluded paths
        let mut query = String::from(
            "SELECT COUNT(*) FROM code_edges ce
             JOIN code_entities src ON ce.src_entity_id = src.id
             JOIN code_entities dst ON ce.dst_entity_id = dst.id
             WHERE ",
        );

        let mut conditions: Vec<String> = Vec::new();
        let pattern_count = like_patterns.len() + like_patterns_start.len();

        for i in 0..pattern_count {
            conditions.push(format!("src.file_path LIKE ?{}", i + 1));
            conditions.push(format!("dst.file_path LIKE ?{}", i + 1));
        }

        query.push_str(&conditions.join(" OR "));

        let mut stmt = conn.prepare(&query)?;
        let all_patterns: Vec<&str> = like_patterns
            .iter()
            .chain(like_patterns_start.iter())
            .map(|s| s.as_str())
            .collect();

        // Duplicate patterns for src and dst matching
        let mut expanded_params: Vec<&str> = Vec::new();
        for pattern in &all_patterns {
            expanded_params.push(pattern);
        }

        let param_refs: Vec<&dyn rusqlite::ToSql> = all_patterns
            .iter()
            .map(|s| s as &dyn rusqlite::ToSql)
            .collect();

        let count: i64 = stmt
            .query_row(&param_refs[..], |row| row.get(0))
            .unwrap_or(0);
        Ok(count as u64)
    }

    fn delete_entities_for_patterns(
        &self,
        conn: &rusqlite::Connection,
        like_patterns: &[String],
        like_patterns_start: &[String],
    ) -> Result<u64> {
        if like_patterns.is_empty() && like_patterns_start.is_empty() {
            return Ok(0);
        }

        let mut query = String::from("DELETE FROM code_entities WHERE ");
        let mut conditions: Vec<String> = Vec::new();

        for (i, _) in like_patterns.iter().enumerate() {
            conditions.push(format!("file_path LIKE ?{}", i + 1));
        }
        for (i, _) in like_patterns_start.iter().enumerate() {
            conditions.push(format!("file_path LIKE ?{}", like_patterns.len() + i + 1));
        }

        query.push_str(&conditions.join(" OR "));

        let mut stmt = conn.prepare(&query)?;
        let all_patterns: Vec<&str> = like_patterns
            .iter()
            .chain(like_patterns_start.iter())
            .map(|s| s.as_str())
            .collect();
        let param_refs: Vec<&dyn rusqlite::ToSql> = all_patterns
            .iter()
            .map(|s| s as &dyn rusqlite::ToSql)
            .collect();

        let deleted = stmt.execute(&param_refs[..])?;
        Ok(deleted as u64)
    }

    fn delete_edges_for_patterns(
        &self,
        conn: &rusqlite::Connection,
        like_patterns: &[String],
        like_patterns_start: &[String],
    ) -> Result<u64> {
        if like_patterns.is_empty() && like_patterns_start.is_empty() {
            return Ok(0);
        }

        // Delete edges where either source or destination entity is in excluded paths
        let mut query = String::from(
            "DELETE FROM code_edges WHERE src_entity_id IN (
                SELECT id FROM code_entities WHERE ",
        );

        let mut conditions: Vec<String> = Vec::new();
        for (i, _) in like_patterns.iter().enumerate() {
            conditions.push(format!("file_path LIKE ?{}", i + 1));
        }
        for (i, _) in like_patterns_start.iter().enumerate() {
            conditions.push(format!("file_path LIKE ?{}", like_patterns.len() + i + 1));
        }

        query.push_str(&conditions.join(" OR "));
        query.push_str(") OR dst_entity_id IN (SELECT id FROM code_entities WHERE ");
        query.push_str(&conditions.join(" OR "));
        query.push_str(")");

        let mut stmt = conn.prepare(&query)?;
        let all_patterns: Vec<&str> = like_patterns
            .iter()
            .chain(like_patterns_start.iter())
            .map(|s| s.as_str())
            .collect();

        // Need to duplicate for the two subqueries
        let mut doubled: Vec<&str> = all_patterns.clone();
        doubled.extend(all_patterns.iter().copied());

        let param_refs: Vec<&dyn rusqlite::ToSql> =
            doubled.iter().map(|s| s as &dyn rusqlite::ToSql).collect();

        let deleted = stmt.execute(&param_refs[..])?;
        Ok(deleted as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_request_defaults() {
        let json = r#"{}"#;
        let request: CleanupExcludedRequest = serde_json::from_str(json).unwrap();
        assert!(!request.dry_run);
        assert!(request.excluded_dirs.is_none());
    }

    #[test]
    fn test_cleanup_request_with_options() {
        let json = r#"{"dry_run": true, "excluded_dirs": ["custom", "dirs"]}"#;
        let request: CleanupExcludedRequest = serde_json::from_str(json).unwrap();
        assert!(request.dry_run);
        assert_eq!(request.excluded_dirs.unwrap(), vec!["custom", "dirs"]);
    }

    #[test]
    fn test_cleanup_data_serialization() {
        let data = CleanupExcludedData {
            entities_deleted: 100,
            edges_deleted: 50,
            files_affected: 10,
            sample_paths: vec!["target/debug/foo.rs".to_string()],
            dry_run: false,
            excluded_dirs: vec!["target".to_string()],
        };

        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("entities_deleted"));
        assert!(json.contains("100"));
    }
}
