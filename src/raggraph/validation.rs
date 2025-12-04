//! Runtime validation for RagGraph real mode
//!
//! Ensures that when running in REAL mode, all dependencies are correctly configured
//! and available. Fails fast with clear errors instead of silently falling back to mocks.

use super::config::RaggraphBackendMode;
use crate::graph::{GraphBackend, Neo4jClient};
use crate::vector::traits::VectorIndex;
use anyhow::{Context, Result};
use std::sync::{Arc, Mutex};

/// Validation error for RagGraph real mode configuration
#[derive(Debug)]
pub enum ValidationError {
    GraphBackendUnavailable(String),
    VectorIndexEmpty,
    DimensionMismatch {
        expected: usize,
        actual: usize,
    },
    BackendMisconfigured(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::GraphBackendUnavailable(msg) => {
                write!(f, "RagGraph real mode requires a working graph backend (SQLiteGraph by default). Check GRAPH_BACKEND configuration: {}", msg)
            }
            ValidationError::VectorIndexEmpty => {
                write!(f, "Vector index is empty. Real mode requires a populated HNSW index with embeddings.")
            }
            ValidationError::DimensionMismatch {
                expected,
                actual,
            } => {
                write!(f, "Vector dimension mismatch: expected {}, got {}. Check embedding configuration.", expected, actual)
            }
            ValidationError::BackendMisconfigured(msg) => {
                write!(f, "Real mode backend misconfigured: {}", msg)
            }
        }
    }
}

impl std::error::Error for ValidationError {}

/// Validate generic graph backend connectivity with a simple health check query
pub async fn validate_graph_backend(backend: &dyn GraphBackend) -> Result<()> {
    // Use backend-agnostic health check: try to get basic statistics
    // This works for both SQLiteGraph (returns empty result) and Neo4j (returns stats)
    let query = "MATCH (n) RETURN count(n) as node_count LIMIT 1";

    backend
        .execute_query(query, vec![])
        .await
        .context("Failed to execute graph backend health check")
        .map_err(|e| ValidationError::GraphBackendUnavailable(e.to_string()))?;

    Ok(())
}

/// Validate Neo4j connectivity with a simple health check query (legacy support)
pub async fn validate_neo4j(client: &Neo4jClient) -> Result<()> {
    // Simple query to verify connection is alive and database is accessible
    let query = "RETURN 1 as health_check";

    client
        .execute_query(query, vec![])
        .await
        .context("Failed to execute Neo4j health check")
        .map_err(|e| ValidationError::GraphBackendUnavailable(e.to_string()))?;

    Ok(())
}

/// Validate vector index has data and correct dimensions
pub fn validate_vector_index(
    index: &Arc<Mutex<dyn VectorIndex>>,
    expected_dim: usize,
) -> Result<()> {
    let index_lock = index.lock().map_err(|e| {
        ValidationError::BackendMisconfigured(format!("Vector index lock poisoned: {}", e))
    })?;

    // Check index is not empty
    if index_lock.is_empty() {
        return Err(ValidationError::VectorIndexEmpty.into());
    }

    // Check dimension matches configuration
    if let Some(actual_dim) = index_lock.dimension() {
        if actual_dim != expected_dim {
            return Err(ValidationError::DimensionMismatch {
                expected: expected_dim,
                actual: actual_dim,
            }
            .into());
        }
    }

    Ok(())
}

/// Validate complete real mode backend configuration (generic GraphBackend version)
///
/// This should be called before executing any RAGGraph query in REAL mode.
/// If validation fails, returns a clear error that should be surfaced to the user.
pub async fn validate_real_backend(
    backend_mode: RaggraphBackendMode,
    graph_backend: Option<&dyn GraphBackend>,
    vector_index: Option<&Arc<Mutex<dyn VectorIndex>>>,
    expected_dim: usize,
) -> Result<()> {
    // Only validate in REAL mode
    if backend_mode != RaggraphBackendMode::Real {
        return Ok(());
    }

    // Validate Graph Backend
    let backend = graph_backend.ok_or_else(|| {
        ValidationError::BackendMisconfigured("Graph backend not provided for REAL mode".to_string())
    })?;
    validate_graph_backend(backend).await?;

    // Validate Vector Index
    let index = vector_index.ok_or_else(|| {
        ValidationError::BackendMisconfigured("Vector index not provided for REAL mode".to_string())
    })?;
    validate_vector_index(index, expected_dim)?;

    Ok(())
}

/// Legacy validation function for Neo4j-specific backends (deprecated)
///
/// Use the generic validate_real_backend instead.
#[deprecated(note = "Use generic validate_real_backend with GraphBackend instead")]
pub async fn validate_real_backend_neo4j(
    backend_mode: RaggraphBackendMode,
    neo4j_client: Option<&Neo4jClient>,
    vector_index: Option<&Arc<Mutex<dyn VectorIndex>>>,
    expected_dim: usize,
) -> Result<()> {
    // Only validate in REAL mode
    if backend_mode != RaggraphBackendMode::Real {
        return Ok(());
    }

    // Validate Neo4j
    let neo4j = neo4j_client.ok_or_else(|| {
        ValidationError::BackendMisconfigured("Neo4j client not provided for REAL mode".to_string())
    })?;
    validate_neo4j(neo4j).await?;

    // Validate Vector Index
    let index = vector_index.ok_or_else(|| {
        ValidationError::BackendMisconfigured("Vector index not provided for REAL mode".to_string())
    })?;
    validate_vector_index(index, expected_dim)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector::hnsw::{HnswConfig, HnswVectorIndex};

    #[test]
    fn test_validation_error_display() {
        let err = ValidationError::GraphBackendUnavailable("connection refused".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Graph backend unavailable"));
        assert!(msg.contains("GRAPH_BACKEND"));
    }

    #[test]
    fn test_validate_empty_vector_index() {
        // Create empty index
        let config = HnswConfig {
            m: 16,
            ef_construction: 200,
            ef_search: 50,
        };
        let index = HnswVectorIndex::new(config, 42).unwrap();
        let index_arc: Arc<Mutex<dyn VectorIndex>> = Arc::new(Mutex::new(index));

        // Should fail with empty index error
        let result = validate_vector_index(&index_arc, 384);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("empty"));
    }

    #[test]
    fn test_validate_dimension_mismatch() {
        // Create index with data
        let config = HnswConfig {
            m: 16,
            ef_construction: 200,
            ef_search: 50,
        };
        let mut index = HnswVectorIndex::new(config, 42).unwrap();

        // Add a 128-dim embedding
        let embedding = vec![0.1; 128];
        index.add(1, embedding).unwrap();

        let index_arc: Arc<Mutex<dyn VectorIndex>> = Arc::new(Mutex::new(index));

        // Validate with wrong expected dimension
        let result = validate_vector_index(&index_arc, 384);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("dimension mismatch"));
        assert!(err_msg.contains("384"));
        assert!(err_msg.contains("128"));
    }

    #[test]
    fn test_validate_correct_index() {
        // Create index with correct data
        let config = HnswConfig {
            m: 16,
            ef_construction: 200,
            ef_search: 50,
        };
        let mut index = HnswVectorIndex::new(config, 42).unwrap();

        // Add a 384-dim embedding
        let embedding = vec![0.1; 384];
        index.add(1, embedding).unwrap();

        let index_arc: Arc<Mutex<dyn VectorIndex>> = Arc::new(Mutex::new(index));

        // Should pass validation
        let result = validate_vector_index(&index_arc, 384);
        assert!(result.is_ok());
    }
}
