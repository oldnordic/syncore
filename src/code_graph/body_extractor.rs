//! Function body extraction for semantic search
//!
//! Extracts and truncates function bodies for indexing.
//! APEX v1.7 Phase 3: Body-aware code search

use anyhow::Result;
use std::fs;
use std::path::Path;

/// Maximum tokens to include in body snippet (configurable)
const MAX_BODY_TOKENS: usize = 200;

/// Extract function body snippet from source file
///
/// Given a file path and line range, extracts the function body text
/// and truncates it to MAX_BODY_TOKENS tokens for efficient indexing.
pub fn extract_body_snippet(
    file_path: &Path,
    line_start: usize,
    line_end: usize,
) -> Result<Option<String>> {
    // Read file content
    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(_) => return Ok(None),
    };

    // Extract lines
    let lines: Vec<&str> = content.lines().collect();

    // Validate line range (1-indexed in CodeEntity)
    if line_start == 0 || line_start > lines.len() || line_end > lines.len() {
        return Ok(None);
    }

    // Extract body lines (convert to 0-indexed)
    let start_idx = line_start.saturating_sub(1);
    let end_idx = line_end.min(lines.len());

    if start_idx >= end_idx {
        return Ok(None);
    }

    let body_lines = &lines[start_idx..end_idx];
    let body_text = body_lines.join("\n");

    // Truncate to max tokens
    let snippet = truncate_to_tokens(&body_text, MAX_BODY_TOKENS);

    if snippet.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(snippet))
    }
}

/// Truncate text to maximum number of tokens (whitespace-separated)
fn truncate_to_tokens(text: &str, max_tokens: usize) -> String {
    let tokens: Vec<&str> = text.split_whitespace().collect();

    if tokens.len() <= max_tokens {
        text.to_string()
    } else {
        // Take first max_tokens and rejoin
        tokens[..max_tokens].join(" ")
    }
}

/// Extract body for function-like entities only
///
/// Returns None for imports, classes (without implementation), etc.
pub fn should_extract_body(entity_type: &str) -> bool {
    matches!(entity_type, "function" | "method" | "impl")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_extract_body_snippet() -> Result<()> {
        // Create temp file with function
        let mut temp_file = NamedTempFile::new()?;
        writeln!(
            temp_file,
            "fn example() {{\n    let x = 42;\n    println!(\"test\");\n    x * 2\n}}"
        )?;
        temp_file.flush()?;

        // Extract body (lines 1-5)
        let snippet = extract_body_snippet(temp_file.path(), 1, 5)?;

        assert!(snippet.is_some());
        let body = snippet.unwrap();
        assert!(body.contains("let x = 42"));
        assert!(body.contains("println"));

        Ok(())
    }

    #[test]
    fn test_truncate_to_tokens() {
        let long_text = "word ".repeat(300);
        let truncated = truncate_to_tokens(&long_text, 200);

        let token_count = truncated.split_whitespace().count();
        assert_eq!(token_count, 200);
    }

    #[test]
    fn test_should_extract_body() {
        assert!(should_extract_body("function"));
        assert!(should_extract_body("method"));
        assert!(should_extract_body("impl"));

        assert!(!should_extract_body("import"));
        assert!(!should_extract_body("class"));
        assert!(!should_extract_body("variable"));
    }

    #[test]
    fn test_extract_invalid_range() -> Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, "fn test() {{}}")?;
        temp_file.flush()?;

        // Invalid line range
        let snippet = extract_body_snippet(temp_file.path(), 10, 20)?;
        assert!(snippet.is_none());

        Ok(())
    }
}
