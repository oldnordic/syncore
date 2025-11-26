// SPEC: SYNCORE-CODE-TOOLS-IMPROVEMENT-01 (APEX v1.2)
// STEP B: mapping_deps with real import extraction tests
//
// Tests verify that mapping_deps extracts real Rust use statements
// when the file_nodes table is empty/incomplete.

use syncore::macro_tools::import_extractor::{extract_rust_imports, RustImport};

// ============================================================
// Part 1: Rust import extraction tests
// ============================================================

#[test]
fn test_extract_simple_use_statement() {
    let code = r#"
use std::collections::HashMap;

fn main() {}
"#;
    let imports = extract_rust_imports(code);
    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].path, "std::collections::HashMap");
}

#[test]
fn test_extract_use_with_alias() {
    let code = r#"
use std::io::Result as IoResult;

fn main() {}
"#;
    let imports = extract_rust_imports(code);
    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].path, "std::io::Result");
    assert_eq!(imports[0].alias, Some("IoResult".to_string()));
}

#[test]
fn test_extract_use_group() {
    let code = r#"
use std::collections::{HashMap, HashSet, BTreeMap};

fn main() {}
"#;
    let imports = extract_rust_imports(code);
    assert_eq!(imports.len(), 3);

    let paths: Vec<&str> = imports.iter().map(|i| i.path.as_str()).collect();
    assert!(paths.contains(&"std::collections::HashMap"));
    assert!(paths.contains(&"std::collections::HashSet"));
    assert!(paths.contains(&"std::collections::BTreeMap"));
}

#[test]
fn test_extract_use_nested_group() {
    let code = r#"
use std::{
    collections::HashMap,
    io::{Read, Write},
};

fn main() {}
"#;
    let imports = extract_rust_imports(code);
    assert!(imports.len() >= 3);

    let paths: Vec<&str> = imports.iter().map(|i| i.path.as_str()).collect();
    assert!(paths.contains(&"std::collections::HashMap"));
    assert!(paths.contains(&"std::io::Read"));
    assert!(paths.contains(&"std::io::Write"));
}

#[test]
fn test_extract_use_self() {
    let code = r#"
use std::collections::{self, HashMap};

fn main() {}
"#;
    let imports = extract_rust_imports(code);
    assert!(imports.len() >= 2);

    let paths: Vec<&str> = imports.iter().map(|i| i.path.as_str()).collect();
    assert!(paths.contains(&"std::collections"));
    assert!(paths.contains(&"std::collections::HashMap"));
}

#[test]
fn test_extract_use_glob() {
    let code = r#"
use std::collections::*;

fn main() {}
"#;
    let imports = extract_rust_imports(code);
    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].path, "std::collections::*");
    assert!(imports[0].is_glob);
}

#[test]
fn test_extract_crate_use() {
    let code = r#"
use crate::memory::MemoryStore;
use crate::tasks::{Task, TaskManager};

fn main() {}
"#;
    let imports = extract_rust_imports(code);
    assert!(imports.len() >= 2);

    let paths: Vec<&str> = imports.iter().map(|i| i.path.as_str()).collect();
    assert!(paths.contains(&"crate::memory::MemoryStore"));
}

#[test]
fn test_extract_super_use() {
    let code = r#"
use super::parent_module;
use super::sibling::{A, B};

fn main() {}
"#;
    let imports = extract_rust_imports(code);
    assert!(imports.len() >= 2);

    let paths: Vec<&str> = imports.iter().map(|i| i.path.as_str()).collect();
    assert!(paths.contains(&"super::parent_module"));
}

#[test]
fn test_extract_extern_crate() {
    let code = r#"
extern crate serde;
extern crate tokio;

fn main() {}
"#;
    let imports = extract_rust_imports(code);
    assert_eq!(imports.len(), 2);

    let paths: Vec<&str> = imports.iter().map(|i| i.path.as_str()).collect();
    assert!(paths.contains(&"serde"));
    assert!(paths.contains(&"tokio"));
}

#[test]
fn test_extract_no_imports() {
    let code = r#"
fn main() {
    println!("Hello, world!");
}
"#;
    let imports = extract_rust_imports(code);
    assert!(imports.is_empty());
}

#[test]
fn test_extract_handles_comments() {
    let code = r#"
// use std::io::Write; // This should be ignored
/* use std::fmt::Display; */
use std::collections::HashMap;

fn main() {}
"#;
    let imports = extract_rust_imports(code);
    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].path, "std::collections::HashMap");
}

#[test]
fn test_extract_pub_use() {
    let code = r#"
pub use crate::memory::MemoryStore;
pub(crate) use crate::tasks::Task;

fn main() {}
"#;
    let imports = extract_rust_imports(code);
    assert_eq!(imports.len(), 2);
    assert!(imports[0].is_pub);
}

#[test]
fn test_extract_multiple_lines() {
    let code = r#"
use std::collections::HashMap;
use std::io::{Read, Write};
use anyhow::Result;
use serde::{Deserialize, Serialize};

fn main() {}
"#;
    let imports = extract_rust_imports(code);
    // HashMap, Read, Write, Result, Deserialize, Serialize = 6 imports
    assert!(imports.len() >= 6);
}

// ============================================================
// Part 2: Resolving crate/super paths to file paths
// ============================================================

use syncore::macro_tools::import_extractor::resolve_import_to_file;

#[test]
fn test_resolve_crate_import_to_file() {
    // In a project with src/memory.rs, "crate::memory::MemoryStore" should resolve
    let resolved = resolve_import_to_file(
        "crate::memory::MemoryStore",
        "/project/src/main.rs",
        "/project",
    );

    // Could be src/memory.rs or src/memory/mod.rs
    assert!(
        resolved == Some("src/memory.rs".to_string())
            || resolved == Some("src/memory/mod.rs".to_string())
    );
}

#[test]
fn test_resolve_super_import() {
    let resolved = resolve_import_to_file(
        "super::sibling",
        "/project/src/module/submodule.rs",
        "/project",
    );

    assert!(
        resolved == Some("src/module/sibling.rs".to_string())
            || resolved == Some("src/module/sibling/mod.rs".to_string())
    );
}

#[test]
fn test_resolve_external_crate_returns_none() {
    // External crates like "serde" shouldn't resolve to local files
    let resolved = resolve_import_to_file("serde::Serialize", "/project/src/main.rs", "/project");

    assert!(resolved.is_none());
}

#[test]
fn test_resolve_std_returns_none() {
    // Standard library imports don't have local files
    let resolved = resolve_import_to_file(
        "std::collections::HashMap",
        "/project/src/main.rs",
        "/project",
    );

    assert!(resolved.is_none());
}
