use anyhow::Result;
use std::sync::Arc;
use syncore::db::DbManager;
use syncore::project_analysis::{
    deps_unified::{UnifiedDependency, UnifiedDependencySummary, UnifiedModule},
    ProjectAnalysisEngine,
};
use tempfile::TempDir;

/// Test helper to create a test database with sample data
async fn setup_test_db() -> Result<(Arc<DbManager>, TempDir)> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");
    let db_manager = Arc::new(DbManager::new(
        &db_path.to_string_lossy(),
        &db_path.to_string_lossy(),
    )?);

    // Initialize with sample data
    init_sample_data(&db_manager).await?;

    Ok((db_manager, temp_dir))
}

/// Initialize the database with sample multi-language project data
async fn init_sample_data(db_manager: &Arc<DbManager>) -> Result<()> {
    let conn = db_manager.code_graph_conn();
    let db = conn.lock().unwrap();

    // Create tables
    db.execute(
        r#"
        CREATE TABLE IF NOT EXISTS code_entities (
            id INTEGER PRIMARY KEY,
            file_path TEXT NOT NULL,
            entity_name TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            language TEXT NOT NULL,
            start_line INTEGER,
            end_line INTEGER
        )
        "#,
        [],
    )?;

    db.execute(
        r#"
        CREATE TABLE IF NOT EXISTS code_edges (
            id INTEGER PRIMARY KEY,
            src_entity_id INTEGER NOT NULL,
            dst_entity_id INTEGER NOT NULL,
            edge_type TEXT NOT NULL,
            FOREIGN KEY (src_entity_id) REFERENCES code_entities (id),
            FOREIGN KEY (dst_entity_id) REFERENCES code_entities (id)
        )
        "#,
        [],
    )?;

    // Insert sample Rust entities
    db.execute(
        r#"
        INSERT INTO code_entities (file_path, entity_name, entity_type, language, start_line, end_line) VALUES
        ('src/main.rs', 'main', 'function', 'rust', 1, 10),
        ('src/main.rs', 'AppConfig', 'struct', 'rust', 12, 20),
        ('src/utils.rs', 'helper_function', 'function', 'rust', 1, 15),
        ('src/utils.rs', 'UtilityStruct', 'struct', 'rust', 17, 25),
        ('src/lib.rs', 'LibraryModule', 'module', 'rust', 1, 5)
        "#,
        [],
    )?;

    // Insert sample Python entities
    db.execute(
        r#"
        INSERT INTO code_entities (file_path, entity_name, entity_type, language, start_line, end_line) VALUES
        ('python/main.py', 'main', 'function', 'python', 1, 10),
        ('python/utils.py', 'helper_function', 'function', 'python', 1, 15),
        ('python/models.py', 'DataModel', 'class', 'python', 1, 20),
        ('python/config.py', 'Config', 'class', 'python', 1, 15)
        "#,
        [],
    )?;

    // Insert sample TypeScript entities
    db.execute(
        r#"
        INSERT INTO code_entities (file_path, entity_name, entity_type, language, start_line, end_line) VALUES
        ('frontend/src/App.tsx', 'App', 'component', 'typescript', 1, 25),
        ('frontend/src/utils.ts', 'formatDate', 'function', 'typescript', 1, 10),
        ('frontend/src/types.ts', 'UserType', 'interface', 'typescript', 1, 15),
        ('frontend/src/components/Button.tsx', 'Button', 'component', 'typescript', 1, 20)
        "#,
        [],
    )?;

    // Get entity IDs for relationships
    let mut stmt = db.prepare("SELECT id, file_path, entity_name FROM code_entities")?;
    let entity_map: std::collections::HashMap<(String, String), i64> = stmt
        .query_map([], |row| {
            Ok((
                (row.get::<_, String>(1)?, row.get::<_, String>(2)?), // (file_path, entity_name)
                row.get::<_, i64>(0)?,                                // id
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .collect();

    // Insert sample relationships (Rust)
    if let (Some(&main_id), Some(&config_id), Some(&helper_id)) = (
        entity_map.get(&("src/main.rs".to_string(), "main".to_string())),
        entity_map.get(&("src/main.rs".to_string(), "AppConfig".to_string())),
        entity_map.get(&("src/utils.rs".to_string(), "helper_function".to_string())),
    ) {
        db.execute(
            "INSERT INTO code_edges (src_entity_id, dst_entity_id, edge_type) VALUES (?1, ?2, 'uses')",
            [main_id, config_id],
        )?;
        db.execute(
            "INSERT INTO code_edges (src_entity_id, dst_entity_id, edge_type) VALUES (?1, ?2, 'calls')",
            [main_id, helper_id],
        )?;
    }

    // Insert sample relationships (Python)
    if let (Some(&py_main_id), Some(&py_config_id), Some(&py_model_id)) = (
        entity_map.get(&("python/main.py".to_string(), "main".to_string())),
        entity_map.get(&("python/config.py".to_string(), "Config".to_string())),
        entity_map.get(&("python/models.py".to_string(), "DataModel".to_string())),
    ) {
        db.execute(
            "INSERT INTO code_edges (src_entity_id, dst_entity_id, edge_type) VALUES (?1, ?2, 'imports')",
            [py_main_id, py_config_id],
        )?;
        db.execute(
            "INSERT INTO code_edges (src_entity_id, dst_entity_id, edge_type) VALUES (?1, ?2, 'instantiates')",
            [py_main_id, py_model_id],
        )?;
    }

    // Insert sample relationships (TypeScript)
    if let (Some(&app_id), Some(&utils_id), Some(&types_id)) = (
        entity_map.get(&("frontend/src/App.tsx".to_string(), "App".to_string())),
        entity_map.get(&(
            "frontend/src/utils.ts".to_string(),
            "formatDate".to_string(),
        )),
        entity_map.get(&("frontend/src/types.ts".to_string(), "UserType".to_string())),
    ) {
        db.execute(
            "INSERT INTO code_edges (src_entity_id, dst_entity_id, edge_type) VALUES (?1, ?2, 'imports')",
            [app_id, utils_id],
        )?;
        db.execute(
            "INSERT INTO code_edges (src_entity_id, dst_entity_id, edge_type) VALUES (?1, ?2, 'uses')",
            [app_id, types_id],
        )?;
    }

    Ok(())
}

#[tokio::test]
async fn test_unified_dependency_summary_basic() -> Result<()> {
    let (db_manager, _temp_dir) = setup_test_db().await?;
    let engine = ProjectAnalysisEngine::new(db_manager, None);

    let summary = engine.build_unified_dependency_summary(None, None)?;

    // Verify basic structure
    assert!(!summary.modules.is_empty(), "Should have modules");
    assert!(!summary.dependencies.is_empty(), "Should have dependencies");

    // Count languages
    let languages: std::collections::HashSet<String> =
        summary.modules.iter().map(|m| m.language.clone()).collect();

    assert!(languages.contains("rust"), "Should contain Rust");
    assert!(languages.contains("python"), "Should contain Python");
    assert!(
        languages.contains("typescript"),
        "Should contain TypeScript"
    );

    Ok(())
}

#[tokio::test]
async fn test_unified_dependency_summary_max_modules() -> Result<()> {
    let (db_manager, _temp_dir) = setup_test_db().await?;
    let engine = ProjectAnalysisEngine::new(db_manager, None);

    let summary = engine.build_unified_dependency_summary(Some(3), None)?;

    // Should limit to 3 modules
    assert!(summary.modules.len() <= 3, "Should limit to 3 modules");

    Ok(())
}

#[tokio::test]
async fn test_unified_module_structure() -> Result<()> {
    let (db_manager, _temp_dir) = setup_test_db().await?;
    let engine = ProjectAnalysisEngine::new(db_manager, None);

    let summary = engine.build_unified_dependency_summary(None, None)?;

    // Find a Rust module
    let rust_module = summary
        .modules
        .iter()
        .find(|m| m.language == "rust")
        .expect("Should have at least one Rust module");

    // Verify module structure
    assert!(
        !rust_module.file_path.is_empty(),
        "File path should not be empty"
    );
    assert_eq!(rust_module.language, "rust", "Language should be rust");
    assert!(rust_module.entity_count > 0, "Should have entity count");

    Ok(())
}

#[tokio::test]
async fn test_cross_language_dependencies() -> Result<()> {
    let (db_manager, _temp_dir) = setup_test_db().await?;
    let engine = ProjectAnalysisEngine::new(db_manager, None);

    let summary = engine.build_unified_dependency_summary(None, None)?;

    // Group modules by language
    let mut lang_groups: std::collections::HashMap<String, Vec<&UnifiedModule>> =
        std::collections::HashMap::new();
    for module in &summary.modules {
        lang_groups
            .entry(module.language.clone())
            .or_default()
            .push(module);
    }

    // Verify we have multiple languages
    assert!(lang_groups.len() >= 3, "Should have at least 3 languages");

    // Verify each language has modules
    for (lang, modules) in &lang_groups {
        assert!(!modules.is_empty(), "Language {} should have modules", lang);
    }

    Ok(())
}

#[tokio::test]
async fn test_empty_database_handling() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("empty.db");
    let db_manager = Arc::new(DbManager::new(
        &db_path.to_string_lossy(),
        &db_path.to_string_lossy(),
    )?);

    // Create empty tables (but no data)
    // IMPORTANT: Scope the lock so it's dropped before calling engine methods
    {
        let conn = db_manager.code_graph_conn();
        let db = conn.lock().unwrap();

        db.execute(
            r#"
            CREATE TABLE IF NOT EXISTS code_entities (
                id INTEGER PRIMARY KEY,
                file_path TEXT NOT NULL,
                entity_name TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                language TEXT NOT NULL,
                start_line INTEGER,
                end_line INTEGER
            )
            "#,
            [],
        )?;

        db.execute(
            r#"
            CREATE TABLE IF NOT EXISTS code_edges (
                id INTEGER PRIMARY KEY,
                src_entity_id INTEGER NOT NULL,
                dst_entity_id INTEGER NOT NULL,
                edge_type TEXT NOT NULL,
                FOREIGN KEY (src_entity_id) REFERENCES code_entities (id),
                FOREIGN KEY (dst_entity_id) REFERENCES code_entities (id)
            )
            "#,
            [],
        )?;
    } // Lock released here

    let engine = ProjectAnalysisEngine::new(db_manager, None);

    let summary = engine.build_unified_dependency_summary(None, None)?;

    // Should handle empty database gracefully
    assert!(
        summary.modules.is_empty(),
        "Empty DB should have no modules"
    );
    assert!(
        summary.dependencies.is_empty(),
        "Empty DB should have no dependencies"
    );

    Ok(())
}

#[tokio::test]
async fn test_dependency_aggregation() -> Result<()> {
    let (db_manager, _temp_dir) = setup_test_db().await?;
    let engine = ProjectAnalysisEngine::new(db_manager, None);

    let summary = engine.build_unified_dependency_summary(None, None)?;

    // Verify dependency aggregation
    let mut total_edges = 0;
    for dep in &summary.dependencies {
        total_edges += dep.edge_count;

        // Verify edge types are aggregated
        assert!(
            !dep.edge_types.is_empty(),
            "Dependency should have aggregated edge types"
        );

        // Verify edge count matches edge types
        assert_eq!(
            dep.edge_types.len() as u32,
            dep.edge_count,
            "Edge count should match number of edge types"
        );
    }

    assert!(total_edges > 0, "Should have total edges");

    Ok(())
}

#[tokio::test]
async fn test_module_language_distribution() -> Result<()> {
    let (db_manager, _temp_dir) = setup_test_db().await?;
    let engine = ProjectAnalysisEngine::new(db_manager, None);

    let summary = engine.build_unified_dependency_summary(None, None)?;

    // Count entities per language
    let mut lang_counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    for module in &summary.modules {
        *lang_counts.entry(module.language.clone()).or_insert(0) += module.entity_count;
    }

    // Verify we have entities for each language
    assert!(
        lang_counts.contains_key("rust"),
        "Should have Rust entities"
    );
    assert!(
        lang_counts.contains_key("python"),
        "Should have Python entities"
    );
    assert!(
        lang_counts.contains_key("typescript"),
        "Should have TypeScript entities"
    );

    // Verify counts are positive
    for (lang, count) in &lang_counts {
        assert!(
            *count > 0,
            "Language {} should have positive entity count: {}",
            lang,
            count
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_dependency_edge_types() -> Result<()> {
    let (db_manager, _temp_dir) = setup_test_db().await?;
    let engine = ProjectAnalysisEngine::new(db_manager, None);

    let summary = engine.build_unified_dependency_summary(None, None)?;

    // Collect all edge types
    let mut edge_types: std::collections::HashSet<String> = std::collections::HashSet::new();
    for dep in &summary.dependencies {
        for edge_type in &dep.edge_types {
            edge_types.insert(edge_type.clone());
        }
    }

    // Verify we have expected edge types
    assert!(edge_types.contains("uses"), "Should have 'uses' edge type");
    assert!(
        edge_types.contains("calls"),
        "Should have 'calls' edge type"
    );
    assert!(
        edge_types.contains("imports"),
        "Should have 'imports' edge type"
    );

    Ok(())
}
