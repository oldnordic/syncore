mod plugin_api;
mod java_indexer;
mod java_diagnostics;
mod config;

use plugin_api::{PluginRequest, PluginResponse};
use java_indexer::JavaIndexer;
use java_diagnostics::JavaDiagnosticsRunner;
use config::JavaConfig;
use std::collections::HashMap;
use std::io::{self, Write};

fn main() {
    let mut line_buffer = String::new();
    let mut indexer = JavaIndexer::new().expect("Failed to create Java indexer");
    
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
                handle_execute(&mut indexer, task, params.clone())
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
    if plugin_name != "java_plugin" {
        return PluginResponse::error(format!("Unexpected plugin name: {}", plugin_name));
    }
    
    let supported_tasks = vec![
        "java_index_directory".to_string(),
        "java_run_compiler_diagnostics".to_string(),
        "java_run_errorprone".to_string(),
        "java_run_pmd".to_string(),
        "java_full_project_analysis".to_string(),
    ];
    
    PluginResponse::ready(plugin_name, supported_tasks)
}

fn handle_capabilities() -> PluginResponse {
    let tasks = vec![
        "java_index_directory".to_string(),
        "java_run_compiler_diagnostics".to_string(),
        "java_run_errorprone".to_string(),
        "java_run_pmd".to_string(),
        "java_full_project_analysis".to_string(),
    ];
    
    PluginResponse::capabilities(tasks)
}

fn handle_execute(
    indexer: &mut JavaIndexer,
    task: &str,
    params: HashMap<String, serde_json::Value>,
) -> PluginResponse {
    match task {
        "java_index_directory" => {
            handle_index_directory(indexer, params)
        }
        "java_run_compiler_diagnostics" => {
            handle_compiler_diagnostics(params)
        }
        "java_run_errorprone" => {
            handle_errorprone(params)
        }
        "java_run_pmd" => {
            handle_pmd(params)
        }
        "java_full_project_analysis" => {
            handle_full_project_analysis(indexer, params)
        }
        _ => {
            PluginResponse::error(format!("Unknown task: {}", task))
        }
    }
}

fn handle_index_directory(
    indexer: &mut JavaIndexer,
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

fn handle_compiler_diagnostics(params: HashMap<String, serde_json::Value>) -> PluginResponse {
    let project_root = match params.get("project_root").and_then(|v| v.as_str()) {
        Some(path) => path,
        None => return PluginResponse::error("Missing project_root parameter".to_string()),
    };
    
    let javac_path = params.get("javac_path").and_then(|v| v.as_str()).map(|s| s.to_string());
    let classpath = params.get("classpath").and_then(|v| v.as_str()).map(|s| s.to_string());
    
    let runner = JavaDiagnosticsRunner::new(javac_path);
    
    match runner.run_compiler_diagnostics(project_root, classpath) {
        Ok(result) => PluginResponse::success(result),
        Err(e) => PluginResponse::error(format!("Compiler diagnostics failed: {}", e)),
    }
}

fn handle_errorprone(params: HashMap<String, serde_json::Value>) -> PluginResponse {
    let project_root = match params.get("project_root").and_then(|v| v.as_str()) {
        Some(path) => path,
        None => return PluginResponse::error("Missing project_root parameter".to_string()),
    };
    
    let errorprone_jar = match params.get("errorprone_jar").and_then(|v| v.as_str()) {
        Some(jar) => jar,
        None => return PluginResponse::error("Missing errorprone_jar parameter".to_string()),
    };
    
    let javac_path = params.get("javac_path").and_then(|v| v.as_str()).map(|s| s.to_string());
    
    let runner = JavaDiagnosticsRunner::new(javac_path.clone());
    
    match runner.run_errorprone(project_root, errorprone_jar, javac_path) {
        Ok(result) => PluginResponse::success(result),
        Err(e) => PluginResponse::error(format!("Error Prone analysis failed: {}", e)),
    }
}

fn handle_pmd(params: HashMap<String, serde_json::Value>) -> PluginResponse {
    let project_root = match params.get("project_root").and_then(|v| v.as_str()) {
        Some(path) => path,
        None => return PluginResponse::error("Missing project_root parameter".to_string()),
    };
    
    let ruleset = params.get("ruleset").and_then(|v| v.as_str())
        .unwrap_or("rulesets/java/quickstart.xml");
    
    let pmd_bin = match params.get("pmd_bin").and_then(|v| v.as_str()) {
        Some(bin) => bin,
        None => return PluginResponse::error("Missing pmd_bin parameter".to_string()),
    };
    
    let runner = JavaDiagnosticsRunner::new(None);
    
    match runner.run_pmd(project_root, ruleset, pmd_bin) {
        Ok(result) => PluginResponse::success(result),
        Err(e) => PluginResponse::error(format!("PMD analysis failed: {}", e)),
    }
}

fn handle_full_project_analysis(
    indexer: &mut JavaIndexer,
    params: HashMap<String, serde_json::Value>,
) -> PluginResponse {
    let project_root = match params.get("project_root").and_then(|v| v.as_str()) {
        Some(path) => path,
        None => return PluginResponse::error("Missing project_root parameter".to_string()),
    };
    
    let java_config = if let Some(config_value) = params.get("java_config") {
        serde_json::from_value(config_value.clone()).unwrap_or_default()
    } else {
        JavaConfig::default()
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
    
    // 2) Run compiler diagnostics
    let runner = JavaDiagnosticsRunner::new(java_config.javac_path.clone());
    match runner.run_compiler_diagnostics(project_root, java_config.classpath) {
        Ok(result) => {
            if let Some(diagnostics) = result.diagnostics {
                all_diagnostics.extend(diagnostics);
            }
        }
        Err(e) => {
            // Log error but continue with other analyses
            eprintln!("Compiler diagnostics failed: {}", e);
        }
    }
    
    // 3) Run Error Prone if configured
    if let Some(errorprone_jar) = &java_config.errorprone_jar {
        match runner.run_errorprone(project_root, errorprone_jar, java_config.javac_path.clone()) {
            Ok(result) => {
                if let Some(diagnostics) = result.diagnostics {
                    all_diagnostics.extend(diagnostics);
                }
            }
            Err(e) => {
                eprintln!("Error Prone analysis failed: {}", e);
            }
        }
    }
    
    // 4) Run PMD if configured
    if let (Some(pmd_bin), Some(ruleset)) = (&java_config.pmd_bin, &java_config.ruleset) {
        match runner.run_pmd(project_root, ruleset, pmd_bin) {
            Ok(result) => {
                if let Some(diagnostics) = result.diagnostics {
                    all_diagnostics.extend(diagnostics);
                }
            }
            Err(e) => {
                eprintln!("PMD analysis failed: {}", e);
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