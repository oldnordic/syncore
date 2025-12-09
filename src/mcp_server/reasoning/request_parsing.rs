//! Request parsing utilities for unified MCP reasoning tools
//!
//! Provides consistent request parameter handling, validation, and normalization
//! across all reasoning tools (raggraph_query, raggraph_multihop, code_graph_fusion_query).

use anyhow::Result;
use serde::Deserialize;

/// Unified request structure for all reasoning tools
#[derive(Debug, Clone)]
pub struct UnifiedReasoningRequest {
    /// Query text or content
    pub query: String,
    /// Request type (query, multihop, fusion)
    pub request_type: RequestType,
    /// Parameters specific to the request type
    pub parameters: RequestParameters,
    /// Optional namespace for scoped search
    pub namespace: Option<String>,
    /// Fusion mode hint (simple, attention, reasoning)
    pub mode_hint: Option<String>,
    /// Maximum number of results to return
    pub top_k: Option<u32>,
    /// Query scope control
    pub scope: Option<String>,
    /// Project label for filtering
    pub project_label: Option<String>,
    /// Local root path for Local scope
    pub local_root: Option<String>,
}

/// Type of reasoning request
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestType {
    /// Single query with vector search and graph expansion
    Query,
    /// Multi-hop graph expansion from seed entities
    MultiHop,
    /// Fusion query with tri-mode reasoning
    Fusion,
}

/// Request parameters specific to each request type
#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub enum RequestParameters {
    /// Parameters for Query requests
    Query {
        /// Include graph connectivity analysis
        include_connectivity: bool,
        /// Include graph embedding scores
        include_embeddings: bool,
    },
    /// Parameters for MultiHop requests
    MultiHop {
        /// Seed entity IDs for expansion
        seed_entities: Vec<i64>,
        /// Maximum number of hops to explore
        max_hops: Option<usize>,
        /// Maximum number of entities to visit
        max_entities: Option<usize>,
        /// Score decay factor per hop
        decay_factor: Option<f32>,
    },
    /// Parameters for Fusion requests
    Fusion {
        /// Fusion mode override (auto/simple/attention/reasoning)
        fusion_mode: Option<String>,
        /// Entity boost configuration
        entity_boost: Option<String>,
        /// Temporal scoring enabled
        enable_temporal: bool,
    },
}

/// Request parsing configuration
#[derive(Debug, Clone)]
pub struct RequestParsingConfig {
    /// Default top_k if not specified
    pub default_top_k: u32,
    /// Maximum allowed top_k
    pub max_top_k: u32,
    /// Validate required parameters
    pub validate_required: bool,
    /// Normalize parameter names (snake_case to camelCase, etc.)
    pub normalize_names: bool,
}

impl Default for RequestParsingConfig {
    fn default() -> Self {
        Self {
            default_top_k: 10,
            max_top_k: 100,
            validate_required: true,
            normalize_names: true,
        }
    }
}

/// Parse and validate request parameters for reasoning tools
///
/// Normalizes different request structures from different MCP tools into a unified format.
/// Handles parameter validation, default value assignment, and type conversion.
///
/// # Arguments
/// * `raw_params` - Raw request parameters from MCP tool call
/// * `request_type` - Type of reasoning request
/// * `config` - Optional parsing configuration
///
/// # Returns
/// Result containing the unified request structure
pub fn parse_unified_request(
    raw_params: serde_json::Map<String, serde_json::Value>,
    request_type: RequestType,
    config: Option<RequestParsingConfig>,
) -> Result<UnifiedReasoningRequest> {
    let config = config.unwrap_or_default();

    // Extract common parameters
    let query = extract_string_param(&raw_params, "query", config.validate_required)?;
    let namespace = extract_optional_string_param(&raw_params, "namespace");
    let mode_hint = extract_optional_string_param(&raw_params, "mode_hint");
    let top_k = extract_numeric_param(&raw_params, "top_k")?
        .map(|v| v as u32)
        .or(Some(config.default_top_k))
        .map(|v| v.min(config.max_top_k));
    let scope = extract_optional_string_param(&raw_params, "scope");
    let project_label = extract_optional_string_param(&raw_params, "project_label");
    let local_root = extract_optional_string_param(&raw_params, "local_root");

    // Extract request-type specific parameters
    let parameters = match request_type {
        RequestType::Query => parse_query_params(&raw_params)?,
        RequestType::MultiHop => parse_multihop_params(&raw_params)?,
        RequestType::Fusion => parse_fusion_params(&raw_params)?,
    };

    Ok(UnifiedReasoningRequest {
        query,
        request_type,
        parameters,
        namespace,
        mode_hint,
        top_k,
        scope,
        project_label,
        local_root,
    })
}

/// Parse Query-specific parameters
fn parse_query_params(
    raw_params: &serde_json::Map<String, serde_json::Value>,
) -> Result<RequestParameters> {
    let include_connectivity =
        extract_bool_param(raw_params, "include_connectivity")?.unwrap_or(true);
    let include_embeddings = extract_bool_param(raw_params, "include_embeddings")?.unwrap_or(true);

    Ok(RequestParameters::Query {
        include_connectivity,
        include_embeddings,
    })
}

/// Parse MultiHop-specific parameters
fn parse_multihop_params(
    raw_params: &serde_json::Map<String, serde_json::Value>,
) -> Result<RequestParameters> {
    let seed_entities = extract_array_param(raw_params, "seed_entities")?.unwrap_or_default();
    let max_hops = extract_numeric_param(raw_params, "max_hops")?.map(|v| v as usize);
    let max_entities = extract_numeric_param(raw_params, "max_entities")?.map(|v| v as usize);
    let decay_factor = extract_numeric_param(raw_params, "decay_factor")?.map(|v| v as f32);

    Ok(RequestParameters::MultiHop {
        seed_entities,
        max_hops,
        max_entities,
        decay_factor,
    })
}

/// Parse Fusion-specific parameters
fn parse_fusion_params(
    raw_params: &serde_json::Map<String, serde_json::Value>,
) -> Result<RequestParameters> {
    let fusion_mode = extract_optional_string_param(raw_params, "fusion_mode");
    let entity_boost = extract_optional_string_param(raw_params, "entity_boost");
    let enable_temporal = extract_bool_param(raw_params, "enable_temporal")?.unwrap_or(true);

    Ok(RequestParameters::Fusion {
        fusion_mode,
        entity_boost,
        enable_temporal,
    })
}

/// Extract string parameter with validation
fn extract_string_param(
    params: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    required: bool,
) -> Result<String> {
    match params.get(key) {
        Some(value) => match value {
            serde_json::Value::String(s) => Ok(s.clone()),
            serde_json::Value::Null => {
                if required {
                    Err(anyhow::anyhow!("Required parameter '{}' is null", key))
                } else {
                    Err(anyhow::anyhow!("Parameter '{}' is null", key))
                }
            }
            _ => Err(anyhow::anyhow!("Parameter '{}' must be a string", key)),
        },
        None => {
            if required {
                Err(anyhow::anyhow!("Required parameter '{}' is missing", key))
            } else {
                Err(anyhow::anyhow!("Parameter '{}' is missing", key))
            }
        }
    }
}

/// Extract optional string parameter
fn extract_optional_string_param(
    params: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Option<String> {
    params.get(key).and_then(|value| match value {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Null => None,
        _ => None,
    })
}

/// Extract numeric parameter (can be integer or float)
fn extract_numeric_param(
    params: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<Option<f64>> {
    match params.get(key) {
        Some(value) => match value {
            serde_json::Value::Number(n) => Ok(Some(n.as_f64().unwrap_or(0.0))),
            serde_json::Value::Null => Ok(None),
            _ => Err(anyhow::anyhow!("Parameter '{}' must be numeric", key)),
        },
        None => Ok(None),
    }
}

/// Extract boolean parameter
fn extract_bool_param(
    params: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<Option<bool>> {
    match params.get(key) {
        Some(value) => match value {
            serde_json::Value::Bool(b) => Ok(Some(*b)),
            serde_json::Value::Null => Ok(None),
            _ => Err(anyhow::anyhow!("Parameter '{}' must be boolean", key)),
        },
        None => Ok(None),
    }
}

/// Extract array of integers parameter
fn extract_array_param(
    params: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<Option<Vec<i64>>> {
    match params.get(key) {
        Some(value) => match value {
            serde_json::Value::Array(arr) => {
                let mut result = Vec::new();
                for (i, item) in arr.iter().enumerate() {
                    match item {
                        serde_json::Value::Number(n) => {
                            result.push(n.as_i64().ok_or_else(|| {
                                anyhow::anyhow!("Array element at index {} is not an integer", i)
                            })?);
                        }
                        _ => {
                            return Err(anyhow::anyhow!(
                                "Array element at index {} is not a number",
                                i
                            ))
                        }
                    }
                }
                Ok(Some(result))
            }
            serde_json::Value::Null => Ok(None),
            _ => Err(anyhow::anyhow!("Parameter '{}' must be an array", key)),
        },
        None => Ok(None),
    }
}

/// Normalize scope string to standard format
pub fn normalize_scope(scope: &str) -> String {
    match scope.to_lowercase().as_str() {
        "local" => "Local".to_string(),
        "project" | "proj" => "Project".to_string(),
        "workspace" | "ws" => "Workspace".to_string(),
        "global" | "g" => "Global".to_string(),
        "auto" => "Auto".to_string(),
        _ => scope.to_string(), // Keep original if unknown
    }
}

/// Validate top_k parameter against limits
pub fn validate_top_k(top_k: Option<u32>, max_allowed: u32) -> Result<Option<u32>> {
    match top_k {
        Some(k) => {
            if k == 0 {
                Ok(None) // Treat 0 as "no limit"
            } else if k > max_allowed {
                Err(anyhow::anyhow!("top_k {} exceeds maximum allowed {}", k, max_allowed))
            } else {
                Ok(Some(k))
            }
        }
        None => Ok(None),
    }
}

/// Convert unified request to tool-specific request structures
pub mod converters {
    use super::*;

    /// Convert unified request to CodeGraphFusionQueryRequest format
    pub fn to_codegraph_fusion_request(
        request: UnifiedReasoningRequest,
    ) -> Result<crate::code_graph::rag_graph::RagGraphQueryRequest> {
        if request.request_type != RequestType::Fusion {
            return Err(anyhow::anyhow!(
                "Request type mismatch: expected Fusion, got {:?}",
                request.request_type
            ));
        }

        Ok(crate::code_graph::rag_graph::RagGraphQueryRequest {
            query: request.query,
            namespace: request.namespace,
            mode_hint: request.mode_hint,
            top_k: request.top_k,
            scope: request.scope,
            project_label: request.project_label,
            local_root: request.local_root,
        })
    }
}

/// Build a unified multihop request from typed RagGraphMultihopRequest
///
/// This helper eliminates the JSON roundtrip for multihop requests by converting
/// the typed struct directly to a UnifiedReasoningRequest.
pub fn build_unified_multihop_request_from_struct(
    multihop_request: &crate::mcp_server::types::RagGraphMultihopRequest,
) -> Result<UnifiedReasoningRequest> {
    let mut raw_params = serde_json::Map::new();

    // Always include seed_entities (required field)
    raw_params.insert(
        "seed_entities".to_string(),
        serde_json::Value::Array(
            multihop_request.seed_nodes
                .iter()
                .map(|&id| serde_json::Value::Number(serde_json::Number::from(id)))
                .collect(),
        ),
    );

    // Include optional fields only if they're Some
    if let Some(max_hops) = multihop_request.max_hops {
        raw_params.insert(
            "max_hops".to_string(),
            serde_json::Value::Number(serde_json::Number::from(max_hops)),
        );
    }

    if let Some(max_entities) = multihop_request.max_entities {
        raw_params.insert(
            "max_entities".to_string(),
            serde_json::Value::Number(serde_json::Number::from(max_entities)),
        );
    }

    if let Some(decay_factor) = multihop_request.decay_factor {
        raw_params.insert(
            "decay_factor".to_string(),
            serde_json::Value::Number(
                serde_json::Number::from_f64(decay_factor as f64)
                    .unwrap_or_else(|| serde_json::Number::from(0)),
            ),
        );
    }

    parse_unified_request(raw_params, RequestType::MultiHop, None)
}

/// Build a unified fusion request from typed RagGraphQueryRequest
///
/// This helper eliminates the JSON roundtrip for fusion requests by converting
/// the typed struct directly to a UnifiedReasoningRequest, injecting enable_temporal=true.
pub fn build_unified_fusion_request_from_struct(
    fusion_request: &crate::code_graph::rag_graph::RagGraphQueryRequest,
) -> Result<UnifiedReasoningRequest> {
    let mut raw_params = serde_json::Map::new();

    // Required query field
    raw_params.insert(
        "query".to_string(),
        serde_json::Value::String(fusion_request.query.clone()),
    );

    // Optional fields
    if let Some(namespace) = &fusion_request.namespace {
        raw_params.insert(
            "namespace".to_string(),
            serde_json::Value::String(namespace.clone()),
        );
    }

    if let Some(mode_hint) = &fusion_request.mode_hint {
        raw_params.insert(
            "mode_hint".to_string(),
            serde_json::Value::String(mode_hint.clone()),
        );
    }

    if let Some(top_k) = fusion_request.top_k {
        raw_params.insert(
            "top_k".to_string(),
            serde_json::Value::Number(serde_json::Number::from(top_k)),
        );
    }

    if let Some(scope) = &fusion_request.scope {
        raw_params.insert(
            "scope".to_string(),
            serde_json::Value::String(scope.clone()),
        );
    }

    if let Some(project_label) = &fusion_request.project_label {
        raw_params.insert(
            "project_label".to_string(),
            serde_json::Value::String(project_label.clone()),
        );
    }

    if let Some(local_root) = &fusion_request.local_root {
        raw_params.insert(
            "local_root".to_string(),
            serde_json::Value::String(local_root.clone()),
        );
    }

    // Inject enable_temporal: true for fusion requests (preserve existing behavior)
    raw_params.insert(
        "enable_temporal".to_string(),
        serde_json::Value::Bool(true),
    );

    parse_unified_request(raw_params, RequestType::Fusion, None)
}


#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_query_request() {
        let mut params = serde_json::Map::new();
        params.insert("query".to_string(), json!("test query"));
        params.insert("top_k".to_string(), json!(5));

        let request = parse_unified_request(
            params,
            RequestType::Query,
            Some(RequestParsingConfig::default()),
        );

        assert!(request.is_ok());
        let unified = request.unwrap();
        assert_eq!(unified.query, "test query");
        assert_eq!(unified.request_type, RequestType::Query);
        assert_eq!(unified.top_k, Some(5));
    }

    #[test]
    fn test_parse_multihop_request() {
        let mut params = serde_json::Map::new();
        params.insert("seed_entities".to_string(), json!([1, 2, 3]));
        params.insert("max_hops".to_string(), json!(5));

        let request = parse_unified_request(
            params,
            RequestType::MultiHop,
            Some(RequestParsingConfig::default()),
        );

        assert!(request.is_ok());
        let unified = request.unwrap();
        assert_eq!(unified.request_type, RequestType::MultiHop);

        if let RequestParameters::MultiHop {
            seed_entities,
            max_hops,
            ..
        } = unified.parameters
        {
            assert_eq!(seed_entities, vec![1, 2, 3]);
            assert_eq!(max_hops, Some(5));
        } else {
            panic!("Expected MultiHop parameters");
        }
    }

    #[test]
    fn test_scope_normalization() {
        assert_eq!(normalize_scope("local"), "Local");
        assert_eq!(normalize_scope("PROJECT"), "Project");
        assert_eq!(normalize_scope("Ws"), "Workspace");
        assert_eq!(normalize_scope("g"), "Global");
        assert_eq!(normalize_scope("auto"), "Auto");
        assert_eq!(normalize_scope("unknown"), "unknown");
    }

    #[test]
    fn test_top_k_validation() {
        assert_eq!(validate_top_k(Some(10), 100).unwrap(), Some(10));
        assert_eq!(validate_top_k(Some(0), 100).unwrap(), None);
        assert!(validate_top_k(Some(150), 100).is_err());
        assert_eq!(validate_top_k(None, 100).unwrap(), None);
    }
}
