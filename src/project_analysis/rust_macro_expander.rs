//! Static Macro Expansion Layer (SMEL)
//!
//! Provides deterministic, partial, static macro expansion for Rust code
//! using lightweight offline pattern matching, enabling PAE to detect
//! entities and edges inside macro-generated code without depending on rust-analyzer.

use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;

/// Macro expansion result
#[derive(Debug, Clone)]
pub struct MacroExpansion {
    pub macro_name: String,
    pub span_start: usize,
    pub span_end: usize,
    pub expanded_code: String,
}

/// Macro expansion context
#[derive(Debug)]
pub struct MacroExpansionContext {
    pub expansions: Vec<MacroExpansion>,
    pub expanded_source: String,
}

/// Static macro expander for Rust code
pub struct RustMacroExpander {
    // Pre-compiled regex patterns for common macros
    vec_pattern: Regex,
    format_pattern: Regex,
    log_pattern: Regex,
    assert_pattern: Regex,
    simple_macro_pattern: Regex,
}

/// Set of known/handled macros to skip in the catch-all pattern
const KNOWN_MACROS: &[&str] = &[
    "vec",
    "format",
    "info",
    "warn",
    "error",
    "debug",
    "trace",
    "assert",
    "println",
    "print",
    "eprintln",
    "eprint",
    "write",
    "writeln",
    "panic",
    "unreachable",
    "todo",
    "unimplemented",
    "cfg",
    "derive",
    "test",
    "bench",
    "doc",
    "allow",
    "deny",
    "warn",
];

impl RustMacroExpander {
    /// Create new macro expander with pre-compiled patterns
    pub fn new() -> Result<Self> {
        Ok(Self {
            // vec![a, b, c] pattern
            vec_pattern: Regex::new(r"vec!\s*\[\s*([^]]+)\s*\]")?,
            // format!("...", args) pattern - capture the format string in group 1
            format_pattern: Regex::new(r#"format!\s*\(\s*"([^"]*)"[^)]*\)"#)?,
            // log macros: info!, warn!, error!, debug!, trace!
            log_pattern: Regex::new(r"(info|warn|error|debug|trace)!\s*\([^)]*\)")?,
            // assert! macros
            assert_pattern: Regex::new(r"assert!\s*\([^)]*\)")?,
            // Simple declarative macros: my_macro!(x => y)
            // Known macros are filtered out in expand_simple_declarative_macros()
            simple_macro_pattern: Regex::new(r"([a-zA-Z_][a-zA-Z0-9_]*)!\s*\([^)]*\)")?,
        })
    }

    /// Expand simple macro invocations in source code
    pub fn expand_simple_macro_invocations(&self, source: &str) -> Result<MacroExpansionContext> {
        let mut expansions = Vec::new();
        let mut expanded_source = source.to_string();

        // Expand vec! macros
        self.expand_vec_macros(&mut expansions, &mut expanded_source)?;

        // Expand format! macros
        self.expand_format_macros(&mut expansions, &mut expanded_source)?;

        // Expand log macros
        self.expand_log_macros(&mut expansions, &mut expanded_source)?;

        // Expand assert! macros
        self.expand_assert_macros(&mut expansions, &mut expanded_source)?;

        // Expand simple declarative macros
        self.expand_simple_declarative_macros(&mut expansions, &mut expanded_source)?;

        Ok(MacroExpansionContext {
            expansions,
            expanded_source,
        })
    }

    /// Extract macro bodies for analysis
    pub fn extract_macro_bodies(
        &self,
        source: &str,
    ) -> Result<Vec<(String, String, usize, usize)>> {
        let mut macro_bodies = Vec::new();

        // Simple pattern to match macro definitions
        // macro_rules! my_macro { ... }
        let macro_def_pattern =
            Regex::new(r"macro_rules!\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\{([^}]*)\}")?;

        for captures in macro_def_pattern.captures_iter(source) {
            if let (Some(name), Some(body)) = (captures.get(1), captures.get(2)) {
                let start = name.start();
                let end = body.end();
                macro_bodies.push((
                    name.as_str().to_string(),
                    body.as_str().to_string(),
                    start,
                    end,
                ));
            }
        }

        Ok(macro_bodies)
    }

    /// Substitute macro patterns with expanded code
    pub fn substitute_macro_patterns(
        &self,
        source: &str,
        substitutions: &HashMap<String, String>,
    ) -> Result<String> {
        let mut result = source.to_string();

        for (pattern, replacement) in substitutions {
            // Simple string substitution for macro patterns
            result = result.replace(pattern, replacement);
        }

        Ok(result)
    }

    /// Sanitize expanded code for safe processing
    pub fn sanitize_expanded_code(&self, expanded: &str) -> Result<String> {
        let mut sanitized = expanded.to_string();

        // Remove potentially unsafe constructs
        sanitized = sanitized.replace("unsafe {", "/* unsafe */ {");

        // Limit recursion depth indicators
        let recursion_limit = 10;
        let mut depth: i32 = 0;
        for char in sanitized.chars() {
            if char == '{' {
                depth += 1;
                if depth > recursion_limit {
                    break;
                }
            } else if char == '}' {
                depth = depth.saturating_sub(1);
            }
        }

        // Truncate if too deep
        if depth > recursion_limit {
            let truncate_pos = sanitized.len().min(10000); // Reasonable limit
            sanitized.truncate(truncate_pos);
            sanitized.push_str("/* expansion truncated */");
        }

        Ok(sanitized)
    }

    /// Expand vec! macros to array literals
    fn expand_vec_macros(
        &self,
        expansions: &mut Vec<MacroExpansion>,
        source: &mut String,
    ) -> Result<()> {
        let mut offset: isize = 0;
        let mut result = source.clone();

        for captures in self.vec_pattern.captures_iter(source.as_str()) {
            if let Some(args) = captures.get(1) {
                let orig_start = captures.get(0).unwrap().start();
                let orig_end = captures.get(0).unwrap().end();
                let start = (orig_start as isize + offset) as usize;
                let end = (orig_end as isize + offset) as usize;

                // Simple expansion: vec![a, b, c] -> [a, b, c]
                let expanded = format!("[{}]", args.as_str());

                expansions.push(MacroExpansion {
                    macro_name: "vec!".to_string(),
                    span_start: start,
                    span_end: end,
                    expanded_code: expanded.clone(),
                });

                // Replace in result
                result.replace_range(start..end, &expanded);
                // Use signed arithmetic to handle both expansion and contraction
                offset += expanded.len() as isize - (orig_end - orig_start) as isize;
            }
        }

        *source = result;
        Ok(())
    }

    /// Expand format! macros to string concatenations
    fn expand_format_macros(
        &self,
        expansions: &mut Vec<MacroExpansion>,
        source: &mut String,
    ) -> Result<()> {
        let mut offset: isize = 0;
        let mut result = source.clone();

        for captures in self.format_pattern.captures_iter(source.as_str()) {
            if let Some(format_str) = captures.get(1) {
                let orig_start = captures.get(0).unwrap().start();
                let orig_end = captures.get(0).unwrap().end();
                let start = (orig_start as isize + offset) as usize;
                let end = (orig_end as isize + offset) as usize;

                // Simple expansion: format!("hello {}", x) -> "hello " + x
                // This is a simplified expansion for demonstration
                let expanded = format!("\"{}\" + /* format args */", format_str.as_str());

                expansions.push(MacroExpansion {
                    macro_name: "format!".to_string(),
                    span_start: start,
                    span_end: end,
                    expanded_code: expanded.clone(),
                });

                result.replace_range(start..end, &expanded);
                // Use signed arithmetic to handle both expansion and contraction
                offset += expanded.len() as isize - (orig_end - orig_start) as isize;
            }
        }

        *source = result;
        Ok(())
    }

    /// Expand log macros to function calls
    fn expand_log_macros(
        &self,
        expansions: &mut Vec<MacroExpansion>,
        source: &mut String,
    ) -> Result<()> {
        let mut offset: isize = 0;
        let mut result = source.clone();

        for captures in self.log_pattern.captures_iter(source.as_str()) {
            if let (Some(level), Some(args)) = (captures.get(1), captures.get(0)) {
                let orig_start = args.start();
                let orig_end = args.end();
                let start = (orig_start as isize + offset) as usize;
                let end = (orig_end as isize + offset) as usize;

                // Simple expansion: info!(...) -> println!("[INFO] ...")
                let expanded = format!(
                    "println!(\"[{}] {}\")",
                    level.as_str().to_uppercase(),
                    args.as_str()
                );

                expansions.push(MacroExpansion {
                    macro_name: format!("{}!", level.as_str()),
                    span_start: start,
                    span_end: end,
                    expanded_code: expanded.clone(),
                });

                result.replace_range(start..end, &expanded);
                // Use signed arithmetic to handle both expansion and contraction
                offset += expanded.len() as isize - (orig_end - orig_start) as isize;
            }
        }

        *source = result;
        Ok(())
    }

    /// Expand assert! macros to conditional checks
    fn expand_assert_macros(
        &self,
        expansions: &mut Vec<MacroExpansion>,
        source: &mut String,
    ) -> Result<()> {
        let mut offset: isize = 0;
        let mut result = source.clone();

        for captures in self.assert_pattern.captures_iter(source.as_str()) {
            if let Some(args) = captures.get(0) {
                let orig_start = args.start();
                let orig_end = args.end();
                let start = (orig_start as isize + offset) as usize;
                let end = (orig_end as isize + offset) as usize;

                // Simple expansion: assert!(condition) -> if !(condition) { panic!(...) }
                let expanded = format!(
                    "if !({}) {{ panic!(\"assertion failed: {{}}\", {}); }}",
                    args.as_str(),
                    args.as_str()
                );

                expansions.push(MacroExpansion {
                    macro_name: "assert!".to_string(),
                    span_start: start,
                    span_end: end,
                    expanded_code: expanded.clone(),
                });

                result.replace_range(start..end, &expanded);
                // Use signed arithmetic to handle both expansion and contraction
                offset += expanded.len() as isize - (orig_end - orig_start) as isize;
            }
        }

        *source = result;
        Ok(())
    }

    /// Expand simple declarative macros
    fn expand_simple_declarative_macros(
        &self,
        expansions: &mut Vec<MacroExpansion>,
        source: &mut String,
    ) -> Result<()> {
        let mut offset: isize = 0;
        let mut result = source.clone();

        // This is a simplified implementation for demonstration
        // Real implementation would need macro_rules! analysis
        for captures in self.simple_macro_pattern.captures_iter(source.as_str()) {
            if let (Some(macro_name), Some(args)) = (captures.get(1), captures.get(0)) {
                // Skip known macros that are handled by specific handlers
                if KNOWN_MACROS.contains(&macro_name.as_str()) {
                    continue;
                }

                let orig_start = args.start();
                let orig_end = args.end();
                let start = (orig_start as isize + offset) as usize;
                let end = (orig_end as isize + offset) as usize;

                // For simple macros like my_macro!(x => y), expand to y
                let expanded = if args.as_str().contains("=>") {
                    // Simple arrow macro: my_macro!(x => y) -> y
                    let parts: Vec<&str> = args.as_str().split("=>").collect();
                    if parts.len() == 2 {
                        parts[1].trim().to_string()
                    } else {
                        format!("/* expanded {} */", macro_name.as_str())
                    }
                } else {
                    // Unknown macro pattern, add comment
                    format!("/* expanded {} */", macro_name.as_str())
                };

                expansions.push(MacroExpansion {
                    macro_name: format!("{}!", macro_name.as_str()),
                    span_start: start,
                    span_end: end,
                    expanded_code: expanded.clone(),
                });

                result.replace_range(start..end, &expanded);
                // Use signed arithmetic to handle both expansion and contraction
                offset += expanded.len() as isize - (orig_end - orig_start) as isize;
            }
        }

        *source = result;
        Ok(())
    }
}

impl Default for RustMacroExpander {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            // Fallback with minimal patterns if regex compilation fails
            Self {
                vec_pattern: Regex::new(r"vec!\s*\[\s*").unwrap(),
                format_pattern: Regex::new(r"format!\s*\(").unwrap(),
                log_pattern: Regex::new(r"(info|warn|error)!\s*\(").unwrap(),
                assert_pattern: Regex::new(r"assert!\s*\(").unwrap(),
                // Known macros are filtered out in expand_simple_declarative_macros()
                simple_macro_pattern: Regex::new(r"([a-zA-Z_][a-zA-Z0-9_]*)!\s*\([^)]*\)").unwrap(),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_vec_macro() -> Result<()> {
        let expander = RustMacroExpander::new()?;
        let source = "let v = vec![1, 2, 3];";
        let context = expander.expand_simple_macro_invocations(source)?;

        assert_eq!(context.expansions.len(), 1);
        assert_eq!(context.expansions[0].macro_name, "vec!");
        assert!(context.expansions[0].expanded_code.contains("[1, 2, 3]"));
        assert!(context.expanded_source.contains("[1, 2, 3]"));

        Ok(())
    }

    #[test]
    fn test_expand_format_macro() -> Result<()> {
        let expander = RustMacroExpander::new()?;
        let source = "let s = format!(\"hello {}\", name);";
        let context = expander.expand_simple_macro_invocations(source)?;

        assert_eq!(context.expansions.len(), 1);
        assert_eq!(context.expansions[0].macro_name, "format!");
        assert!(context.expansions[0].expanded_code.contains("hello"));

        Ok(())
    }

    #[test]
    fn test_expand_log_macro() -> Result<()> {
        let expander = RustMacroExpander::new()?;
        let source = "info!(\"processing item: {}\", item);";
        let context = expander.expand_simple_macro_invocations(source)?;

        assert_eq!(context.expansions.len(), 1);
        assert_eq!(context.expansions[0].macro_name, "info!");
        assert!(context.expansions[0].expanded_code.contains("[INFO]"));

        Ok(())
    }

    #[test]
    fn test_expand_assert_macro() -> Result<()> {
        let expander = RustMacroExpander::new()?;
        let source = "assert!(x > 0);";
        let context = expander.expand_simple_macro_invocations(source)?;

        assert_eq!(context.expansions.len(), 1);
        assert_eq!(context.expansions[0].macro_name, "assert!");
        assert!(context.expansions[0].expanded_code.contains("if"));

        Ok(())
    }

    #[test]
    fn test_extract_macro_bodies() -> Result<()> {
        let expander = RustMacroExpander::new()?;
        let source = r#"
        macro_rules! my_macro {
            ($x:expr) => { $x * 2 }
        }
        "#;
        let bodies = expander.extract_macro_bodies(source)?;

        assert_eq!(bodies.len(), 1);
        assert_eq!(bodies[0].0, "my_macro");
        assert!(bodies[0].1.contains("$x:expr"));

        Ok(())
    }

    #[test]
    fn test_sanitize_expanded_code() -> Result<()> {
        let expander = RustMacroExpander::new()?;
        let unsafe_code = "let x = unsafe { 42 };";
        let sanitized = expander.sanitize_expanded_code(unsafe_code)?;

        assert!(sanitized.contains("/* unsafe */"));
        assert!(!sanitized.contains("unsafe {"));

        Ok(())
    }

    #[test]
    fn test_substitute_macro_patterns() -> Result<()> {
        let expander = RustMacroExpander::new()?;
        let source = "MY_MACRO!(placeholder)";
        let mut substitutions = HashMap::new();
        substitutions.insert(
            "MY_MACRO!(placeholder)".to_string(),
            "replaced_value".to_string(),
        );

        let result = expander.substitute_macro_patterns(source, &substitutions)?;

        assert_eq!(result, "replaced_value");

        Ok(())
    }

    #[test]
    fn test_multiple_macros() -> Result<()> {
        let expander = RustMacroExpander::new()?;
        let source = r#"
        let v = vec![1, 2, 3];
        info!("created vector: {:?}", v);
        assert!(v.len() > 0);
        "#;
        let context = expander.expand_simple_macro_invocations(source)?;

        assert_eq!(context.expansions.len(), 3);

        let macro_names: Vec<String> = context
            .expansions
            .iter()
            .map(|e| e.macro_name.clone())
            .collect();
        assert!(macro_names.contains(&"vec!".to_string()));
        assert!(macro_names.contains(&"info!".to_string()));
        assert!(macro_names.contains(&"assert!".to_string()));

        Ok(())
    }

    #[test]
    fn test_performance_large_input() -> Result<()> {
        let expander = RustMacroExpander::new()?;
        let mut source = String::new();

        // Create a large input (>5k LOC)
        for i in 0..6000 {
            source.push_str(&format!(
                "let v{} = vec![{}, {}, {}];\n",
                i,
                i * 3,
                i * 3 + 1,
                i * 3 + 2
            ));
        }

        let start = std::time::Instant::now();
        let _context = expander.expand_simple_macro_invocations(&source)?;
        let duration = start.elapsed();

        // Should complete in under 300ms
        assert!(
            duration.as_millis() < 300,
            "Macro expansion took too long: {:?}",
            duration
        );

        Ok(())
    }
}
