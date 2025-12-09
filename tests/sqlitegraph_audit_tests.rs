use anyhow::Result;
use rusqlite::params;
use std::sync::Arc;
use tempfile::{tempdir, NamedTempFile};

use syncore::db::DbManager;
use syncore::project_analysis::{
    sqlitegraph_audit::SQLiteGraphAuditRequest, ProjectAnalysisEngine,
};

fn ensure_code_graph_schema(db_manager: &DbManager) -> Result<()> {
    let conn = db_manager.code_graph_conn();
    let db = conn.lock().unwrap();

    db.execute_batch(
        r#"
        PRAGMA foreign_keys=ON;

        DROP TABLE IF EXISTS code_edges;
        DROP TABLE IF EXISTS code_embeddings;
        DROP TABLE IF EXISTS code_entities;

        CREATE TABLE code_entities (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_path TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            name TEXT NOT NULL,
            signature TEXT,
            line_start INTEGER,
            line_end INTEGER,
            docstring TEXT,
            language TEXT NOT NULL,
            indexed_at INTEGER NOT NULL,
            created_at INTEGER,
            last_modified_at INTEGER,
            change_count INTEGER,
            author_count INTEGER,
            body_snippet TEXT,
            UNIQUE(file_path, entity_type, name, line_start)
        );
        CREATE INDEX IF NOT EXISTS idx_entities_name ON code_entities(name);
        CREATE INDEX IF NOT EXISTS idx_entities_file ON code_entities(file_path);
        CREATE INDEX IF NOT EXISTS idx_entities_type ON code_entities(entity_type);
        CREATE INDEX IF NOT EXISTS idx_entities_lang ON code_entities(language);

        CREATE TABLE code_edges (
            src_entity_id INTEGER NOT NULL REFERENCES code_entities(id) ON DELETE CASCADE,
            dst_entity_id INTEGER NOT NULL REFERENCES code_entities(id) ON DELETE CASCADE,
            edge_type TEXT NOT NULL,
            PRIMARY KEY (src_entity_id, dst_entity_id, edge_type)
        );
        CREATE INDEX IF NOT EXISTS idx_edges_src ON code_edges(src_entity_id);
        CREATE INDEX IF NOT EXISTS idx_edges_dst ON code_edges(dst_entity_id);
        CREATE INDEX IF NOT EXISTS idx_edges_type ON code_edges(edge_type);

        CREATE TABLE code_embeddings (
            entity_id INTEGER PRIMARY KEY REFERENCES code_entities(id) ON DELETE CASCADE,
            vector_id INTEGER NOT NULL,
            model_version TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_code_embeddings_vector ON code_embeddings(vector_id);
        "#,
    )?;

    Ok(())
}

fn setup_engine() -> Result<(Arc<DbManager>, ProjectAnalysisEngine)> {
    let manager = Arc::new(DbManager::new(":memory:", ":memory:")?);
    ensure_code_graph_schema(&manager)?;
    let engine = ProjectAnalysisEngine::new(manager.clone(), None);
    Ok((manager, engine))
}

fn reset_code_graph(conn: &rusqlite::Connection) -> Result<()> {
    conn.execute("DELETE FROM code_edges", [])?;
    conn.execute("DELETE FROM code_embeddings", [])?;
    conn.execute("DELETE FROM code_entities", [])?;
    Ok(())
}

fn insert_entity(
    conn: &rusqlite::Connection,
    id: i64,
    file_path: &str,
    created_at: Option<i64>,
    last_modified_at: Option<i64>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO code_entities (
            id, file_path, entity_type, name, signature,
            line_start, line_end, docstring, language,
            indexed_at, created_at, last_modified_at,
            change_count, author_count, body_snippet
        ) VALUES (?1, ?2, 'function', ?3, NULL, 1, 10, NULL, 'rust', 0, ?4, ?5, 0, 0, NULL)",
        params![id, file_path, format!("entity_{id}"), created_at, last_modified_at],
    )?;
    Ok(())
}

fn insert_embedding(conn: &rusqlite::Connection, entity_id: i64, vector_id: i64) -> Result<()> {
    conn.execute(
        "INSERT INTO code_embeddings (entity_id, vector_id, model_version, created_at)
         VALUES (?1, ?2, 'test-model', 0)",
        params![entity_id, vector_id],
    )?;
    Ok(())
}

fn insert_orphan_embedding(conn: &rusqlite::Connection, entity_id: i64) -> Result<()> {
    conn.execute("PRAGMA foreign_keys=OFF", [])?;
    conn.execute(
        "INSERT INTO code_embeddings (entity_id, vector_id, model_version, created_at)
         VALUES (?1, 9999, 'test-model', 0)",
        params![entity_id],
    )?;
    conn.execute("PRAGMA foreign_keys=ON", [])?;
    Ok(())
}

fn insert_orphan_edge(conn: &rusqlite::Connection, src: i64, dst: i64) -> Result<()> {
    conn.execute("PRAGMA foreign_keys=OFF", [])?;
    conn.execute(
        "INSERT INTO code_edges (src_entity_id, dst_entity_id, edge_type)
         VALUES (?1, ?2, 'references')",
        params![src, dst],
    )?;
    conn.execute("PRAGMA foreign_keys=ON", [])?;
    Ok(())
}

#[tokio::test]
async fn sqlitegraph_audit_reports_empty_state() -> Result<()> {
    let (db_manager, engine) = setup_engine()?;
    {
        let conn = db_manager.code_graph_conn();
        let guard = conn.lock().unwrap();
        reset_code_graph(&guard)?;
    }

    let response = engine
        .audit_sqlitegraph(SQLiteGraphAuditRequest {
            max_examples: Some(5),
            project_root: None,
            excluded_dirs: None,
        })
        .await?;

    assert!(response.ok);
    let report = response.data.unwrap();
    assert_eq!(report.entities_total, 0);
    assert_eq!(report.edges_total, 0);
    assert_eq!(report.embeddings_total, 0);
    assert_eq!(report.orphan_edges_count, 0);
    assert_eq!(report.orphan_embeddings_count, 0);
    assert_eq!(report.missing_embeddings_count, 0);
    assert_eq!(report.missing_files_count, 0);
    assert_eq!(report.temporal_missing_count, 0);
    Ok(())
}

#[tokio::test]
async fn sqlitegraph_audit_detects_drift_conditions() -> Result<()> {
    let (db_manager, engine) = setup_engine()?;
    let temp_file = NamedTempFile::new()?;
    let excluded_dir = tempdir()?;
    let target_file = excluded_dir
        .path()
        .join("target/debug/build");
    std::fs::create_dir_all(&target_file)?;
    let generated_file = target_file.join("generated.rs");
    std::fs::write(&generated_file, "pub fn generated() {}")?;
    let generated_path = generated_file.to_string_lossy().to_string();
    {
        let conn = db_manager.code_graph_conn();
        let guard = conn.lock().unwrap();
        reset_code_graph(&guard)?;

        insert_entity(&guard, 1, "/no/such/file.rs", Some(0), Some(0))?;
        insert_entity(&guard, 2, temp_file.path().to_string_lossy().as_ref(), Some(0), Some(0))?;
        insert_entity(&guard, 3, &generated_path, Some(0), Some(0))?;

        insert_embedding(&guard, 2, 200)?;
        insert_embedding(&guard, 3, 300)?;
        insert_orphan_edge(&guard, 1, 999)?;
        insert_orphan_embedding(&guard, 4242)?;
    }

    let response = engine
        .audit_sqlitegraph(SQLiteGraphAuditRequest {
            max_examples: Some(5),
            project_root: None,
            excluded_dirs: None,
        })
        .await?;

    assert!(response.ok);
    let report = response.data.unwrap();
    assert_eq!(report.orphan_edges_count, 1);
    assert_eq!(report.orphan_embeddings_count, 1);
    assert_eq!(report.vector_mismatches_count, report.orphan_embeddings_count);
    assert!(report.orphan_edges_examples.iter().any(|example| example.missing_side == "dst"));
    assert!(report.orphan_embeddings_examples.iter().any(|entity_id| *entity_id == 4242));
    assert_eq!(report.missing_embeddings_count, 1);
    assert!(report.missing_embeddings_examples.iter().any(|example| example.entity_id == 1));
    assert_eq!(report.missing_files_count, 1);
    assert!(report.missing_files_examples.iter().any(|path| path == "/no/such/file.rs"));
    assert!(report.excluded_dir_violations.iter().any(|violation| violation.dir == "target"));
    Ok(())
}

#[tokio::test]
async fn sqlitegraph_audit_resolves_relative_paths_and_temporal_gaps() -> Result<()> {
    let (db_manager, engine) = setup_engine()?;
    let tmp_dir = tempdir()?;
    let file_path = tmp_dir.path().join("src/lib.rs");
    std::fs::create_dir_all(file_path.parent().unwrap())?;
    std::fs::write(&file_path, "fn main() {}\n")?;

    {
        let conn = db_manager.code_graph_conn();
        let guard = conn.lock().unwrap();
        reset_code_graph(&guard)?;
        insert_entity(
            &guard,
            10,
            "src/lib.rs",
            None, // created_at missing
            None,
        )?;
    }

    let response = engine
        .audit_sqlitegraph(SQLiteGraphAuditRequest {
            max_examples: Some(5),
            project_root: Some(tmp_dir.path().to_string_lossy().to_string()),
            excluded_dirs: None,
        })
        .await?;

    assert!(response.ok);
    let report = response.data.unwrap();
    assert_eq!(report.missing_files_count, 0);
    assert_eq!(report.temporal_missing_count, 1);
    Ok(())
}
