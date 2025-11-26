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

fn create_realistic_cpp_project(dir: &TempDir) {
    let src_dir = dir.path().join("src");
    let include_dir = dir.path().join("include");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&include_dir).unwrap();

    std::fs::write(include_dir.join("engine.h"), r#"
#pragma once
#include <string>
#include <vector>

namespace engine {

class Entity {
public:
    virtual ~Entity() = default;
    virtual void update(float dt) = 0;
};

class Player : public Entity {
public:
    Player(const std::string& name);
    void update(float dt) override;
    static int getPlayerCount();
private:
    std::string name_;
    static int player_count_;
};

struct Vec2 {
    float x, y;
};

enum class GameState {
    Menu,
    Playing,
    Paused
};

} // namespace engine
"#).unwrap();

    std::fs::write(src_dir.join("engine.cpp"), r#"
#include "engine.h"
#include <iostream>

namespace engine {

int Player::player_count_ = 0;

Player::Player(const std::string& name) : name_(name) {
    player_count_++;
}

void Player::update(float dt) {
    std::cout << "Updating: " << name_ << std::endl;
}

int Player::getPlayerCount() {
    return player_count_;
}

} // namespace engine
"#).unwrap();

    std::fs::write(src_dir.join("main.cpp"), r#"
#include "engine.h"

int main() {
    engine::Player player("Hero");
    player.update(0.016f);
    return 0;
}
"#).unwrap();
}

#[test]
fn test_fixture_project_indexing() {
    let temp_dir = TempDir::new().unwrap();
    create_realistic_cpp_project(&temp_dir);

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

    // Index header
    let header_path = temp_dir.path().join("include/engine.h");
    let response = send_command(&mut stdin, &mut stdout, &json!({
        "type": "execute",
        "task": "c.index_file",
        "file_path": header_path.to_str().unwrap()
    }));
    assert!(response["ok"].as_bool().unwrap());

    let entities = response["entities"].as_array().unwrap();
    assert!(!entities.is_empty(), "Should find entities");

    // Shutdown
    send_command(&mut stdin, &mut stdout, &json!({"type": "shutdown"}));
    let _ = child.kill();
}

#[test]
fn test_fixture_directory_indexing() {
    let temp_dir = TempDir::new().unwrap();
    create_realistic_cpp_project(&temp_dir);

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
        "directory_path": temp_dir.path().to_str().unwrap(),
        "recursive": true
    }));
    assert!(response["ok"].as_bool().unwrap());

    // Shutdown
    send_command(&mut stdin, &mut stdout, &json!({"type": "shutdown"}));
    let _ = child.kill();
}
