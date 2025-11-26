//! Simple connection tests for PAE
#[cfg(test)]
mod simple_connection_tests {
    use std::sync::Arc;
    use syncore::db::DbManager;
    use syncore::project_analysis::ProjectAnalysisEngine;

    #[test]
    fn test_simple_pae_creation() {
        let test_dir = "/tmp/simple_test";

        // Clean up any existing test directory
        let _ = std::fs::remove_dir_all(test_dir);

        std::fs::create_dir_all(test_dir).unwrap();
        std::fs::write(
            format!("{}/Cargo.toml", test_dir),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(format!("{}/src", test_dir)).unwrap();
        std::fs::write(format!("{}/src/main.rs", test_dir), "fn main() {}").unwrap();

        println!("Creating PAE for test directory: {}", test_dir);

        // Create DbManager first
        let db_manager = Arc::new(DbManager::new(":memory:", ":memory:").unwrap());

        // Test just creating the PAE instance
        let pae = ProjectAnalysisEngine::new(db_manager, None);
        println!("✓ PAE created successfully");

        // Test a simple database query
        let conn = pae.code_graph_conn();
        let db = conn.lock().unwrap();

        let query_result = db.prepare("SELECT COUNT(*) FROM code_entities");
        match query_result {
            Ok(_) => println!("✓ Database connection works"),
            Err(e) => println!("✗ Database query failed: {}", e),
        }

        // Clean up
        let _ = std::fs::remove_dir_all(test_dir);
    }
}
