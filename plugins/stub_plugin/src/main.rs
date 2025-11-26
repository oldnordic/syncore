use serde_json;
use std::io::{self, Write};

fn main() {
    let mut line_buffer = String::new();
    while io::stdin().read_line(&mut line_buffer).unwrap() > 0 {
        let line = line_buffer.trim();
        if line.is_empty() { continue; }
        
        let request: serde_json::Value = serde_json::from_str(line).unwrap();
        
        if let Some(event) = request.get("event") {
            if event == "init" {
                let response = serde_json::json!({
                    "status": "ready"
                });
                println!("{}", serde_json::to_string(&response).unwrap());
                io::stdout().flush().unwrap();
            } else if event == "shutdown" {
                let response = serde_json::json!({
                    "status": "ok"
                });
                println!("{}", serde_json::to_string(&response).unwrap());
                io::stdout().flush().unwrap();
                break;
            }
        } else if let Some(task) = request.get("task") {
            if task == "capabilities" {
                let response = serde_json::json!({
                    "capabilities": ["index_directory", "lsp_ingest", "lint_ingest", "diagnostics_export"]
                });
                println!("{}", serde_json::to_string(&response).unwrap());
                io::stdout().flush().unwrap();
            } else if task == "index_directory" {
                let params = request.get("params").unwrap().as_object().unwrap();
                let directory = params.get("directory").unwrap().as_str().unwrap();
                
                let response = serde_json::json!({
                    "result": {
                        "indexed_files": 42,
                        "directory": directory
                    },
                    "diagnostics": [
                        {"level": "info", "message": "Successfully indexed directory", "file": "test.rs"}
                    ],
                    "entities": [
                        {"type": "function", "name": "test_function", "file": "test.rs", "line": 10},
                        {"type": "class", "name": "TestClass", "file": "test.rs", "line": 20}
                    ]
                });
                println!("{}", serde_json::to_string(&response).unwrap());
                io::stdout().flush().unwrap();
            } else if task == "lsp_ingest" {
                let response = serde_json::json!({
                    "result": {
                        "symbols_processed": 15,
                        "completions_generated": 8
                    },
                    "diagnostics": [
                        {"level": "warning", "message": "Unused variable", "file": "test.rs", "line": 5}
                    ],
                    "entities": [
                        {"type": "variable", "name": "test_var", "file": "test.rs", "line": 5}
                    ]
                });
                println!("{}", serde_json::to_string(&response).unwrap());
                io::stdout().flush().unwrap();
            } else {
                let response = serde_json::json!({
                    "result": {"task": task.as_str().unwrap_or("unknown")},
                    "diagnostics": [],
                    "entities": []
                });
                println!("{}", serde_json::to_string(&response).unwrap());
                io::stdout().flush().unwrap();
            }
        }
        
        line_buffer.clear();
    }
}