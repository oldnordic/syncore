// SPEC: SYNCORE-CODE-TOOLS-IMPROVEMENT-01 (APEX v1.2)
// STEP D: explain_function MCP tool implementation
//
// Provides structured function explanation with:
// - Full signature and docstring
// - Callers (functions that call this function)
// - Callees (functions this function calls)
// - Complexity metrics (lines, cyclomatic, cognitive)

use serde::{Deserialize, Serialize};

/// Request for explain_function MCP tool
#[derive(Debug, Clone, Deserialize)]
pub struct ExplainFunctionRequest {
    pub function_name: String,
    pub file_path: String,
}

/// Response from explain_function MCP tool
#[derive(Debug, Clone, Serialize)]
pub struct ExplainFunctionResponse {
    pub function_name: String,
    pub file_path: String,
    pub signature: String,
    pub docstring: Option<String>,
    pub line_start: usize,
    pub line_end: usize,
    pub callers: Vec<String>,
    pub callees: Vec<String>,
    pub complexity: ComplexityMetrics,
}

/// Complexity metrics for a function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    /// Number of lines in the function
    pub lines: usize,
    /// Cyclomatic complexity (number of decision points + 1)
    pub cyclomatic: usize,
    /// Cognitive complexity (accounts for nesting depth)
    pub cognitive: usize,
}

impl ComplexityMetrics {
    /// Calculate complexity metrics from source code
    ///
    /// Cyclomatic = 1 + number of decision points (if, else if, match arms, loops, ?)
    /// Cognitive = sum of (1 + nesting_depth) for each decision point
    pub fn from_code(code: &str) -> Self {
        let lines = code.lines().filter(|l| !l.trim().is_empty()).count();
        let (cyclomatic, cognitive) = compute_complexity(code);

        ComplexityMetrics {
            lines,
            cyclomatic,
            cognitive,
        }
    }
}

/// Compute cyclomatic and cognitive complexity from code
fn compute_complexity(code: &str) -> (usize, usize) {
    let mut cyclomatic = 1; // Base complexity
    let mut cognitive = 0;
    let mut nesting_depth = 0;

    // Track brace depth for nesting
    let mut in_string = false;
    let mut in_char = false;
    let mut prev_char = ' ';

    for line in code.lines() {
        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            continue;
        }

        // Count decision points
        let decision_patterns =
            ["if ", "else if ", "match ", "for ", "while ", "loop ", "&&", "||", "?"];

        for pattern in &decision_patterns {
            let count = count_pattern(trimmed, pattern);
            if count > 0 {
                cyclomatic += count;
                // Cognitive: each decision point adds (1 + nesting_depth)
                cognitive += count * (1 + nesting_depth);
            }
        }

        // Count match arms (=> in match blocks)
        if trimmed.contains("=>") && !trimmed.contains("->") {
            let arrow_count = trimmed.matches("=>").count();
            cyclomatic += arrow_count.saturating_sub(1); // First arm doesn't add complexity
        }

        // Update nesting depth based on braces
        for ch in trimmed.chars() {
            if ch == '"' && prev_char != '\\' && !in_char {
                in_string = !in_string;
            } else if ch == '\'' && prev_char != '\\' && !in_string {
                in_char = !in_char;
            } else if !in_string && !in_char {
                if ch == '{' {
                    nesting_depth += 1;
                } else if ch == '}' {
                    nesting_depth = nesting_depth.saturating_sub(1);
                }
            }
            prev_char = ch;
        }
    }

    (cyclomatic, cognitive.max(1))
}

/// Count non-overlapping occurrences of a pattern in a string
fn count_pattern(text: &str, pattern: &str) -> usize {
    if pattern.is_empty() {
        return 0;
    }

    let mut count = 0;
    let mut remaining = text;

    while let Some(idx) = remaining.find(pattern) {
        count += 1;
        remaining = &remaining[idx + pattern.len()..];
    }

    count
}

/// Function explainer that uses code graph for caller/callee analysis
pub struct FunctionExplainer {
    // In a full implementation, this would hold a reference to CodeGraph
    // For now, we implement a standalone version
}

impl FunctionExplainer {
    /// Create a new function explainer
    pub fn new() -> Self {
        Self {}
    }

    /// Explain a function by parsing the file and analyzing the code graph
    #[allow(clippy::similar_names)]
    pub fn explain(
        &self,
        function_name: &str,
        file_path: &str,
        code: &str,
        callers: Vec<String>,
        callees: Vec<String>,
    ) -> Option<ExplainFunctionResponse> {
        // Find the function in the code
        let (signature, docstring, line_start, line_end, body) =
            self.find_function(function_name, code)?;

        let complexity = ComplexityMetrics::from_code(&body);

        Some(ExplainFunctionResponse {
            function_name: function_name.to_string(),
            file_path: file_path.to_string(),
            signature,
            docstring,
            line_start,
            line_end,
            callers,
            callees,
            complexity,
        })
    }

    /// Find a function in source code and extract its details
    fn find_function(
        &self,
        name: &str,
        code: &str,
    ) -> Option<(String, Option<String>, usize, usize, String)> {
        let lines: Vec<&str> = code.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            // Look for function definition patterns
            let trimmed = line.trim();

            // Rust: fn name(
            // Python: def name(
            let is_fn_def = (trimmed.starts_with("fn ")
                || trimmed.starts_with("pub fn ")
                || trimmed.starts_with("async fn ")
                || trimmed.starts_with("pub async fn ")
                || trimmed.starts_with("def "))
                && trimmed.contains(&format!("{}(", name));

            if is_fn_def {
                let line_start = i + 1;

                // Extract signature (everything up to opening brace or colon)
                let signature = if let Some(brace_idx) = trimmed.find('{') {
                    trimmed[..brace_idx].trim().to_string()
                } else {
                    trimmed.trim_end_matches(':').to_string()
                };

                // Look for docstring above the function
                let docstring = self.extract_docstring(&lines, i);

                // Find function end (matching braces)
                let (line_end, body) = self.find_function_body(&lines, i);

                return Some((signature, docstring, line_start, line_end, body));
            }
        }

        None
    }

    /// Extract docstring from lines above a function
    fn extract_docstring(&self, lines: &[&str], fn_line: usize) -> Option<String> {
        if fn_line == 0 {
            return None;
        }

        let mut doc_lines = Vec::new();
        let mut i = fn_line - 1;

        // Look for /// or #[doc] comments
        loop {
            let trimmed = lines[i].trim();

            if trimmed.starts_with("///") {
                doc_lines.insert(0, trimmed.trim_start_matches("///").trim());
            } else if trimmed.starts_with("#[doc") {
                // Skip attribute-style docs
            } else if trimmed.is_empty() {
                // Allow empty lines in doc
            } else {
                break;
            }

            if i == 0 {
                break;
            }
            i -= 1;
        }

        if doc_lines.is_empty() {
            None
        } else {
            Some(doc_lines.join("\n"))
        }
    }

    /// Find the body of a function (from opening brace to matching close)
    fn find_function_body(&self, lines: &[&str], start_line: usize) -> (usize, String) {
        let mut brace_depth = 0;
        let mut body_lines = Vec::new();
        let mut found_open = false;

        for (i, line) in lines.iter().enumerate().skip(start_line) {
            for ch in line.chars() {
                if ch == '{' {
                    brace_depth += 1;
                    found_open = true;
                } else if ch == '}' {
                    brace_depth -= 1;
                }
            }

            body_lines.push(*line);

            if found_open && brace_depth == 0 {
                return (i + 1, body_lines.join("\n"));
            }
        }

        // If we couldn't find matching braces, return what we have
        (lines.len(), body_lines.join("\n"))
    }
}

impl Default for FunctionExplainer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complexity_simple() {
        let code = "fn foo() { println!(\"hi\"); }";
        let metrics = ComplexityMetrics::from_code(code);
        assert_eq!(metrics.cyclomatic, 1);
    }

    #[test]
    fn test_complexity_with_if() {
        let code = "fn foo(x: i32) { if x > 0 { } }";
        let metrics = ComplexityMetrics::from_code(code);
        assert!(metrics.cyclomatic >= 2);
    }

    #[test]
    fn test_find_function() {
        let explainer = FunctionExplainer::new();
        let code = r#"
/// Does something
fn my_func(x: i32) {
    println!("{}", x);
}
"#;
        let result = explainer.find_function("my_func", code);
        assert!(result.is_some());
        let (sig, doc, start, end, _body) = result.unwrap();
        assert!(sig.contains("my_func"));
        assert!(doc.is_some());
        assert!(start > 0);
        assert!(end >= start);
    }

    #[test]
    fn test_count_pattern() {
        assert_eq!(count_pattern("if x && y", "&&"), 1);
        assert_eq!(count_pattern("a && b && c", "&&"), 2);
        assert_eq!(count_pattern("hello", "x"), 0);
    }
}
