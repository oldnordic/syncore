//! Hang isolation tests for PAE methods
//!
//! Tests individual PAE methods to identify potential hangs.

#[cfg(test)]
mod hang_isolation_tests {
    use std::sync::Arc;
    use std::time::Instant;
    use syncore::db::DbManager;
    use syncore::project_analysis::ProjectAnalysisEngine;
    use tempfile::TempDir;

    fn create_test_db() -> (TempDir, Arc<DbManager>) {
        let temp_dir = TempDir::new().unwrap();
        let main_db_path = temp_dir.path().join("main.db");
        let code_graph_db_path = temp_dir.path().join("code_graph.db");

        let db_manager = Arc::new(
            DbManager::new(main_db_path.to_str().unwrap(), code_graph_db_path.to_str().unwrap())
                .unwrap(),
        );

        // Create minimal schema
        let conn = db_manager.code_graph_conn();
        let conn_guard = conn.lock().unwrap();
        conn_guard
            .execute_batch(
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
                UNIQUE(file_path, entity_type, name, line_start)
            );
            CREATE INDEX IF NOT EXISTS idx_entities_name ON code_entities(name);
            CREATE INDEX IF NOT EXISTS idx_entities_file ON code_entities(file_path);

            CREATE TABLE IF NOT EXISTS code_edges (
                src_entity_id INTEGER NOT NULL,
                dst_entity_id INTEGER NOT NULL,
                edge_type TEXT NOT NULL,
                PRIMARY KEY (src_entity_id, dst_entity_id, edge_type)
            );
            "#,
            )
            .unwrap();

        drop(conn_guard);
        (temp_dir, db_manager)
    }

    #[tokio::test]
    async fn test_individual_pae_methods() {
        let (_temp_dir, db_manager) = create_test_db();

        println!("Testing individual PAE methods...");

        // Test 1: Dead code analysis
        println!("\n1. Testing dead code method...");
        let start = Instant::now();
        let engine = ProjectAnalysisEngine::new(db_manager.clone(), None);
        let request = syncore::project_analysis::dead_code::DeadCodeRequest {
            exclude_public: Some(false),
            limit: Some(10),
        };
        let result = engine.dead_code(request).await;
        let duration = start.elapsed();

        match result {
            Ok(_) => println!("✓ Dead code completed in {:?}", duration),
            Err(e) => println!("✗ Dead code failed: {:?}", e),
        }

        // Test 2: Hotspots
        println!("\n2. Testing hotspots method...");
        let start = Instant::now();
        let request = syncore::project_analysis::hotspots::HotspotsRequest {
            limit: 10,
            min_loc: None,
            min_fan_in: None,
            min_fan_out: None,
            min_entity_count: None,
        };
        let result = engine.hotspots(request).await;
        let duration = start.elapsed();

        match result {
            Ok(_) => println!("✓ Hotspots completed in {:?}", duration),
            Err(e) => println!("✗ Hotspots failed: {:?}", e),
        }

        // Test 3: Cycles
        println!("\n3. Testing cycles method...");
        let start = Instant::now();
        let request = syncore::project_analysis::cycles::CyclesRequest {
            max_cycles: 10,
            max_depth: 5,
        };
        let result = engine.cycles(request).await;
        let duration = start.elapsed();

        match result {
            Ok(_) => println!("✓ Cycles completed in {:?}", duration),
            Err(e) => println!("✗ Cycles failed: {:?}", e),
        }
    }
}
