// SPEC: SYNCORE-CODE-TOOLS-IMPROVEMENT-01 (APEX v1.2)
// STEP B: Rust import extraction module
//
// Extracts use statements and extern crate declarations from Rust source code
// for mapping_deps functionality.

use std::path::Path;

/// Represents a Rust import statement
#[derive(Debug, Clone, PartialEq)]
pub struct RustImport {
    /// Full path of the import (e.g., "std::collections::HashMap")
    pub path: String,
    /// Optional alias (from `as` keyword)
    pub alias: Option<String>,
    /// Whether this is a glob import (*)
    pub is_glob: bool,
    /// Whether this is a pub use re-export
    pub is_pub: bool,
}

/// Extract all Rust imports from source code
///
/// Handles:
/// - Simple use statements: `use std::collections::HashMap;`
/// - Grouped imports: `use std::{io, fs};`
/// - Nested groups: `use std::{io::{Read, Write}, fs};`
/// - Glob imports: `use std::*;`
/// - Aliases: `use std::io::Result as IoResult;`
/// - pub use re-exports
/// - extern crate declarations
pub fn extract_rust_imports(code: &str) -> Vec<RustImport> {
    let mut imports = Vec::new();

    // Remove comments first to avoid false positives
    let code = remove_comments(code);

    // Normalize multi-line statements by joining lines and splitting on semicolons
    let normalized = code.replace(['\n', '\r'], " ");

    // Split into statements
    for stmt in normalized.split(';') {
        let trimmed = stmt.trim();

        // Handle extern crate
        if trimmed.starts_with("extern crate ") {
            if let Some(import) = parse_extern_crate(&format!("{};", trimmed)) {
                imports.push(import);
            }
            continue;
        }

        // Handle use statements (including pub use)
        let (is_pub, use_line) = if let Some(line) = trimmed.strip_prefix("pub use ") {
            (true, line)
        } else if let Some(line) = trimmed.strip_prefix("pub(crate) use ") {
            (true, line)
        } else if let Some(line) = trimmed.strip_prefix("pub(super) use ") {
            (true, line)
        } else if let Some(line) = trimmed.strip_prefix("use ") {
            (false, line)
        } else {
            continue;
        };

        // Parse the use statement
        let mut parsed = parse_use_statement(use_line.trim());
        for import in &mut parsed {
            import.is_pub = is_pub;
        }
        imports.extend(parsed);
    }

    imports
}

/// Remove line and block comments from code
fn remove_comments(code: &str) -> String {
    let mut result = String::with_capacity(code.len());
    let mut chars = code.chars().peekable();
    let mut in_block_comment = false;

    while let Some(ch) = chars.next() {
        if in_block_comment {
            if ch == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block_comment = false;
            }
            continue;
        }

        if ch == '/' {
            match chars.peek() {
                Some('/') => {
                    // Line comment - skip to end of line
                    while let Some(c) = chars.next() {
                        if c == '\n' {
                            result.push('\n');
                            break;
                        }
                    }
                    continue;
                }
                Some('*') => {
                    chars.next();
                    in_block_comment = true;
                    continue;
                }
                _ => {}
            }
        }
        result.push(ch);
    }

    result
}

/// Parse extern crate statement
fn parse_extern_crate(line: &str) -> Option<RustImport> {
    // extern crate serde;
    // extern crate serde as serde_crate;
    let rest = line.strip_prefix("extern crate ")?.trim_end_matches(';');

    let (path, alias) = if let Some(idx) = rest.find(" as ") {
        (
            rest[..idx].trim().to_string(),
            Some(rest[idx + 4..].trim().to_string()),
        )
    } else {
        (rest.trim().to_string(), None)
    };

    Some(RustImport {
        path,
        alias,
        is_glob: false,
        is_pub: false,
    })
}

/// Parse a use statement (without the "use" keyword)
fn parse_use_statement(stmt: &str) -> Vec<RustImport> {
    let mut imports = Vec::new();

    // Handle grouped imports: std::{A, B}
    if let Some(brace_start) = stmt.find('{') {
        let prefix = stmt[..brace_start].trim().trim_end_matches(':');
        let brace_end = find_matching_brace(stmt, brace_start);
        let group = &stmt[brace_start + 1..brace_end];

        parse_import_group(prefix, group, &mut imports);
    } else {
        // Simple import: std::collections::HashMap
        if let Some(import) = parse_single_import(stmt) {
            imports.push(import);
        }
    }

    imports
}

/// Find matching closing brace
fn find_matching_brace(s: &str, start: usize) -> usize {
    let mut depth = 1;
    let chars: Vec<char> = s.chars().collect();
    for i in (start + 1)..chars.len() {
        match chars[i] {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => {}
        }
    }
    s.len() - 1
}

/// Parse a group of imports with a common prefix
fn parse_import_group(prefix: &str, group: &str, imports: &mut Vec<RustImport>) {
    // Split by commas, but respect nested braces
    let items = split_preserving_braces(group);

    for item in items {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }

        if item == "self" {
            imports.push(RustImport {
                path: prefix.to_string(),
                alias: None,
                is_glob: false,
                is_pub: false,
            });
        } else if item.contains('{') {
            // Nested group
            if let Some(brace_start) = item.find('{') {
                let nested_prefix = item[..brace_start].trim().trim_end_matches(':');
                let full_prefix = if prefix.is_empty() {
                    nested_prefix.to_string()
                } else {
                    format!("{}::{}", prefix, nested_prefix)
                };
                let brace_end = find_matching_brace(item, brace_start);
                let nested_group = &item[brace_start + 1..brace_end];
                parse_import_group(&full_prefix, nested_group, imports);
            }
        } else if let Some(import) = parse_single_import(item) {
            let full_path = if prefix.is_empty() {
                import.path
            } else {
                format!("{}::{}", prefix, import.path)
            };
            imports.push(RustImport {
                path: full_path,
                alias: import.alias,
                is_glob: import.is_glob,
                is_pub: false,
            });
        }
    }
}

/// Split a string by commas, but preserve content inside braces
fn split_preserving_braces(s: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut depth = 0;
    let mut start = 0;

    for (i, ch) in s.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => depth -= 1,
            ',' if depth == 0 => {
                result.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }

    if start < s.len() {
        result.push(&s[start..]);
    }

    result
}

/// Parse a single import (no braces)
fn parse_single_import(item: &str) -> Option<RustImport> {
    let item = item.trim();
    if item.is_empty() {
        return None;
    }

    // Handle alias: HashMap as MyMap
    let (path, alias) = if let Some(idx) = item.find(" as ") {
        (item[..idx].trim(), Some(item[idx + 4..].trim().to_string()))
    } else {
        (item, None)
    };

    // Handle glob: *
    let is_glob = path.ends_with('*');

    Some(RustImport {
        path: path.to_string(),
        alias,
        is_glob,
        is_pub: false,
    })
}

/// Resolve a crate:: or super:: import to a file path
///
/// Returns None for external crates (serde, std, etc.)
pub fn resolve_import_to_file(
    import_path: &str,
    current_file: &str,
    project_root: &str,
) -> Option<String> {
    let current = Path::new(current_file);
    let root = Path::new(project_root);

    // Get relative path from project root
    let relative_current = current.strip_prefix(root).ok()?;
    let current_dir = relative_current.parent()?;

    if let Some(module_path) = import_path.strip_prefix("crate::") {
        // crate::memory::MemoryStore -> src/memory.rs or src/memory/mod.rs
        let parts: Vec<&str> = module_path.split("::").collect();

        // First part is the module name
        if parts.is_empty() {
            return None;
        }

        let module_name = parts[0];
        let possible_paths = vec![
            format!("src/{}.rs", module_name),
            format!("src/{}/mod.rs", module_name),
        ];

        // Return first possible path (we don't check if file exists here)
        possible_paths.into_iter().next()
    } else if let Some(module_path) = import_path.strip_prefix("super::") {
        // super::sibling in src/module/submodule.rs -> src/module/sibling.rs
        // For a file like submodule.rs, super:: refers to siblings in the same directory
        let parts: Vec<&str> = module_path.split("::").collect();

        if parts.is_empty() {
            return None;
        }

        let module_name = parts[0];
        let possible_paths = vec![
            format!("{}/{}.rs", current_dir.display(), module_name),
            format!("{}/{}/mod.rs", current_dir.display(), module_name),
        ];

        possible_paths.into_iter().next()
    } else if let Some(module_path) = import_path.strip_prefix("self::") {
        // self::child -> current_dir/child.rs
        let parts: Vec<&str> = module_path.split("::").collect();

        if parts.is_empty() {
            return None;
        }

        let module_name = parts[0];
        Some(format!("{}/{}.rs", current_dir.display(), module_name))
    } else {
        // External crate - return None
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_line_comments() {
        let code = "use std::io;\n// comment\nuse std::fs;";
        let cleaned = remove_comments(code);
        assert!(!cleaned.contains("// comment"));
    }

    #[test]
    fn test_remove_block_comments() {
        let code = "use std::io;\n/* block */\nuse std::fs;";
        let cleaned = remove_comments(code);
        assert!(!cleaned.contains("/* block */"));
    }

    #[test]
    fn test_parse_extern_crate() {
        let import = parse_extern_crate("extern crate serde;").unwrap();
        assert_eq!(import.path, "serde");
    }
}
