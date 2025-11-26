#[cfg(test)]
mod tests {
    use syncore_ts_js_plugin::*;
    use plugin_api::{Severity};

    #[tokio::test]
    async fn test_create_diagnostics_runner() {
        let runner = TsJsDiagnosticsRunner::new(None, Some("eslint".to_string()), Some("prettier".to_string()));
        // Should not panic
    }

    #[tokio::test]
    async fn test_run_eslint_with_mock_data() {
        let runner = TsJsDiagnosticsRunner::new(None, Some("eslint".to_string()), None);
        
        // Mock ESLint JSON output
        let mock_eslint_output = r#"
        [
            {
                "filePath": "/test/file.js",
                "messages": [
                    {
                        "ruleId": "no-unused-vars",
                        "severity": 2,
                        "message": "unused variable",
                        "line": 10,
                        "column": 5
                    },
                    {
                        "ruleId": "semi",
                        "severity": 1,
                        "message": "Missing semicolon",
                        "line": 15,
                        "column": 20
                    }
                ]
            }
        ]
        "#;

        let diagnostics = runner.parse_eslint_output(mock_eslint_output);
        assert!(diagnostics.is_ok(), "Failed to parse ESLint output");

        let diagnostics = diagnostics.unwrap();
        assert_eq!(diagnostics.len(), 2, "Expected 2 diagnostics from mock ESLint output");

        // Check first diagnostic (error)
        let first = &diagnostics[0];
        assert_eq!(first.file_path, "/test/file.js");
        assert_eq!(first.line, 10);
        assert_eq!(first.column, 5);
        assert_eq!(first.severity, Severity::Error);
        assert_eq!(first.code, "no-unused-vars");
        assert_eq!(first.message, "unused variable");
        assert_eq!(first.tool, "eslint");

        // Check second diagnostic (warning)
        let second = &diagnostics[1];
        assert_eq!(second.file_path, "/test/file.js");
        assert_eq!(second.line, 15);
        assert_eq!(second.column, 20);
        assert_eq!(second.severity, Severity::Warning);
        assert_eq!(second.code, "semi");
        assert_eq!(second.message, "Missing semicolon");
        assert_eq!(second.tool, "eslint");
    }

    #[tokio::test]
    async fn test_parse_tsc_errors() {
        let runner = TsJsDiagnosticsRunner::new(None, None, None);
        
        // Mock TypeScript compiler error output
        let mock_tsc_output = r#"
        src/test.ts(10,5): error TS2322: Type 'string' is not assignable to type 'number'.
        src/test.ts(15,10): warning TS6133: 'unusedVar' is declared but its value is never read.
        "#;

        let project_root = "/project";
        let diagnostics = runner.parse_tsc_errors(mock_tsc_output, project_root);
        assert!(diagnostics.is_ok(), "Failed to parse TSC errors");

        let diagnostics = diagnostics.unwrap();
        assert_eq!(diagnostics.len(), 2, "Expected 2 diagnostics from mock TSC output");

        // Check first diagnostic (error)
        let first = &diagnostics[0];
        assert_eq!(first.file_path, "/project/src/test.ts");
        assert_eq!(first.line, 10);
        assert_eq!(first.column, 5);
        assert_eq!(first.severity, Severity::Error);
        assert_eq!(first.code, "error TS2322");
        assert_eq!(first.message, "Type 'string' is not assignable to type 'number'.");
        assert_eq!(first.tool, "tsserver");

        // Check second diagnostic (warning)
        let second = &diagnostics[1];
        assert_eq!(second.file_path, "/project/src/test.ts");
        assert_eq!(second.line, 15);
        assert_eq!(second.column, 10);
        assert_eq!(second.severity, Severity::Warning);
        assert_eq!(second.code, "warning TS6133");
        assert_eq!(second.message, "'unusedVar' is declared but its value is never read.");
        assert_eq!(second.tool, "tsserver");
    }

    #[tokio::test]
    async fn test_parse_prettier_output() {
        let runner = TsJsDiagnosticsRunner::new(None, None, None);
        
        // Mock Prettier check output
        let mock_prettier_output = r#"
        src/test.ts
        Some code that needs formatting...
        
        src/other.js
        More code that needs formatting...
        "#;

        let project_root = "/project";
        let diagnostics = runner.parse_prettier_output(mock_prettier_output, project_root);
        assert!(diagnostics.is_ok(), "Failed to parse Prettier output");

        let diagnostics = diagnostics.unwrap();
        assert_eq!(diagnostics.len(), 2, "Expected 2 diagnostics from mock Prettier output");

        // Check first diagnostic
        let first = &diagnostics[0];
        assert_eq!(first.file_path, "/project/src/test.ts");
        assert_eq!(first.line, 1);
        assert_eq!(first.column, 1);
        assert_eq!(first.severity, Severity::Info);
        assert_eq!(first.code, "prettier-format");
        assert_eq!(first.message, "File needs formatting");
        assert_eq!(first.tool, "prettier");

        // Check second diagnostic
        let second = &diagnostics[1];
        assert_eq!(second.file_path, "/project/src/other.js");
        assert_eq!(second.line, 1);
        assert_eq!(second.column, 1);
        assert_eq!(second.severity, Severity::Info);
        assert_eq!(second.code, "prettier-format");
        assert_eq!(second.message, "File needs formatting");
        assert_eq!(second.tool, "prettier");
    }

    #[tokio::test]
    async fn test_invalid_eslint_json() {
        let runner = TsJsDiagnosticsRunner::new(None, Some("eslint".to_string()), None);
        
        let invalid_json = "This is not valid JSON";
        let result = runner.parse_eslint_output(invalid_json);
        assert!(result.is_err(), "Should fail to parse invalid JSON");
    }

    #[tokio::test]
    async fn test_malformed_tsc_error() {
        let runner = TsJsDiagnosticsRunner::new(None, None, None);
        
        let malformed_error = "This is not a valid TSC error format";
        let project_root = "/project";
        let diagnostics = runner.parse_tsc_errors(malformed_error, project_root);
        assert!(diagnostics.is_ok(), "Should handle malformed TSC errors gracefully");

        let diagnostics = diagnostics.unwrap();
        assert_eq!(diagnostics.len(), 0, "Should return no diagnostics for malformed TSC errors");
    }

    #[tokio::test]
    async fn test_empty_eslint_output() {
        let runner = TsJsDiagnosticsRunner::new(None, Some("eslint".to_string()), None);
        
        let empty_output = "[]";
        let diagnostics = runner.parse_eslint_output(empty_output);
        assert!(diagnostics.is_ok(), "Should handle empty ESLint output");

        let diagnostics = diagnostics.unwrap();
        assert_eq!(diagnostics.len(), 0, "Should return no diagnostics for empty ESLint output");
    }

    #[tokio::test]
    async fn test_eslint_without_rule_id() {
        let runner = TsJsDiagnosticsRunner::new(None, Some("eslint".to_string()), None);
        
        let eslint_output = r#"
        [
            {
                "filePath": "/test/file.js",
                "messages": [
                    {
                        "ruleId": null,
                        "severity": 2,
                        "message": "Some error without rule ID",
                        "line": 1,
                        "column": 1
                    }
                ]
            }
        ]
        "#;

        let diagnostics = runner.parse_eslint_output(eslint_output);
        assert!(diagnostics.is_ok(), "Failed to parse ESLint output without rule ID");

        let diagnostics = diagnostics.unwrap();
        assert_eq!(diagnostics.len(), 1, "Expected 1 diagnostic");

        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.code, "unknown", "Should use 'unknown' for missing rule ID");
    }

    #[tokio::test]
    async fn test_skip_unconfigured_tools() {
        let runner = TsJsDiagnosticsRunner::new(None, None, None);
        
        // Test with no tools configured
        let result = runner.run_tsserver_diagnostics("/test").await;
        assert!(result.is_ok(), "Should skip tsserver gracefully");
        
        let plugin_result = result.unwrap();
        assert!(plugin_result.diagnostics.is_some(), "Should have diagnostics field");
        let diagnostics = plugin_result.diagnostics.unwrap();
        assert_eq!(diagnostics.len(), 0, "Should return empty diagnostics when tsserver not configured");

        let result = runner.run_eslint_diagnostics("/test", None).await;
        assert!(result.is_ok(), "Should skip eslint gracefully");
        
        let plugin_result = result.unwrap();
        assert!(plugin_result.diagnostics.is_some(), "Should have diagnostics field");
        let diagnostics = plugin_result.diagnostics.unwrap();
        assert_eq!(diagnostics.len(), 0, "Should return empty diagnostics when eslint not configured");

        let result = runner.run_prettier_diagnostics("/test").await;
        assert!(result.is_ok(), "Should skip prettier gracefully");
        
        let plugin_result = result.unwrap();
        assert!(plugin_result.diagnostics.is_some(), "Should have diagnostics field");
        let diagnostics = plugin_result.diagnostics.unwrap();
        assert_eq!(diagnostics.len(), 0, "Should return empty diagnostics when prettier not configured");
    }
}