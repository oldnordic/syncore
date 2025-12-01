//! Code Diagnostics Storage
//!
//! Provides storage and retrieval of static analysis diagnostics from tools like Clippy.
//! Part of the PAE (Project Analysis Engine) system.

use crate::project_analysis::rust_macro_expander::{MacroExpansionContext, RustMacroExpander};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Input diagnostic for ingestion from external tools
/// Matches the exact schema of code_diagnostics table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticInput {
    pub file_path: String,
    pub line: u32,
    pub column: u32,
    pub severity: String,
    pub tool: String,
    pub code: Option<String>,
    pub message: String,
}

/// Code diagnostic information from static analysis tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeDiagnostic {
    pub file_path: String,
    pub line_start: i64,
    pub severity: String,
    pub diagnostic_type: String,
    pub message: String,
    pub tool: String,
}

impl CodeDiagnostic {
    /// Create a new diagnostic
    pub fn new(
        file_path: String,
        line_start: i64,
        severity: String,
        diagnostic_type: String,
        message: String,
        tool: String,
    ) -> Self {
        Self {
            file_path,
            line_start,
            severity,
            diagnostic_type,
            message,
            tool,
        }
    }
}

/// Macro expansion diagnostic information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroExpansionDiagnostic {
    pub file_path: String,
    pub macro_name: String,
    pub span_start: i64,
    pub span_end: i64,
    pub original_code: String,
    pub expanded_code: String,
    pub expansion_type: String,
}

/// Diagnostics storage manager
pub struct DiagnosticsManager {
    db_manager: Arc<crate::db::DbManager>,
}

impl DiagnosticsManager {
    /// Create new diagnostics manager
    pub fn new(db_manager: Arc<crate::db::DbManager>) -> Self {
        Self {
            db_manager,
        }
    }

    /// Store diagnostics from external tools using DiagnosticInput format
    /// This is the unified entry point for all language backends
    pub fn store_diagnostics(&self, diagnostics: &[DiagnosticInput]) -> Result<usize> {
        if diagnostics.is_empty() {
            return Ok(0);
        }

        // Convert DiagnosticInput to CodeDiagnostic format
        let code_diagnostics: Vec<CodeDiagnostic> = diagnostics
            .iter()
            .map(|diag| {
                let diagnostic_type =
                    diag.code.clone().unwrap_or_else(|| format!("{}::unknown", diag.tool));

                CodeDiagnostic::new(
                    diag.file_path.clone(),
                    diag.line as i64,
                    diag.severity.clone(),
                    diagnostic_type,
                    diag.message.clone(),
                    diag.tool.clone(),
                )
            })
            .collect();

        // Use existing insert_diagnostics method
        self.insert_diagnostics(&code_diagnostics)
    }

    /// Insert a batch of diagnostics into the database
    pub fn insert_diagnostics(&self, diagnostics: &[CodeDiagnostic]) -> Result<usize> {
        if diagnostics.is_empty() {
            return Ok(0);
        }

        let conn = self.db_manager.code_graph_conn();
        let mut conn_guard = conn.lock().unwrap();

        let tx = conn_guard.transaction()?;

        // Clear existing diagnostics for the same tool (simple approach for this phase)
        let tool_name = &diagnostics[0].tool;
        tx.execute("DELETE FROM code_diagnostics WHERE tool = ?", [tool_name])?;

        // Insert new diagnostics
        let mut stmt = tx.prepare(
            r#"
            INSERT INTO code_diagnostics (
                file_path, line_start, severity, diagnostic_type, message, tool
            ) VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )?;

        let mut inserted = 0;
        for diagnostic in diagnostics {
            stmt.execute((
                &diagnostic.file_path,
                diagnostic.line_start,
                &diagnostic.severity,
                &diagnostic.diagnostic_type,
                &diagnostic.message,
                &diagnostic.tool,
            ))?;
            inserted += 1;
        }

        drop(stmt);
        tx.commit()?;

        Ok(inserted)
    }

    /// Query diagnostics by file path
    pub fn query_diagnostics_by_file(&self, file_path: &str) -> Result<Vec<CodeDiagnostic>> {
        let conn = self.db_manager.code_graph_conn();
        let conn_guard = conn.lock().unwrap();

        let mut stmt = conn_guard.prepare(
            r#"
            SELECT file_path, line_start, severity, diagnostic_type, message, tool
            FROM code_diagnostics
            WHERE file_path = ?
            ORDER BY line_start
            "#,
        )?;

        let diagnostics = stmt
            .query_map([file_path], |row| {
                Ok(CodeDiagnostic {
                    file_path: row.get(0)?,
                    line_start: row.get(1)?,
                    severity: row.get(2)?,
                    diagnostic_type: row.get(3)?,
                    message: row.get(4)?,
                    tool: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(diagnostics)
    }

    /// Query diagnostics by tool
    pub fn query_diagnostics_by_tool(&self, tool: &str) -> Result<Vec<CodeDiagnostic>> {
        let conn = self.db_manager.code_graph_conn();
        let conn_guard = conn.lock().unwrap();

        let mut stmt = conn_guard.prepare(
            r#"
            SELECT file_path, line_start, severity, diagnostic_type, message, tool
            FROM code_diagnostics
            WHERE tool = ?
            ORDER BY file_path, line_start
            "#,
        )?;

        let diagnostics = stmt
            .query_map([tool], |row| {
                Ok(CodeDiagnostic {
                    file_path: row.get(0)?,
                    line_start: row.get(1)?,
                    severity: row.get(2)?,
                    diagnostic_type: row.get(3)?,
                    message: row.get(4)?,
                    tool: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(diagnostics)
    }

    /// Get count of diagnostics for a specific file and tool
    pub fn count_diagnostics_for_file(&self, file_path: &str, tool: &str) -> Result<i64> {
        let conn = self.db_manager.code_graph_conn();
        let conn_guard = conn.lock().unwrap();
        Self::count_diagnostics_for_file_with_conn(&conn_guard, file_path, tool)
    }

    /// Get count of diagnostics for a specific file and tool using an existing connection
    /// Use this when you already have a lock on the code_graph connection to avoid deadlocks
    pub fn count_diagnostics_for_file_with_conn(
        conn: &rusqlite::Connection,
        file_path: &str,
        tool: &str,
    ) -> Result<i64> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM code_diagnostics WHERE file_path = ? AND tool = ?",
            [file_path, tool],
            |row| row.get(0),
        )?;

        Ok(count)
    }

    /// Get all diagnostics for a specific file using an existing connection
    /// Use this when you already have a lock on the code_graph connection to avoid deadlocks
    pub fn get_diagnostics_for_file_with_conn(
        conn: &rusqlite::Connection,
        file_path: &str,
    ) -> Result<Vec<CodeDiagnostic>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT file_path, line_start, severity, diagnostic_type, message, tool
            FROM code_diagnostics
            WHERE file_path = ?
            ORDER BY line_start
            "#,
        )?;

        let diagnostics_iter = stmt.query_map([file_path], |row| {
            Ok(CodeDiagnostic {
                file_path: row.get(0)?,
                line_start: row.get(1)?,
                severity: row.get(2)?,
                diagnostic_type: row.get(3)?,
                message: row.get(4)?,
                tool: row.get(5)?,
            })
        })?;

        let mut diagnostics = Vec::new();
        for diagnostic in diagnostics_iter {
            diagnostics.push(diagnostic?);
        }

        Ok(diagnostics)
    }

    /// Get count of all diagnostics for a specific tool
    pub fn count_diagnostics_for_tool(&self, tool: &str) -> Result<i64> {
        let conn = self.db_manager.code_graph_conn();
        let conn_guard = conn.lock().unwrap();

        let count: i64 = conn_guard.query_row(
            "SELECT COUNT(*) FROM code_diagnostics WHERE tool = ?",
            [tool],
            |row| row.get(0),
        )?;

        Ok(count)
    }

    /// Query diagnostics in a line range for a file
    pub fn query_diagnostics_in_range(
        &self,
        file_path: &str,
        line_start: i32,
        line_end: i32,
    ) -> Result<Vec<CodeDiagnostic>> {
        let conn = self.db_manager.code_graph_conn();
        let conn_guard = conn.lock().unwrap();

        let mut stmt = conn_guard.prepare(
            r#"
            SELECT file_path, line_start, severity, diagnostic_type, message, tool
            FROM code_diagnostics
            WHERE file_path = ? AND line_start >= ? AND line_start <= ?
            ORDER BY line_start
            "#,
        )?;

        let diagnostics = stmt
            .query_map([file_path, &line_start.to_string(), &line_end.to_string()], |row| {
                Ok(CodeDiagnostic {
                    file_path: row.get(0)?,
                    line_start: row.get(1)?,
                    severity: row.get(2)?,
                    diagnostic_type: row.get(3)?,
                    message: row.get(4)?,
                    tool: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(diagnostics)
    }

    /// List diagnostics for a specific file and tool (PAE helper)
    pub fn list_diagnostics_for_file_by_tool(
        &self,
        file_path: &str,
        tool: &str,
    ) -> Result<Vec<CodeDiagnostic>> {
        let conn = self.db_manager.code_graph_conn();
        let conn_guard = conn.lock().unwrap();

        let mut stmt = conn_guard.prepare(
            r#"
            SELECT file_path, line_start, severity, diagnostic_type, message, tool
            FROM code_diagnostics
            WHERE file_path = ? AND tool = ?
            ORDER BY line_start
            "#,
        )?;

        let diagnostics = stmt
            .query_map([file_path, tool], |row| {
                Ok(CodeDiagnostic {
                    file_path: row.get(0)?,
                    line_start: row.get(1)?,
                    severity: row.get(2)?,
                    diagnostic_type: row.get(3)?,
                    message: row.get(4)?,
                    tool: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(diagnostics)
    }

    /// Count diagnostics for a specific file and tool (PAE helper)
    pub fn count_diagnostics_by_tool(&self, file_path: &str, tool: &str) -> Result<usize> {
        let conn = self.db_manager.code_graph_conn();
        let conn_guard = conn.lock().unwrap();

        let count: i64 = conn_guard.query_row(
            "SELECT COUNT(*) FROM code_diagnostics WHERE file_path = ? AND tool = ?",
            [file_path, tool],
            |row| row.get(0),
        )?;

        Ok(count as usize)
    }

    /// Store macro expansion results for a file
    pub fn store_macro_expansions(
        &self,
        file_path: &str,
        macro_context: &MacroExpansionContext,
    ) -> Result<usize> {
        let conn = self.db_manager.code_graph_conn();
        let mut conn_guard = conn.lock().unwrap();

        let tx = conn_guard.transaction()?;

        // Clear existing macro expansions for this file
        tx.execute("DELETE FROM code_macro_expansions WHERE file_path = ?", [file_path])?;

        // Insert new macro expansions
        let mut stmt = tx.prepare(
            r#"
            INSERT INTO code_macro_expansions (
                file_path, macro_name, span_start, span_end, 
                original_code, expanded_code, expansion_type
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )?;

        let mut inserted = 0;
        for expansion in &macro_context.expansions {
            // Determine expansion type based on macro name
            let expansion_type = if expansion.macro_name == "vec!" {
                "vec".to_string()
            } else if expansion.macro_name == "format!" {
                "format".to_string()
            } else if expansion.macro_name.contains("info")
                || expansion.macro_name.contains("warn")
                || expansion.macro_name.contains("error")
                || expansion.macro_name.contains("debug")
                || expansion.macro_name.contains("trace")
            {
                "log".to_string()
            } else if expansion.macro_name == "assert!" {
                "assert".to_string()
            } else {
                "declarative".to_string()
            };

            stmt.execute((
                file_path,
                &expansion.macro_name,
                expansion.span_start,
                expansion.span_end,
                &macro_context.expanded_source[expansion.span_start
                    ..expansion.span_end.min(macro_context.expanded_source.len())],
                &expansion.expanded_code,
                &expansion_type,
            ))?;
            inserted += 1;
        }

        drop(stmt);
        tx.commit()?;

        Ok(inserted)
    }

    /// Get macro expansions for a file
    pub fn get_macro_expansions_for_file(
        &self,
        file_path: &str,
    ) -> Result<Vec<MacroExpansionDiagnostic>> {
        let conn = self.db_manager.code_graph_conn();
        let conn_guard = conn.lock().unwrap();

        let mut stmt = conn_guard.prepare(
            r#"
            SELECT file_path, macro_name, span_start, span_end, 
                   original_code, expanded_code, expansion_type
            FROM code_macro_expansions
            WHERE file_path = ?
            ORDER BY span_start
            "#,
        )?;

        let expansions = stmt
            .query_map([file_path], |row| {
                Ok(MacroExpansionDiagnostic {
                    file_path: row.get(0)?,
                    macro_name: row.get(1)?,
                    span_start: row.get(2)?,
                    span_end: row.get(3)?,
                    original_code: row.get(4)?,
                    expanded_code: row.get(5)?,
                    expansion_type: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(expansions)
    }

    /// Process macro expansions for a Rust file and store results
    pub fn process_macro_expansions_for_file(&self, file_path: &str) -> Result<usize> {
        // Only process Rust files
        if !file_path.ends_with(".rs") {
            return Ok(0);
        }

        let expander = RustMacroExpander::new()?;
        let source_code = std::fs::read_to_string(file_path)?;

        match expander.expand_simple_macro_invocations(&source_code) {
            Ok(macro_context) => {
                if !macro_context.expansions.is_empty() {
                    self.store_macro_expansions(file_path, &macro_context)
                } else {
                    Ok(0)
                }
            }
            Err(_) => Ok(0), // Silently ignore expansion errors
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DbManager;
    use std::fs;

    #[test]
    fn test_diagnostics_basic_operations() -> Result<()> {
        let test_main_db = "/tmp/test_diagnostics_main.db";
        let test_code_graph_db = "/tmp/test_diagnostics_code_graph.db";

        // Cleanup
        let _ = fs::remove_file(test_main_db);
        let _ = fs::remove_file(test_code_graph_db);

        // Create test database manager
        let db_manager = Arc::new(DbManager::new(test_main_db, test_code_graph_db)?);
        let diagnostics = DiagnosticsManager::new(db_manager);

        // Create test diagnostics
        let test_diagnostics = vec![
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
        ];

        // Insert diagnostics
        let inserted = diagnostics.insert_diagnostics(&test_diagnostics)?;
        assert_eq!(inserted, 2);

        // Query by file
        let found = diagnostics.query_diagnostics_by_file("src/main.rs")?;
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].diagnostic_type, "clippy::dead_code");
        assert_eq!(found[1].diagnostic_type, "clippy::unimplemented");

        // Query by tool
        let clippy_diagnostics = diagnostics.query_diagnostics_by_tool("clippy")?;
        assert_eq!(clippy_diagnostics.len(), 2);

        // Count diagnostics
        let count = diagnostics.count_diagnostics_for_file("src/main.rs", "clippy")?;
        assert_eq!(count, 2);

        let total_count = diagnostics.count_diagnostics_for_tool("clippy")?;
        assert_eq!(total_count, 2);

        // Cleanup
        drop(diagnostics);
        let _ = fs::remove_file(test_main_db);
        let _ = fs::remove_file(test_code_graph_db);

        Ok(())
    }

    #[test]
    fn test_diagnostics_range_query() -> Result<()> {
        let test_main_db = "/tmp/test_diagnostics_range_main.db";
        let test_code_graph_db = "/tmp/test_diagnostics_range_code_graph.db";

        // Cleanup
        let _ = fs::remove_file(test_main_db);
        let _ = fs::remove_file(test_code_graph_db);

        let db_manager = Arc::new(DbManager::new(test_main_db, test_code_graph_db)?);
        let diagnostics = DiagnosticsManager::new(db_manager);

        // Create test diagnostics at different lines
        let test_diagnostics = vec![
            CodeDiagnostic::new(
                "src/test.rs".to_string(),
                5,
                "warning".to_string(),
                "clippy::dead_code".to_string(),
                "line 5".to_string(),
                "clippy".to_string(),
            ),
            CodeDiagnostic::new(
                "src/test.rs".to_string(),
                15,
                "warning".to_string(),
                "clippy::dead_code".to_string(),
                "line 15".to_string(),
                "clippy".to_string(),
            ),
            CodeDiagnostic::new(
                "src/test.rs".to_string(),
                25,
                "warning".to_string(),
                "clippy::dead_code".to_string(),
                "line 25".to_string(),
                "clippy".to_string(),
            ),
        ];

        diagnostics.insert_diagnostics(&test_diagnostics)?;

        // Query range 10-20 should find only the middle diagnostic
        let range_diagnostics = diagnostics.query_diagnostics_in_range("src/test.rs", 10, 20)?;
        assert_eq!(range_diagnostics.len(), 1);
        assert_eq!(range_diagnostics[0].line_start, 15);

        // Cleanup
        drop(diagnostics);
        let _ = fs::remove_file(test_main_db);
        let _ = fs::remove_file(test_code_graph_db);

        Ok(())
    }

    #[test]
    fn test_diagnostics_replace_existing() -> Result<()> {
        let test_main_db = "/tmp/test_diagnostics_replace_main.db";
        let test_code_graph_db = "/tmp/test_diagnostics_replace_code_graph.db";

        // Cleanup
        let _ = fs::remove_file(test_main_db);
        let _ = fs::remove_file(test_code_graph_db);

        let db_manager = Arc::new(DbManager::new(test_main_db, test_code_graph_db)?);
        let diagnostics = DiagnosticsManager::new(db_manager);

        // Insert initial diagnostics
        let initial_diagnostics = vec![CodeDiagnostic::new(
            "src/test.rs".to_string(),
            10,
            "warning".to_string(),
            "clippy::dead_code".to_string(),
            "old message".to_string(),
            "clippy".to_string(),
        )];

        diagnostics.insert_diagnostics(&initial_diagnostics)?;
        let initial_count = diagnostics.count_diagnostics_for_tool("clippy")?;
        assert_eq!(initial_count, 1);

        // Insert new diagnostics (should replace old ones)
        let new_diagnostics = vec![
            CodeDiagnostic::new(
                "src/test.rs".to_string(),
                20,
                "error".to_string(),
                "clippy::unimplemented".to_string(),
                "new message".to_string(),
                "clippy".to_string(),
            ),
            CodeDiagnostic::new(
                "src/other.rs".to_string(),
                30,
                "warning".to_string(),
                "clippy::dead_code".to_string(),
                "other message".to_string(),
                "clippy".to_string(),
            ),
        ];

        let inserted = diagnostics.insert_diagnostics(&new_diagnostics)?;
        assert_eq!(inserted, 2);

        // Verify old diagnostics are gone and new ones are present
        let final_count = diagnostics.count_diagnostics_for_tool("clippy")?;
        assert_eq!(final_count, 2);

        let found = diagnostics.query_diagnostics_by_tool("clippy")?;
        assert_eq!(found.len(), 2);
        assert!(found.iter().all(|d| d.message != "old message"));

        // Cleanup
        drop(diagnostics);
        let _ = fs::remove_file(test_main_db);
        let _ = fs::remove_file(test_code_graph_db);

        Ok(())
    }
}
