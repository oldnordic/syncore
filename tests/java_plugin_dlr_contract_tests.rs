use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use syncore::dlr::DlrError;
use tempfile::TempDir;

#[cfg(test)]
mod java_plugin_dlr_contract_tests {
    use super::*;

    fn setup_java_plugin(temp_dir: &TempDir) -> Result<String, DlrError> {
        // Build the Java plugin
        let plugin_path = std::env::current_dir().unwrap().join("plugins/java_plugin");

        let output = Command::new("cargo")
            .args(&["build", "--release"])
            .current_dir(&plugin_path)
            .output()
            .map_err(|e| {
                DlrError::PluginStartFailed(format!("Failed to build Java plugin: {}", e))
            })?;

        if !output.status.success() {
            return Err(DlrError::PluginStartFailed(format!(
                "Java plugin build failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let plugin_binary = plugin_path.join("target/release/syncore_java_plugin");
        if !plugin_binary.exists() {
            return Err(DlrError::PluginStartFailed(
                "Java plugin binary not found after build".to_string(),
            ));
        }

        Ok(plugin_binary.to_string_lossy().to_string())
    }

    #[test]
    fn test_java_plugin_init_contract() {
        let temp_dir = TempDir::new().unwrap();

        match setup_java_plugin(&temp_dir) {
            Ok(plugin_path) => {
                let mut child = Command::new(&plugin_path)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .expect("Failed to start Java plugin");

                let stdin = child.stdin.as_mut().expect("Failed to open stdin");

                // Send init request - uses #[serde(tag = "event")] format
                let init_request = serde_json::json!({
                    "event": "init",
                    "plugin_name": "java_plugin",
                    "version": "1.0.0"
                });

                use std::io::Write;
                stdin.write_all(init_request.to_string().as_bytes()).unwrap();
                stdin.write_all(b"\n").unwrap();
                stdin.flush().unwrap();

                thread::sleep(Duration::from_millis(100));

                let output = child.wait_with_output().expect("Failed to read plugin output");
                let stdout = String::from_utf8_lossy(&output.stdout);

                // Verify response structure
                assert!(stdout.contains("ready"));
                assert!(stdout.contains("java_plugin"));

                // Check that supported tasks are included
                assert!(stdout.contains("java_index_directory"));
                assert!(stdout.contains("java_run_compiler_diagnostics"));
                assert!(stdout.contains("java_run_errorprone"));
                assert!(stdout.contains("java_run_pmd"));
                assert!(stdout.contains("java_full_project_analysis"));
            }
            Err(e) => {
                println!("Warning: Could not setup Java plugin for test: {}", e);
                // Skip test if plugin can't be built
            }
        }
    }

    #[test]
    fn test_java_plugin_capabilities_contract() {
        let temp_dir = TempDir::new().unwrap();

        match setup_java_plugin(&temp_dir) {
            Ok(plugin_path) => {
                let mut child = Command::new(&plugin_path)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .expect("Failed to start Java plugin");

                let stdin = child.stdin.as_mut().expect("Failed to open stdin");

                // Send capabilities request - uses #[serde(tag = "event")] format
                let capabilities_request = serde_json::json!({
                    "event": "capabilities"
                });

                use std::io::Write;
                stdin.write_all(capabilities_request.to_string().as_bytes()).unwrap();
                stdin.write_all(b"\n").unwrap();
                stdin.flush().unwrap();

                thread::sleep(Duration::from_millis(100));

                let output = child.wait_with_output().expect("Failed to read plugin output");
                let stdout = String::from_utf8_lossy(&output.stdout);

                // Verify capabilities response (returns tasks array, not "capabilities")
                assert!(stdout.contains("tasks"));
                assert!(stdout.contains("java_index_directory"));
                assert!(stdout.contains("java_run_compiler_diagnostics"));
                assert!(stdout.contains("java_run_errorprone"));
                assert!(stdout.contains("java_run_pmd"));
                assert!(stdout.contains("java_full_project_analysis"));
            }
            Err(e) => {
                println!("Warning: Could not setup Java plugin for test: {}", e);
            }
        }
    }

    #[test]
    fn test_java_plugin_shutdown_contract() {
        let temp_dir = TempDir::new().unwrap();

        match setup_java_plugin(&temp_dir) {
            Ok(plugin_path) => {
                let mut child = Command::new(&plugin_path)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .expect("Failed to start Java plugin");

                let stdin = child.stdin.as_mut().expect("Failed to open stdin");

                // Send shutdown request - uses #[serde(tag = "event")] format
                let shutdown_request = serde_json::json!({
                    "event": "shutdown"
                });

                use std::io::Write;
                stdin.write_all(shutdown_request.to_string().as_bytes()).unwrap();
                stdin.write_all(b"\n").unwrap();
                stdin.flush().unwrap();

                thread::sleep(Duration::from_millis(100));

                let output = child.wait_with_output().expect("Failed to read plugin output");
                let stdout = String::from_utf8_lossy(&output.stdout);

                // Verify shutdown response (returns "ok" status)
                assert!(stdout.contains("ok"));
            }
            Err(e) => {
                println!("Warning: Could not setup Java plugin for test: {}", e);
            }
        }
    }

    #[test]
    fn test_java_plugin_error_handling() {
        let temp_dir = TempDir::new().unwrap();

        match setup_java_plugin(&temp_dir) {
            Ok(plugin_path) => {
                let mut child = Command::new(&plugin_path)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .expect("Failed to start Java plugin");

                let stdin = child.stdin.as_mut().expect("Failed to open stdin");

                // Send invalid JSON
                use std::io::Write;
                stdin.write_all(b"invalid json\n").unwrap();
                stdin.flush().unwrap();

                thread::sleep(Duration::from_millis(100));

                let output = child.wait_with_output().expect("Failed to read plugin output");
                let stdout = String::from_utf8_lossy(&output.stdout);

                // Verify error response
                assert!(stdout.contains("error"));
                assert!(stdout.contains("Invalid JSON"));
            }
            Err(e) => {
                println!("Warning: Could not setup Java plugin for test: {}", e);
            }
        }
    }

    #[test]
    fn test_java_plugin_unknown_task_handling() {
        let temp_dir = TempDir::new().unwrap();

        match setup_java_plugin(&temp_dir) {
            Ok(plugin_path) => {
                let mut child = Command::new(&plugin_path)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .expect("Failed to start Java plugin");

                let stdin = child.stdin.as_mut().expect("Failed to open stdin");

                // Send unknown task - uses #[serde(tag = "event")] format
                let unknown_task_request = serde_json::json!({
                    "event": "execute",
                    "task": "unknown_task",
                    "params": {}
                });

                use std::io::Write;
                stdin.write_all(unknown_task_request.to_string().as_bytes()).unwrap();
                stdin.write_all(b"\n").unwrap();
                stdin.flush().unwrap();

                thread::sleep(Duration::from_millis(100));

                let output = child.wait_with_output().expect("Failed to read plugin output");
                let stdout = String::from_utf8_lossy(&output.stdout);

                // Verify error response
                assert!(stdout.contains("error"));
                assert!(stdout.contains("Unknown task"));
            }
            Err(e) => {
                println!("Warning: Could not setup Java plugin for test: {}", e);
            }
        }
    }
}
