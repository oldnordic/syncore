#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::process::{Command, Stdio};
    use serde_json::{json, Value};
    use std::path::Path;
    use tempfile::TempDir;

    fn get_plugin_binary() -> String {
        // In a real build, this would point to the compiled binary
        // For testing, we'll assume it's built and available
        "target/release/syncore_ts_js_plugin".to_string()
    }

    fn send_plugin_request(request: &Value) -> Result<Value, Box<dyn std::error::Error>> {
        let mut child = Command::new(get_plugin_binary())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        if let Some(stdin) = child.stdin.as_mut() {
            use std::io::Write;
            stdin.write_all(request.to_string().as_bytes())?;
            stdin.write_all(b"\n")?;
        }

        let output = child.wait_with_output()?;
        
        if !output.status.success() {
            return Err(format!("Plugin failed: {}", String::from_utf8_lossy(&output.stderr)).into());
        }

        let response_str = String::from_utf8(output.stdout)?;
        let response: Value = serde_json::from_str(&response_str.trim())?;
        
        Ok(response)
    }

    fn create_test_project() -> Result<TempDir, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        
        // Create package.json
        let package_json = r#"
        {
            "name": "test-project",
            "version": "1.0.0",
            "scripts": {
                "test": "echo \"Error: no test specified\" && exit 1"
            },
            "devDependencies": {
                "typescript": "^4.0.0",
                "eslint": "^7.0.0",
                "prettier": "^2.0.0"
            }
        }
        "#;
        
        std::fs::write(temp_dir.path().join("package.json"), package_json)?;
        
        // Create tsconfig.json
        let tsconfig_json = r#"
        {
            "compilerOptions": {
                "target": "es2018",
                "module": "commonjs",
                "strict": true,
                "esModuleInterop": true,
                "skipLibCheck": true,
                "forceConsistentCasingInFileNames": true,
                "outDir": "./dist"
            },
            "include": ["src/**/*"],
            "exclude": ["node_modules", "dist"]
        }
        "#;
        
        std::fs::write(temp_dir.path().join("tsconfig.json"), tsconfig_json)?;
        
        // Create src directory
        let src_dir = temp_dir.path().join("src");
        std::fs::create_dir_all(&src_dir)?;
        
        // Create a TypeScript file with some intentional issues for testing
        let ts_file = r#"
        import { SomeInterface } from './types';

        export class TestClass implements SomeInterface {
            private unusedVar: string = "test";
            public name: string;
            
            constructor(name: string) {
                this.name = name;
                console.log("TestClass created");
            }
            
            public greet(): void {
                console.log(`Hello, ${this.name}!`);
                // Missing semicolon on purpose for Prettier test
            }
            
            public unusedMethod(): string {
                return "This method is never used";
            }
        }

        function createTest(name: string): TestClass {
            return new TestClass(name);
        }

        const instance = createTest("World");
        instance.greet();
        "#;
        
        std::fs::write(src_dir.join("index.ts"), ts_file)?;
        
        // Create types file
        let types_file = r#"
        export interface SomeInterface {
            name: string;
            greet(): void;
        }
        "#;
        
        std::fs::write(src_dir.join("types.ts"), types_file)?;
        
        Ok(temp_dir)
    }

    #[test]
    fn test_full_project_analysis_workflow() {
        // Create a temporary test project
        let temp_project = create_test_project().expect("Failed to create test project");
        let project_root = temp_project.path().to_str().unwrap();

        // First, initialize the plugin
        let init_request = json!({
            "event": "init",
            "plugin_name": "ts_js_plugin",
            "version": "1.0"
        });

        let init_response = send_plugin_request(&init_request).expect("Failed to initialize plugin");
        assert_eq!(init_response["status"], "ready");

        // Execute full project analysis
        let analysis_request = json!({
            "event": "execute",
            "task": "ts_js_full_project_analysis",
            "params": {
                "project_root": project_root,
                "ts_js_config": {
                    "eslint_path": "eslint",
                    "prettier_path": "prettier"
                }
            }
        });

        let analysis_response = send_plugin_request(&analysis_request).expect("Failed to execute full project analysis");

        assert_eq!(analysis_response["status"], "ok");
        assert!(analysis_response["result"].is_object());
        
        let result = &analysis_response["result"];
        
        // Check that we got entities
        assert!(result["entities"].is_array());
        let entities = result["entities"].as_array().unwrap();
        assert!(!entities.is_empty(), "Should have found entities in the test project");
        
        // Check for expected entity types
        let mut found_class = false;
        let mut found_interface = false;
        let mut found_function = false;
        
        for entity in entities {
            if let Some(kind) = entity["kind"].as_str() {
                match kind {
                    "Class" => found_class = true,
                    "Interface" => found_interface = true,
                    "Function" => found_function = true,
                    _ => {}
                }
            }
        }
        
        assert!(found_class, "Should have found at least one class");
        assert!(found_interface, "Should have found at least one interface");
        assert!(found_function, "Should have found at least one function");
        
        // Check that we got edges
        assert!(result["edges"].is_array());
        let edges = result["edges"].as_array().unwrap();
        // Edges might be empty in our simple test, but the field should exist
        
        // Check that we got diagnostics
        assert!(result["diagnostics"].is_array());
        let diagnostics = result["diagnostics"].as_array().unwrap();
        // Diagnostics might be empty if tools aren't available, but the field should exist
        
        // Check entity structure
        for entity in entities {
            assert!(entity["file_path"].is_string(), "Entity should have file_path");
            assert!(entity["name"].is_string(), "Entity should have name");
            assert!(entity["kind"].is_string(), "Entity should have kind");
            assert!(entity["span"].is_object(), "Entity should have span");
            
            let span = &entity["span"];
            assert!(span["start_line"].is_number(), "Span should have start_line");
            assert!(span["start_col"].is_number(), "Span should have start_col");
            assert!(span["end_line"].is_number(), "Span should have end_line");
            assert!(span["end_col"].is_number(), "Span should have end_col");
        }
        
        // Check edge structure if we have any
        for edge in edges {
            assert!(edge["from"].is_string(), "Edge should have from");
            assert!(edge["to"].is_string(), "Edge should have to");
            assert!(edge["kind"].is_string(), "Edge should have kind");
        }
        
        // Check diagnostic structure if we have any
        for diagnostic in diagnostics {
            assert!(diagnostic["file_path"].is_string(), "Diagnostic should have file_path");
            assert!(diagnostic["line"].is_number(), "Diagnostic should have line");
            assert!(diagnostic["column"].is_number(), "Diagnostic should have column");
            assert!(diagnostic["severity"].is_string(), "Diagnostic should have severity");
            assert!(diagnostic["code"].is_string(), "Diagnostic should have code");
            assert!(diagnostic["message"].is_string(), "Diagnostic should have message");
            assert!(diagnostic["tool"].is_string(), "Diagnostic should have tool");
        }
    }

    #[test]
    fn test_individual_tasks() {
        // Create a temporary test project
        let temp_project = create_test_project().expect("Failed to create test project");
        let project_root = temp_project.path().to_str().unwrap();

        // Initialize the plugin
        let init_request = json!({
            "event": "init",
            "plugin_name": "ts_js_plugin",
            "version": "1.0"
        });

        let init_response = send_plugin_request(&init_request).expect("Failed to initialize plugin");
        assert_eq!(init_response["status"], "ready");

        // Test individual indexing task
        let index_request = json!({
            "event": "execute",
            "task": "ts_js_index_directory",
            "params": {
                "root_path": project_root
            }
        });

        let index_response = send_plugin_request(&index_request).expect("Failed to execute indexing");
        assert_eq!(index_response["status"], "ok");
        assert!(index_response["result"].is_object());
        assert!(index_response["result"]["entities"].is_array());
        assert!(index_response["result"]["edges"].is_array());

        // Test LSP diagnostics task
        let lsp_request = json!({
            "event": "execute",
            "task": "ts_js_lsp_diagnostics",
            "params": {
                "project_root": project_root
            }
        });

        let lsp_response = send_plugin_request(&lsp_request).expect("Failed to execute LSP diagnostics");
        assert_eq!(lsp_response["status"], "ok");
        assert!(lsp_response["result"].is_object());
        assert!(lsp_response["result"]["diagnostics"].is_array());

        // Test ESLint task
        let eslint_request = json!({
            "event": "execute",
            "task": "ts_js_eslint",
            "params": {
                "project_root": project_root
            }
        });

        let eslint_response = send_plugin_request(&eslint_request).expect("Failed to execute ESLint");
        assert_eq!(eslint_response["status"], "ok");
        assert!(eslint_response["result"].is_object());
        assert!(eslint_response["result"]["diagnostics"].is_array());

        // Test Prettier task
        let prettier_request = json!({
            "event": "execute",
            "task": "ts_js_prettier",
            "params": {
                "project_root": project_root
            }
        });

        let prettier_response = send_plugin_request(&prettier_request).expect("Failed to execute Prettier");
        assert_eq!(prettier_response["status"], "ok");
        assert!(prettier_response["result"].is_object());
        assert!(prettier_response["result"]["diagnostics"].is_array());
    }

    #[test]
    fn test_error_handling() {
        // Test with non-existent project root
        let init_request = json!({
            "event": "init",
            "plugin_name": "ts_js_plugin",
            "version": "1.0"
        });

        let init_response = send_plugin_request(&init_request).expect("Failed to initialize plugin");
        assert_eq!(init_response["status"], "ready");

        let error_request = json!({
            "event": "execute",
            "task": "ts_js_index_directory",
            "params": {
                "root_path": "/non/existent/path"
            }
        });

        let error_response = send_plugin_request(&error_request).expect("Failed to get error response");
        // The plugin should handle this gracefully, either returning an empty result or an error
        assert!(error_response["status"] == "ok" || error_response["status"] == "error");
    }

    #[test]
    fn test_plugin_shutdown() {
        let shutdown_request = json!({
            "event": "shutdown"
        });

        let shutdown_response = send_plugin_request(&shutdown_request).expect("Failed to shutdown plugin");
        assert_eq!(shutdown_response["status"], "ok");
    }
}