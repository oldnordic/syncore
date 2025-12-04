//! Streaming Contract Enforcer
//!
//! Prevents MCP tools from returning massive responses (>200 lines, >50KB)
//! directly into LLM context by truncating, chunking, or paging results.

use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use std::collections::HashMap;
use sha2::{Sha256, Digest};

/// Hard limits for output size
const MAX_LINES: usize = 200;
const MAX_BYTES: usize = 50_000;
const PREVIEW_LINES: usize = 10;
const TAIL_LINES: usize = 10;

/// Storage for truncated data (in-memory fallback for now)
static mut TRUNCATED_STORAGE: Option<HashMap<String, String>> = None;

fn get_storage() -> &'static mut HashMap<String, String> {
    unsafe {
        if TRUNCATED_STORAGE.is_none() {
            TRUNCATED_STORAGE = Some(HashMap::new());
        }
        TRUNCATED_STORAGE.as_mut().unwrap()
    }
}

#[derive(Debug, Clone)]
pub struct OutputLimiter {
    max_lines: usize,
    max_bytes: usize,
}

impl Default for OutputLimiter {
    fn default() -> Self {
        Self {
            max_lines: MAX_LINES,
            max_bytes: MAX_BYTES,
        }
    }
}

impl OutputLimiter {
    pub fn new(max_lines: usize, max_bytes: usize) -> Self {
        Self {
            max_lines,
            max_bytes,
        }
    }

    /// Apply contract enforcement to any JSON value
    pub fn apply_json(&self, value: &Value) -> Result<Value> {
        // Convert to string for analysis
        let json_str = serde_json::to_string_pretty(value)?;

        // Check if truncation is needed
        let lines: Vec<&str> = json_str.lines().collect();
        let bytes = json_str.as_bytes().len();

        if lines.len() <= self.max_lines && bytes <= self.max_bytes {
            // No truncation needed
            return Ok(value.clone());
        }

        // Truncation required
        self.truncate_json(value, &json_str, lines.len(), bytes)
    }

    /// Apply contract enforcement to string content
    pub fn apply_string(&self, content: &str) -> Result<Value> {
        let lines: Vec<&str> = content.lines().collect();
        let bytes = content.as_bytes().len();

        if lines.len() <= self.max_lines && bytes <= self.max_bytes {
            // No truncation needed
            return Ok(json!({
                "content": content,
                "meta": {
                    "truncated": false,
                    "total_lines": lines.len(),
                    "total_bytes": bytes,
                    "lines_returned": lines.len(),
                    "hash": self.calculate_hash(content)
                }
            }));
        }

        // Truncation required
        self.truncate_string(content, lines.len(), bytes)
    }

    /// Apply paging mode to arrays
    pub fn apply_paging(&self, array: &Value, page: Option<usize>, page_size: Option<usize>) -> Result<Value> {
        if !array.is_array() {
            return Err(anyhow!("Paging only applies to arrays"));
        }

        let page = page.unwrap_or(1);
        let page_size = page_size.unwrap_or(100);
        let array = array.as_array().unwrap();

        let total_items = array.len();
        let total_pages = (total_items + page_size - 1) / page_size;

        if page < 1 || page > total_pages {
            return Ok(json!({
                "error": format!("Invalid page {}. Valid range: 1-{}", page, total_pages),
                "page": page,
                "total_pages": total_pages,
                "page_size": page_size
            }));
        }

        let start_idx = (page - 1) * page_size;
        let end_idx = std::cmp::min(start_idx + page_size, total_items);
        let page_items: Vec<&Value> = array[start_idx..end_idx].iter().collect();

        Ok(json!({
            "page": page,
            "total_pages": total_pages,
            "page_size": page_size,
            "total_items": total_items,
            "items": page_items
        }))
    }

    /// Apply chunking mode to large strings
    pub fn apply_chunking(&self, content: &str, chunk_size: Option<usize>) -> Result<Value> {
        let chunk_size = chunk_size.unwrap_or(512);
        let bytes = content.as_bytes();

        let mut chunks = Vec::new();
        for i in (0..bytes.len()).step_by(chunk_size) {
            let end = std::cmp::min(i + chunk_size, bytes.len());
            let chunk = String::from_utf8(bytes[i..end].to_vec())?;
            chunks.push(chunk);
        }

        Ok(json!({
            "chunks": chunks,
            "chunk_count": chunks.len(),
            "chunk_size": chunk_size,
            "total_bytes": bytes.len()
        }))
    }

    fn truncate_json(&self, original: &Value, json_str: &str, total_lines: usize, total_bytes: usize) -> Result<Value> {
        // Store full content
        let storage_key = self.store_truncated(json_str)?;

        // Check if this is an MCP-style response with "command" field
        if let Some(command) = original.get("command") {
            // Preserve command, but truncate data section
            let truncated_data = if let Some(data) = original.get("data") {
                self.truncate_json_recursively(data, PREVIEW_LINES)
            } else {
                json!(null)
            };

            Ok(json!({
                "command": command,
                "meta": {
                    "truncated": true,
                    "total_lines": total_lines,
                    "total_bytes": total_bytes,
                    "lines_returned": "truncated",
                    "storage_key": storage_key,
                    "hash": self.calculate_hash(json_str)
                },
                "truncated_data": truncated_data
            }))
        } else {
            // Regular JSON - truncate everything
            let truncated = self.truncate_json_recursively(original, PREVIEW_LINES);

            Ok(json!({
                "meta": {
                    "truncated": true,
                    "total_lines": total_lines,
                    "total_bytes": total_bytes,
                    "lines_returned": "truncated",
                    "storage_key": storage_key,
                    "hash": self.calculate_hash(json_str)
                },
                "truncated_data": truncated
            }))
        }
    }

    fn truncate_json_recursively(&self, value: &Value, max_items: usize) -> Value {
        match value {
            Value::Array(arr) => {
                let limited: Vec<Value> = arr.iter()
                    .take(max_items)
                    .map(|v| self.truncate_json_recursively(v, max_items))
                    .collect();
                Value::Array(limited)
            },
            Value::Object(obj) => {
                let mut limited_obj = serde_json::Map::new();
                for (k, v) in obj {
                    limited_obj.insert(k.clone(), self.truncate_json_recursively(v, max_items));
                }
                Value::Object(limited_obj)
            },
            Value::String(s) => {
                if s.lines().count() > PREVIEW_LINES {
                    let lines: Vec<&str> = s.lines().collect();
                    let preview_end = std::cmp::min(PREVIEW_LINES, lines.len());
                    let preview: String = lines[0..preview_end].join("\n");
                    Value::String(format!("{}...\n[{} lines truncated]", preview, lines.len() - preview_end))
                } else {
                    Value::String(s.clone())
                }
            },
            _ => value.clone(),
        }
    }

    fn truncate_string(&self, content: &str, total_lines: usize, total_bytes: usize) -> Result<Value> {
        let lines: Vec<&str> = content.lines().collect();

        // Take first and last portions
        let preview_end = std::cmp::min(PREVIEW_LINES, lines.len());
        let tail_start = if lines.len() > TAIL_LINES {
            lines.len() - TAIL_LINES
        } else {
            preview_end
        };

        let preview: Vec<&str> = lines[0..preview_end].to_vec();
        let tail: Vec<&str> = if tail_start < lines.len() {
            lines[tail_start..].to_vec()
        } else {
            Vec::new()
        };

        // Store full content
        let storage_key = self.store_truncated(content)?;

        Ok(json!({
            "meta": {
                "truncated": true,
                "total_lines": total_lines,
                "total_bytes": total_bytes,
                "lines_returned": preview.len() + tail.len(),
                "storage_key": storage_key,
                "hash": self.calculate_hash(content)
            },
            "preview": preview,
            "tail": tail,
            "message": format!("Output truncated: {}/{} lines, {}/{} bytes. Use storage_key '{}' to retrieve full content.",
                preview.len() + tail.len(), total_lines,
                preview.iter().map(|l| l.len()).sum::<usize>() + tail.iter().map(|l| l.len()).sum::<usize>(),
                total_bytes, storage_key)
        }))
    }

    pub fn store_truncated(&self, content: &str) -> Result<String> {
        let hash = self.calculate_hash(content);
        let storage_key = format!("trunc_{}", hash);

        get_storage().insert(storage_key.clone(), content.to_string());

        Ok(storage_key)
    }

    fn calculate_hash(&self, content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("sha256:{}", hex::encode(hasher.finalize()))
    }

    /// Retrieve stored truncated content
    pub fn retrieve_stored(&self, storage_key: &str) -> Result<Option<String>> {
        Ok(get_storage().get(storage_key).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_no_truncation_small_output() {
        let limiter = OutputLimiter::default();
        let small_json = json!({
            "message": "hello",
            "data": vec![1, 2, 3]
        });

        let result = limiter.apply_json(&small_json).unwrap();
        assert_eq!(result, small_json);
    }

    #[test]
    fn test_truncates_large_output() {
        let limiter = OutputLimiter::new(5, 1000); // Very small limits for testing

        // Create a large JSON array
        let large_data: Vec<i32> = (0..100).collect();
        let large_json = json!({
            "data": large_data,
            "metadata": "test"
        });

        let result = limiter.apply_json(&large_json).unwrap();

        // Should have truncation metadata
        assert!(result.get("meta").is_some());
        let meta = result.get("meta").unwrap();
        assert_eq!(meta["truncated"], true);
        assert!(meta["total_lines"].as_u64().unwrap() > 5);

        // Should have preview and tail
        assert!(result.get("preview").is_some());
        assert!(result.get("tail").is_some());
    }

    #[test]
    fn test_paging_mode_basic() {
        let limiter = OutputLimiter::default();

        let data: Vec<i32> = (0..25).collect();
        let array_json = json!(data);

        let result = limiter.apply_paging(&array_json, Some(2), Some(10)).unwrap();

        assert_eq!(result["page"], 2);
        assert_eq!(result["page_size"], 10);
        assert_eq!(result["total_items"], 25);
        assert_eq!(result["total_pages"], 3);
        assert_eq!(result["items"].as_array().unwrap().len(), 10);
    }

    #[test]
    fn test_chunking_mode_basic() {
        let limiter = OutputLimiter::default();
        let long_string = "a".repeat(1500);

        let result = limiter.apply_chunking(&long_string, Some(512)).unwrap();

        assert_eq!(result["chunk_size"], 512);
        assert!(result["chunk_count"].as_u64().unwrap() > 2);
        assert!(result["total_bytes"].as_u64().unwrap() == 1500);

        let chunks = result["chunks"].as_array().unwrap();
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].as_str().unwrap().len(), 512);
        assert_eq!(chunks[1].as_str().unwrap().len(), 512);
        assert_eq!(chunks[2].as_str().unwrap().len(), 476);
    }

    #[test]
    fn test_hash_stability() {
        let limiter = OutputLimiter::default();
        let content = "deterministic content";

        let hash1 = limiter.calculate_hash(content);
        let hash2 = limiter.calculate_hash(content);

        assert_eq!(hash1, hash2);
        assert!(hash1.starts_with("sha256:"));
    }

    #[test]
    fn test_storage_and_retrieval() {
        let limiter = OutputLimiter::default();
        let content = "test content for storage";
        let storage_key = limiter.store_truncated(content).unwrap();

        let retrieved = limiter.retrieve_stored(&storage_key).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), content);
    }
}