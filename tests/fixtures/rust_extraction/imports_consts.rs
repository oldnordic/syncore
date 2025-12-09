//! Test fixture for Rust imports and constants extraction
//! Expected counts:
//! - Imports: 12 (line 6, 8, 9, 10, 11, 24, 25, 26, 27, 31, 32, 33)
//! - Constants: 7 (4 global, 2 in TestConstants, 1 in impl)

// Top-level imports
use std::collections::HashMap;
use std::fmt::{self, Debug, Display};
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

mod nested_mod {
    // Import inside module
    use super::*;
    use std::sync::Arc;
}

// Grouped imports (should be counted as individual imports or grouped - need to verify)
use std::io::{self, prelude::*, BufReader, BufWriter};
use std::thread::{self, sleep, spawn};

// Use with alias
use std::time::Duration as TimeDuration;

// Global constants
const MAX_SIZE: usize = 1024;
const DEFAULT_TIMEOUT: u64 = 30;
const APP_NAME: &str = "TestApp";
const VERSION: (u8, u8, u8) = (1, 0, 0);

// Static variable
static GLOBAL_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

pub struct TestConstants {
    module_id: u32,
}

impl TestConstants {
    // Associated constants
    const DEFAULT_ID: u32 = 100;
    pub const MAX_CONNECTIONS: u32 = 1000;

    pub fn new(module_id: u32) -> Self {
        Self { module_id }
    }

    // Method that uses constants
    pub fn is_valid(&self) -> bool {
        self.module_id >= Self::DEFAULT_ID && self.module_id <= Self::MAX_CONNECTIONS
    }
}

// Implementation with constants
impl Default for TestConstants {
    fn default() -> Self {
        Self {
            module_id: Self::DEFAULT_ID,
        }
    }
}

// More imports to test edge cases
use std::result::Result as StdResult;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};

// Function that uses imports
pub fn process_data() -> StdResult<HashMap<String, Vec<i32>>, Box<dyn std::error::Error>> {
    let mut data = HashMap::new();
    data.insert("test".to_string(), vec![1, 2, 3]);
    Ok(data)
}