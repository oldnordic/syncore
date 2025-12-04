//! APEX 1.8 REFRAG Suite - Selective expansion pipeline tools
//!
//! Commands:
//! - query: Execute REFRAG query with selective expansion
//! - configure: Update REFRAG configuration
//! - help: Show available commands

use super::{SuiteDispatcher, SuiteResult};
use crate::mcp_tools::streaming::OutputLimiter;
use crate::refrag::{RefragConfig, RefragPipeline, SelectionPolicy};
use crate::router::SynCoreState;
use serde::Deserialize;
use serde_json::json;

/// REFRAG query request
#[derive(Debug, Deserialize)]
pub struct RefragQueryRequest {
    /// Query text
    pub query: String,
    /// Optional: top-k raw chunks (default: 5)
    #[serde(default)]
    pub top_k_raw: Option<usize>,
    /// Optional: max tokens (default: 4000)
    #[serde(default)]
    pub max_tokens: Option<usize>,
    /// Optional: selection policy
    #[serde(default)]
    pub policy: Option<String>,
}

/// REFRAG configuration request
#[derive(Debug, Deserialize)]
pub struct RefragConfigRequest {
    /// top-k raw chunks
    #[serde(default)]
    pub top_k_raw: Option<usize>,
    /// max tokens
    #[serde(default)]
    pub max_tokens: Option<usize>,
    /// selection policy (TopK, TopPercent, Weighted, GraphPriority)
    #[serde(default)]
    pub policy: Option<String>,
}

/// REFRAG suite dispatcher
pub struct RefragSuite {
    state: SynCoreState,
}

impl RefragSuite {
    pub fn new(state: SynCoreState) -> Self {
        Self {
            state,
        }
    }

    fn handle_query(&self, args: serde_json::Value) -> SuiteResult {
        let req: RefragQueryRequest = match serde_json::from_value(args) {
            Ok(r) => r,
            Err(e) => return SuiteResult::err("query", format!("Invalid request: {}", e)),
        };

        // Build configuration
        let mut config = RefragConfig::default();
        if let Some(k) = req.top_k_raw {
            config.top_k_raw = k;
        }
        if let Some(tokens) = req.max_tokens {
            config.max_tokens = tokens;
        }
        if let Some(policy_str) = req.policy {
            config.selection_policy = parse_selection_policy(&policy_str);
        }

        // Execute pipeline
        let pipeline = RefragPipeline::new(self.state.clone(), config);
        match pipeline.query(&req.query) {
            Ok(result) => {
                let data = json!({
                    "prompt": result.prompt,
                    "raw_count": result.raw_count,
                    "compressed_count": result.compressed_count,
                    "total_tokens": result.total_tokens,
                    "metadata": result.metadata,
                });
                SuiteResult::ok("query", data)
            }
            Err(e) => SuiteResult::err("query", format!("Pipeline error: {}", e)),
        }
    }

    fn handle_configure(&self, args: serde_json::Value) -> SuiteResult {
        let req: RefragConfigRequest = match serde_json::from_value(args) {
            Ok(r) => r,
            Err(e) => return SuiteResult::err("configure", format!("Invalid request: {}", e)),
        };

        let mut config = RefragConfig::default();
        if let Some(k) = req.top_k_raw {
            config.top_k_raw = k;
        }
        if let Some(tokens) = req.max_tokens {
            config.max_tokens = tokens;
        }
        if let Some(policy_str) = req.policy {
            config.selection_policy = parse_selection_policy(&policy_str);
        }

        let data = json!({
            "top_k_raw": config.top_k_raw,
            "max_tokens": config.max_tokens,
            "selection_policy": format!("{:?}", config.selection_policy),
        });

        SuiteResult::ok("configure", data)
    }

    fn handle_help(&self) -> SuiteResult {
        let help_text = "
REFRAG Suite - Selective Expansion Pipeline

Commands:
  query       Execute REFRAG query with selective expansion
  configure   Update REFRAG configuration
  help        Show this help message

Examples:
  refrag_suite query {\"query\": \"implement authentication\"}
  refrag_suite query {\"query\": \"error handling\", \"top_k_raw\": 10, \"max_tokens\": 8000}
  refrag_suite configure {\"top_k_raw\": 7, \"policy\": \"GraphPriority\"}
";
        SuiteResult::ok("help", json!({"message": help_text}))
    }
}

impl SuiteDispatcher for RefragSuite {
    fn dispatch(&self, command: &str, args: serde_json::Value) -> SuiteResult {
        let result = match command {
            "query" => self.handle_query(args),
            "configure" => self.handle_configure(args),
            "help" => self.handle_help(),
            _ => SuiteResult::err(
                command,
                format!("Unknown command: {}. Use 'help' to see available commands.", command),
            ),
        };

        // Apply streaming contract enforcement to successful responses
        if result.success {
            let limiter = OutputLimiter::default();
            let result_json = json!({
                "command": command,
                "data": result.data
            });

            match limiter.apply_json(&result_json) {
                Ok(limited_json) => {
                    if let Some(limited_data) = limited_json.get("data") {
                        SuiteResult {
                            success: true,
                            command: command.to_string(),
                            data: limited_data.clone(),
                            error: None,
                        }
                    } else {
                        result
                    }
                },
                Err(_) => result, // Fallback to original on error
            }
        } else {
            result
        }
    }

    fn list_commands(&self) -> Vec<&'static str> {
        vec!["query", "configure", "help"]
    }

    fn help(&self, command: &str) -> Option<&'static str> {
        match command {
            "query" => Some("Execute REFRAG query with selective expansion (params: query, top_k_raw?, max_tokens?, policy?)"),
            "configure" => Some("Update REFRAG configuration (params: top_k_raw?, max_tokens?, policy?)"),
            "help" => Some("Show available commands and usage examples"),
            _ => None,
        }
    }
}

/// Parse selection policy from string
fn parse_selection_policy(policy_str: &str) -> SelectionPolicy {
    match policy_str {
        "TopK" => SelectionPolicy::TopK(5),
        "TopPercent" => SelectionPolicy::TopPercent(20),
        "GraphPriority" => SelectionPolicy::GraphPriority,
        "Weighted" => SelectionPolicy::Weighted {
            fusion_weight: 0.5,
            graph_weight: 0.3,
            structural_weight: 0.2,
        },
        _ => SelectionPolicy::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_selection_policy() {
        let policy = parse_selection_policy("TopK");
        assert!(matches!(policy, SelectionPolicy::TopK(5)));

        let policy = parse_selection_policy("GraphPriority");
        assert!(matches!(policy, SelectionPolicy::GraphPriority));
    }
}
