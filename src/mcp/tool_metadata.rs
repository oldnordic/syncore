//! Tool metadata layer for all SynCore MCP tools
//!
//! Provides categorization, cost estimation, and side-effect tracking
//! for all 49 MCP tools.

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tool category for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolCategory {
    Memory,
    Task,
    Vector,
    Parser,
    Code,
    Document,
    Graph,
    Sequential,
    IntelliTask,
    Agent,
    Mapping,
    Application,
    Logs,
}

/// Relative cost estimate for tool execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolCost {
    Low,      // < 10ms
    Medium,   // 10-100ms
    High,     // 100-1000ms
    VeryHigh, // > 1000ms
    CpuHeavy, // CPU intensive operations
}

/// Side effects that a tool may have
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SideEffects {
    pub modifies_database: bool,
    pub modifies_filesystem: bool,
    pub modifies_vector_store: bool,
    pub modifies_graph: bool,
    pub network_call: bool,
}

impl SideEffects {
    pub fn none() -> Self {
        Self {
            modifies_database: false,
            modifies_filesystem: false,
            modifies_vector_store: false,
            modifies_graph: false,
            network_call: false,
        }
    }

    pub fn read_only() -> Self {
        Self::none()
    }

    pub fn database_write() -> Self {
        Self {
            modifies_database: true,
            ..Self::none()
        }
    }

    pub fn filesystem_write() -> Self {
        Self {
            modifies_filesystem: true,
            ..Self::none()
        }
    }

    pub fn vector_write() -> Self {
        Self {
            modifies_vector_store: true,
            ..Self::none()
        }
    }

    pub fn graph_write() -> Self {
        Self {
            modifies_graph: true,
            network_call: true, // Neo4j is external
            ..Self::none()
        }
    }

    pub fn has_side_effects(&self) -> bool {
        self.modifies_database
            || self.modifies_filesystem
            || self.modifies_vector_store
            || self.modifies_graph
    }

    pub fn cpu_heavy() -> Self {
        Self {
            modifies_database: false,
            modifies_filesystem: false,
            modifies_vector_store: false,
            modifies_graph: false,
            network_call: false,
        }
    }
}

/// Metadata for a single tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub name: &'static str,
    pub version: &'static str,
    pub category: ToolCategory,
    pub cost: ToolCost,
    pub side_effects: SideEffects,
    pub description: &'static str,
}

/// Global registry of all tool metadata
pub static TOOL_REGISTRY: Lazy<HashMap<&'static str, ToolMetadata>> = Lazy::new(|| {
    let mut registry = HashMap::new();

    // Memory tools
    registry.insert(
        "memory_store",
        ToolMetadata {
            name: "memory_store",
            version: "1.0.0",
            category: ToolCategory::Memory,
            cost: ToolCost::Low,
            side_effects: SideEffects::database_write(),
            description: "Store a key-value pair in memory",
        },
    );

    registry.insert(
        "memory_query",
        ToolMetadata {
            name: "memory_query",
            version: "1.0.0",
            category: ToolCategory::Memory,
            cost: ToolCost::Low,
            side_effects: SideEffects::read_only(),
            description: "Query a value from memory by key",
        },
    );

    // Task tools
    registry.insert(
        "task_create",
        ToolMetadata {
            name: "task_create",
            version: "1.0.0",
            category: ToolCategory::Task,
            cost: ToolCost::Low,
            side_effects: SideEffects::database_write(),
            description: "Create a new task",
        },
    );

    registry.insert(
        "task_list",
        ToolMetadata {
            name: "task_list",
            version: "1.0.0",
            category: ToolCategory::Task,
            cost: ToolCost::Medium,
            side_effects: SideEffects::read_only(),
            description: "List all tasks",
        },
    );

    registry.insert(
        "task_get",
        ToolMetadata {
            name: "task_get",
            version: "1.0.0",
            category: ToolCategory::Task,
            cost: ToolCost::Medium,
            side_effects: SideEffects::read_only(),
            description: "Get specific task by ID",
        },
    );

    registry.insert(
        "task_update",
        ToolMetadata {
            name: "task_update",
            version: "1.0.0",
            category: ToolCategory::Task,
            cost: ToolCost::Medium,
            side_effects: SideEffects::database_write(),
            description: "Update task status",
        },
    );

    registry.insert(
        "task_next",
        ToolMetadata {
            name: "task_next",
            version: "1.0.0",
            category: ToolCategory::Task,
            cost: ToolCost::Medium,
            side_effects: SideEffects::read_only(),
            description: "Get next task ready to work on",
        },
    );

    // Vector tools
    registry.insert(
        "vector_insert",
        ToolMetadata {
            name: "vector_insert",
            version: "1.0.0",
            category: ToolCategory::Vector,
            cost: ToolCost::Medium,
            side_effects: SideEffects::vector_write(),
            description: "Insert text into vector store with embeddings",
        },
    );

    registry.insert(
        "vector_search",
        ToolMetadata {
            name: "vector_search",
            version: "1.0.0",
            category: ToolCategory::Vector,
            cost: ToolCost::Medium,
            side_effects: SideEffects::read_only(),
            description: "Search vector store by semantic similarity",
        },
    );

    // Parser tools
    registry.insert(
        "parser_analyze",
        ToolMetadata {
            name: "parser_analyze",
            version: "1.0.0",
            category: ToolCategory::Parser,
            cost: ToolCost::Medium,
            side_effects: SideEffects::read_only(),
            description: "Analyze code structure using tree-sitter",
        },
    );

    registry.insert(
        "parser_search",
        ToolMetadata {
            name: "parser_search",
            version: "1.0.0",
            category: ToolCategory::Parser,
            cost: ToolCost::Medium,
            side_effects: SideEffects::read_only(),
            description: "Search code patterns using ripgrep",
        },
    );

    // Code tools
    registry.insert(
        "code_index",
        ToolMetadata {
            name: "code_index",
            version: "1.0.0",
            category: ToolCategory::Code,
            cost: ToolCost::High,
            side_effects: SideEffects::vector_write(),
            description: "Index a code file for semantic search",
        },
    );

    registry.insert(
        "code_search",
        ToolMetadata {
            name: "code_search",
            version: "1.0.0",
            category: ToolCategory::Code,
            cost: ToolCost::Medium,
            side_effects: SideEffects::read_only(),
            description: "Search code by semantic meaning",
        },
    );

    registry.insert(
        "code_index_directory",
        ToolMetadata {
            name: "code_index_directory",
            version: "1.0.0",
            category: ToolCategory::Code,
            cost: ToolCost::VeryHigh,
            side_effects: SideEffects::vector_write(),
            description: "Index all code files in a directory",
        },
    );

    // Document tools
    registry.insert(
        "document_index",
        ToolMetadata {
            name: "document_index",
            version: "1.0.0",
            category: ToolCategory::Document,
            cost: ToolCost::VeryHigh,
            side_effects: SideEffects::vector_write(),
            description: "Index documents from a directory",
        },
    );

    registry.insert(
        "document_search",
        ToolMetadata {
            name: "document_search",
            version: "1.0.0",
            category: ToolCategory::Document,
            cost: ToolCost::Medium,
            side_effects: SideEffects::read_only(),
            description: "Search documents by semantic similarity",
        },
    );

    // Graph tools
    registry.insert(
        "graph_query",
        ToolMetadata {
            name: "graph_query",
            version: "1.0.0",
            category: ToolCategory::Graph,
            cost: ToolCost::High,
            side_effects: SideEffects {
                network_call: true,
                ..SideEffects::none()
            },
            description: "Execute Cypher read query on Neo4j",
        },
    );

    registry.insert(
        "graph_insert",
        ToolMetadata {
            name: "graph_insert",
            version: "1.0.0",
            category: ToolCategory::Graph,
            cost: ToolCost::High,
            side_effects: SideEffects::graph_write(),
            description: "Execute Cypher write query on Neo4j",
        },
    );

    registry.insert(
        "graph_relate",
        ToolMetadata {
            name: "graph_relate",
            version: "1.0.0",
            category: ToolCategory::Graph,
            cost: ToolCost::Medium,
            side_effects: SideEffects::graph_write(),
            description: "Create relationship between nodes",
        },
    );

    registry.insert(
        "graph_suite",
        ToolMetadata {
            name: "graph_suite",
            version: "1.0.0",
            category: ToolCategory::Graph,
            cost: ToolCost::Medium,
            side_effects: SideEffects::read_only(),
            description: "Unified graph operations suite",
        },
    );

    // Debug suite
    registry.insert(
        "debug_suite",
        ToolMetadata {
            name: "debug_suite",
            version: "1.0.0",
            category: ToolCategory::Logs,
            cost: ToolCost::Medium,
            side_effects: SideEffects::read_only(),
            description: "Debugging, logs, and project analysis suite",
        },
    );

    // Mapping suite
    registry.insert(
        "mapping_suite",
        ToolMetadata {
            name: "mapping_suite",
            version: "1.0.0",
            category: ToolCategory::Application,
            cost: ToolCost::Medium,
            side_effects: SideEffects::database_write(),
            description: "Application structure mapping suite",
        },
    );

    // Reasoning tools
    registry.insert(
        "reasoning_session_create",
        ToolMetadata {
            name: "reasoning_session_create",
            version: "1.0.0",
            category: ToolCategory::Sequential,
            cost: ToolCost::Medium,
            side_effects: SideEffects::database_write(),
            description: "Create a new reasoning session",
        },
    );

    registry.insert(
        "reasoning_branch_expand",
        ToolMetadata {
            name: "reasoning_branch_expand",
            version: "1.0.0",
            category: ToolCategory::Sequential,
            cost: ToolCost::CpuHeavy,
            side_effects: SideEffects::database_write(),
            description: "Expand a reasoning branch with new thought",
        },
    );

    registry.insert(
        "reasoning_tree_get",
        ToolMetadata {
            name: "reasoning_tree_get",
            version: "1.0.0",
            category: ToolCategory::Sequential,
            cost: ToolCost::Medium,
            side_effects: SideEffects::read_only(),
            description: "Get reasoning tree structure",
        },
    );

    registry.insert(
        "reasoning_tree_prune",
        ToolMetadata {
            name: "reasoning_tree_prune",
            version: "1.0.0",
            category: ToolCategory::Sequential,
            cost: ToolCost::Medium,
            side_effects: SideEffects::database_write(),
            description: "Prune reasoning tree subtree",
        },
    );

    // IntelliTask tools
    registry.insert(
        "intellitask_generate",
        ToolMetadata {
            name: "intellitask_generate",
            version: "1.0.0",
            category: ToolCategory::IntelliTask,
            cost: ToolCost::CpuHeavy,
            side_effects: SideEffects::read_only(),
            description: "Generate tasks from PRD using AI",
        },
    );

    registry.insert(
        "intellitask_subtasks",
        ToolMetadata {
            name: "intellitask_subtasks",
            version: "1.0.0",
            category: ToolCategory::IntelliTask,
            cost: ToolCost::CpuHeavy,
            side_effects: SideEffects::read_only(),
            description: "Generate subtasks using AI",
        },
    );

    registry.insert(
        "intellitask_prioritize",
        ToolMetadata {
            name: "intellitask_prioritize",
            version: "1.0.0",
            category: ToolCategory::IntelliTask,
            cost: ToolCost::CpuHeavy,
            side_effects: SideEffects::read_only(),
            description: "Prioritize tasks using AI",
        },
    );

    registry.insert(
        "intellitask_next",
        ToolMetadata {
            name: "intellitask_next",
            version: "1.0.0",
            category: ToolCategory::IntelliTask,
            cost: ToolCost::Medium,
            side_effects: SideEffects::read_only(),
            description: "Suggest next task using AI",
        },
    );

    registry.insert(
        "intellitask_save",
        ToolMetadata {
            name: "intellitask_save",
            version: "1.0.0",
            category: ToolCategory::IntelliTask,
            cost: ToolCost::Medium,
            side_effects: SideEffects::database_write(),
            description: "Save task breakdown to database",
        },
    );

    registry.insert(
        "intellitask_get",
        ToolMetadata {
            name: "intellitask_get",
            version: "1.0.0",
            category: ToolCategory::IntelliTask,
            cost: ToolCost::Medium,
            side_effects: SideEffects::read_only(),
            description: "Get specific task by ID",
        },
    );

    registry.insert(
        "intellitask_list",
        ToolMetadata {
            name: "intellitask_list",
            version: "1.0.0",
            category: ToolCategory::IntelliTask,
            cost: ToolCost::Medium,
            side_effects: SideEffects::read_only(),
            description: "List all tasks",
        },
    );

    registry.insert(
        "intellitask_update_status",
        ToolMetadata {
            name: "intellitask_update_status",
            version: "1.0.0",
            category: ToolCategory::IntelliTask,
            cost: ToolCost::Medium,
            side_effects: SideEffects::database_write(),
            description: "Update task status",
        },
    );

    registry
});

/// Get metadata for a specific tool
pub fn get_tool_metadata(name: &str) -> Option<&'static ToolMetadata> {
    TOOL_REGISTRY.get(name)
}

/// List all tool metadata
pub fn list_all_metadata() -> Vec<&'static ToolMetadata> {
    TOOL_REGISTRY.values().collect()
}

/// List tools by category
pub fn list_by_category(category: ToolCategory) -> Vec<&'static ToolMetadata> {
    TOOL_REGISTRY.values().filter(|meta| meta.category == category).collect()
}
