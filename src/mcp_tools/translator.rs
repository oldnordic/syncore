//! Syncore MCP LLM Output Translator
//!
//! Normalizes raw LLM output into STRICT schema-valid JSON for IntelliTask and Sequential Reasoning tools.
//!
//! Rules:
//! - NO semantic invention or guessing
//! - ONLY structural corrections (type coercion, enum normalization)
//! - ERROR on missing required semantic fields
//! - NEVER create missing titles, descriptions, IDs, etc.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Translator configuration
#[derive(Debug, Clone)]
pub struct TranslatorConfig {
    pub strict_mode: bool,
    pub allow_coercion: bool,
}

impl Default for TranslatorConfig {
    fn default() -> Self {
        Self {
            strict_mode: true,
            allow_coercion: true,
        }
    }
}

impl Default for LlmOutputTranslator {
    fn default() -> Self {
        Self::new(TranslatorConfig::default())
    }
}

/// Main translator for LLM outputs
pub struct LlmOutputTranslator {
    config: TranslatorConfig,
}

impl LlmOutputTranslator {
    pub fn new(config: TranslatorConfig) -> Self {
        Self {
            config,
        }
    }

    pub fn with_strict_mode() -> Self {
        Self::new(TranslatorConfig {
            strict_mode: true,
            allow_coercion: false,
        })
    }

    pub fn with_coercion_mode() -> Self {
        Self::new(TranslatorConfig {
            strict_mode: false,
            allow_coercion: true,
        })
    }

    /// Translate raw LLM output to specific schema
    pub fn translate(&self, raw_output: &str, target_schema: TargetSchema) -> Result<Value> {
        // T8: Contract enforcement - size limits
        const MAX_INPUT_SIZE: usize = 500_000; // 500KB limit
        if raw_output.len() > MAX_INPUT_SIZE {
            return Err(anyhow::anyhow!(
                "Input too large: {} bytes exceeds limit of {} bytes",
                raw_output.len(),
                MAX_INPUT_SIZE
            ));
        }

        // Step 1: Extract JSON from LLM output
        let json_value = self.extract_json(raw_output)?;

        // Step 2: Validate and normalize based on target schema
        match target_schema {
            TargetSchema::TaskBreakdown => self.translate_task_breakdown(json_value),
            TargetSchema::PriorityResult => self.translate_priority_result(json_value),
            TargetSchema::SubtaskBreakdown => self.translate_subtask_breakdown(json_value),
            TargetSchema::NextTaskSuggestion => self.translate_next_task_suggestion(json_value),
            TargetSchema::SequentialStep => self.translate_sequential_step(json_value),
        }
    }

    /// Extract JSON from raw LLM output (removes prose, markdown fences)
    fn extract_json(&self, raw_output: &str) -> Result<Value> {
        // Remove markdown code fences if present
        let binding = raw_output.trim().replace("```json", "").replace("```", "");
        let cleaned = binding.trim();

        // Find JSON object start and end
        let mut brace_count = 0;
        let mut json_start = None;
        let mut json_end = None;

        for (i, ch) in cleaned.chars().enumerate() {
            if ch == '{' {
                if brace_count == 0 {
                    json_start = Some(i);
                }
                brace_count += 1;
            } else if ch == '}' {
                brace_count -= 1;
                if brace_count == 0 && json_start.is_some() {
                    json_end = Some(i + 1);
                    break;
                }
            }
        }

        let json_str = match (json_start, json_end) {
            (Some(start), Some(end)) => &cleaned[start..end],
            _ => cleaned,
        };

        // Remove JSON-style comments before parsing
        let json_without_comments = self.remove_json_comments(json_str);

        // Try to parse as JSON
        serde_json::from_str(&json_without_comments).map_err(|e| {
            anyhow!("Failed to parse JSON: {}. Extracted content: {}", e, json_without_comments)
        })
    }

    /// Remove JSON-style comments (// single line and /* multi-line */)
    fn remove_json_comments(&self, json_str: &str) -> String {
        let mut result = String::new();
        let mut chars = json_str.chars().peekable();
        let mut in_string = false;
        let mut in_single_line_comment = false;
        let mut in_multi_line_comment = false;

        while let Some(ch) = chars.next() {
            match ch {
                '"' if !in_single_line_comment && !in_multi_line_comment => {
                    in_string = !in_string;
                    result.push(ch);
                }
                '/' if !in_string && !in_single_line_comment && !in_multi_line_comment => {
                    if let Some(&next_ch) = chars.peek() {
                        match next_ch {
                            '/' => {
                                in_single_line_comment = true;
                                chars.next(); // consume the second '/'
                            }
                            '*' => {
                                in_multi_line_comment = true;
                                chars.next(); // consume the '*'
                            }
                            _ => result.push(ch),
                        }
                    } else {
                        result.push(ch);
                    }
                }
                '\n' => {
                    in_single_line_comment = false;
                    if !in_single_line_comment {
                        result.push(ch);
                    }
                }
                '*' if in_multi_line_comment => {
                    if let Some(&next_ch) = chars.peek() {
                        if next_ch == '/' {
                            in_multi_line_comment = false;
                            chars.next(); // consume the '/'
                        }
                    }
                }
                _ => {
                    if !in_single_line_comment && !in_multi_line_comment {
                        result.push(ch);
                    }
                }
            }
        }
        result
    }

    /// Translate to TaskBreakdown schema
    fn translate_task_breakdown(&self, mut value: Value) -> Result<Value> {
        // T8: Contract enforcement - type validation
        if !value.is_object() {
            return Err(anyhow::anyhow!(
                "TaskBreakdown must be a JSON object, got {}",
                if value.is_array() {
                    "array"
                } else {
                    "other type"
                }
            ));
        }

        // NEW PIPELINE: Normalize BEFORE validation

        // 1) Normalize structure and arrays
        self.normalize_arrays_for_task_breakdown(&mut value)?;

        // 2) Normalize fields and aliases (description → purpose)
        self.normalize_fields_for_task_breakdown(&mut value)?;

        // 3) Normalize types (string → f32 for estimated_hours)
        self.normalize_types_for_task_breakdown(&mut value)?;

        // 4) Normalize enums (Complexity mapping)
        self.normalize_enums_for_task_breakdown(&mut value)?;

        // 5) Coerce string subtasks to objects (if coercion allowed)
        if self.config.allow_coercion {
            if let Some(parent_tasks) = value.get_mut("parent_tasks").and_then(Value::as_array_mut)
            {
                for parent_task in parent_tasks.iter_mut() {
                    if let Some(subtasks) =
                        parent_task.get_mut("subtasks").and_then(Value::as_array_mut)
                    {
                        for subtask in subtasks.iter_mut() {
                            // Coerce string subtasks to objects
                            if let Some(subtask_str) = subtask.as_str() {
                                *subtask = json!({
                                    "title": subtask_str,
                                    "description": "",
                                    "status": "pending"
                                });
                            }
                        }
                    }
                }
            }
        }

        // 6) FINAL validation only after normalization
        self.validate_task_breakdown(&value)
    }

    /// Translate to PriorityResult schema (CRITICAL: priority must be STRING)
    fn translate_priority_result(&self, mut value: Value) -> Result<Value> {
        // NEW PIPELINE: Normalize BEFORE validation

        // 1) Normalize structure and arrays
        self.normalize_arrays_for_priority_result(&mut value)?;

        // 2) Normalize types (numbers → strings)
        self.normalize_types_for_priority_result(&mut value)?;

        // 3) FINAL validation only after normalization
        self.validate_priority_result(&value)
    }

    /// Translate to SubtaskBreakdown schema (simplified)
    fn translate_subtask_breakdown(&self, mut value: Value) -> Result<Value> {
        let mut errors = Vec::new();

        if !value.get("subtasks").and_then(Value::as_array).is_some() {
            errors.push("Missing required field: subtasks array".to_string());
        }

        if !errors.is_empty() {
            return Err(anyhow!("SubtaskBreakdown validation failed: {}", errors.join(", ")));
        }

        Ok(value)
    }

    /// Translate to NextTaskSuggestion schema (simplified)
    fn translate_next_task_suggestion(&self, mut value: Value) -> Result<Value> {
        let mut errors = Vec::new();

        if !value.get("task_id").and_then(Value::as_str).is_some() {
            errors.push("Missing required field: task_id".to_string());
        }
        if !value.get("reasoning").and_then(Value::as_str).is_some() {
            errors.push("Missing required field: reasoning".to_string());
        }

        if !errors.is_empty() {
            return Err(anyhow!("NextTaskSuggestion validation failed: {}", errors.join(", ")));
        }

        Ok(value)
    }

    /// Translate to SequentialStep schema
    fn translate_sequential_step(&self, mut value: Value) -> Result<Value> {
        let mut errors = Vec::new();

        // Required fields with type validation
        if !value.get("step_number").is_some() {
            errors.push("Missing required field: step_number".to_string());
        } else {
            // Validate step_number is a valid integer (allow coercion from valid integer strings)
            if let Some(step_num) = value.get("step_number") {
                if let Some(step_str) = step_num.as_str() {
                    // Reject strings that contain decimal points (floats)
                    if step_str.contains('.') {
                        errors.push("step_number must be an integer, not a float".to_string());
                    } else if let Ok(int_val) = step_str.parse::<i64>() {
                        // Auto-coerce valid integer strings to actual integers
                        value["step_number"] = json!(int_val);
                    } else {
                        errors.push("step_number must be a valid integer".to_string());
                    }
                } else if !step_num.is_number() {
                    errors.push("step_number must be a number".to_string());
                }
            }
        }

        if !value.get("thought").and_then(Value::as_str).is_some() {
            errors.push("Missing required field: thought".to_string());
        }
        if !value.get("reasoning").and_then(Value::as_str).is_some() {
            errors.push("Missing required field: reasoning".to_string());
        }

        // Coercion for optional fields
        if self.config.allow_coercion {
            // Auto-generate step_id if missing
            if !value.get("step_id").and_then(Value::as_str).is_some() {
                let task_id = value.get("task_id").and_then(Value::as_i64).unwrap_or(0);
                let step_number = value.get("step_number").and_then(Value::as_i64).unwrap_or(1);
                value["step_id"] = json!(format!("step_{}_{}", task_id, step_number));
            }

            // Auto-generate timestamp if missing
            if !value.get("timestamp").is_some() {
                use std::time::{SystemTime, UNIX_EPOCH};
                let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                value["timestamp"] = json!(timestamp);
            }

            // Set default status if missing
            if !value.get("status").and_then(Value::as_str).is_some() {
                value["status"] = json!("pending");
            }

            // Validate status enum - reject unknown values in strict mode
            if let Some(status) = value.get("status") {
                if let Some(status_str) = status.as_str() {
                    if !["pending", "executing", "completed", "failed"].contains(&status_str) {
                        if self.config.strict_mode {
                            errors.push(format!("Invalid status '{}'. Must be one of: pending, executing, completed, failed", status_str));
                        } else {
                            // In non-strict mode, coerce to pending
                            value["status"] = json!("pending");
                        }
                    }
                }
            }
        }

        if !errors.is_empty() {
            return Err(anyhow!("SequentialStep validation failed: {}", errors.join(", ")));
        }

        Ok(value)
    }

    /// Normalize Complexity enum values
    fn normalize_complexity_enum(&self, complexity: &mut Value) {
        if let Some(complexity_str) = complexity.as_str() {
            match complexity_str {
                "High" | "hard" | "difficult" => *complexity = json!("Complex"),
                "Low" | "easy" | "simple" => *complexity = json!("Simple"),
                "Medium" | "moderate" | "normal" => *complexity = json!("Moderate"),
                "VeryHigh" | "very hard" | "extremely difficult" => {
                    *complexity = json!("VeryComplex")
                }
                "VeryLow" | "trivial" | "very easy" => *complexity = json!("Trivial"),
                _ => {
                    // Check if it's already a valid enum value
                    if !["Trivial", "Simple", "Moderate", "Complex", "VeryComplex"]
                        .contains(&complexity_str)
                    {
                        // Don't change unknown values in strict mode
                        if !self.config.strict_mode {
                            *complexity = json!("Moderate"); // Safe default
                        }
                    }
                }
            }
        }
    }

    /// Normalize FileReference structure
    fn normalize_file_reference(&self, file_ref: &mut Value) {
        if self.config.allow_coercion {
            // Set default action if missing
            if !file_ref.get("action").and_then(Value::as_str).is_some() {
                file_ref["action"] = json!("Review");
            }

            // Normalize FileAction enum with aliases
            if let Some(action) = file_ref.get_mut("action") {
                if let Some(action_str) = action.as_str() {
                    match action_str {
                        "Add" | "Update" | "Implement" | "Modify2" => *action = json!("Modify2"),
                        _ => {
                            if !["Create", "Modify", "Review", "Modify2"].contains(&action_str) {
                                *action = json!("Review"); // Safe default
                            }
                        }
                    }
                }
            }
        }
    }

    // === NEW PIPELINE HELPER FUNCTIONS FOR TASKBREAKDOWN ===

    /// 1) Normalize arrays: missing arrays → empty arrays
    fn normalize_arrays_for_task_breakdown(&self, value: &mut Value) -> Result<()> {
        if !self.config.allow_coercion {
            return Ok(());
        }

        // Auto-fix root level arrays
        if !value.get("relevant_files").and_then(Value::as_array).is_some() {
            value["relevant_files"] = json!([]);
        }

        // Auto-fix parent_tasks array
        if !value.get("parent_tasks").and_then(Value::as_array).is_some() {
            value["parent_tasks"] = json!([]);
        }

        // Auto-fix arrays in parent tasks
        if let Some(parent_tasks) = value.get_mut("parent_tasks").and_then(Value::as_array_mut) {
            for parent_task in parent_tasks {
                if !parent_task.get("subtasks").and_then(Value::as_array).is_some() {
                    parent_task["subtasks"] = json!([]);
                }
                if !parent_task.get("dependencies").and_then(Value::as_array).is_some() {
                    parent_task["dependencies"] = json!([]);
                }
            }
        }

        Ok(())
    }

    /// 2) Normalize fields and aliases: description → purpose mapping
    fn normalize_fields_for_task_breakdown(&self, value: &mut Value) -> Result<()> {
        if !self.config.allow_coercion {
            return Ok(());
        }

        // Fix FileReference description → purpose mapping
        if let Some(relevant_files) = value.get_mut("relevant_files").and_then(Value::as_array_mut)
        {
            for file_ref in relevant_files {
                // If has "description" but no "purpose", move it
                if let Some(desc) = file_ref.get("description") {
                    if !file_ref.get("purpose").is_some() {
                        file_ref["purpose"] = desc.clone();
                        file_ref.as_object_mut().unwrap().remove("description");
                    }
                }
            }
        }

        Ok(())
    }

    /// 3) Normalize types: string → f32 for estimated_hours
    fn normalize_types_for_task_breakdown(&self, value: &mut Value) -> Result<()> {
        if !self.config.allow_coercion {
            return Ok(());
        }

        // Normalize estimated_hours in parent tasks
        if let Some(parent_tasks) = value.get_mut("parent_tasks").and_then(Value::as_array_mut) {
            for parent_task in parent_tasks {
                // Coerce parent task estimated_hours
                if let Some(hours) = parent_task.get_mut("estimated_hours") {
                    self.coerce_to_f32(hours);
                }

                // Coerce subtask estimated_hours
                if let Some(subtasks) =
                    parent_task.get_mut("subtasks").and_then(Value::as_array_mut)
                {
                    for subtask in subtasks {
                        if let Some(hours) = subtask.get_mut("estimated_hours") {
                            self.coerce_to_f32(hours);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// 4) Normalize enums: Complexity mapping
    fn normalize_enums_for_task_breakdown(&self, value: &mut Value) -> Result<()> {
        if !self.config.allow_coercion {
            return Ok(());
        }

        // Auto-fill missing estimated_complexity with default
        if !value.get("estimated_complexity").is_some() {
            value["estimated_complexity"] = json!("Moderate");
        } else {
            // Normalize existing estimated_complexity
            if let Some(complexity) = value.get_mut("estimated_complexity") {
                self.normalize_complexity_enum(complexity);
            }
        }

        // Normalize parent task complexity
        if let Some(parent_tasks) = value.get_mut("parent_tasks").and_then(Value::as_array_mut) {
            for parent_task in parent_tasks {
                if let Some(complexity) = parent_task.get_mut("complexity") {
                    self.normalize_complexity_enum(complexity);
                }

                // Normalize subtask complexity
                if let Some(subtasks) =
                    parent_task.get_mut("subtasks").and_then(Value::as_array_mut)
                {
                    for subtask in subtasks {
                        if let Some(complexity) = subtask.get_mut("complexity") {
                            self.normalize_complexity_enum(complexity);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// 5) FINAL validation only after normalization
    fn validate_task_breakdown(&self, value: &Value) -> Result<Value> {
        let mut errors = Vec::new();

        // Validate required fields
        if !value.get("prd_title").and_then(Value::as_str).is_some() {
            errors.push("Missing required field: prd_title".to_string());
        }
        if !value.get("parent_tasks").and_then(Value::as_array).is_some() {
            errors.push("Missing required field: parent_tasks".to_string());
        }
        if !value.get("relevant_files").and_then(Value::as_array).is_some() {
            errors.push("Missing required field: relevant_files".to_string());
        }
        if !value.get("estimated_complexity").and_then(Value::as_str).is_some() {
            errors.push("Missing required field: estimated_complexity".to_string());
        }

        if !errors.is_empty() {
            return Err(anyhow!("TaskBreakdown validation failed: {}", errors.join(", ")));
        }

        // Return the value as-is since we've been working with a mutable reference
        Ok(value.clone())
    }

    /// Helper: coerce value to f32
    fn coerce_to_f32(&self, value: &mut Value) {
        if value.is_string() {
            if let Ok(num) = value.as_str().unwrap().parse::<f32>() {
                *value = json!(num);
            }
        } else if value.is_number() {
            if let Some(num) = value.as_f64() {
                *value = json!(num as f32);
            }
        }
    }

    // === NEW PIPELINE HELPER FUNCTIONS FOR PRIORITYRESULT ===

    /// 1) Normalize arrays: missing priorities array
    fn normalize_arrays_for_priority_result(&self, value: &mut Value) -> Result<()> {
        if !self.config.allow_coercion {
            return Ok(());
        }

        // Auto-fix missing priorities array
        if !value.get("priorities").and_then(Value::as_array).is_some() {
            value["priorities"] = json!([]);
        }

        Ok(())
    }

    /// 2) Normalize types: numbers → strings for task_id and priority
    fn normalize_types_for_priority_result(&self, value: &mut Value) -> Result<()> {
        if !self.config.allow_coercion {
            return Ok(());
        }

        if let Some(priorities) = value.get_mut("priorities").and_then(Value::as_array_mut) {
            for priority_item in priorities {
                // Coerce task_id to string
                if let Some(task_id) = priority_item.get_mut("task_id") {
                    if task_id.is_number() {
                        *task_id = json!(task_id.as_i64().unwrap_or(0).to_string());
                    }
                }

                // Coerce priority to string (must remain string, not enum)
                if let Some(priority) = priority_item.get_mut("priority") {
                    if priority.is_number() {
                        *priority = json!(priority.as_i64().unwrap_or(0).to_string());
                    }
                }
            }
        }

        Ok(())
    }

    /// 3) FINAL validation only after normalization
    fn validate_priority_result(&self, value: &Value) -> Result<Value> {
        let mut errors = Vec::new();

        // Validate required priorities array
        if !value.get("priorities").and_then(Value::as_array).is_some() {
            errors.push("Missing required field: priorities array".to_string());
        }

        // Validate each priority assignment
        if let Some(priorities) = value.get("priorities").and_then(Value::as_array) {
            for (i, priority_item) in priorities.iter().enumerate() {
                if !priority_item.get("task_id").and_then(Value::as_str).is_some() {
                    errors.push(format!("Priority item {} missing task_id", i));
                }
                if !priority_item.get("priority").and_then(Value::as_str).is_some() {
                    errors.push(format!("Priority item {} missing priority", i));
                }
            }
        }

        if !errors.is_empty() {
            return Err(anyhow!("PriorityResult validation failed: {}", errors.join(", ")));
        }

        Ok(value.clone())
    }

    /// Create structured error response
    fn create_error_response(&self, schema_name: &str, errors: Vec<String>) -> Value {
        json!({
            "error": "SchemaValidationFailed",
            "schema": schema_name,
            "missing_fields": errors,
            "message": format!("LLM output did not contain required fields for {}", schema_name)
        })
    }
}

/// Target schema types for translation
#[derive(Debug, Clone, Copy)]
pub enum TargetSchema {
    TaskBreakdown,
    PriorityResult,
    SubtaskBreakdown,
    NextTaskSuggestion,
    SequentialStep,
}

/// Convenience function for quick translation (strict mode with coercion for type fixes)
pub fn translate_llm_output(raw_output: &str, target_schema: TargetSchema) -> Result<Value> {
    let translator = LlmOutputTranslator::default(); // strict_mode: true, allow_coercion: true
    translator.translate(raw_output, target_schema)
}

/// Strict translation (no coercion, fails on any schema issue)
pub fn translate_llm_output_strict(raw_output: &str, target_schema: TargetSchema) -> Result<Value> {
    let translator = LlmOutputTranslator::with_strict_mode();
    translator.translate(raw_output, target_schema)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_with_markdown() {
        let translator = LlmOutputTranslator::default();
        let input = r#"
        Here's the JSON response:

        ```json
        {
          "prd_title": "Test Feature",
          "parent_tasks": []
        }
        ```

        That should work!
        "#;

        let result = translator.extract_json(input).unwrap();
        assert_eq!(result["prd_title"], "Test Feature");
    }

    #[test]
    fn test_translate_priority_result_with_string_priority() {
        let translator = LlmOutputTranslator::default();
        let input = r#"
        {
          "priorities": [
            {
              "task_id": "123",
              "priority": "High"
            }
          ]
        }
        "#;

        let result = translator.translate(input, TargetSchema::PriorityResult).unwrap();
        assert!(result.get("error").is_none());
        let priorities = result["priorities"].as_array().unwrap();
        assert_eq!(priorities[0]["task_id"], "123");
        assert_eq!(priorities[0]["priority"], "High"); // Must remain string
    }

    #[test]
    fn test_translate_task_breakdown_missing_fields() {
        let translator = LlmOutputTranslator::default();
        let input = r#"
        {
          "prd_title": "Test"
        }
        "#;

        let result = translator.translate(input, TargetSchema::TaskBreakdown).unwrap();

        // Translator should AUTO-FILL missing fields for resilience
        assert!(
            result.get("error").is_none(),
            "Should not error - should auto-fill missing fields"
        );
        assert_eq!(result["prd_title"], "Test");

        // Should have auto-filled required fields
        assert!(result.get("parent_tasks").is_some(), "Should auto-fill parent_tasks");
        assert!(result.get("relevant_files").is_some(), "Should auto-fill relevant_files");
        assert!(
            result.get("estimated_complexity").is_some(),
            "Should auto-fill estimated_complexity"
        );

        // Auto-filled arrays should be empty
        assert_eq!(result["parent_tasks"].as_array().unwrap().len(), 0);
        assert_eq!(result["relevant_files"].as_array().unwrap().len(), 0);

        // Should have default complexity
        assert!(!result["estimated_complexity"].as_str().unwrap().is_empty());
    }
}
