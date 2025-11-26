//! APEX 2.5-LSP: LSP Diagnostic Event Type
//!
//! Normalized diagnostic representation for internal use.

use std::path::PathBuf;

/// LSP diagnostic event (normalized from LSP publishDiagnostics)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspDiagnosticEvent {
    /// File path (converted from file:// URI)
    pub path: PathBuf,

    /// Diagnostic message
    pub message: String,

    /// Source range: (start_line, start_col, end_line, end_col)
    /// LSP uses 0-based line/column numbers
    pub range: (u32, u32, u32, u32),

    /// Severity: 1=Error, 2=Warning, 3=Info, 4=Hint
    pub severity: Option<u32>,

    /// Diagnostic code (e.g., "E0308" for Rust)
    pub code: Option<String>,
}

impl LspDiagnosticEvent {
    /// Check if diagnostic is an error (severity 1)
    pub fn is_error(&self) -> bool {
        self.severity == Some(1)
    }

    /// Check if diagnostic is a warning (severity 2)
    pub fn is_warning(&self) -> bool {
        self.severity == Some(2)
    }

    /// Get human-readable severity string
    pub fn severity_str(&self) -> &'static str {
        match self.severity {
            Some(1) => "error",
            Some(2) => "warning",
            Some(3) => "info",
            Some(4) => "hint",
            _ => "unknown",
        }
    }

    /// Get start position as (line, col)
    pub fn start_pos(&self) -> (u32, u32) {
        (self.range.0, self.range.1)
    }

    /// Get end position as (line, col)
    pub fn end_pos(&self) -> (u32, u32) {
        (self.range.2, self.range.3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_diagnostic_severity_helpers() {
        let error_diag = LspDiagnosticEvent {
            path: PathBuf::from("/tmp/test.rs"),
            message: "error message".to_string(),
            range: (0, 0, 0, 10),
            severity: Some(1),
            code: None,
        };

        assert!(error_diag.is_error());
        assert!(!error_diag.is_warning());
        assert_eq!(error_diag.severity_str(), "error");

        let warning_diag = LspDiagnosticEvent {
            path: PathBuf::from("/tmp/test.rs"),
            message: "warning message".to_string(),
            range: (1, 0, 1, 10),
            severity: Some(2),
            code: None,
        };

        assert!(!warning_diag.is_error());
        assert!(warning_diag.is_warning());
        assert_eq!(warning_diag.severity_str(), "warning");
    }

    #[test]
    fn test_diagnostic_position_helpers() {
        let diag = LspDiagnosticEvent {
            path: PathBuf::from("/tmp/test.rs"),
            message: "test".to_string(),
            range: (5, 10, 7, 20),
            severity: None,
            code: None,
        };

        assert_eq!(diag.start_pos(), (5, 10));
        assert_eq!(diag.end_pos(), (7, 20));
    }
}
