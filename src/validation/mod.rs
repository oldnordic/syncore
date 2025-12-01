//! Validation Module
//!
//! Cross-domain consistency validation for SynCore
//! Detects desynchronization between CodeGraph, VectorStore, MemoryStore, and Neo4j

pub mod cross_domain_validator;

pub use cross_domain_validator::{CrossDomainReport, CrossDomainValidator};
