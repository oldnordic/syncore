#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::process::{Command, Stdio};
    use serde_json::{json, Value};

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

    #[test]
    fn test_plugin_init() {
        let request = json!({
            "event": "init",
            "plugin_name": "ts_js_plugin",
            "version": "1.0"
        });

        let response = send_plugin_request(&request).expect("Failed to get plugin response");
        
        assert_eq!(response["status"], "ready");
        assert_eq!(response["plugin_name"], "ts_js_plugin");
        assert!(response["supported_tasks"].is_array());
        
        let tasks = response["supported_tasks"].as_array().unwrap();
        assert!(tasks.len() >= 5);
        
        let task_names: Vec<&str> = tasks.iter()
            .filter_map(|t| t.as_str())
            .collect();
        
        assert!(task_names.contains(&"ts_js_index_directory"));
        assert!(task_names.contains(&"ts_js_lsp_diagnostics"));
        assert!(task_names.contains(&"ts_js_eslint"));
        assert!(task_names.contains(&"ts_js_prettier"));
        assert!(task_names.contains(&"ts_js_full_project_analysis"));
    }

    #[test]
    fn test_plugin_capabilities() {
        let request = json!({
            "event": "capabilities"
        });

        let response = send_plugin_request(&request).expect("Failed to get plugin response");
        
        assert_eq!(response["status"], "ok");
        assert!(response["tasks"].is_array());
        
        let tasks = response["tasks"].as_array().unwrap();
        assert!(tasks.len() >= 5);
        
        let task_names: Vec<&str> = tasks.iter()
            .filter_map(|t| t.as_str())
            .collect();
        
        assert!(task_names.contains(&"ts_js_index_directory"));
        assert!(task_names.contains(&"ts_js_lsp_diagnostics"));
        assert!(task_names.contains(&"ts_js_eslint"));
        assert!(task_names.contains(&"ts_js_prettier"));
        assert!(task_names.contains(&"ts_js_full_project_analysis"));
    }

    #[test]
    fn test_unknown_task() {
        let request = json!({
            "event": "execute",
            "task": "unknown_task",
            "params": {}
        });

        let response = send_plugin_request(&request).expect("Failed to get plugin response");
        
        assert_eq!(response["status"], "error");
        assert!(response["error"].is_string());
        let error_msg = response["error"].as_str().unwrap();
        assert!(error_msg.contains("Unknown task"));
    }

    #[test]
    fn test_missing_required_params() {
        let request = json!({
            "event": "execute",
            "task": "ts_js_index_directory",
            "params": {}
        });

        let response = send_plugin_request(&request).expect("Failed to get plugin response");
        
        assert_eq!(response["status"], "error");
        assert!(response["error"].is_string());
        let error_msg = response["error"].as_str().unwrap();
        assert!(error_msg.contains("Missing root_path parameter"));
    }

    #[test]
    fn test_plugin_shutdown() {
        let request = json!({
            "event": "shutdown"
        });

        let response = send_plugin_request(&request).expect("Failed to get plugin response");
        
        assert_eq!(response["status"], "ok");
    }

    #[test]
    fn test_wrong_plugin_name() {
        let request = json!({
            "event": "init",
            "plugin_name": "wrong_plugin",
            "version": "1.0"
        });

        let response = send_plugin_request(&request).expect("Failed to get plugin response");
        
        assert_eq!(response["status"], "error");
        assert!(response["error"].is_string());
        let error_msg = response["error"].as_str().unwrap();
        assert!(error_msg.contains("Unexpected plugin name"));
    }

    #[test]
    fn test_invalid_json() {
        // This test would need to be done differently since we're using serde_json
        // For now, we'll test that the plugin handles malformed JSON gracefully
        let invalid_json = "{ invalid json }";
        
        let mut child = Command::new(get_plugin_binary())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start plugin");

        if let Some(stdin) = child.stdin.as_mut() {
            use std::io::Write;
            stdin.write_all(invalid_json.as_bytes()).unwrap();
            stdin.write_all(b"\n").unwrap();
        }

        let output = child.wait_with_output().expect("Failed to wait for plugin");
        
        // Plugin should respond with an error about invalid JSON
        let response_str = String::from_utf8(output.stdout).expect("Invalid UTF-8 in output");
        let response: Value = serde_json::from_str(&response_str.trim()).expect("Invalid JSON in response");
        
        assert_eq!(response["status"], "error");
        assert!(response["error"].is_string());
        let error_msg = response["error"].as_str().unwrap();
        assert!(error_msg.contains("Invalid JSON"));
    }
}