use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tempfile::TempDir;
use serde_json::{json, Value};

fn get_plugin_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug/syncore_c_cpp_plugin")
}

fn send_command(stdin: &mut std::process::ChildStdin, stdout: &mut BufReader<std::process::ChildStdout>, cmd: &Value) -> Value {
    let cmd_str = serde_json::to_string(cmd).unwrap();
    writeln!(stdin, "{}", cmd_str).unwrap();
    stdin.flush().unwrap();

    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    serde_json::from_str(&line).unwrap()
}

#[test]
fn test_diagnostics_returns_array() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.cpp");
    std::fs::write(&file_path, r#"
int main() {
    return 0;
}
"#).unwrap();

    let mut child = Command::new(get_plugin_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    // Init
    let response = send_command(&mut stdin, &mut stdout, &json!({
        "type": "init",
        "workspace_root": temp_dir.path().to_str().unwrap()
    }));
    assert!(response["ok"].as_bool().unwrap());

    // Run diagnostics
    let response = send_command(&mut stdin, &mut stdout, &json!({
        "type": "execute",
        "task": "c.run_diagnostics",
        "file_path": file_path.to_str().unwrap()
    }));
    assert!(response["ok"].as_bool().unwrap());

    // Should have diagnostics array (may be empty if no issues)
    assert!(response.get("diagnostics").is_some());
    assert!(response["diagnostics"].is_array());

    // Shutdown
    send_command(&mut stdin, &mut stdout, &json!({"type": "shutdown"}));
    let _ = child.kill();
}

#[test]
fn test_diagnostics_missing_file_path() {
    let temp_dir = TempDir::new().unwrap();

    let mut child = Command::new(get_plugin_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    // Init
    let response = send_command(&mut stdin, &mut stdout, &json!({
        "type": "init",
        "workspace_root": temp_dir.path().to_str().unwrap()
    }));
    assert!(response["ok"].as_bool().unwrap());

    // Run diagnostics without file_path
    let response = send_command(&mut stdin, &mut stdout, &json!({
        "type": "execute",
        "task": "c.run_diagnostics"
    }));
    assert!(!response["ok"].as_bool().unwrap());
    assert_eq!(response["error"].as_str().unwrap(), "invalid_request");

    // Shutdown
    send_command(&mut stdin, &mut stdout, &json!({"type": "shutdown"}));
    let _ = child.kill();
}

#[test]
fn test_diagnostics_nonexistent_file() {
    let temp_dir = TempDir::new().unwrap();

    let mut child = Command::new(get_plugin_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    // Init
    let response = send_command(&mut stdin, &mut stdout, &json!({
        "type": "init",
        "workspace_root": temp_dir.path().to_str().unwrap()
    }));
    assert!(response["ok"].as_bool().unwrap());

    // Run diagnostics on nonexistent file
    let response = send_command(&mut stdin, &mut stdout, &json!({
        "type": "execute",
        "task": "c.run_diagnostics",
        "file_path": "/nonexistent/file.cpp"
    }));
    // Should fail or return empty diagnostics
    // Either behavior is acceptable
    if response["ok"].as_bool().unwrap() {
        assert!(response["diagnostics"].is_array());
    } else {
        assert!(response.get("error").is_some());
    }

    // Shutdown
    send_command(&mut stdin, &mut stdout, &json!({"type": "shutdown"}));
    let _ = child.kill();
}

#[test]
fn test_diagnostics_malformed_cpp() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("malformed.cpp");
    std::fs::write(&file_path, r#"
// Malformed C++ code
int main() {
    int x = 5  // Missing semicolon
    return 0
// Missing closing brace
"#).unwrap();

    let mut child = Command::new(get_plugin_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    // Init
    let response = send_command(&mut stdin, &mut stdout, &json!({
        "type": "init",
        "workspace_root": temp_dir.path().to_str().unwrap()
    }));
    assert!(response["ok"].as_bool().unwrap());

    // Run diagnostics on malformed file
    let response = send_command(&mut stdin, &mut stdout, &json!({
        "type": "execute",
        "task": "c.run_diagnostics",
        "file_path": file_path.to_str().unwrap()
    }));
    // Should succeed (plugin shouldn't crash on bad input)
    assert!(response["ok"].as_bool().unwrap());
    assert!(response["diagnostics"].is_array());

    // Shutdown
    send_command(&mut stdin, &mut stdout, &json!({"type": "shutdown"}));
    let _ = child.kill();
}

#[test]
fn test_diagnostics_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("empty.cpp");
    std::fs::write(&file_path, "").unwrap();

    let mut child = Command::new(get_plugin_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    // Init
    let response = send_command(&mut stdin, &mut stdout, &json!({
        "type": "init",
        "workspace_root": temp_dir.path().to_str().unwrap()
    }));
    assert!(response["ok"].as_bool().unwrap());

    // Run diagnostics on empty file
    let response = send_command(&mut stdin, &mut stdout, &json!({
        "type": "execute",
        "task": "c.run_diagnostics",
        "file_path": file_path.to_str().unwrap()
    }));
    assert!(response["ok"].as_bool().unwrap());
    assert!(response["diagnostics"].is_array());

    // Shutdown
    send_command(&mut stdin, &mut stdout, &json!({"type": "shutdown"}));
    let _ = child.kill();
}
