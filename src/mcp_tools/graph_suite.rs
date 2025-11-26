//! Graph Suite - Unified Neo4j graph operations
//!
//! Commands:
//! - `query`: Execute a Cypher read query
//! - `insert`: Execute a Cypher write query (CREATE, MERGE, SET)
//! - `relate`: Create a relationship between two nodes
//! - `rag_query`: RAG graph query with multi-hop reasoning
//! - `rag_multihop`: Multi-hop graph diffusion from seed nodes
//! - `help`: Show available commands

use crate::mcp_tools::{SuiteDispatcher, SuiteResult};
use crate::router::SynCoreState;
use crate::databases::neo4j::{RelationType, create_relationship};
use serde::{Deserialize, Serialize};

/// Graph suite arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSuiteArgs {
    pub command: String,
    #[serde(default)]
    pub cypher: Option<String>,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
    #[serde(default)]
    pub from_id: Option<i64>,
    #[serde(default)]
    pub to_id: Option<i64>,
    #[serde(default)]
    pub rel_type: Option<String>,
    #[serde(default)]
    pub from_label: Option<String>,
    #[serde(default)]
    pub to_label: Option<String>,
    // RAG graph query fields
    #[serde(default)]
    pub query_text: Option<String>,
    #[serde(default)]
    pub seed_nodes: Option<Vec<i64>>,
}

/// Graph suite implementation
pub struct GraphSuite {
    state: SynCoreState,
}

impl GraphSuite {
    pub fn new(state: SynCoreState) -> Self {
        Self { state }
    }

    /// Execute the suite command
    pub fn execute(&self, args: GraphSuiteArgs) -> SuiteResult {
        match args.command.as_str() {
            "query" => self.cmd_query(args),
            "insert" => self.cmd_insert(args),
            "relate" => self.cmd_relate(args),
            "rag_query" => self.cmd_rag_query(args),
            "rag_multihop" => self.cmd_rag_multihop(args),
            "help" => self.cmd_help(),
            _ => SuiteResult::err(
                &args.command,
                format!(
                    "Unknown command '{}'. Available: query, insert, relate, rag_query, rag_multihop, help",
                    args.command
                ),
            ),
        }
    }

    fn cmd_query(&self, args: GraphSuiteArgs) -> SuiteResult {
        let cypher = match args.cypher {
            Some(c) => c,
            None => return SuiteResult::err("query", "Missing required parameter: cypher"),
        };

        let neo4j = match &self.state.neo4j {
            Some(n) => n.clone(),
            None => return SuiteResult::err("query", "Neo4j not connected"),
        };

        // Convert params to Vec<(String, serde_json::Value)>
        let params: Vec<(&str, serde_json::Value)> = args
            .params
            .as_ref()
            .and_then(|p| p.as_object())
            .map(|obj| obj.iter().map(|(k, v)| (k.as_str(), v.clone())).collect())
            .unwrap_or_default();

        // Execute query synchronously using block_in_place
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { neo4j.execute_query(&cypher, params).await })
        });

        match result {
            Ok(records) => SuiteResult::ok(
                "query",
                serde_json::json!({
                    "cypher": cypher,
                    "record_count": records.len(),
                    "records": records
                }),
            ),
            Err(e) => SuiteResult::err("query", e.to_string()),
        }
    }

    fn cmd_insert(&self, args: GraphSuiteArgs) -> SuiteResult {
        let cypher = match args.cypher {
            Some(c) => c,
            None => return SuiteResult::err("insert", "Missing required parameter: cypher"),
        };

        let neo4j = match &self.state.neo4j {
            Some(n) => n.clone(),
            None => return SuiteResult::err("insert", "Neo4j not connected"),
        };

        let params: Vec<(&str, serde_json::Value)> = args
            .params
            .as_ref()
            .and_then(|p| p.as_object())
            .map(|obj| obj.iter().map(|(k, v)| (k.as_str(), v.clone())).collect())
            .unwrap_or_default();

        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { neo4j.execute_query(&cypher, params).await })
        });

        match result {
            Ok(_) => SuiteResult::ok(
                "insert",
                serde_json::json!({
                    "executed": true,
                    "cypher": cypher
                }),
            ),
            Err(e) => SuiteResult::err("insert", e.to_string()),
        }
    }

    fn cmd_relate(&self, args: GraphSuiteArgs) -> SuiteResult {
        let from_id = match args.from_id {
            Some(id) => id,
            None => return SuiteResult::err("relate", "Missing required parameter: from_id"),
        };

        let to_id = match args.to_id {
            Some(id) => id,
            None => return SuiteResult::err("relate", "Missing required parameter: to_id"),
        };

        let rel_type_str = match args.rel_type {
            Some(r) => r,
            None => return SuiteResult::err("relate", "Missing required parameter: rel_type"),
        };

        let neo4j = match &self.state.neo4j {
            Some(n) => n.clone(),
            None => return SuiteResult::err("relate", "Neo4j not connected"),
        };

        // Parse relationship type string to canonical RelationType
        let rel_type = match RelationType::from_str(&rel_type_str) {
            Some(rt) => rt,
            None => {
                return SuiteResult::err(
                    "relate",
                    format!("Unknown relationship type: {}. Valid types: CALLS, IMPORTS, IMPLEMENTS, USES, REFERENCES, INHERITS, CONTAINS, USES_FIELD, USES_TYPE, MODULE_CHILD, DECLARES, HAS_MEMBER, OWNS", rel_type_str)
                )
            }
        };

        // Use canonical create_relationship (handles namespace, :SynCore filtering, idempotency)
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { create_relationship(&*neo4j, from_id, to_id, rel_type).await })
        });

        match result {
            Ok(_) => SuiteResult::ok(
                "relate",
                serde_json::json!({
                    "created": true,
                    "from_id": from_id,
                    "to_id": to_id,
                    "rel_type": rel_type_str
                }),
            ),
            Err(e) => SuiteResult::err("relate", e.to_string()),
        }
    }

    fn cmd_rag_query(&self, args: GraphSuiteArgs) -> SuiteResult {
        let query_text = match args.query_text {
            Some(q) => q,
            None => return SuiteResult::err("rag_query", "Missing required parameter: query_text"),
        };

        use crate::raggraph::{
            validate_real_backend, RagGraphConfig, RagQuery, RaggraphBackendMode,
            RealStorageAdapter,
        };
        use std::sync::{Arc, Mutex};

        // Read config from environment
        let config = RagGraphConfig::from_env();

        let query_engine = if config.backend_mode == RaggraphBackendMode::Real {
            // Real mode: requires VectorStore + Neo4j
            if let Some(ref neo4j) = self.state.neo4j {
                // Use CODE domain store for graph operations (code entities)
                let vector_index =
                    self.state.code_store.clone() as Arc<Mutex<dyn crate::vector::VectorIndex>>;

                let dimension = {
                    use crate::vector::VectorIndex;
                    let store = self.state.code_store.lock().unwrap();
                    VectorIndex::dimension(&*store).unwrap_or(384)
                };

                // Validate backend
                let validation_result = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        validate_real_backend(
                            config.backend_mode.clone(),
                            Some(&**neo4j),
                            Some(&vector_index),
                            dimension,
                        )
                        .await
                    })
                });

                if let Err(e) = validation_result {
                    return SuiteResult::err("rag_query", format!("Validation failed: {}", e));
                }

                let storage = Arc::new(RealStorageAdapter::new(
                    vector_index,
                    (**neo4j).clone(),
                    dimension,
                ));

                RagQuery::with_storage(config.clone(), storage)
            } else {
                return SuiteResult::err("rag_query", "Neo4j not available for real mode");
            }
        } else {
            // Mock mode
            RagQuery::new()
        };

        match query_engine.query(&query_text) {
            Ok(result) => SuiteResult::ok(
                "rag_query",
                serde_json::json!({
                    "top_nodes": result.top_nodes,
                    "context_embedding_dim": result.context_embedding.len(),
                    "reasoning_path": result.reasoning_path
                }),
            ),
            Err(e) => SuiteResult::err("rag_query", e.to_string()),
        }
    }

    fn cmd_rag_multihop(&self, args: GraphSuiteArgs) -> SuiteResult {
        let seed_nodes = match args.seed_nodes {
            Some(s) => s,
            None => {
                return SuiteResult::err("rag_multihop", "Missing required parameter: seed_nodes")
            }
        };

        use crate::raggraph::{
            validate_real_backend, HopGraphTransformer, RagGraphConfig, RaggraphBackendMode,
            RealStorageAdapter,
        };
        use std::sync::{Arc, Mutex};

        // Read config from environment
        let config = RagGraphConfig::from_env();

        let transformer = if config.backend_mode == RaggraphBackendMode::Real {
            // Real mode: requires VectorStore + Neo4j
            if let Some(ref neo4j) = self.state.neo4j {
                // Use CODE domain store for graph operations (code entities)
                let vector_index =
                    self.state.code_store.clone() as Arc<Mutex<dyn crate::vector::VectorIndex>>;

                let dimension = {
                    use crate::vector::VectorIndex;
                    let store = self.state.code_store.lock().unwrap();
                    VectorIndex::dimension(&*store).unwrap_or(384)
                };

                // Validate backend
                let validation_result = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        validate_real_backend(
                            config.backend_mode.clone(),
                            Some(&**neo4j),
                            Some(&vector_index),
                            dimension,
                        )
                        .await
                    })
                });

                if let Err(e) = validation_result {
                    return SuiteResult::err("rag_multihop", format!("Validation failed: {}", e));
                }

                let storage = Arc::new(RealStorageAdapter::new(
                    vector_index,
                    (**neo4j).clone(),
                    dimension,
                ));

                HopGraphTransformer::with_storage(config.clone(), storage)
            } else {
                return SuiteResult::err("rag_multihop", "Neo4j not available for real mode");
            }
        } else {
            // Mock mode
            HopGraphTransformer::new(config)
        };

        match transformer.multi_hop_reasoning(&seed_nodes) {
            Ok(result) => SuiteResult::ok(
                "rag_multihop",
                serde_json::json!({
                    "top_nodes": result.top_nodes,
                    "reasoning_steps": result.reasoning_path.len()
                }),
            ),
            Err(e) => SuiteResult::err("rag_multihop", e.to_string()),
        }
    }

    fn cmd_help(&self) -> SuiteResult {
        SuiteResult::ok(
            "help",
            serde_json::json!({
                "suite": "graph_suite",
                "description": "Neo4j graph operations and RAG reasoning",
                "commands": {
                    "query": {
                        "description": "Execute a Cypher read query",
                        "params": ["cypher (required)", "params (optional)"]
                    },
                    "insert": {
                        "description": "Execute a Cypher write query",
                        "params": ["cypher (required)", "params (optional)"]
                    },
                    "relate": {
                        "description": "Create a relationship between two nodes",
                        "params": ["from_id (required)", "to_id (required)", "rel_type (required)", "from_label (optional)", "to_label (optional)"]
                    },
                    "rag_query": {
                        "description": "RAG graph query with multi-hop reasoning",
                        "params": ["query_text (required)"]
                    },
                    "rag_multihop": {
                        "description": "Multi-hop graph diffusion from seed nodes",
                        "params": ["seed_nodes (required, array of node IDs)"]
                    }
                }
            }),
        )
    }
}

impl SuiteDispatcher for GraphSuite {
    fn dispatch(&self, command: &str, args: serde_json::Value) -> SuiteResult {
        let mut suite_args: GraphSuiteArgs = match serde_json::from_value(args) {
            Ok(a) => a,
            Err(e) => return SuiteResult::err(command, format!("Invalid arguments: {}", e)),
        };
        suite_args.command = command.to_string();
        self.execute(suite_args)
    }

    fn list_commands(&self) -> Vec<&'static str> {
        vec![
            "query",
            "insert",
            "relate",
            "rag_query",
            "rag_multihop",
            "help",
        ]
    }

    fn help(&self, command: &str) -> Option<&'static str> {
        match command {
            "query" => Some("Execute Cypher read query. Params: cypher, params"),
            "insert" => Some("Execute Cypher write query. Params: cypher, params"),
            "relate" => Some("Create relationship. Params: from_id, to_id, rel_type, labels"),
            "rag_query" => Some("RAG graph query. Params: query_text"),
            "rag_multihop" => Some("Multi-hop diffusion. Params: seed_nodes"),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_suite_args_deserialization() {
        let json = serde_json::json!({
            "command": "query",
            "cypher": "MATCH (n) RETURN n LIMIT 10"
        });

        let args: GraphSuiteArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.command, "query");
        assert_eq!(args.cypher, Some("MATCH (n) RETURN n LIMIT 10".to_string()));
    }
}
