mod plugin_api;
mod ts_js_indexer;
mod ts_js_diagnostics;
mod config;

use plugin_api::{PluginRequest, PluginResponse};
use ts_js_indexer::TsJsIndexer;
use ts_js_diagnostics::TsJsDiagnosticsRunner;
use config::TsJsConfig;
use std::collections::HashMap;
use std::io::{self, Write};
use tokio::runtime::Runtime;

fn main() {
    let mut line_buffer = String::new();
    let mut indexer = TsJsIndexer::new().expect("Failed to create TS/JS indexer");
    
    // Create a tokio runtime for async operations
    let rt = Runtime::new().expect("Failed to create tokio runtime");
    
    while io::stdin().read_line(&mut line_buffer).unwrap() > 0 {
        let line = line_buffer.trim();
        if line.is_empty() { continue; }
        
        let request: PluginRequest = match serde_json::from_str(line) {
            Ok(req) => req,
            Err(e) => {
                let response = PluginResponse::error(format!("Invalid JSON: {}", e));
                send_response(&response);
                line_buffer.clear();
                continue;
            }
        };
        
        let response = match request {
            PluginRequest::Init { ref plugin_name, ref version } => {
                handle_init(plugin_name.clone(), version.clone())
            }
            PluginRequest::Capabilities => {
                handle_capabilities()
            }
            PluginRequest::Execute { ref task, ref params } => {
                handle_execute(&mut indexer, task, params.clone(), &rt)
            }
            PluginRequest::Shutdown => {
                handle_shutdown()
            }
        };
        
        send_response(&response);
        
        // Break on shutdown
        if matches!(request, PluginRequest::Shutdown) {
            break;
        }
        
        line_buffer.clear();
    }
}

fn send_response(response: &PluginResponse) {
    let json = serde_json::to_string(response).expect("Failed to serialize response");
    println!("{}", json);
    io::stdout().flush().expect("Failed to flush stdout");
}

fn handle_init(plugin_name: String, _version: String) -> PluginResponse {
    if plugin_name != "ts_js_plugin" {
        return PluginResponse::error(format!("Unexpected plugin name: {}", plugin_name));
    }
    
    let supported_tasks = vec![
        "ts_js_index_directory".to_string(),
        "ts_js_lsp_diagnostics".to_string(),
        "ts_js_eslint".to_string(),
        "ts_js_prettier".to_string(),
        "ts_js_full_project_analysis".to_string(),
    ];
    
    PluginResponse::ready(plugin_name, supported_tasks)
}

fn handle_capabilities() -> PluginResponse {
    let tasks = vec![
        "ts_js_index_directory".to_string(),
        "ts_js_lsp_diagnostics".to_string(),
        "ts_js_eslint".to_string(),
        "ts_js_prettier".to_string(),
        "ts_js_full_project_analysis".to_string(),
    ];
    
    PluginResponse::capabilities(tasks)
}

fn handle_execute(
    indexer: &mut TsJsIndexer,
    task: &str,
    params: HashMap<String, serde_json::Value>,
    rt: &Runtime,
) -> PluginResponse {
    match task {
        "ts_js_index_directory" => {
            handle_index_directory(indexer, params)
        }
        "ts_js_lsp_diagnostics" => {
            rt.block_on(handle_lsp_diagnostics(params))
        }
        "ts_js_eslint" => {
            rt.block_on(handle_eslint(params))
        }
        "ts_js_prettier" => {
            rt.block_on(handle_prettier(params))
        }
        "ts_js_full_project_analysis" => {
            rt.block_on(handle_full_project_analysis(indexer, params))
        }
        _ => {
            PluginResponse::error(format!("Unknown task: {}", task))
        }
    }
}

fn handle_index_directory(
    indexer: &mut TsJsIndexer,
    params: HashMap<String, serde_json::Value>,
) -> PluginResponse {
    let root_path = match params.get("root_path").and_then(|v| v.as_str()) {
        Some(path) => path,
        None => return PluginResponse::error("Missing root_path parameter".to_string()),
    };
    
    match indexer.index_directory(root_path) {
        Ok(result) => PluginResponse::success(result),
        Err(e) => PluginResponse::error(format!("Indexing failed: {}", e)),
    }
}

async fn handle_lsp_diagnostics(params: HashMap<String, serde_json::Value>) -> PluginResponse {
    let project_root = match params.get("project_root").and_then(|v| v.as_str()) {
        Some(path) => path,
        None => return PluginResponse::error("Missing project_root parameter".to_string()),
    };
    
    let tsserver_path = params.get("tsserver_path").and_then(|v| v.as_str()).map(|s| s.to_string());
    
    let runner = TsJsDiagnosticsRunner::new(tsserver_path, None, None);
    
    match runner.run_tsserver_diagnostics(project_root).await {
        Ok(result) => PluginResponse::success(result),
        Err(e) => PluginResponse::error(format!("LSP diagnostics failed: {}", e)),
    }
}

async fn handle_eslint(params: HashMap<String, serde_json::Value>) -> PluginResponse {
    let project_root = match params.get("project_root").and_then(|v| v.as_str()) {
        Some(path) => path,
        None => return PluginResponse::error("Missing project_root parameter".to_string()),
    };
    
    let eslint_path = params.get("eslint_path").and_then(|v| v.as_str())
        .unwrap_or("eslint");
    let eslint_config = params.get("eslint_config").and_then(|v| v.as_str());
    
    let runner = TsJsDiagnosticsRunner::new(None, Some(eslint_path.to_string()), None);
    
    match runner.run_eslint_diagnostics(project_root, eslint_config).await {
        Ok(result) => PluginResponse::success(result),
        Err(e) => PluginResponse::error(format!("ESLint analysis failed: {}", e)),
    }
}

async fn handle_prettier(params: HashMap<String, serde_json::Value>) -> PluginResponse {
    let project_root = match params.get("project_root").and_then(|v| v.as_str()) {
        Some(path) => path,
        None => return PluginResponse::error("Missing project_root parameter".to_string()),
    };
    
    let prettier_path = params.get("prettier_path").and_then(|v| v.as_str())
        .unwrap_or("prettier");
    
    let runner = TsJsDiagnosticsRunner::new(None, None, Some(prettier_path.to_string()));
    
    match runner.run_prettier_diagnostics(project_root).await {
        Ok(result) => PluginResponse::success(result),
        Err(e) => PluginResponse::error(format!("Prettier analysis failed: {}", e)),
    }
}

async fn handle_full_project_analysis(
    indexer: &mut TsJsIndexer,
    params: HashMap<String, serde_json::Value>,
) -> PluginResponse {
    let project_root = match params.get("project_root").and_then(|v| v.as_str()) {
        Some(path) => path,
        None => return PluginResponse::error("Missing project_root parameter".to_string()),
    };
    
    let ts_js_config = if let Some(config_value) = params.get("ts_js_config") {
        serde_json::from_value(config_value.clone()).unwrap_or_default()
    } else {
        TsJsConfig::default()
    };
    
    let mut all_entities = Vec::new();
    let mut all_edges = Vec::new();
    let mut all_diagnostics = Vec::new();
    
    // 1) Index directory
    match indexer.index_directory(project_root) {
        Ok(result) => {
            if let Some(entities) = result.entities {
                all_entities.extend(entities);
            }
            if let Some(edges) = result.edges {
                all_edges.extend(edges);
            }
        }
        Err(e) => {
            return PluginResponse::error(format!("Indexing failed: {}", e));
        }
    }
    
    // 2) Run LSP diagnostics
    let lsp_runner = TsJsDiagnosticsRunner::new(
        ts_js_config.tsserver_path.clone(),
        None,
        None,
    );
    match lsp_runner.run_tsserver_diagnostics(project_root).await {
        Ok(result) => {
            if let Some(diagnostics) = result.diagnostics {
                all_diagnostics.extend(diagnostics);
            }
        }
        Err(e) => {
            // Log error but continue with other analyses
            eprintln!("LSP diagnostics failed: {}", e);
        }
    }
    
    // 3) Run ESLint if configured
    if let Some(eslint_path) = &ts_js_config.eslint_path {
        let eslint_runner = TsJsDiagnosticsRunner::new(
            None,
            Some(eslint_path.clone()),
            None,
        );
        match eslint_runner.run_eslint_diagnostics(project_root, ts_js_config.eslint_config.as_deref()).await {
            Ok(result) => {
                if let Some(diagnostics) = result.diagnostics {
                    all_diagnostics.extend(diagnostics);
                }
            }
            Err(e) => {
                eprintln!("ESLint analysis failed: {}", e);
            }
        }
    }
    
    // 4) Run Prettier if configured
    if let Some(prettier_path) = &ts_js_config.prettier_path {
        let prettier_runner = TsJsDiagnosticsRunner::new(
            None,
            None,
            Some(prettier_path.clone()),
        );
        match prettier_runner.run_prettier_diagnostics(project_root).await {
            Ok(result) => {
                if let Some(diagnostics) = result.diagnostics {
                    all_diagnostics.extend(diagnostics);
                }
            }
            Err(e) => {
                eprintln!("Prettier analysis failed: {}", e);
            }
        }
    }
    
    let result = plugin_api::PluginResult {
        entities: Some(all_entities),
        edges: Some(all_edges),
        diagnostics: Some(all_diagnostics),
        meta: None,
    };
    
    PluginResponse::success(result)
}

fn handle_shutdown() -> PluginResponse {
    PluginResponse::shutdown()
}