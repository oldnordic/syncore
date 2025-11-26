use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{anyhow, Result};
use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct IncludeGraph {
    graph: HashMap<String, HashSet<String>>,
    header_files: HashMap<String, PathBuf>,
    system_includes: Vec<PathBuf>,
}

impl IncludeGraph {
    pub fn new() -> Self {
        Self {
            graph: HashMap::new(),
            header_files: HashMap::new(),
            system_includes: Self::get_default_system_includes(),
        }
    }

    pub fn process_file(&mut self, file_path: &str) -> Result<Vec<Value>> {
        let file_path = Path::new(file_path);
        let file_path_str = file_path.to_string_lossy().to_string();

        // Read the file content
        let content = fs::read_to_string(file_path)
            .map_err(|e| anyhow!("Failed to read file {}: {}", file_path.display(), e))?;

        // Extract includes from the file
        let includes = Self::extract_includes(&content);

        // Process each include
        let mut edges = Vec::new();
        let mut included_files = Vec::new();

        for include in includes {
            let resolved_path = self.resolve_include_path(file_path.parent(), &include.path, include.kind == "system")?;
            if let Some(resolved) = resolved_path {
                let resolved_str = resolved.to_string_lossy().to_string();

                // Add edge in graph
                self.graph.entry(file_path_str.clone()).or_insert_with(HashSet::new).insert(resolved_str.clone());
                included_files.push(resolved_str);

                // Create edge entity
                edges.push(json!({
                    "type": "includes",
                    "source": file_path_str,
                    "target": include.path,
                    "include_type": include.kind,
                }));
            }
        }

        Ok(edges)
    }

    pub fn get_graph(&self) -> Value {
        let mut graph_value = json!({});

        for (source, targets) in &self.graph {
            let targets_array: Vec<String> = targets.iter().cloned().collect();
            graph_value[source] = json!(targets_array);
        }

        graph_value
    }

    fn extract_includes(content: &str) -> Vec<IncludeDirective> {
        let mut includes = Vec::new();
        let mut lines = content.lines();

        while let Some(line) = lines.next() {
            let trimmed = line.trim();
            if trimmed.starts_with("#include") {
                // Remove any leading/trailing whitespace
                let include_line = trimmed.strip_prefix("#include").unwrap().trim();

                // Determine include type and extract path
                let (kind, path) = if include_line.starts_with('<') && include_line.ends_with('>') {
                    ("system", &include_line[1..include_line.len()-1])
                } else if include_line.starts_with('"') && include_line.ends_with('"') {
                    ("local", &include_line[1..include_line.len()-1])
                } else {
                    continue; // Invalid include format
                };

                includes.push(IncludeDirective {
                    kind: kind.to_string(),
                    path: path.to_string(),
                });
            }
        }

        includes
    }

    fn resolve_include_path(
        &mut self,
        source_dir: Option<&Path>,
        include_path: &str,
        is_system: bool,
    ) -> Result<Option<PathBuf>> {
        if is_system {
            // Search in system include directories
            for system_dir in &self.system_includes {
                let candidate = system_dir.join(include_path);
                if candidate.exists() {
                    self.header_files.insert(include_path.to_string(), candidate.clone());
                    return Ok(Some(candidate));
                }
            }
            // For system headers that we can't find, just return the path as is
            Ok(None)
        } else {
            // For local includes, check relative to source file first
            if let Some(parent_dir) = source_dir {
                let candidate = parent_dir.join(include_path);
                if candidate.exists() {
                    self.header_files.insert(include_path.to_string(), candidate.clone());
                    return Ok(Some(candidate));
                }
            }

            // Try to resolve from workspace root if available
            // This would be set when initializing the plugin
            // For now, just return None if not found
            Ok(None)
        }
    }

    fn get_default_system_includes() -> Vec<PathBuf> {
        let mut includes = Vec::new();

        // Add common system include paths
        // These are typical paths for GCC/Clang on Linux, macOS, and Windows
        let common_paths = [
            "/usr/include",
            "/usr/local/include",
            "/usr/include/x86_64-linux-gnu",
            "/usr/include/c++/*",
            "/usr/include/x86_64-linux-gnu/c++/*",
            "/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/include",
            "C:/Program Files/LLVM/lib/clang/*/include",
            "C:/Program Files (x86)/Microsoft Visual Studio/*/VC/Tools/MSVC/*/include",
            "C:/MinGW/lib/gcc/mingw32/*/include",
        ];

        for path in &common_paths {
            // Handle wildcard paths
            if path.contains('*') {
                if let Some(parent) = Path::new(path).parent() {
                    if let Ok(entries) = fs::read_dir(parent) {
                        for entry in entries.flatten() {
                            let entry_path = entry.path();
                            if entry_path.is_dir() {
                                includes.push(entry_path);
                            }
                        }
                    }
                }
            } else if Path::new(path).exists() {
                includes.push(PathBuf::from(path));
            }
        }

        includes
    }

    #[allow(dead_code)]
    pub fn check_cycles(&self) -> Vec<Vec<String>> {
        let mut visited = HashSet::new();
        let mut recursion_stack = HashSet::new();
        let mut path = Vec::new();
        let mut cycles = Vec::new();

        for file in self.graph.keys() {
            if !visited.contains(file) {
                self.detect_cycles(
                    file,
                    &mut visited,
                    &mut recursion_stack,
                    &mut path,
                    &mut cycles,
                );
            }
        }

        cycles
    }

    fn detect_cycles(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        recursion_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        visited.insert(node.to_string());
        recursion_stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(neighbors) = self.graph.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    self.detect_cycles(neighbor, visited, recursion_stack, path, cycles);
                } else if recursion_stack.contains(neighbor) {
                    // Found a cycle
                    let cycle_start = path.iter().position(|n| n == neighbor).unwrap();
                    let cycle = path[cycle_start..].to_vec();
                    cycles.push(cycle);
                }
            }
        }

        path.pop();
        recursion_stack.remove(node);
    }
}

#[derive(Debug, Clone)]
struct IncludeDirective {
    kind: String, // "system" or "local"
    path: String,
}

impl Default for IncludeGraph {
    fn default() -> Self {
        Self::new()
    }
}
