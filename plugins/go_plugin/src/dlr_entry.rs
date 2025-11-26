use anyhow::Result;

use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Write};

use syncore_go_plugin::{GoIndexer, GoDiagnostics, GoplsClient};
use syncore_go_plugin::plugin_api::{PluginRequest, PluginResponse};

fn main() {
    if let Err(e) = run() {
        // Output error as JSON response to stderr and stdout
        let error_response = PluginResponse::error(e.to_string());
        if let Ok(output) = serde_json::to_string(&error_response) {
            let _ = writeln!(io::stdout(), "{}", output);
            let _ = io::stdout().flush();
        }
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let mut stdout = io::stdout();

    // Read one line at a time for interactive/test use
    let mut line = String::new();
    let bytes_read = reader.read_line(&mut line)?;

    if bytes_read == 0 {
        return Err(anyhow::anyhow!("No input received"));
    }

    let response = match serde_json::from_str::<PluginRequest>(line.trim()) {
        Ok(request) => process_request(request),
        Err(e) => PluginResponse::error(format!("Failed to parse JSON request: {}", e)),
    };

    let output = serde_json::to_string(&response)?;
    writeln!(stdout, "{}", output)?;
    stdout.flush()?;
    Ok(())
}

fn process_request(request: PluginRequest) -> PluginResponse {
    match request {
        PluginRequest::Init { plugin_name, version: _ } => {
            PluginResponse::ready(plugin_name, vec![
                "go.index_file".to_string(),
                "go.index_directory".to_string(),
                "go.run_diagnostics".to_string(),
                "go.find_references".to_string(),
                "go.symbol_graph".to_string(),
            ])
        }
        PluginRequest::Capabilities => {
            PluginResponse::capabilities(vec![
                "go.index_file".to_string(),
                "go.index_directory".to_string(),
                "go.run_diagnostics".to_string(),
                "go.find_references".to_string(),
                "go.symbol_graph".to_string(),
            ])
        }
        PluginRequest::Execute { task, params } => {
            match handle_execute(task, params) {
                Ok(response) => response,
                Err(e) => PluginResponse::error(e.to_string()),
            }
        }
        PluginRequest::Shutdown => {
            PluginResponse::shutdown()
        }
    }
}

fn handle_execute(task: String, params: HashMap<String, serde_json::Value>) -> Result<PluginResponse> {
    match task.as_str() {
        "go.index_file" => {
            let file_path = params.get("file_path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing file_path parameter"))?;

            let mut indexer = GoIndexer::new()?;
            let result = indexer.index_file(file_path)?;
            Ok(PluginResponse::success(result))
        }
        "go.index_directory" => {
            let directory_path = params.get("directory_path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing directory_path parameter"))?;

            let mut indexer = GoIndexer::new()?;
            let result = indexer.index_directory(directory_path)?;
            Ok(PluginResponse::success(result))
        }
        "go.run_diagnostics" => {
            let file_path = params.get("file_path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing file_path parameter"))?;

            let diagnostics = GoDiagnostics::new()?;
            let result = diagnostics.run_diagnostics(file_path)?;
            Ok(PluginResponse::success(result))
        }
        "go.find_references" => {
            let file_path = params.get("file_path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing file_path parameter"))?;

            let line = params.get("line")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| anyhow::anyhow!("Missing line parameter"))? as u32;

            let column = params.get("column")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| anyhow::anyhow!("Missing column parameter"))? as u32;

            let gopls = GoplsClient::new()?;
            let result = gopls.find_references(file_path, line, column)?;
            Ok(PluginResponse::success(result))
        }
        "go.symbol_graph" => {
            let query = params.get("query")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing query parameter"))?;

            let workspace_path = params.get("workspace_path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing workspace_path parameter"))?;

            let gopls = GoplsClient::new()?;
            let result = gopls.workspace_symbol(query, workspace_path)?;
            Ok(PluginResponse::success(result))
        }
        _ => {
            Ok(PluginResponse::error(format!("Unknown task: {}", task)))
        }
    }
}
