//! Neo4j Backend Adapter - FROZEN
//!
//! Neo4j integration is disabled. All methods return errors or stubs.
//! This file exists only for compilation compatibility.

use anyhow::Result;
use std::collections::HashMap;

/// Neo4j backend adapter stub - disabled
#[derive(Clone)]
pub struct Neo4jBackendAdapter {
    namespace: String,
}

impl std::fmt::Debug for Neo4jBackendAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Neo4jBackendAdapter (frozen)")
            .field("namespace", &self.namespace)
            .finish()
    }
}

impl Neo4jBackendAdapter {
    /// Create a Neo4j backend adapter with frozen namespace
    pub fn with_namespace(namespace: String) -> Self {
        Self { namespace }
    }
}

// Note: The GraphBackend trait implementation is removed since Neo4j is disabled
// All methods would just return "Neo4j backend is disabled" errors anyway

// Stub types that might be referenced elsewhere
pub type EntityResult = ();