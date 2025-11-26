//! Project Analysis Engine (PAE)
//!
//! Provides LLM-free, deterministic, human-readable codebase intelligence
//! on top of existing infrastructure (SQLite, Neo4j, HNSW).

pub mod architecture_overview;
pub mod cleanup;
pub mod code_smells;
pub mod complexity_dashboard;
pub mod cycles;
pub mod dead_code;
pub mod deps;
pub mod deps_unified;
pub mod diagnostics;
pub mod diagnostics_severity;
pub mod file_report;
pub mod file_report_core;
pub mod file_report_summary;
pub mod hotspots;
pub mod improvement_roadmap;
pub mod metrics;
pub mod python_backend_ingestion;
pub mod refactor;
pub mod refactor_action_plan;
pub mod refactor_hotspots;
pub mod refactor_patterns;
pub mod risk_score;
pub mod rust_backend_ingestion;
pub mod rust_macro_expander;
pub mod unused_imports;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Common response envelope for all PAE tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PAEResponse<T> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> PAEResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(message),
        }
    }
}

/// Entity information from code_entities table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityInfo {
    pub id: i64,
    pub name: String,
    pub entity_type: String,
    pub file_path: String,
    pub line_start: i32,
    pub line_end: i32,
    pub signature: Option<String>,
    pub docstring: Option<String>,
    pub language: String,
    pub visibility: Option<String>,
}

/// Relationship information from code_edges table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipInfo {
    pub src_entity_id: i64,
    pub dst_entity_id: i64,
    pub edge_type: String,
    pub src_entity_name: String,
    pub dst_entity_name: String,
    pub src_file_path: String,
    pub dst_file_path: String,
}

/// Module-level information for project mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub id: String,
    pub file_path: String,
    pub entity_count: u32,
    pub fan_in: u32,
    pub fan_out: u32,
    pub loc: Option<u32>,
}

/// Edge between modules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleEdge {
    pub from_file: String,
    pub to_file: String,
    pub relationship_type: String,
}

/// Hotspot analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotspotInfo {
    pub file_path: String,
    pub fan_in: u32,
    pub fan_out: u32,
    pub entity_count: u32,
    pub loc: Option<u32>,
    pub score: f32,
}

/// Circular dependency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleInfo {
    pub files: Vec<String>,
    pub relation_kinds: Vec<String>,
    pub cycle_length: usize,
}

/// Dead code entity information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadCodeInfo {
    pub id: i64,
    pub name: String,
    pub entity_type: String,
    pub file_path: String,
    pub visibility: Option<String>,
    pub line_start: i32,
}

/// File-level metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetrics {
    pub fan_in: u32,
    pub fan_out: u32,
    pub entity_count: u32,
}

/// Unused import information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnusedImportInfo {
    pub file_path: String,
    pub import_name: String,
    pub line: Option<i32>,
    pub module: Option<String>,
}

/// Refactor suggestion kinds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RefactorKind {
    SplitFile,
    ExtractFacade,
    ReduceCycle,
    PruneDeadCode,
    SimplifyDependency,
}

/// Refactor suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactorSuggestion {
    pub kind: RefactorKind,
    pub description: String,
    pub file_path: Option<String>,
    pub related_files: Option<Vec<String>>,
    pub metrics: HashMap<String, serde_json::Value>,
}

/// Main PAE engine that coordinates all analysis tools
pub struct ProjectAnalysisEngine {
    db_manager: std::sync::Arc<crate::db::DbManager>,
    neo4j: Option<std::sync::Arc<crate::graph::Neo4jClient>>,
}

impl ProjectAnalysisEngine {
    pub fn new(
        db_manager: std::sync::Arc<crate::db::DbManager>,
        neo4j: Option<std::sync::Arc<crate::graph::Neo4jClient>>,
    ) -> Self {
        Self { db_manager, neo4j }
    }

    /// Get database connection for code graph
    pub fn code_graph_conn(&self) -> std::sync::Arc<std::sync::Mutex<rusqlite::Connection>> {
        self.db_manager.code_graph_conn()
    }

    /// Get Neo4j client if available
    pub fn neo4j(&self) -> Option<&std::sync::Arc<crate::graph::Neo4jClient>> {
        self.neo4j.as_ref()
    }

    /// Get database manager
    pub fn db_manager(&self) -> &std::sync::Arc<crate::db::DbManager> {
        &self.db_manager
    }
}

// Re-export all the submodules for easier access
pub use architecture_overview::*;
pub use cleanup::*;
pub use complexity_dashboard::*;
pub use cycles::*;
pub use dead_code::*;
pub use deps::*;
pub use deps_unified::*;
pub use diagnostics::*;
pub use diagnostics_severity::*;
pub use file_report::*;
pub use hotspots::*;
pub use improvement_roadmap::*;
pub use metrics::*;
pub use python_backend_ingestion::*;
pub use refactor::*;
pub use refactor_action_plan::*;
pub use risk_score::*;
pub use rust_backend_ingestion::*;
pub use rust_macro_expander::*;
pub use unused_imports::*;
