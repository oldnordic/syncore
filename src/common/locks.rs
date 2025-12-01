//! Internal lock aliases for hot paths in SynCore
//!
//! This module provides type aliases for parking_lot locks that are used
//! in performance-critical code paths. These are NOT part of the public API
//! and should only be used internally where contention is expected.

use parking_lot::{Mutex as PlMutex, RwLock as PlRwLock};

/// Fast mutex backed by parking_lot::Mutex
///
/// Use this in hot paths where contention is expected and performance is critical.
/// parking_lot::Mutex is faster than std::sync::Mutex in contended scenarios.
pub type FastMutex<T> = PlMutex<T>;

/// Fast RwLock backed by parking_lot::RwLock
///
/// Use this in hot paths where contention is expected and performance is critical.
/// parking_lot::RwLock is faster than std::sync::RwLock in contended scenarios.
pub type FastRwLock<T> = PlRwLock<T>;
