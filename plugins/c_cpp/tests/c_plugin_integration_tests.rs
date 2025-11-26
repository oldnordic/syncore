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
fn test_full_workflow() {
    let temp_dir = TempDir::new().unwrap();

    // Create a simple C++ project
    let main_cpp = temp_dir.path().join("main.cpp");
    std::fs::write(&main_cpp, r#"
#include "utils.h"

int main() {
    greet("World");
    return 0;
}
"#).unwrap();

    let utils_h = temp_dir.path().join("utils.h");
    std::fs::write(&utils_h, r#"
#pragma once
#include <string>

void greet(const std::string& name);
"#).unwrap();

    let utils_cpp = temp_dir.path().join("utils.cpp");
    std::fs::write(&utils_cpp, r#"
#include "utils.h"
#include <iostream>

void greet(const std::string& name) {
    std::cout << "Hello, " << name << "!" << std::endl;
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

    // Index main.cpp
    let response = send_command(&mut stdin, &mut stdout, &json!({
        "type": "execute",
        "task": "c.index_file",
        "file_path": main_cpp.to_str().unwrap()
    }));
    assert!(response["ok"].as_bool().unwrap());
    assert!(response["entities"].is_array());

    // Index utils.cpp
    let response = send_command(&mut stdin, &mut stdout, &json!({
        "type": "execute",
        "task": "c.index_file",
        "file_path": utils_cpp.to_str().unwrap()
    }));
    assert!(response["ok"].as_bool().unwrap());
    assert!(response["entities"].is_array());

    // Run diagnostics on main.cpp
    let response = send_command(&mut stdin, &mut stdout, &json!({
        "type": "execute",
        "task": "c.run_diagnostics",
        "file_path": main_cpp.to_str().unwrap()
    }));
    assert!(response["ok"].as_bool().unwrap());
    assert!(response["diagnostics"].is_array());

    // Shutdown
    send_command(&mut stdin, &mut stdout, &json!({"type": "shutdown"}));
    let _ = child.kill();
}

#[test]
fn test_index_directory() {
    let temp_dir = TempDir::new().unwrap();

    // Create multiple C++ files
    std::fs::write(temp_dir.path().join("a.cpp"), "void a() {}").unwrap();
    std::fs::write(temp_dir.path().join("b.cpp"), "void b() {}").unwrap();
    std::fs::write(temp_dir.path().join("c.cpp"), "void c() {}").unwrap();

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

    // Index directory
    let response = send_command(&mut stdin, &mut stdout, &json!({
        "type": "execute",
        "task": "c.index_directory",
        "directory_path": temp_dir.path().to_str().unwrap()
    }));
    assert!(response["ok"].as_bool().unwrap());

    // Should have indexed files
    if let Some(indexed_files) = response.get("indexed_files") {
        let files = indexed_files.as_array().unwrap();
        assert!(files.len() >= 3, "Should index at least 3 files");
    }

    // Shutdown
    send_command(&mut stdin, &mut stdout, &json!({"type": "shutdown"}));
    let _ = child.kill();
}

#[test]
fn test_plugin_handles_concurrent_requests() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.cpp");
    std::fs::write(&file_path, "int main() { return 0; }").unwrap();

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

    // Send multiple index requests in sequence
    for _ in 0..5 {
        let response = send_command(&mut stdin, &mut stdout, &json!({
            "type": "execute",
            "task": "c.index_file",
            "file_path": file_path.to_str().unwrap()
        }));
        assert!(response["ok"].as_bool().unwrap());
    }

    // Shutdown
    send_command(&mut stdin, &mut stdout, &json!({"type": "shutdown"}));
    let _ = child.kill();
}

#[test]
fn test_plugin_robustness() {
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

    // Test with empty file
    let empty_file = temp_dir.path().join("empty.cpp");
    std::fs::write(&empty_file, "").unwrap();

    let response = send_command(&mut stdin, &mut stdout, &json!({
        "type": "execute",
        "task": "c.index_file",
        "file_path": empty_file.to_str().unwrap()
    }));
    assert!(response["ok"].as_bool().unwrap());

    // Test with comment-only file
    let comment_file = temp_dir.path().join("comment.cpp");
    std::fs::write(&comment_file, "// Just a comment").unwrap();

    let response = send_command(&mut stdin, &mut stdout, &json!({
        "type": "execute",
        "task": "c.index_file",
        "file_path": comment_file.to_str().unwrap()
    }));
    assert!(response["ok"].as_bool().unwrap());

    // Shutdown
    send_command(&mut stdin, &mut stdout, &json!({"type": "shutdown"}));
    let _ = child.kill();
}
