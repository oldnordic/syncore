//! SQLiteGraph Async Backend Module
//!
//! Provides a hybrid async façade for synchronous GraphBackend implementations.
//! This module enables seamless async/await usage while keeping the core SQLiteGraph
//! implementation fully synchronous.
//!
//! ## Architecture
//!
//! - **AsyncSQLiteBackend**: Thin async wrapper using spawn_blocking
//! - **Core SQLiteGraph**: Remains fully synchronous (unchanged)
//! - **Error Handling**: Proper JoinError mapping and propagation
//! - **Thread Safety**: Uses Arc<dyn GraphBackend> for thread-safe sharing

pub mod async_sqlite_backend;

pub use async_sqlite_backend::AsyncSQLiteBackend;