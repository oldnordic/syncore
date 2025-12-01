//! PAE Meta Tools Diagnostics Tests
//!
//! Validates metadata integration in meta-tools:
//! - architecture_overview includes total_clippy_warnings
//! - complexity_dashboard includes total_clippy_warnings
//! - improvement_roadmap includes total_clippy_warnings
//! - refactor_action_plan includes total_clippy_warnings

use anyhow::Result;
use std::sync::Arc;
use syncore::db::DbManager;
use syncore::project_analysis::diagnostics::{CodeDiagnostic, DiagnosticsManager};
use syncore::project_analysis::{
    architecture_overview::ArchitectureOverviewRequest,
    complexity_dashboard::ComplexityDashboardRequest,
    improvement_roadmap::ImprovementRoadmapRequest,
    refactor_action_plan::RefactorActionPlanRequest, ProjectAnalysisEngine,
};
use tempfile::TempDir;

/// Initialize code_graph schema on the code_graph database
fn ensure_code_graph_schema(db_manager: &DbManager) -> Result<()> {
    let conn = db_manager.code_graph_conn();
    let db = conn.lock().unwrap();
    db.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS code_entities (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_path TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            name TEXT NOT NULL,
            signature TEXT,
            line_start INTEGER NOT NULL,
            line_end INTEGER NOT NULL,
            docstring TEXT,
            language TEXT NOT NULL,
            indexed_at INTEGER NOT NULL,
            created_at INTEGER,
            last_modified_at INTEGER,
            change_count INTEGER,
            author_count INTEGER,
            UNIQUE(file_path, entity_type, name, line_start)
        );
        CREATE INDEX IF NOT EXISTS idx_entities_name ON code_entities(name);
        CREATE INDEX IF NOT EXISTS idx_entities_file ON code_entities(file_path);
        CREATE INDEX IF NOT EXISTS idx_entities_type ON code_entities(entity_type);

        CREATE TABLE IF NOT EXISTS code_edges (
            src_entity_id INTEGER NOT NULL REFERENCES code_entities(id) ON DELETE CASCADE,
            dst_entity_id INTEGER NOT NULL REFERENCES code_entities(id) ON DELETE CASCADE,
            edge_type TEXT NOT NULL,
            PRIMARY KEY (src_entity_id, dst_entity_id, edge_type)
        );
        CREATE INDEX IF NOT EXISTS idx_edges_src ON code_edges(src_entity_id);
        CREATE INDEX IF NOT EXISTS idx_edges_dst ON code_edges(dst_entity_id);

        CREATE TABLE IF NOT EXISTS code_diagnostics (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_path TEXT NOT NULL,
            line_start INTEGER NOT NULL,
            severity TEXT NOT NULL,
            diagnostic_type TEXT NOT NULL,
            message TEXT NOT NULL,
            tool TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_diagnostics_file ON code_diagnostics(file_path);
        CREATE INDEX IF NOT EXISTS idx_diagnostics_tool ON code_diagnostics(tool);
        CREATE INDEX IF NOT EXISTS idx_diagnostics_type ON code_diagnostics(diagnostic_type);
        CREATE INDEX IF NOT EXISTS idx_diagnostics_severity ON code_diagnostics(severity);
        "#,
    )?;
    Ok(())
}

/// Insert a test entity into the code_entities table
fn insert_test_entity(
    db_manager: &DbManager,
    file_path: &str,
    entity_type: &str,
    name: &str,
    line_start: i32,
    line_end: i32,
) -> Result<i64> {
    let conn = db_manager.code_graph_conn();
    let db = conn.lock().unwrap();
    db.execute(
        "INSERT INTO code_entities (file_path, entity_type, name, signature, line_start, line_end, language, indexed_at)
         VALUES (?1, ?2, ?3, NULL, ?4, ?5, 'rust', strftime('%s', 'now'))",
        rusqlite::params![file_path, entity_type, name, line_start, line_end],
    )?;
    Ok(db.last_insert_rowid())
}

#[tokio::test]
async fn test_architecture_overview_clippy_integration() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let main_db_path = temp_dir.path().join("main.db");
    let code_graph_db_path = temp_dir.path().join("code_graph.db");

    let db_manager = Arc::new(DbManager::new(
        main_db_path.to_str().unwrap(),
        code_graph_db_path.to_str().unwrap(),
    )?);

    // Initialize code_graph schema and insert test entities
    ensure_code_graph_schema(&db_manager)?;
    insert_test_entity(&db_manager, "src/main.rs", "function", "main", 1, 50)?;
    insert_test_entity(&db_manager, "src/main.rs", "function", "helper", 52, 80)?;
    insert_test_entity(&db_manager, "src/lib.rs", "function", "lib_func", 1, 30)?;

    let engine = ProjectAnalysisEngine::new(db_manager.clone(), None);

    // Create synthetic clippy diagnostics
    let diagnostics = DiagnosticsManager::new(db_manager.clone());
    let clippy_diagnostics = vec![
        CodeDiagnostic::new(
            "src/main.rs".to_string(),
            10,
            "warning".to_string(),
            "clippy::dead_code".to_string(),
            "unused function".to_string(),
            "clippy".to_string(),
        ),
        CodeDiagnostic::new(
            "src/main.rs".to_string(),
            20,
            "error".to_string(),
            "clippy::unimplemented".to_string(),
            "unimplemented code".to_string(),
            "clippy".to_string(),
        ),
        CodeDiagnostic::new(
            "src/lib.rs".to_string(),
            15,
            "warning".to_string(),
            "clippy::unused_import".to_string(),
            "unused import".to_string(),
            "clippy".to_string(),
        ),
    ];

    // Insert clippy diagnostics
    let inserted = diagnostics.insert_diagnostics(&clippy_diagnostics)?;
    assert_eq!(inserted, 3);

    // Run architecture overview
    let request = ArchitectureOverviewRequest {
        limit_hotspots: Some(10),
        limit_modules: Some(50),
        loc_threshold: Some(1), // Low threshold to include test files
    };

    let response = engine.architecture_overview(request).await?;
    assert!(response.ok, "Architecture overview failed: {:?}", response.error);

    let data = response.data.unwrap();

    // Verify total_clippy_warnings is included in summary
    assert_eq!(data.summary.clippy_warning_count, 3);

    // Verify total_clippy_warnings is included in module overviews
    let main_rs_module = data.modules.iter().find(|m| m.file_path == "src/main.rs");
    assert!(main_rs_module.is_some(), "Should have module for src/main.rs");

    let main_rs_module = main_rs_module.unwrap();
    assert_eq!(main_rs_module.clippy_warning_count, 2);

    let lib_rs_module = data.modules.iter().find(|m| m.file_path == "src/lib.rs");
    assert!(lib_rs_module.is_some(), "Should have module for src/lib.rs");

    let lib_rs_module = lib_rs_module.unwrap();
    assert_eq!(lib_rs_module.clippy_warning_count, 1);

    Ok(())
}

#[tokio::test]
async fn test_complexity_dashboard_clippy_integration() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let main_db_path = temp_dir.path().join("main.db");
    let code_graph_db_path = temp_dir.path().join("code_graph.db");

    let db_manager = Arc::new(DbManager::new(
        main_db_path.to_str().unwrap(),
        code_graph_db_path.to_str().unwrap(),
    )?);

    // Initialize code_graph schema and insert test entities
    ensure_code_graph_schema(&db_manager)?;
    insert_test_entity(&db_manager, "src/complex.rs", "function", "complex_fn", 1, 50)?;
    insert_test_entity(&db_manager, "src/simple.rs", "function", "simple_fn", 1, 20)?;

    let engine = ProjectAnalysisEngine::new(db_manager.clone(), None);

    // Create synthetic clippy diagnostics
    let diagnostics = DiagnosticsManager::new(db_manager.clone());
    let clippy_diagnostics = vec![
        CodeDiagnostic::new(
            "src/complex.rs".to_string(),
            5,
            "warning".to_string(),
            "clippy::too_many_arguments".to_string(),
            "too many function arguments".to_string(),
            "clippy".to_string(),
        ),
        CodeDiagnostic::new(
            "src/complex.rs".to_string(),
            15,
            "warning".to_string(),
            "clippy::cognitive_complexity".to_string(),
            "function is too complex".to_string(),
            "clippy".to_string(),
        ),
        CodeDiagnostic::new(
            "src/simple.rs".to_string(),
            10,
            "warning".to_string(),
            "clippy::dead_code".to_string(),
            "unused function".to_string(),
            "clippy".to_string(),
        ),
    ];

    // Insert clippy diagnostics
    let inserted = diagnostics.insert_diagnostics(&clippy_diagnostics)?;
    assert_eq!(inserted, 3);

    // Run complexity dashboard
    let request = ComplexityDashboardRequest {
        limit_hotspots: Some(20),
        loc_threshold: Some(1), // Low threshold for test
    };

    let response = engine.complexity_dashboard(request).await?;
    assert!(response.ok, "Complexity dashboard failed: {:?}", response.error);

    let data = response.data.unwrap();

    // Verify total_clippy_warnings is included in summary
    assert_eq!(data.summary.total_clippy_warnings, 3);

    // Verify total_clippy_warnings is included in module complexity data
    let complex_rs_module = data.files.iter().find(|m| m.file_path == "src/complex.rs");
    assert!(complex_rs_module.is_some(), "Should have module for src/complex.rs");

    let complex_rs_module = complex_rs_module.unwrap();
    assert_eq!(complex_rs_module.clippy_warning_count, 2);

    let simple_rs_module = data.files.iter().find(|m| m.file_path == "src/simple.rs");
    assert!(simple_rs_module.is_some(), "Should have module for src/simple.rs");

    let simple_rs_module = simple_rs_module.unwrap();
    assert_eq!(simple_rs_module.clippy_warning_count, 1);

    Ok(())
}

#[tokio::test]
async fn test_improvement_roadmap_clippy_integration() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let main_db_path = temp_dir.path().join("main.db");
    let code_graph_db_path = temp_dir.path().join("code_graph.db");

    let db_manager = Arc::new(DbManager::new(
        main_db_path.to_str().unwrap(),
        code_graph_db_path.to_str().unwrap(),
    )?);

    let engine = ProjectAnalysisEngine::new(db_manager.clone(), None);

    // Create synthetic clippy diagnostics
    let diagnostics = DiagnosticsManager::new(db_manager.clone());
    let clippy_diagnostics = vec![
        CodeDiagnostic::new(
            "src/needs_refactor.rs".to_string(),
            8,
            "warning".to_string(),
            "clippy::large_enum_variant".to_string(),
            "large enum variant".to_string(),
            "clippy".to_string(),
        ),
        CodeDiagnostic::new(
            "src/needs_refactor.rs".to_string(),
            18,
            "error".to_string(),
            "clippy::unimplemented".to_string(),
            "unimplemented code".to_string(),
            "clippy".to_string(),
        ),
        CodeDiagnostic::new(
            "src/clean.rs".to_string(),
            12,
            "warning".to_string(),
            "clippy::unused_import".to_string(),
            "unused import".to_string(),
            "clippy".to_string(),
        ),
    ];

    // Insert clippy diagnostics
    let inserted = diagnostics.insert_diagnostics(&clippy_diagnostics)?;
    assert_eq!(inserted, 3);

    // Run improvement roadmap
    let request = ImprovementRoadmapRequest {
        limit_per_category: Some(10),
        high_priority_only: Some(false),
        hotspot_loc_threshold: Some(1),
        project_label: Some("test".to_string()),
    };

    let response = engine.improvement_roadmap(request).await?;
    assert!(response.ok, "Improvement roadmap failed: {:?}", response.error);

    let data = response.data.unwrap();

    // Verify total_clippy_warnings is included in summary
    assert_eq!(data.summary.clippy_warning_count, 3);

    // Verify improvement suggestions have required fields
    for suggestion in &data.improvements {
        assert!(!suggestion.id.is_empty(), "Each suggestion should have an ID");
        assert!(!suggestion.file_path.is_empty(), "Each suggestion should have a file path");
        assert!(suggestion.effort >= 1 && suggestion.effort <= 5, "Effort should be 1-5");
        assert!(suggestion.impact >= 1 && suggestion.impact <= 5, "Impact should be 1-5");
    }

    Ok(())
}

#[tokio::test]
async fn test_refactor_action_plan_clippy_integration() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let main_db_path = temp_dir.path().join("main.db");
    let code_graph_db_path = temp_dir.path().join("code_graph.db");

    let db_manager = Arc::new(DbManager::new(
        main_db_path.to_str().unwrap(),
        code_graph_db_path.to_str().unwrap(),
    )?);

    let engine = ProjectAnalysisEngine::new(db_manager.clone(), None);

    // Create synthetic clippy diagnostics
    let diagnostics = DiagnosticsManager::new(db_manager.clone());
    let clippy_diagnostics = vec![
        CodeDiagnostic::new(
            "src/legacy.rs".to_string(),
            25,
            "warning".to_string(),
            "clippy::dead_code".to_string(),
            "unused function".to_string(),
            "clippy".to_string(),
        ),
        CodeDiagnostic::new(
            "src/legacy.rs".to_string(),
            35,
            "warning".to_string(),
            "clippy::unused_import".to_string(),
            "unused import".to_string(),
            "clippy".to_string(),
        ),
        CodeDiagnostic::new(
            "src/modern.rs".to_string(),
            45,
            "warning".to_string(),
            "clippy::perf".to_string(),
            "performance suggestion".to_string(),
            "clippy".to_string(),
        ),
    ];

    // Insert clippy diagnostics
    let inserted = diagnostics.insert_diagnostics(&clippy_diagnostics)?;
    assert_eq!(inserted, 3);

    // Run refactor action plan
    let request = RefactorActionPlanRequest {};

    let response = engine.refactor_action_plan(request).await?;
    assert!(response.ok, "Refactor action plan failed: {:?}", response.error);

    let data = response.data.unwrap();

    // Verify total_clippy_warnings is included in summary
    assert_eq!(data.summary.clippy_warning_count, 3);

    // Verify refactor actions have required fields
    for action in &data.dead_code_cleanup {
        assert!(action.id > 0, "Each action should have a valid ID");
        assert!(!action.name.is_empty(), "Each action should have a name");
        assert!(!action.entity_type.is_empty(), "Each action should have an entity type");
        assert!(!action.file_path.is_empty(), "Each action should have a file path");
        assert!(action.line_start >= 0, "Line start should be non-negative");
    }

    Ok(())
}

#[tokio::test]
async fn test_meta_tools_clippy_counts_consistency() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let main_db_path = temp_dir.path().join("main.db");
    let code_graph_db_path = temp_dir.path().join("code_graph.db");

    let db_manager = Arc::new(DbManager::new(
        main_db_path.to_str().unwrap(),
        code_graph_db_path.to_str().unwrap(),
    )?);

    // Initialize code_graph schema and insert test entities
    ensure_code_graph_schema(&db_manager)?;
    insert_test_entity(&db_manager, "src/test1.rs", "function", "test1_fn", 1, 20)?;
    insert_test_entity(&db_manager, "src/test2.rs", "function", "test2_fn", 1, 25)?;

    let engine = ProjectAnalysisEngine::new(db_manager.clone(), None);

    // Create a consistent set of clippy diagnostics
    let diagnostics = DiagnosticsManager::new(db_manager.clone());
    let clippy_diagnostics = vec![
        CodeDiagnostic::new(
            "src/test1.rs".to_string(),
            5,
            "warning".to_string(),
            "clippy::dead_code".to_string(),
            "unused function".to_string(),
            "clippy".to_string(),
        ),
        CodeDiagnostic::new(
            "src/test1.rs".to_string(),
            10,
            "warning".to_string(),
            "clippy::unused_import".to_string(),
            "unused import".to_string(),
            "clippy".to_string(),
        ),
        CodeDiagnostic::new(
            "src/test2.rs".to_string(),
            15,
            "error".to_string(),
            "clippy::unimplemented".to_string(),
            "unimplemented code".to_string(),
            "clippy".to_string(),
        ),
    ];

    // Insert clippy diagnostics
    let inserted = diagnostics.insert_diagnostics(&clippy_diagnostics)?;
    assert_eq!(inserted, 3);

    // Run all meta-tools and verify clippy counts are consistent
    let arch_request = ArchitectureOverviewRequest {
        limit_hotspots: Some(10),
        limit_modules: Some(50),
        loc_threshold: Some(1),
    };
    let arch_response = engine.architecture_overview(arch_request).await?;
    assert!(arch_response.ok);
    let arch_data = arch_response.data.unwrap();

    let complex_request = ComplexityDashboardRequest {
        limit_hotspots: Some(20),
        loc_threshold: Some(1),
    };
    let complex_response = engine.complexity_dashboard(complex_request).await?;
    assert!(complex_response.ok);
    let complex_data = complex_response.data.unwrap();

    let roadmap_request = ImprovementRoadmapRequest {
        limit_per_category: Some(10),
        high_priority_only: Some(false),
        hotspot_loc_threshold: Some(1),
        project_label: Some("test".to_string()),
    };
    let roadmap_response = engine.improvement_roadmap(roadmap_request).await?;
    assert!(roadmap_response.ok);
    let roadmap_data = roadmap_response.data.unwrap();

    let refactor_request = RefactorActionPlanRequest {};
    let refactor_response = engine.refactor_action_plan(refactor_request).await?;
    assert!(refactor_response.ok);
    let refactor_data = refactor_response.data.unwrap();

    // All summaries should have the same total_clippy_warnings
    assert_eq!(arch_data.summary.clippy_warning_count, 3);
    assert_eq!(complex_data.summary.total_clippy_warnings, 3);
    assert_eq!(roadmap_data.summary.clippy_warning_count, 3);
    assert_eq!(refactor_data.summary.clippy_warning_count, 3);

    // Verify per-file counts are consistent across tools
    let test1_counts = vec![
        arch_data
            .modules
            .iter()
            .find(|m| m.file_path == "src/test1.rs")
            .map(|m| m.clippy_warning_count)
            .unwrap_or(0),
        complex_data
            .files
            .iter()
            .find(|m| m.file_path == "src/test1.rs")
            .map(|m| m.clippy_warning_count)
            .unwrap_or(0),
    ];

    for count in test1_counts {
        assert_eq!(count, 2, "src/test1.rs should have 2 clippy warnings in all tools");
    }

    let test2_counts = vec![
        arch_data
            .modules
            .iter()
            .find(|m| m.file_path == "src/test2.rs")
            .map(|m| m.clippy_warning_count)
            .unwrap_or(0),
        complex_data
            .files
            .iter()
            .find(|m| m.file_path == "src/test2.rs")
            .map(|m| m.clippy_warning_count)
            .unwrap_or(0),
    ];

    for count in test2_counts {
        assert_eq!(count, 1, "src/test2.rs should have 1 clippy warning in all tools");
    }

    Ok(())
}
