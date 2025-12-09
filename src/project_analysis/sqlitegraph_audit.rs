use crate::macro_tools::path_filter::get_excluded_dirs;
use crate::project_analysis::{PAEResponse, ProjectAnalysisEngine};
use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const DEFAULT_MAX_EXAMPLES: usize = 10;
const MAX_ALLOWED_EXAMPLES: usize = 100;

/// Configuration for the SQLiteGraph auditor
#[derive(Debug, Clone, Deserialize)]
pub struct SQLiteGraphAuditRequest {
    /// Maximum number of example rows returned per check (default: 10, max: 100)
    #[serde(default)]
    pub max_examples: Option<usize>,
    /// Optional project root to resolve relative file paths
    #[serde(default)]
    pub project_root: Option<String>,
    /// Override excluded directories (uses config defaults when None)
    #[serde(default)]
    pub excluded_dirs: Option<Vec<String>>,
}

/// Structured report returned by the auditor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SQLiteGraphAuditReport {
    pub entities_total: u64,
    pub edges_total: u64,
    pub embeddings_total: u64,
    pub orphan_edges_count: u64,
    pub orphan_edges_examples: Vec<OrphanEdgeExample>,
    pub orphan_embeddings_count: u64,
    pub orphan_embeddings_examples: Vec<i64>,
    pub missing_embeddings_count: u64,
    pub missing_embeddings_examples: Vec<MissingEmbeddingExample>,
    pub missing_files_count: u64,
    pub missing_files_examples: Vec<String>,
    pub excluded_dir_violations: Vec<ExcludedDirViolation>,
    pub temporal_missing_count: u64,
    pub vector_mismatches_count: u64,
}

/// Example orphan edge row
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrphanEdgeExample {
    pub edge_type: String,
    pub src_entity_id: i64,
    pub dst_entity_id: i64,
    pub missing_side: String,
}

/// Example entity lacking embeddings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingEmbeddingExample {
    pub entity_id: i64,
    pub name: String,
    pub file_path: String,
}

/// Summary of entries that live under excluded directories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcludedDirViolation {
    pub dir: String,
    pub count: u64,
    pub sample_paths: Vec<String>,
}

impl ProjectAnalysisEngine {
    /// Run the SQLiteGraph auditor and return a structured PAEResponse
    pub async fn audit_sqlitegraph(
        &self,
        request: SQLiteGraphAuditRequest,
    ) -> Result<PAEResponse<SQLiteGraphAuditReport>> {
        match self.perform_sqlitegraph_audit(request).await {
            Ok(report) => Ok(PAEResponse::success(report)),
            Err(e) => Ok(PAEResponse::error(e.to_string())),
        }
    }

    async fn perform_sqlitegraph_audit(
        &self,
        request: SQLiteGraphAuditRequest,
    ) -> Result<SQLiteGraphAuditReport> {
        let max_examples =
            request.max_examples.unwrap_or(DEFAULT_MAX_EXAMPLES).clamp(1, MAX_ALLOWED_EXAMPLES);
        let excluded_dirs =
            request.excluded_dirs.filter(|dirs| !dirs.is_empty()).unwrap_or_else(get_excluded_dirs);
        let project_root = request
            .project_root
            .map(PathBuf::from)
            .or_else(|| std::env::var("PROJECT_ROOT").ok().map(PathBuf::from))
            .or_else(|| std::env::current_dir().ok());

        let conn = self.code_graph_conn();
        let conn_guard = conn.lock().unwrap();

        let entities_total = query_count(&conn_guard, "SELECT COUNT(*) FROM code_entities")?;
        let edges_total = query_count(&conn_guard, "SELECT COUNT(*) FROM code_edges")?;
        let embeddings_total = query_count(&conn_guard, "SELECT COUNT(*) FROM code_embeddings")?;

        let orphan_edges_count = query_count(
            &conn_guard,
            "
            SELECT COUNT(*)
            FROM code_edges e
            LEFT JOIN code_entities src ON src.id = e.src_entity_id
            LEFT JOIN code_entities dst ON dst.id = e.dst_entity_id
            WHERE src.id IS NULL OR dst.id IS NULL
        ",
        )?;

        let orphan_edges_examples = fetch_orphan_edge_examples(&conn_guard, max_examples)?;

        let orphan_embeddings_count = query_count(
            &conn_guard,
            "
            SELECT COUNT(*)
            FROM code_embeddings emb
            LEFT JOIN code_entities ce ON ce.id = emb.entity_id
            WHERE ce.id IS NULL
        ",
        )?;

        let orphan_embeddings_examples = fetch_single_column_examples::<i64>(
            &conn_guard,
            "
            SELECT emb.entity_id
            FROM code_embeddings emb
            LEFT JOIN code_entities ce ON ce.id = emb.entity_id
            WHERE ce.id IS NULL
            LIMIT ?1
        ",
            max_examples,
        )?;

        let (missing_embeddings_count, missing_embeddings_examples) =
            fetch_missing_embeddings(&conn_guard, max_examples)?;

        let file_paths = fetch_all_file_paths(&conn_guard)?;
        let excluded_dir_violations =
            collect_excluded_dir_violations(&conn_guard, &excluded_dirs, max_examples)?;

        let temporal_missing_count = query_count(
            &conn_guard,
            "
            SELECT COUNT(*) FROM code_entities
            WHERE created_at IS NULL OR last_modified_at IS NULL
        ",
        )?;

        drop(conn_guard);

        let (missing_files_count, missing_files_examples) =
            evaluate_missing_files(&file_paths, project_root.as_deref(), max_examples);

        let vector_mismatches_count = orphan_embeddings_count;

        Ok(SQLiteGraphAuditReport {
            entities_total,
            edges_total,
            embeddings_total,
            orphan_edges_count,
            orphan_edges_examples,
            orphan_embeddings_count,
            orphan_embeddings_examples,
            missing_embeddings_count,
            missing_embeddings_examples,
            missing_files_count,
            missing_files_examples,
            excluded_dir_violations,
            temporal_missing_count,
            vector_mismatches_count,
        })
    }
}

fn query_count(conn: &Connection, sql: &str) -> Result<u64> {
    let count: i64 = conn.query_row(sql, [], |row| row.get(0))?;
    Ok(count.max(0) as u64)
}

fn fetch_orphan_edge_examples(conn: &Connection, limit: usize) -> Result<Vec<OrphanEdgeExample>> {
    let mut stmt = conn.prepare(
        "
        SELECT
            e.edge_type,
            e.src_entity_id,
            e.dst_entity_id,
            CASE
                WHEN src.id IS NULL AND dst.id IS NULL THEN 'both'
                WHEN src.id IS NULL THEN 'src'
                ELSE 'dst'
            END AS missing_side
        FROM code_edges e
        LEFT JOIN code_entities src ON src.id = e.src_entity_id
        LEFT JOIN code_entities dst ON dst.id = e.dst_entity_id
        WHERE src.id IS NULL OR dst.id IS NULL
        LIMIT ?1
    ",
    )?;

    let rows = stmt.query_map([limit as i64], |row| {
        Ok(OrphanEdgeExample {
            edge_type: row.get(0)?,
            src_entity_id: row.get(1)?,
            dst_entity_id: row.get(2)?,
            missing_side: row.get(3)?,
        })
    })?;

    let mut examples = Vec::new();
    for row in rows {
        examples.push(row?);
    }
    Ok(examples)
}

fn fetch_single_column_examples<T: rusqlite::types::FromSql>(
    conn: &Connection,
    sql: &str,
    limit: usize,
) -> Result<Vec<T>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([limit as i64], |row| row.get(0))?;
    let mut values = Vec::new();
    for row in rows {
        values.push(row?);
    }
    Ok(values)
}

fn fetch_missing_embeddings(
    conn: &Connection,
    limit: usize,
) -> Result<(u64, Vec<MissingEmbeddingExample>)> {
    let count = query_count(
        conn,
        "
        SELECT COUNT(*)
        FROM code_entities ce
        LEFT JOIN code_embeddings emb ON emb.entity_id = ce.id
        WHERE emb.entity_id IS NULL
    ",
    )?;

    let mut stmt = conn.prepare(
        "
        SELECT ce.id, ce.name, ce.file_path
        FROM code_entities ce
        LEFT JOIN code_embeddings emb ON emb.entity_id = ce.id
        WHERE emb.entity_id IS NULL
        LIMIT ?1
    ",
    )?;
    let rows = stmt.query_map([limit as i64], |row| {
        Ok(MissingEmbeddingExample {
            entity_id: row.get(0)?,
            name: row.get(1)?,
            file_path: row.get(2)?,
        })
    })?;

    let mut examples = Vec::new();
    for row in rows {
        examples.push(row?);
    }

    Ok((count, examples))
}

fn fetch_all_file_paths(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT DISTINCT file_path FROM code_entities")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut paths = Vec::new();
    for row in rows {
        paths.push(row?);
    }
    Ok(paths)
}

fn evaluate_missing_files(
    file_paths: &[String],
    project_root: Option<&Path>,
    max_examples: usize,
) -> (u64, Vec<String>) {
    let mut missing_count = 0;
    let mut examples = Vec::new();

    for path in file_paths {
        let resolved = resolve_path(path, project_root);
        if !resolved.exists() {
            missing_count += 1;
            if examples.len() < max_examples {
                examples.push(path.clone());
            }
        }
    }

    (missing_count, examples)
}

fn resolve_path(path: &str, project_root: Option<&Path>) -> PathBuf {
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else if let Some(root) = project_root {
        root.join(candidate)
    } else {
        candidate.to_path_buf()
    }
}

fn collect_excluded_dir_violations(
    conn: &Connection,
    dirs: &[String],
    max_examples: usize,
) -> Result<Vec<ExcludedDirViolation>> {
    let mut violations = Vec::new();

    for dir in dirs {
        let pattern_mid = format!("%/{}/%", dir);
        let pattern_start = format!("{}/%", dir);

        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM code_entities WHERE file_path LIKE ?1 OR file_path LIKE ?2",
            [pattern_mid.as_str(), pattern_start.as_str()],
            |row| row.get(0),
        )?;

        if count > 0 {
            let mut stmt = conn.prepare(
                "
                SELECT DISTINCT file_path
                FROM code_entities
                WHERE file_path LIKE ?1 OR file_path LIKE ?2
                LIMIT ?3
            ",
            )?;
            let rows = stmt.query_map(
                (pattern_mid.as_str(), pattern_start.as_str(), max_examples as i64),
                |row| row.get::<_, String>(0),
            )?;

            let mut sample_paths = Vec::new();
            for row in rows {
                sample_paths.push(row?);
            }

            violations.push(ExcludedDirViolation {
                dir: dir.clone(),
                count: count.max(0) as u64,
                sample_paths,
            });
        }
    }

    Ok(violations)
}
