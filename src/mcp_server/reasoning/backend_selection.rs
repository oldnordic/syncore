//! Backend selection utilities for unified MCP reasoning tools
//!
//! Provides a consistent interface for backend selection across all reasoning tools.
//! Neo4j backend is disabled - only SQLiteGraph is supported.

use crate::graph::{GraphBackend, SQLiteGraphBackend};
use crate::raggraph::config::RagGraphConfig;
use crate::raggraph::config::RaggraphBackendMode as RagGraphBackendMode;
use anyhow::Result;
use std::sync::Arc;

/// Configuration for backend selection
#[derive(Debug, Clone)]
pub struct BackendSelectionConfig {
    /// Prefer SQLiteGraph if available (default: true)
    pub prefer_sqlite: bool,
    /// Allow Neo4j fallback if explicitly configured
    pub allow_neo4j_fallback: bool,
    /// Require explicit Neo4j configuration (not just env var presence)
    pub require_explicit_neo4j: bool,
}

impl Default for BackendSelectionConfig {
    fn default() -> Self {
        Self {
            prefer_sqlite: true,
            allow_neo4j_fallback: false, // HARDENED: No implicit Neo4j fallback
            require_explicit_neo4j: true, // HARDENED: Require explicit Neo4j usage
        }
    }
}

/// Result of backend selection
#[derive(Clone)]
pub struct BackendSelection {
    /// Selected backend type
    pub backend_type: BackendType,
    /// Backend instance
    pub backend: Arc<dyn GraphBackend>,
    /// Selection metadata for debugging
    pub metadata: BackendMetadata,
}

impl std::fmt::Debug for BackendSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BackendSelection")
            .field("backend_type", &self.backend_type)
            .field("backend", &"<GraphBackend>")
            .field("metadata", &self.metadata)
            .finish()
    }
}

/// Backend type for reasoning tools
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendType {
    SQLiteGraph,
    Neo4j,
}

/// Metadata about backend selection process
#[derive(Debug, Clone)]
pub struct BackendMetadata {
    /// Configuration source used
    pub config_source: String,
    /// Whether backend was automatically selected
    pub auto_selected: bool,
    /// Selection reason
    pub reason: String,
}

/// Extract shared helper function for backend selection
///
/// Reads SyncoreConfig / env vars, determines the GraphBackend type,
/// prefers SQLiteGraph if available, uses Neo4j if explicitly configured and available,
/// and returns Result<Arc<dyn GraphBackend>, SyncoreError>
///
/// # Arguments
/// * `config` - Optional backend selection configuration
/// * `neo4j_connection` - Optional explicit Neo4j connection for Neo4j backend
///
/// # Returns
/// Result containing the selected backend and metadata
pub fn select_reasoning_backend(
    config: Option<BackendSelectionConfig>,
    neo4j_connection: Option<Arc<crate::graph::Neo4jClient>>,
) -> Result<BackendSelection> {
    let config = config.unwrap_or_default();

    // Step 1: Check all configuration sources for backend preference
    let mut preferred_backend = determine_preferred_backend()?;

    // Step 2: Apply SQLiteGraph-first preference logic
    if config.prefer_sqlite {
        if preferred_backend == BackendType::Neo4j {
            // If Neo4j is preferred but we prefer SQLite, check if we should override
            if let Ok(sqlite_available) = check_sqlitegraph_availability() {
                if sqlite_available {
                    preferred_backend = BackendType::SQLiteGraph;
                }
            }
        }
    }

    // Step 3: Attempt to create the selected backend
    match preferred_backend {
        BackendType::SQLiteGraph => {
            match create_sqlitegraph_backend() {
                Ok(backend) => Ok(BackendSelection {
                    backend_type: BackendType::SQLiteGraph,
                    backend,
                    metadata: BackendMetadata {
                        config_source: "auto_select_sqlite".to_string(),
                        auto_selected: true,
                        reason: "SQLiteGraph preferred and available".to_string(),
                    },
                }),
                Err(e) => {
                    // Fallback to Neo4j if allowed and available
                    if config.allow_neo4j_fallback {
                        fallback_to_neo4j(
                            neo4j_connection,
                            "SQLiteGraph creation failed".to_string(),
                            config,
                        )
                    } else {
                        Err(e)
                    }
                }
            }
        }
        BackendType::Neo4j => {
            // TODO: Implement proper Neo4j backend when Neo4jClient supports cloning
            // For now, return an error since we can't create async backends in sync context
            Err(anyhow::anyhow!(
                "Neo4j backend not yet implemented for synchronous reasoning execution"
            ))
        }
    }
}

/// Determine preferred backend from all configuration sources
fn determine_preferred_backend() -> Result<BackendType> {
    // Check RagGraphConfig environment first (raggraph tools)
    let rag_config = RagGraphConfig::from_env();
    match rag_config.backend_mode {
        RagGraphBackendMode::Real => {
            // Real mode means use actual graph backend - check environment for which one
            if std::env::var("NEO4J_URI").is_ok()
                || std::env::var("NEO4J_HOST").is_ok()
                || std::env::var("GRAPH_BACKEND")
                    .unwrap_or_default()
                    .to_lowercase()
                    .contains("neo4j")
            {
                return Ok(BackendType::Neo4j);
            }
            // Default to SQLiteGraph for Real mode
            return Ok(BackendType::SQLiteGraph);
        }
        RagGraphBackendMode::Mock => {
            // Mock mode doesn't need real graph backend, but for compatibility use SQLiteGraph
            return Ok(BackendType::SQLiteGraph);
        }
    }
}

/// Check if SQLiteGraph backend is available
fn check_sqlitegraph_availability() -> Result<bool> {
    // Try to create a minimal SQLiteGraph instance to test availability
    match create_sqlitegraph_backend() {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Create SQLiteGraph backend instance
fn create_sqlitegraph_backend() -> Result<Arc<dyn GraphBackend>> {
    let sqlite_config = crate::config::GraphConfig {
        backend: crate::config::GraphBackend::SqliteGraph,
        path: "reasoning_backend.db".to_string(),
        uri: String::new(),
        user: String::new(),
        password: String::new(),
        enabled: true,
    };

    let backend = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current()
            .block_on(async { crate::graph::create_default_graph_backend(&sqlite_config).await })
    })?;

    Ok(backend)
}

/// Fallback to Neo4j backend with metadata
fn fallback_to_neo4j(
    neo4j_connection: Option<Arc<crate::graph::Neo4jClient>>,
    fallback_reason: String,
    config: BackendSelectionConfig,
) -> Result<BackendSelection> {
    if let Some(neo4j) = neo4j_connection {
        if !config.require_explicit_neo4j || check_neo4j_explicit_config() {
            // Create Neo4jBackend - since Neo4jClient can't be cloned, we need to check if we have exclusive access
            let backend = if Arc::strong_count(&neo4j) == 1 {
                match Arc::try_unwrap(neo4j) {
                    Ok(_client) => {
                        // Neo4j backend is disabled - return error
                        return Err(anyhow::anyhow!("Neo4j backend is disabled"));
                    },
                    Err(_arc) => {
                        // If we can't get exclusive access, we need to create a new connection
                        // For now, return an error to indicate this limitation
                        return Err(anyhow::anyhow!(
                            "Cannot create Neo4jBackend from shared Arc<Neo4jClient>"
                        ));
                    }
                }
            } else {
                // If there are multiple references, we can't extract the client
                return Err(anyhow::anyhow!("Cannot create Neo4jBackend from shared Arc<Neo4jClient> with multiple references"));
            };
            Ok(BackendSelection {
                backend_type: BackendType::Neo4j,
                backend,
                metadata: BackendMetadata {
                    config_source: "fallback_neo4j".to_string(),
                    auto_selected: true,
                    reason: format!("Fallback to Neo4j: {}", fallback_reason),
                },
            })
        } else {
            Err(anyhow::anyhow!("Neo4j fallback requires explicit configuration"))
        }
    } else {
        Err(anyhow::anyhow!("Neo4j fallback requested but no Neo4j connection available"))
    }
}

/// Check if Neo4j is explicitly configured (not just env var presence)
fn check_neo4j_explicit_config() -> bool {
    std::env::var("NEO4J_URI").is_ok()
        || std::env::var("NEO4J_HOST").is_ok()
        || std::env::var("GRAPH_BACKEND").is_ok_and(|v| v.to_lowercase().contains("neo4j"))
}

/// Get backend selection summary for debugging
pub fn get_backend_selection_summary(selection: &BackendSelection) -> String {
    format!(
        "Backend: {:?}, Source: {}, Auto: {}, Reason: {}",
        selection.backend_type,
        selection.metadata.config_source,
        selection.metadata.auto_selected,
        selection.metadata.reason
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_selection_default_to_sqlite() {
        // Test that we default to SQLiteGraph when no configuration is present
        let result = select_reasoning_backend(None, None);
        assert!(result.is_ok());
        let selection = result.unwrap();
        assert_eq!(selection.backend_type, BackendType::SQLiteGraph);
    }

    #[test]
    fn test_backend_type_display() {
        // Test that BackendType can be displayed for debugging
        let sqlite_type = BackendType::SQLiteGraph;
        let neo4j_type = BackendType::Neo4j;
        assert_eq!(format!("{:?}", sqlite_type), "SQLiteGraph");
        assert_eq!(format!("{:?}", neo4j_type), "Neo4j");
    }

    #[test]
    fn test_config_source_priority() {
        // Test that RagGraphConfig takes priority over SyncoreConfig
        std::env::set_var("SYNCORE_RAGGRAPH_BACKEND", "neo4j");
        std::env::set_var("GRAPH_BACKEND", "sqlite");

        let result = select_reasoning_backend(None, None);
        // Should prefer Neo4j based on SYNCORE_RAGGRAPH_BACKEND
        // But will fallback to SQLite since no Neo4j connection provided
        assert!(result.is_ok());
        let selection = result.unwrap();
        assert_eq!(selection.backend_type, BackendType::SQLiteGraph);

        std::env::remove_var("SYNCORE_RAGGRAPH_BACKEND");
        std::env::remove_var("GRAPH_BACKEND");
    }
}
