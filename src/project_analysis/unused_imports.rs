//! Project Unused Imports Detection Tool
//!
//! Identifies imports that are never used in their containing files.

use crate::project_analysis::{
    diagnostics::DiagnosticsManager, PAEResponse, ProjectAnalysisEngine, UnusedImportInfo,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Request parameters for project_unused_imports
#[derive(Debug, Deserialize)]
pub struct UnusedImportsRequest {
    pub file_path: Option<String>,
    pub limit: Option<u32>,
}

/// Unused imports analysis response data
#[derive(Debug, Serialize, Deserialize)]
pub struct UnusedImportsData {
    pub unused_imports: Vec<UnusedImportInfo>,
}

impl ProjectAnalysisEngine {
    /// Identify unused imports in the project
    pub async fn unused_imports(
        &self,
        request: UnusedImportsRequest,
    ) -> Result<PAEResponse<UnusedImportsData>> {
        match self
            .find_unused_imports(request.file_path, request.limit)
            .await
        {
            Ok(data) => Ok(PAEResponse::success(data)),
            Err(e) => Ok(PAEResponse::error(e.to_string())),
        }
    }

    async fn find_unused_imports(
        &self,
        file_path: Option<String>,
        limit: Option<u32>,
    ) -> Result<UnusedImportsData> {
        // Query unused imports in its own scope to release the lock before heuristics
        let unused_imports = {
            let conn = self.code_graph_conn();
            let conn_guard = conn.lock().unwrap();
            self.query_unused_imports(&conn_guard, file_path.as_deref(), limit)?
        };
        // conn_guard dropped here - lock released

        // Apply heuristics to filter false positives
        // This may acquire locks internally (e.g., for Clippy diagnostics)
        let filtered_imports = self.apply_unused_imports_heuristics(unused_imports)?;

        Ok(UnusedImportsData {
            unused_imports: filtered_imports,
        })
    }

    fn query_unused_imports(
        &self,
        conn: &rusqlite::Connection,
        file_path: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<UnusedImportInfo>> {
        let mut query = r#"
            SELECT 
                ce.file_path,
                ce.name,
                ce.line_start,
                ce.signature
            FROM code_entities ce
            WHERE ce.entity_type = 'import'
            AND NOT EXISTS (
                SELECT 1 FROM code_edges ce_use
                JOIN code_entities ce_used ON ce_use.dst_entity_id = ce_used.id
                WHERE ce_use.src_entity_id IN (
                    SELECT id FROM code_entities 
                    WHERE file_path = ce.file_path 
                    AND entity_type != 'import'
                )
                AND (
                    ce_used.name = ce.name 
                    OR ce_used.name LIKE '%' || ce.name || '%'
                    OR ce.signature LIKE '%' || ce.name || '%'
                )
            )
        "#
        .to_string();

        let mut params = Vec::new();
        let mut param_idx = 1;

        if let Some(fp) = file_path {
            query.push_str(&format!(" AND ce.file_path = ?{}", param_idx));
            params.push(fp.to_string());
            param_idx += 1;
        }

        // Exclude test files
        query.push_str(&format!(" AND ce.file_path NOT LIKE ?{}", param_idx));
        params.push("%/tests/%".to_string());
        param_idx += 1;

        query.push_str(&format!(" AND ce.file_path NOT LIKE ?{}", param_idx));
        params.push("%_test.rs".to_string());
        param_idx += 1;

        query.push_str(" ORDER BY ce.file_path, ce.line_start");

        if let Some(limit_val) = limit {
            query.push_str(&format!(" LIMIT {}", limit_val));
        }

        let mut stmt = conn.prepare(&query)?;

        let mut param_refs: Vec<&dyn rusqlite::ToSql> = Vec::new();
        for param in &params {
            param_refs.push(param);
        }

        let imports = stmt.query_map(&param_refs[..], |row| {
            let file_path: String = row.get(0)?;
            let name: String = row.get(1)?;
            let line: Option<i32> = row.get(2)?;
            let signature: Option<String> = row.get(3)?;

            // Extract module from signature if available
            let module = signature.as_ref().and_then(|sig| {
                // Try to extract module from patterns like "use std::collections::HashMap;"
                if let Some(start) = sig.find("use ") {
                    let after_use = &sig[start + 4..];
                    if let Some(end) = after_use.find(';') {
                        Some(after_use[..end].to_string())
                    } else {
                        Some(after_use.to_string())
                    }
                } else {
                    None
                }
            });

            Ok(UnusedImportInfo {
                file_path,
                import_name: name,
                line,
                module,
            })
        })?;

        let mut result = Vec::new();
        for import in imports {
            result.push(import?);
        }
        Ok(result)
    }

    /// Apply heuristics to filter out false positives from unused imports detection
    /// Also leverages Clippy diagnostics for cross-validation when available
    fn apply_unused_imports_heuristics(
        &self,
        imports: Vec<UnusedImportInfo>,
    ) -> Result<Vec<UnusedImportInfo>> {
        let mut filtered = Vec::new();

        for import in imports {
            // First check basic heuristics
            if self.is_likely_used_import(&import) {
                continue;
            }

            // Cross-validate with Clippy diagnostics if available
            match self.validate_import_with_clippy(&import) {
                Ok(_clippy_confirmed) => {
                    // If Clippy confirms this as unused, keep it in the list
                    // If Clippy doesn't confirm but graph says unused, still include
                    // Future: could use clippy_confirmed to boost confidence score
                }
                Err(_) => {
                    // No Clippy data available, proceed with heuristics only
                }
            }

            filtered.push(import);
        }

        Ok(filtered)
    }

    /// Check if an import is likely actually used (false positive)
    fn is_likely_used_import(&self, import: &UnusedImportInfo) -> bool {
        // Prelude imports are always used implicitly
        if self.is_prelude_import(&import.import_name) {
            return true;
        }

        // Derive macro imports are often used implicitly
        if self.is_derive_import(&import.import_name) {
            return true;
        }

        false
    }

    /// Check if this is a prelude-style import that's always implicitly used
    fn is_prelude_import(&self, name: &str) -> bool {
        const PRELUDE_PATTERNS: &[&str] = &[
            "prelude",    // std::prelude, etc.
            "Sized",      // Core trait
            "Send",       // Core trait
            "Sync",       // Core trait
            "Unpin",      // Core trait
            "Drop",       // Core trait
            "Copy",       // Core trait
            "Clone",      // Core trait
            "Default",    // Core trait
            "PartialEq",  // Core trait
            "Eq",         // Core trait
            "PartialOrd", // Core trait
            "Ord",        // Core trait
            "Hash",       // Core trait
            "Debug",      // Core trait
            "Display",    // Core trait
        ];

        for pattern in PRELUDE_PATTERNS {
            if name.contains(pattern) {
                return true;
            }
        }

        false
    }

    /// Check if this is a derive macro import
    fn is_derive_import(&self, name: &str) -> bool {
        // Common derive macros that may appear unused but are used by #[derive(...)]
        const DERIVE_MACROS: &[&str] = &[
            "Serialize",
            "Deserialize",
            "Debug",
            "Clone",
            "Copy",
            "Default",
            "PartialEq",
            "Eq",
            "Hash",
            "PartialOrd",
            "Ord",
        ];

        DERIVE_MACROS.contains(&name)
    }

    /// Cross-validate with Clippy diagnostics if available
    fn validate_import_with_clippy(&self, import: &UnusedImportInfo) -> Result<bool> {
        // Create diagnostics manager using the db_manager
        let diagnostics = DiagnosticsManager::new(Arc::clone(self.db_manager()));

        // Check if Clippy also reports this as unused
        let line = import.line.unwrap_or(0);
        let clippy_diagnostics =
            diagnostics.query_diagnostics_in_range(&import.file_path, line, line)?;

        // Look for unused_imports warnings at this location
        for diagnostic in clippy_diagnostics {
            if diagnostic.tool == "clippy"
                && (diagnostic.diagnostic_type == "unused_imports"
                    || diagnostic.diagnostic_type.contains("unused"))
                && diagnostic.line_start == line as i64
            {
                return Ok(true); // Clippy confirms this is unused
            }
        }

        Ok(false) // No Clippy confirmation, but still potentially unused
    }
}
