//! Portfolio Enhancement Tools for SynCore
//!
//! Three production-quality tools that demonstrate real multi-system integration:
//! - mapping_tool: Application structure mapping with dependency tracking
//! - sequential_step: Reasoning chain tracking with semantic search
//! - application_tool: Code change tracking with history and search
//!
//! All tools integrate with:
//! - MessageBus for event broadcasting
//! - SQLite for persistence
//! - FAISS for semantic search
//! - IntelliTask for task correlation

pub mod mapping_tool;
pub mod sequential_step;
pub mod application_tool;
pub mod code_dependency_extractor;
pub mod code_relationship_store;
pub mod code_graph_extractor;
pub mod code_graph_store;
pub mod code_graph_refactor;

pub use mapping_tool::MappingTool;
pub use sequential_step::SequentialStep;
pub use application_tool::ApplicationTool;
pub use code_dependency_extractor::{CodeDependencies, CodeDependencyExtractor};
pub use code_relationship_store::CodeRelationshipStore;
pub use code_graph_extractor::{
    CallEdge, CodeGraph, CodeGraphExtractor, FunctionNode, ImplementationEdge, ImportNode,
    NodeKind, StructNode, TraitNode,
};
pub use code_graph_store::{CodeGraphStore, GraphQuery, GraphResult};
pub use code_graph_refactor::{
    DeadCode, DuplicateFunction, LongFunction, RefactoringCheckResult, RefactoringPlan,
    RefactoringStep, RefactoringSuggestionEngine,
};
