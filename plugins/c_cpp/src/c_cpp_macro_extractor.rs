use std::collections::HashMap;
use anyhow::Result;
use serde_json::{json, Value};

pub struct MacroExtractor {
    defined_macros: HashMap<String, MacroDefinition>,
    conditional_macros: Vec<ConditionalMacro>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct MacroDefinition {
    name: String,
    parameters: Vec<String>,
    replacement: String,
    is_function_like: bool,
    line: u32,
    column: u32,
    file_path: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct ConditionalMacro {
    condition: String,
    file_path: String,
    line: u32,
    column: u32,
}

impl MacroExtractor {
    pub fn new() -> Self {
        Self {
            defined_macros: HashMap::new(),
            conditional_macros: Vec::new(),
        }
    }

    pub fn extract_from_file(&mut self, file_path: &str) -> Result<(Vec<Value>, Vec<Value>)> {
        let content = std::fs::read_to_string(file_path)?;
        let mut entities = Vec::new();
        let mut edges = Vec::new();

        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();
            let line_num = (i + 1) as u32;

            if trimmed.starts_with("#define") {
                self.process_define(line, line_num, file_path, &mut entities, &mut edges)?;
            } else if trimmed.starts_with("#undef") {
                self.process_undef(line, line_num, file_path, &mut entities, &mut edges)?;
            } else if trimmed.starts_with("#ifdef") {
                self.process_conditional(&lines, &mut i, "ifdef", file_path, &mut entities, &mut edges)?;
                continue;
            } else if trimmed.starts_with("#ifndef") {
                self.process_conditional(&lines, &mut i, "ifndef", file_path, &mut entities, &mut edges)?;
                continue;
            } else if trimmed.starts_with("#if") {
                self.process_conditional(&lines, &mut i, "if", file_path, &mut entities, &mut edges)?;
                continue;
            } else if trimmed.starts_with("#endif") {
                // This will be handled by process_conditional
            } else if trimmed.starts_with("#elif") {
                // This will be handled by process_conditional
            } else if trimmed.starts_with("#else") {
                // This will be handled by process_conditional
            }

            // Check for macro usage in the line
            self.check_macro_usage(line, line_num, file_path, &mut edges);

            i += 1;
        }

        Ok((entities, edges))
    }

    fn process_define(
        &mut self,
        line: &str,
        line_num: u32,
        file_path: &str,
        entities: &mut Vec<Value>,
        edges: &mut Vec<Value>,
    ) -> Result<()> {
        // Parse the define directive
        // Format: #define NAME replacement
        // or: #define NAME(param1, param2) replacement

        let after_define = line.trim_start_matches("#define").trim();
        if after_define.is_empty() {
            return Ok(());
        }

        // Check if it's a function-like macro
        let (_name, _remainder) = if let Some(paren_idx) = after_define.find('(') {
            // Function-like macro
            let name = after_define[..paren_idx].trim();
            let (after_paren, is_function_like) = if let Some(closing_paren) = after_define.find(')') {
                // Extract parameters
                let params_str = &after_define[paren_idx+1..closing_paren];
                let _parameters: Vec<String> = params_str
                    .split(',')
                    .map(|p| p.trim().to_string())
                    .collect();

                // Extract replacement (after the closing parenthesis)
                let replacement = after_define[closing_paren+1..].trim();
                (replacement, true)
            } else {
                // Malformed function-like macro, treat as object-like
                (after_define[paren_idx+1..].trim(), false)
            };

            // Store the macro definition
            self.defined_macros.insert(
                name.to_string(),
                MacroDefinition {
                    name: name.to_string(),
                    parameters: if is_function_like {
                        // Parse parameters correctly
                        if let Some(paren_idx) = after_define.find('(') {
                            if let Some(closing_paren) = after_define.find(')') {
                                let params_str = &after_define[paren_idx+1..closing_paren];
                                params_str
                                    .split(',')
                                    .map(|p| p.trim().to_string())
                                    .collect()
                            } else {
                                Vec::new()
                            }
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    },
                    replacement: after_paren.to_string(),
                    is_function_like,
                    line: line_num,
                    column: (line.find("#define").unwrap() + 1) as u32,
                    file_path: file_path.to_string(),
                }
            );

            // Create entity
            let _entity_id = entities.len();
            entities.push(json!({
                "type": "macro",
                "name": name,
                "parameters": if is_function_like {
                    // Parse parameters correctly
                    if let Some(paren_idx) = after_define.find('(') {
                        if let Some(closing_paren) = after_define.find(')') {
                            let params_str = &after_define[paren_idx+1..closing_paren];
                            params_str
                                .split(',')
                                .map(|p| p.trim().to_string())
                                .collect()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                },
                "replacement": after_paren,
                "is_function_like": is_function_like,
                "line": line_num,
                "column": (line.find("#define").unwrap() + 1) as u32,
                "end_line": line_num,
                "end_column": line.len() as u32,
            }));

            // Add edge
            edges.push(json!({
                "type": "defines_macro",
                "source": name,
            }));

            (name, after_paren)
        } else {
            // Object-like macro
            let parts: Vec<&str> = after_define.splitn(2, ' ').collect();
            let name = parts[0];
            let replacement = if parts.len() > 1 { parts[1] } else { "" };

            // Store the macro definition
            self.defined_macros.insert(
                name.to_string(),
                MacroDefinition {
                    name: name.to_string(),
                    parameters: Vec::new(),
                    replacement: replacement.to_string(),
                    is_function_like: false,
                    line: line_num,
                    column: (line.find("#define").unwrap() + 1) as u32,
                    file_path: file_path.to_string(),
                }
            );

            // Create entity
            let _entity_id = entities.len();
            entities.push(json!({
                "type": "macro",
                "name": name,
                "replacement": replacement,
                "is_function_like": false,
                "line": line_num,
                "column": (line.find("#define").unwrap() + 1) as u32,
                "end_line": line_num,
                "end_column": line.len() as u32,
            }));

            // Add edge
            edges.push(json!({
                "type": "defines_macro",
                "source": name,
            }));

            (name, replacement)
        };

        Ok(())
    }

    fn process_undef(
        &mut self,
        line: &str,
        _line_num: u32,
        _file_path: &str,
        _entities: &mut Vec<Value>,
        _edges: &mut Vec<Value>,
    ) -> Result<()> {
        // Parse the undef directive
        // Format: #undef NAME

        let after_undef = line.trim_start_matches("#undef").trim();
        if after_undef.is_empty() {
            return Ok(());
        }

        let name = after_undef.split_whitespace().next().unwrap_or("");
        if !name.is_empty() {
            self.defined_macros.remove(name);
        }

        Ok(())
    }

    fn process_conditional(
        &mut self,
        lines: &[&str],
        i: &mut usize,
        condition_type: &str,
        file_path: &str,
        entities: &mut Vec<Value>,
        edges: &mut Vec<Value>,
    ) -> Result<()> {
        let line_num = (*i + 1) as u32;
        let line = lines[*i];

        // Extract the condition
        let after_cond = line.trim_start_matches(&format!("#{}", condition_type)).trim();
        let condition = if condition_type == "ifdef" || condition_type == "ifndef" {
            after_cond.split_whitespace().next().unwrap_or("")
        } else {
            after_cond
        };

        // Store the conditional macro
        self.conditional_macros.push(ConditionalMacro {
            condition: condition.to_string(),
            file_path: file_path.to_string(),
            line: line_num,
            column: (line.find(&format!("#{}", condition_type)).unwrap() + 1) as u32,
        });

        // Process the block until #endif
        let mut nested_level = 1;
        *i += 1;

        while *i < lines.len() && nested_level > 0 {
            let current_line = lines[*i].trim();

            if current_line.starts_with("#if") || current_line.starts_with("#ifdef") || current_line.starts_with("#ifndef") {
                nested_level += 1;
            } else if current_line.starts_with("#endif") {
                nested_level -= 1;
            }

            if nested_level > 0 {
                // Process the line within the conditional block
                if current_line.starts_with("#define") {
                    self.process_define(lines[*i], (*i + 1) as u32, file_path, entities, edges)?;
                } else if current_line.starts_with("#undef") {
                    self.process_undef(lines[*i], (*i + 1) as u32, file_path, entities, edges)?;
                }

                // Check for macro usage
                self.check_macro_usage(lines[*i], (*i + 1) as u32, file_path, edges);
            }

            *i += 1;
        }

        Ok(())
    }

    fn check_macro_usage(
        &self,
        line: &str,
        line_num: u32,
        file_path: &str,
        edges: &mut Vec<Value>,
    ) {
        // This is a simplified implementation
        // In practice, you'd need to handle more complex cases of macro expansion

        for macro_name in self.defined_macros.keys() {
            if line.contains(macro_name) {
                // Check if this looks like a macro usage
                // This is a simplified check and may have false positives
                let parts: Vec<&str> = line.split_whitespace().collect();
                for (_idx, part) in parts.iter().enumerate() {
                    if part.starts_with(macro_name) {
                        let macro_def = self.defined_macros.get(macro_name).unwrap();

                        if macro_def.is_function_like && part.contains('(') {
                            // Function-like macro usage
                            edges.push(json!({
                                "type": "uses_macro",
                                "source": file_path,
                                "target": macro_name,
                                "line": line_num,
                                "column": self.find_column(line, macro_name) as u32,
                                "macro_type": "function",
                            }));
                        } else if !macro_def.is_function_like && part == macro_name {
                            // Object-like macro usage
                            edges.push(json!({
                                "type": "uses_macro",
                                "source": file_path,
                                "target": macro_name,
                                "line": line_num,
                                "column": self.find_column(line, macro_name) as u32,
                                "macro_type": "object",
                            }));
                        }
                    }
                }
            }
        }
    }

    fn find_column(&self, line: &str, substr: &str) -> usize {
        line.find(substr).unwrap_or(0) + 1 // Convert to 1-based indexing
    }
}

impl Default for MacroExtractor {
    fn default() -> Self {
        Self::new()
    }
}
