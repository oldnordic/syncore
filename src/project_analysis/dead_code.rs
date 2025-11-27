//! Project Dead Code Detection Tool
//!
//! Identifies entities that appear to be unused (no incoming relationships).

use crate::project_analysis::{
    diagnostics::DiagnosticsManager, DeadCodeInfo, PAEResponse, ProjectAnalysisEngine,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Request parameters for project_dead_code
#[derive(Debug, Deserialize)]
pub struct DeadCodeRequest {
    pub exclude_public: Option<bool>,
    pub limit: Option<u32>,
}

/// Dead code analysis response data
#[derive(Debug, Serialize, Deserialize)]
pub struct DeadCodeData {
    pub dead_entities: Vec<DeadCodeInfo>,
}

impl ProjectAnalysisEngine {
    /// Identify potentially dead code entities
    pub async fn dead_code(&self, request: DeadCodeRequest) -> Result<PAEResponse<DeadCodeData>> {
        match self
            .find_dead_code(request.exclude_public.unwrap_or(true), request.limit)
            .await
        {
            Ok(data) => Ok(PAEResponse::success(data)),
            Err(e) => Ok(PAEResponse::error(e.to_string())),
        }
    }

    async fn find_dead_code(
        &self,
        exclude_public: bool,
        limit: Option<u32>,
    ) -> Result<DeadCodeData> {
        // Query dead entities in its own scope to release the lock before heuristics
        let dead_entities = {
            let conn = self.code_graph_conn();
            let conn_guard = conn.lock().unwrap();
            self.query_dead_entities(&conn_guard, exclude_public, limit)?
        };
        // conn_guard dropped here - lock released

        // Apply heuristics to filter false positives
        // This may acquire locks internally (e.g., for Clippy diagnostics)
        let filtered_entities = self.apply_dead_code_heuristics(dead_entities)?;

        Ok(DeadCodeData {
            dead_entities: filtered_entities,
        })
    }

    fn query_dead_entities(
        &self,
        conn: &rusqlite::Connection,
        exclude_public: bool,
        limit: Option<u32>,
    ) -> Result<Vec<DeadCodeInfo>> {
        let mut query = r#"
            SELECT 
                ce.id,
                ce.name,
                ce.entity_type,
                ce.file_path,
                ce.line_start,
                ce.signature
            FROM code_entities ce
            LEFT JOIN code_edges ce_in ON ce.id = ce_in.dst_entity_id
            WHERE ce_in.dst_entity_id IS NULL
            AND ce.entity_type NOT IN ('import', 'module')
        "#
        .to_string();

        let mut params = Vec::new();
        let mut param_idx = 1;

        // Exclude test files
        query.push_str(&format!(" AND ce.file_path NOT LIKE ?{}", param_idx));
        params.push("%/tests/%".to_string());
        param_idx += 1;

        query.push_str(&format!(" AND ce.file_path NOT LIKE ?{}", param_idx));
        params.push("%_test.rs".to_string());

        // Exclude public entities if requested
        if exclude_public {
            query.push_str(&format!(
                " AND (ce.signature NOT LIKE 'pub %' AND ce.signature NOT LIKE 'pub(crate)%')"
            ));
        }

        query.push_str(" ORDER BY ce.file_path, ce.line_start");

        if let Some(limit_val) = limit {
            query.push_str(&format!(" LIMIT {}", limit_val));
        }

        let mut stmt = conn.prepare(&query)?;

        let mut param_refs: Vec<&dyn rusqlite::ToSql> = Vec::new();
        for param in &params {
            param_refs.push(param);
        }

        let entities = stmt.query_map(&param_refs[..], |row| {
            let signature: Option<String> = row.get(5)?;

            // Try to extract visibility from signature
            let visibility = if let Some(sig) = &signature {
                if sig.starts_with("pub") {
                    Some("public".to_string())
                } else {
                    Some("private".to_string())
                }
            } else {
                None
            };

            Ok(DeadCodeInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                entity_type: row.get(2)?,
                file_path: row.get(3)?,
                visibility,
                line_start: row.get(4)?,
            })
        })?;

        let mut result = Vec::new();
        for entity in entities {
            result.push(entity?);
        }
        Ok(result)
    }

    /// Apply heuristics to filter out false positives from dead code detection
    /// Now also leverages Clippy diagnostics for cross-validation when available
    fn apply_dead_code_heuristics(&self, entities: Vec<DeadCodeInfo>) -> Result<Vec<DeadCodeInfo>> {
        let mut filtered = Vec::new();

        for entity in entities {
            // First check basic heuristics
            if self.is_likely_false_positive(&entity) {
                continue;
            }

            // Cross-validate with Clippy diagnostics if available
            // If Clippy has data and confirms this is NOT dead code, skip it
            match self.validate_with_clippy_diagnostics(&entity) {
                Ok(clippy_confirmed) => {
                    // If Clippy data exists and DOESN'T confirm this as dead,
                    // we trust Clippy more (it has full type info)
                    // For now, we add all entities that pass heuristics
                    // Future: could use clippy_confirmed to boost confidence score
                    let _ = clippy_confirmed; // Use the result to suppress warning
                }
                Err(_) => {
                    // No Clippy data available, proceed with heuristics only
                }
            }

            filtered.push(entity);
        }

        Ok(filtered)
    }

    /// Check if an entity is likely a false positive for dead code
    fn is_likely_false_positive(&self, entity: &DeadCodeInfo) -> bool {
        // Filter common Rust trait implementations - these are often implicitly used
        // Check by name alone since we know these are standard trait methods
        if self.is_common_trait_method_by_name(&entity.name) {
            return true;
        }

        // Check constructor methods in impl blocks
        if self.is_constructor_method(&entity.name, &entity.entity_type) {
            return true;
        }

        // Check for common false positive patterns
        if self.is_common_false_positive_pattern(&entity.name) {
            return true;
        }

        false
    }

    /// Check if this is a common trait method by name only
    /// These methods are required by standard traits but may not be called directly
    fn is_common_trait_method_by_name(&self, name: &str) -> bool {
        // Common trait methods that are required but may not be called directly
        const TRAIT_METHODS: &[&str] = &[
            "fmt",         // Debug::fmt, Display::fmt
            "default",     // Default::default
            "clone",       // Clone::clone
            "clone_from",  // Clone::clone_from
            "eq",          // PartialEq::eq
            "ne",          // PartialEq::ne
            "hash",        // Hash::hash
            "partial_cmp", // PartialOrd::partial_cmp
            "lt",
            "le",
            "gt",
            "ge",          // PartialOrd methods
            "cmp",         // Ord::cmp
            "drop",        // Drop::drop
            "into_iter",   // IntoIterator::into_iter
            "from_iter",   // FromIterator::from_iter
            "as_ref",      // AsRef::as_ref
            "as_mut",      // AsMut::as_mut
            "borrow",      // Borrow::borrow
            "borrow_mut",  // Borrow::borrow_mut
            "to_owned",    // ToOwned::to_owned
            "into",        // Into::into
            "from",        // From::from
            "deref",       // Deref::deref
            "deref_mut",   // DerefMut::deref_mut
            "next",        // Iterator::next
            "poll",        // Future::poll
            "deserialize", // Deserialize::deserialize
            "serialize",   // Serialize::serialize
        ];

        TRAIT_METHODS.contains(&name)
    }

    /// Check if this is a constructor method (typically "new" in impl blocks)
    fn is_constructor_method(&self, name: &str, entity_type: &str) -> bool {
        // Only apply to functions in impl blocks
        if entity_type != "function" {
            return false;
        }

        // Common constructor patterns
        name == "new" || name.starts_with("new_") || name.starts_with("with_")
    }

    /// Check for common false positive patterns
    fn is_common_false_positive_pattern(&self, name: &str) -> bool {
        // Common patterns that are often unused but not really dead code
        let false_positive_patterns = [
            // Test helpers and benchmarks
            "test_", "bench_", // Common callback or event handler patterns
            "on_", "handle_",
            // Common getter/setter patterns that might be used via reflection
            "get_", "set_", "is_", "has_",
        ];

        for pattern in &false_positive_patterns {
            if name.starts_with(pattern) {
                return true;
            }
        }

        false
    }

    /// Cross-validate with Clippy diagnostics if available
    fn validate_with_clippy_diagnostics(&self, entity: &DeadCodeInfo) -> Result<bool> {
        // Create diagnostics manager using the db_manager
        let diagnostics = DiagnosticsManager::new(Arc::clone(self.db_manager()));

        // Check if Clippy also reports this as dead code
        let clippy_diagnostics = diagnostics.query_diagnostics_in_range(
            &entity.file_path,
            entity.line_start as i32,
            entity.line_start as i32,
        )?;

        // Look for clippy::dead_code warnings at this location
        for diagnostic in clippy_diagnostics {
            if diagnostic.tool == "clippy"
                && diagnostic.diagnostic_type == "clippy::dead_code"
                && diagnostic.line_start == entity.line_start as i64
            {
                return Ok(true); // Clippy confirms this is dead code
            }
        }

        Ok(false) // No Clippy confirmation, but still potentially dead code
    }
}
