//! Fast map utilities for performance-critical paths
//!
//! This module provides type aliases for fast hash maps using non-cryptographic
//! hashers that are optimized for performance in hot paths.

use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::HashMap;
use std::hash::BuildHasherDefault;

/// Fast HashMap using rustc-hash's FxHasher
///
/// This is a drop-in replacement for std::collections::HashMap that provides
/// faster hashing for performance-critical code paths. FxHasher is a
/// non-cryptographic hash function that is optimized for speed.
///
/// Note: Use `FastHashMap::default()` instead of `FastHashMap::new()` to create instances.
pub type FastHashMap<K, V> = FxHashMap<K, V>;

/// Fast HashSet using rustc-hash's FxHasher
///
/// This is a drop-in replacement for std::collections::HashSet that provides
/// faster hashing for performance-critical code paths.
///
/// Note: Use `FastHashSet::default()` instead of `FastHashSet::new()` to create instances.
pub type FastHashSet<K> = FxHashSet<K>;

/// Alternative FastHashMap using std::collections::HashMap with FxHasher
/// This provides more compatibility with HashMap methods including `new()`
pub type StdFastHashMap<K, V> = HashMap<K, V, BuildHasherDefault<rustc_hash::FxHasher>>;
