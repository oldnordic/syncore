//! Runtime Configuration Module
//!
//! Phase 7 - Runtime executor selection and configuration

pub mod executor_selector;

pub use executor_selector::{create_executor, ExecutorKind};
