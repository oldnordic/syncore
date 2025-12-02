//! Dual Parity Test Suite
//!
//! Comprehensive TDD-driven tests to ensure identical behavior between
//! Neo4j and SQLiteGraph backends across all operations.
//!
//! Test Categories:
//! - relationship_parity_tests.rs: CRUD operations parity
//! - pattern_parity_tests.rs: ALL pattern combinations
//! - ordering_parity_tests.rs: Deterministic ordering
//! - cache_parity_tests.rs: Cache path validation

pub mod relationship_parity_tests;
pub mod pattern_parity_tests;
pub mod ordering_parity_tests;
pub mod cache_parity_tests;