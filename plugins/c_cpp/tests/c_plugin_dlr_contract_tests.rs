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
fn test_init_command() {
    let temp_dir = TempDir::new().unwrap();
    let mut child = Command::new(get_plugin_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    // Test init command
    let response = send_command(&mut stdin, &mut stdout, &json!({
        "type": "init",
        "workspace_root": temp_dir.path().to_str().unwrap()
    }));
    assert!(response["ok"].as_bool().unwrap());
    assert_eq!(
        response["workspace"].as_str().unwrap(),
        temp_dir.path().to_str().unwrap()
    );

    // Shutdown
    send_command(&mut stdin, &mut stdout, &json!({"type": "shutdown"}));
    let _ = child.kill();
}

#[test]
fn test_capabilities_command() {
    let mut child = Command::new(get_plugin_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    // Test capabilities (works without init)
    let response = send_command(&mut stdin, &mut stdout, &json!({
        "type": "capabilities"
    }));
    assert!(response["ok"].as_bool().unwrap());

    let capabilities = response["capabilities"].as_object().unwrap();
    let tasks = capabilities["tasks"].as_array().unwrap();

    assert!(tasks.contains(&json!("c.index_file")));
    assert!(tasks.contains(&json!("c.index_directory")));
    assert!(tasks.contains(&json!("c.run_diagnostics")));

    // Shutdown
    send_command(&mut stdin, &mut stdout, &json!({"type": "shutdown"}));
    let _ = child.kill();
}

#[test]
fn test_invalid_command_type() {
    let mut child = Command::new(get_plugin_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    // Test invalid command type
    let response = send_command(&mut stdin, &mut stdout, &json!({
        "type": "invalid_command"
    }));
    assert!(!response["ok"].as_bool().unwrap());
    assert_eq!(response["error"].as_str().unwrap(), "invalid_request");

    // Shutdown
    send_command(&mut stdin, &mut stdout, &json!({"type": "shutdown"}));
    let _ = child.kill();
}

#[test]
fn test_invalid_json() {
    let mut child = Command::new(get_plugin_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    // Send invalid JSON
    writeln!(stdin, "invalid json").unwrap();
    stdin.flush().unwrap();

    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    let response: Value = serde_json::from_str(&line).unwrap();

    assert!(!response["ok"].as_bool().unwrap());
    assert_eq!(response["error"].as_str().unwrap(), "invalid_request");

    // Shutdown
    send_command(&mut stdin, &mut stdout, &json!({"type": "shutdown"}));
    let _ = child.kill();
}

#[test]
fn test_execute_before_init() {
    let mut child = Command::new(get_plugin_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    // Try to execute without init
    let response = send_command(&mut stdin, &mut stdout, &json!({
        "type": "execute",
        "task": "c.index_file",
        "file_path": "test.cpp"
    }));
    assert!(!response["ok"].as_bool().unwrap());
    assert_eq!(response["error"].as_str().unwrap(), "not_initialized");

    // Shutdown
    send_command(&mut stdin, &mut stdout, &json!({"type": "shutdown"}));
    let _ = child.kill();
}

#[test]
fn test_unsupported_task() {
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

    // Test unsupported task
    let response = send_command(&mut stdin, &mut stdout, &json!({
        "type": "execute",
        "task": "c.unsupported_task"
    }));
    assert!(!response["ok"].as_bool().unwrap());
    assert_eq!(response["error"].as_str().unwrap(), "unsupported_task");

    // Shutdown
    send_command(&mut stdin, &mut stdout, &json!({"type": "shutdown"}));
    let _ = child.kill();
}

#[test]
fn test_missing_task_parameter() {
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

    // Execute without task
    let response = send_command(&mut stdin, &mut stdout, &json!({
        "type": "execute"
    }));
    assert!(!response["ok"].as_bool().unwrap());
    assert_eq!(response["error"].as_str().unwrap(), "invalid_request");

    // Shutdown
    send_command(&mut stdin, &mut stdout, &json!({"type": "shutdown"}));
    let _ = child.kill();
}

#[test]
fn test_missing_file_path() {
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

    // Index file without file_path
    let response = send_command(&mut stdin, &mut stdout, &json!({
        "type": "execute",
        "task": "c.index_file"
    }));
    assert!(!response["ok"].as_bool().unwrap());
    assert_eq!(response["error"].as_str().unwrap(), "invalid_request");

    // Shutdown
    send_command(&mut stdin, &mut stdout, &json!({"type": "shutdown"}));
    let _ = child.kill();
}

#[test]
fn test_line_based_io() {
    let temp_dir = TempDir::new().unwrap();
    let mut child = Command::new(get_plugin_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    // Send multiple commands in sequence
    // Init
    let response = send_command(&mut stdin, &mut stdout, &json!({
        "type": "init",
        "workspace_root": temp_dir.path().to_str().unwrap()
    }));
    assert!(response["ok"].as_bool().unwrap());

    // Capabilities
    let response = send_command(&mut stdin, &mut stdout, &json!({
        "type": "capabilities"
    }));
    assert!(response["ok"].as_bool().unwrap());

    // Shutdown
    let response = send_command(&mut stdin, &mut stdout, &json!({
        "type": "shutdown"
    }));
    assert!(response["ok"].as_bool().unwrap());

    let _ = child.kill();
}

#[test]
fn test_shutdown_command() {
    let mut child = Command::new(get_plugin_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    // Shutdown without init (should still work)
    let response = send_command(&mut stdin, &mut stdout, &json!({
        "type": "shutdown"
    }));
    assert!(response["ok"].as_bool().unwrap());

    let _ = child.kill();
}
