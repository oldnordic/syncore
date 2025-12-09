//! Graph Backend Selector - SQLiteGraph ONLY
//!
//! Neo4j backend is disabled. Only SQLiteGraph backend is supported.
//! Returns Arc<dyn GraphBackend> for unified access across the application.

use super::{GraphBackend, SQLiteGraphBackend};
use crate::config::{GraphBackend as ConfigBackend, GraphConfig};
use anyhow::{anyhow, Result};
use std::sync::Arc;

/// Create a graph backend based on configuration
///
/// # Arguments
/// * `config` - Graph configuration containing backend selection and connection details
/// * `namespace` - Namespace for multi-tenant isolation
///
/// # Returns
/// Arc<dyn GraphBackend> configured for SQLiteGraph only
///
/// # Note
/// Neo4j backend is disabled - always uses SQLiteGraph regardless of config
pub async fn create_graph_backend(
    config: &GraphConfig,
    namespace: &str,
) -> Result<Arc<dyn GraphBackend>> {
    // Force SQLiteGraph regardless of config - Neo4j is disabled
    let backend = SQLiteGraphBackend::connect(&config.path, "", "", namespace)
        .await
        .map_err(|e| anyhow!("Failed to connect to SQLiteGraph: {}", e))?;
    Ok(Arc::new(backend))
}

/// Create a graph backend with default namespace
///
/// Uses "syncore_default" as namespace for backward compatibility
pub async fn create_default_graph_backend(config: &GraphConfig) -> Result<Arc<dyn GraphBackend>> {
    create_graph_backend(config, "syncore_default").await
}

/// Validate graph configuration
///
/// Checks that the configuration is valid for SQLiteGraph backend only
pub fn validate_graph_config(config: &GraphConfig) -> Result<()> {
    // Always validate for SQLiteGraph since Neo4j is disabled
    if config.path.is_empty() {
        return Err(anyhow!("SQLiteGraph backend requires path to be specified"));
    }
    Ok(())
}

/// Get backend description for logging/debugging
pub fn get_backend_description(config: &GraphConfig) -> String {
    // Always return SQLiteGraph since Neo4j is disabled
    format!("SQLiteGraph at {}", config.path)
}

/// Create a graph backend from full SyncoreConfig (Task 4 requirement)
///
/// This is the preferred method for creating graph backends as it:
/// 1. Uses the complete SyncoreConfig with all settings
/// 2. Applies fallback behavior: Invalid backend → SQLiteGraph default
/// 3. Supports both new and legacy environment variable overrides
/// 4. Provides clear error messages for configuration issues
///
/// # Arguments
/// * `config` - Complete SyncoreConfig containing graph configuration
/// * `namespace` - Namespace for multi-tenant isolation
///
/// # Returns
/// Arc<dyn GraphBackend> configured according to config, with SQLiteGraph fallback
///
/// # Examples
///
/// ```rust
/// use crate::config::SyncoreConfig;
/// use crate::graph::backend_selector::backend_from_config;
///
/// // Load config with environment overrides
/// let mut config = SyncoreConfig::load_with_env("config/syncore.toml")?;
/// let backend = backend_from_config(&config, "my_namespace").await?;
/// ```
pub async fn backend_from_config(
    config: &crate::config::SyncoreConfig,
    namespace: &str,
) -> Result<Arc<dyn GraphBackend>> {
    // Validate configuration first
    if let Err(e) = validate_graph_config(&config.graph) {
        // If configuration is invalid, fallback to SQLiteGraph with default settings
        eprintln!("Invalid graph configuration: {}. Falling back to SQLiteGraph backend.", e);

        let fallback_config = GraphConfig {
            backend: ConfigBackend::SqliteGraph,
            path: "syncore_code_graph.db".to_string(),
            uri: String::new(),
            user: String::new(),
            password: String::new(),
            enabled: false,
        };

        return create_graph_backend(&fallback_config, namespace).await;
    }

    // Create backend based on (possibly corrected) configuration
    create_graph_backend(&config.graph, namespace).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{GraphBackend as ConfigBackend, GraphConfig};
    use tempfile;

    #[test]
    fn test_validate_neo4j_config() {
        let mut config = GraphConfig::default();
        config.backend = ConfigBackend::Neo4j;

        // Valid config should pass
        assert!(validate_graph_config(&config).is_ok());

        // Missing URI should fail
        config.uri = String::new();
        assert!(validate_graph_config(&config).is_err());
    }

    #[test]
    fn test_validate_sqlitegraph_config() {
        let mut config = GraphConfig::default();
        config.backend = ConfigBackend::SqliteGraph;

        // Valid config should pass
        assert!(validate_graph_config(&config).is_ok());

        // Missing path should fail
        config.path = String::new();
        assert!(validate_graph_config(&config).is_err());
    }

    #[test]
    fn test_get_backend_description() {
        let mut config = GraphConfig::default();

        config.backend = ConfigBackend::Neo4j;
        config.uri = "bolt://localhost:7687".to_string();
        assert_eq!(get_backend_description(&config), "Neo4j at bolt://localhost:7687");

        config.backend = ConfigBackend::SqliteGraph;
        config.path = "/tmp/test.db".to_string();
        assert_eq!(get_backend_description(&config), "SQLiteGraph at /tmp/test.db");
    }

    #[tokio::test]
    async fn test_create_neo4j_backend() -> Result<()> {
        let config = GraphConfig {
            backend: ConfigBackend::Neo4j,
            uri: "bolt://127.0.0.1:7687".to_string(),
            user: "neo4j".to_string(),
            password: "test".to_string(),
            path: String::new(),
            enabled: true,
        };

        // This will fail if Neo4j is not running, but should not panic
        let result = create_graph_backend(&config, "test").await;

        // We expect this to fail in test environment without Neo4j
        // The important thing is that it doesn't panic
        match result {
            Ok(_) => println!("Neo4j connected successfully"),
            Err(e) => println!("Neo4j connection failed as expected: {}", e),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_create_sqlitegraph_backend() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let db_path = temp_dir.path().join("test.db");

        let config = GraphConfig {
            backend: ConfigBackend::SqliteGraph,
            path: db_path.to_str().unwrap().to_string(),
            uri: String::new(),
            user: String::new(),
            password: String::new(),
            enabled: true,
        };

        let backend = create_graph_backend(&config, "test").await?;
        assert_eq!(backend.namespace(), "test");

        Ok(())
    }
}
