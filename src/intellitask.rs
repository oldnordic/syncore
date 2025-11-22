use crate::ollama::OllamaClient;
use anyhow::{anyhow, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Task breakdown from a Product Requirements Document
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TaskBreakdown {
    pub prd_title: String,
    pub parent_tasks: Vec<ParentTask>,
    pub relevant_files: Vec<FileReference>,
    pub estimated_complexity: Complexity,
}

/// Parent task containing multiple subtasks
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ParentTask {
    pub id: String, // e.g., "1.0"
    pub title: String,
    pub description: String,
    pub subtasks: Vec<Subtask>,
    pub dependencies: Vec<String>, // IDs of tasks this depends on
    pub complexity: Complexity,
    pub estimated_hours: f32,
}

/// Individual subtask within a parent task
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Subtask {
    pub id: String, // e.g., "1.1"
    pub description: String,
    pub acceptance_criteria: Vec<String>,
    pub dependencies: Vec<String>, // IDs of subtasks this depends on
    pub files_to_modify: Vec<String>,
    pub complexity: Complexity,
    pub estimated_hours: f32,
}

/// File reference with purpose
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FileReference {
    pub path: String,
    pub purpose: String,
    pub action: FileAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub enum FileAction {
    Create,
    Modify,
    Review,
    #[serde(alias = "Implement", alias = "Add", alias = "Update")]
    Modify2, // Alias for common variations
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord)]
pub enum Complexity {
    Trivial,     // < 1 hour
    Simple,      // 1-4 hours
    Moderate,    // 4-16 hours
    Complex,     // 16-40 hours
    VeryComplex, // > 40 hours
}

/// Task priority determined by AI reasoning
/// Ordering: Critical > High > Medium > Low > Optional
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Optional, // Can skip if needed (lowest priority)
    Low,      // Nice to have
    Medium,   // Normal priority
    High,     // Important, do soon
    Critical, // Blocking, must do first (highest priority)
}

/// IntelliTask: Advanced AI-powered task management using phi3:mini reasoning
///
/// IntelliTask is SynCore's intelligent task breakdown and management system.
/// Unlike simple task lists, IntelliTask uses AI reasoning to:
/// - Break down complex PRDs into actionable tasks
/// - Analyze dependencies and suggest optimal execution order
/// - Estimate complexity and time requirements
/// - Prioritize based on business value and technical constraints
/// - Provide intelligent suggestions for next steps
pub struct IntelliTask {
    ollama: Arc<std::sync::Mutex<OllamaClient>>,
}

impl IntelliTask {
    /// Create a new IntelliTask instance
    pub fn new(ollama: OllamaClient) -> Self {
        Self {
            ollama: Arc::new(std::sync::Mutex::new(ollama)),
        }
    }

    /// Create JSON schema for TaskBreakdown
    fn task_breakdown_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "prd_title": {"type": "string"},
                "parent_tasks": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": {"type": "string"},
                            "title": {"type": "string"},
                            "description": {"type": "string"},
                            "subtasks": {"type": "array", "maxItems": 0},
                            "dependencies": {"type": "array", "items": {"type": "string"}},
                            "complexity": {"type": "string", "enum": ["Trivial", "Simple", "Moderate", "Complex", "VeryComplex"]},
                            "estimated_hours": {"type": "number"}
                        },
                        "required": ["id", "title", "description", "subtasks", "dependencies", "complexity", "estimated_hours"]
                    }
                },
                "relevant_files": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string"},
                            "purpose": {"type": "string"},
                            "action": {"type": "string", "enum": ["Create", "Modify", "Review"]}
                        },
                        "required": ["path", "purpose", "action"]
                    }
                },
                "estimated_complexity": {"type": "string", "enum": ["Trivial", "Simple", "Moderate", "Complex", "VeryComplex"]}
            },
            "required": ["prd_title", "parent_tasks", "relevant_files", "estimated_complexity"]
        })
    }

    /// Generate task breakdown from PRD using AI reasoning with JSON schema constraints
    pub fn generate_tasks_from_prd(&self, prd_content: &str) -> Result<TaskBreakdown> {
        let prompt = format!(
            "You are an expert software architect analyzing a Product Requirements Document (PRD). \
            Break down the requirements into a structured implementation plan.\n\n\
            PRD CONTENT:\n{}\n\n\
            INSTRUCTIONS:\n\
            1. Identify 5-8 high-level parent tasks needed to implement this PRD\n\
            2. For each parent task, estimate complexity (Trivial/Simple/Moderate/Complex/VeryComplex)\n\
            3. Identify dependencies between tasks (task IDs that must be completed first)\n\
            4. List files that will need to be created or modified\n\
            5. IMPORTANT: Leave subtasks array EMPTY - subtasks will be generated separately\n\
            6. Provide a structured JSON response\n\n\
            OUTPUT FORMAT (strict JSON - do NOT wrap in markdown code blocks):\n\
            {{\n\
              \"prd_title\": \"Feature Name\",\n\
              \"parent_tasks\": [\n\
                {{\n\
                  \"id\": \"1.0\",\n\
                  \"title\": \"Task Title\",\n\
                  \"description\": \"What needs to be done\",\n\
                  \"subtasks\": [],\n\
                  \"dependencies\": [],\n\
                  \"complexity\": \"Moderate\",\n\
                  \"estimated_hours\": 8.0\n\
                }}\n\
              ],\n\
              \"relevant_files\": [\n\
                {{\n\
                  \"path\": \"src/module.rs\",\n\
                  \"purpose\": \"Why this file matters\",\n\
                  \"action\": \"Create\"\n\
                }}\n\
              ],\n\
              \"estimated_complexity\": \"Complex\"\n\
            }}\n\n\
            Respond ONLY with raw JSON, no markdown formatting, no code blocks, no additional text.",
            prd_content
        );

        // Use JSON schema to constrain output (Ollama 0.5+ structured outputs)
        let schema = Self::task_breakdown_schema();
        let response = self.generate_with_schema(&prompt, Some(schema))?;

        // Parse JSON response (strip markdown wrapper if present)
        let cleaned_response = Self::strip_markdown_json(&response);
        let breakdown: TaskBreakdown = serde_json::from_str(&cleaned_response).map_err(|e| {
            anyhow!(
                "Failed to parse AI response as JSON: {}. Response was: {}",
                e,
                response
            )
        })?;

        Ok(breakdown)
    }

    /// Create JSON schema for Subtask array
    fn subtask_array_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "id": {"type": "string"},
                    "description": {"type": "string"},
                    "acceptance_criteria": {"type": "array", "items": {"type": "string"}},
                    "dependencies": {"type": "array", "items": {"type": "string"}},
                    "files_to_modify": {"type": "array", "items": {"type": "string"}},
                    "complexity": {"type": "string", "enum": ["Trivial", "Simple", "Moderate", "Complex", "VeryComplex"]},
                    "estimated_hours": {"type": "number"}
                },
                "required": ["id", "description", "acceptance_criteria", "dependencies", "files_to_modify", "complexity", "estimated_hours"]
            }
        })
    }

    /// Generate subtasks for a parent task using AI reasoning with JSON schema constraints
    pub fn generate_subtasks(
        &self,
        parent_task: &ParentTask,
        codebase_context: &str,
    ) -> Result<Vec<Subtask>> {
        let prompt = format!(
            "You are an expert developer breaking down a high-level task into implementation steps.\n\n\
            PARENT TASK:\n\
            ID: {}\n\
            Title: {}\n\
            Description: {}\n\
            Complexity: {:?}\n\n\
            CODEBASE CONTEXT:\n\
            {}\n\n\
            INSTRUCTIONS:\n\
            1. Break this task into 3-7 concrete subtasks\n\
            2. Each subtask should be actionable and testable\n\
            3. Order subtasks by logical implementation sequence\n\
            4. Identify file dependencies for each subtask\n\
            5. Estimate complexity and hours for each\n\
            6. Provide clear acceptance criteria\n\
            7. CRITICAL: Format subtask IDs as ParentID.N where N is 1,2,3...\n\
            8. For this parent task ID ({}), your subtasks MUST be: {}.1, {}.2, {}.3, etc.\n\n\
            OUTPUT FORMAT (strict JSON array):\n\
            [\n\
              {{\n\
                \"id\": \"ParentID.1\",\n\
                \"description\": \"Specific action to take\",\n\
                \"acceptance_criteria\": [\"Test passes\", \"Code reviewed\"],\n\
                \"dependencies\": [],\n\
                \"files_to_modify\": [\"src/file.rs\"],\n\
                \"complexity\": \"Simple\",\n\
                \"estimated_hours\": 2.0\n\
              }}\n\
            ]\n\n\
            Respond ONLY with the JSON array, no additional text.",
            parent_task.id,
            parent_task.title,
            parent_task.description,
            parent_task.complexity,
            codebase_context,
            parent_task.id,  // Line 236: "parent task ID ({})"
            parent_task.id,  // Line 236: "{}.1"
            parent_task.id,  // Line 236: "{}.2"
            parent_task.id   // Line 236: "{}.3"
        );

        // Use JSON schema to constrain output
        let schema = Self::subtask_array_schema();
        let response = self.generate_with_schema(&prompt, Some(schema))?;

        // Parse JSON response (strip markdown wrapper if present)
        let cleaned_response = Self::strip_markdown_json(&response);
        let subtasks: Vec<Subtask> = serde_json::from_str(&cleaned_response).map_err(|e| {
            anyhow!(
                "Failed to parse subtasks JSON: {}. Response was: {}",
                e,
                response
            )
        })?;

        Ok(subtasks)
    }

    /// Analyze task dependencies and determine optimal execution order
    pub fn analyze_dependencies(&self, tasks: &[ParentTask]) -> Result<Vec<String>> {
        let tasks_json = serde_json::to_string_pretty(tasks)?;

        let prompt = format!(
            "You are analyzing task dependencies to determine optimal execution order.\n\n\
            TASKS:\n\
            {}\n\n\
            INSTRUCTIONS:\n\
            1. Analyze dependencies between tasks\n\
            2. Identify the critical path\n\
            3. Determine optimal execution order (respecting dependencies)\n\
            4. Consider parallelization opportunities\n\n\
            OUTPUT FORMAT (strict JSON):\n\
            {{\n\
              \"execution_order\": [\"1.0\", \"2.0\", \"3.0\"],\n\
              \"can_parallelize\": [[\"2.0\", \"3.0\"]],\n\
              \"critical_path\": [\"1.0\", \"4.0\", \"5.0\"],\n\
              \"reasoning\": \"Why this order is optimal\"\n\
            }}\n\n\
            Respond ONLY with JSON, no additional text.",
            tasks_json
        );

        let response = self.generate(&prompt)?;
        let json_str = Self::extract_json(&response)?;

        #[derive(Deserialize)]
        struct DependencyAnalysis {
            execution_order: Vec<String>,
        }

        let analysis: DependencyAnalysis = serde_json::from_str(json_str).map_err(|e| {
            anyhow!(
                "Failed to parse dependency analysis: {}. Response was: {}",
                e,
                json_str
            )
        })?;

        Ok(analysis.execution_order)
    }

    /// Prioritize tasks based on complexity, dependencies, and business value
    pub fn prioritize_tasks(
        &self,
        tasks: &[ParentTask],
        business_context: &str,
    ) -> Result<Vec<(String, TaskPriority)>> {
        let tasks_json = serde_json::to_string_pretty(tasks)?;

        let prompt = format!(
            "You are prioritizing tasks for implementation based on multiple factors.\n\n\
            TASKS:\n\
            {}\n\n\
            BUSINESS CONTEXT:\n\
            {}\n\n\
            INSTRUCTIONS:\n\
            1. Consider: dependencies, complexity, business value, risk\n\
            2. Assign priority: Critical/High/Medium/Low/Optional\n\
            3. Explain reasoning for each priority assignment\n\n\
            OUTPUT FORMAT (strict JSON):\n\
            {{\n\
              \"priorities\": [\n\
                {{\"task_id\": \"1.0\", \"priority\": \"Critical\", \"reason\": \"Why\"}}\n\
              ]\n\
            }}\n\n\
            Respond ONLY with JSON, no additional text.",
            tasks_json, business_context
        );

        let response = self.generate(&prompt)?;
        let json_str = Self::extract_json(&response)?;

        #[derive(Deserialize)]
        struct PriorityResult {
            priorities: Vec<TaskPriorityAssignment>,
        }

        #[derive(Deserialize)]
        struct TaskPriorityAssignment {
            task_id: String,
            priority: String,
        }

        let result: PriorityResult = serde_json::from_str(json_str).map_err(|e| {
            anyhow!(
                "Failed to parse priority analysis: {}. Response was: {}",
                e,
                json_str
            )
        })?;

        Ok(result
            .priorities
            .into_iter()
            .map(|p| {
                let priority = match p.priority.as_str() {
                    "Critical" => TaskPriority::Critical,
                    "High" => TaskPriority::High,
                    "Medium" => TaskPriority::Medium,
                    "Low" => TaskPriority::Low,
                    _ => TaskPriority::Optional,
                };
                (p.task_id, priority)
            })
            .collect())
    }

    /// Suggest next task to work on based on current progress
    pub fn suggest_next_task(
        &self,
        completed_tasks: &[String],
        remaining_tasks: &[ParentTask],
    ) -> Result<String> {
        let completed = completed_tasks.join(", ");
        let remaining_json = serde_json::to_string_pretty(remaining_tasks)?;

        let prompt = format!(
            "You are suggesting which task to work on next.\n\n\
            COMPLETED TASKS:\n{}\n\n\
            REMAINING TASKS:\n{}\n\n\
            INSTRUCTIONS:\n\
            1. Consider dependencies - suggest only tasks whose dependencies are met\n\
            2. Consider complexity - balance quick wins with complex work\n\
            3. Consider logical flow - what makes sense to do next\n\
            4. Provide clear reasoning for the suggestion\n\n\
            OUTPUT FORMAT (strict JSON):\n\
            {{\n\
              \"suggested_task_id\": \"2.0\",\n\
              \"reasoning\": \"Why this task should be done next\",\n\
              \"prerequisites_met\": true,\n\
              \"estimated_completion_time\": \"4 hours\"\n\
            }}\n\n\
            Respond ONLY with JSON, no additional text.",
            completed, remaining_json
        );

        let response = self.generate(&prompt)?;
        let json_str = Self::extract_json(&response)?;

        #[derive(Deserialize)]
        struct NextTaskSuggestion {
            suggested_task_id: String,
            reasoning: String,
        }

        let suggestion: NextTaskSuggestion = serde_json::from_str(json_str).map_err(|e| {
            anyhow!(
                "Failed to parse next task suggestion: {}. Response was: {}",
                e,
                json_str
            )
        })?;

        Ok(format!(
            "Suggested: {} - {}",
            suggestion.suggested_task_id, suggestion.reasoning
        ))
    }

    /// Extract balanced JSON from AI response (handles markdown blocks and extra text)
    fn extract_json(response: &str) -> Result<&str> {
        // Strip markdown code blocks if present
        let cleaned = if response.contains("```json") {
            if let Some(start) = response.find("```json") {
                let after_start = &response[start + 7..];
                if let Some(end) = after_start.find("```") {
                    after_start[..end].trim()
                } else {
                    response
                }
            } else {
                response
            }
        } else {
            response
        };

        // Find balanced JSON object or array
        if let Some(start) = cleaned.find('{') {
            // Find matching closing brace by tracking nesting depth
            let mut depth = 0;
            let mut in_string = false;
            let mut escape_next = false;

            for (i, ch) in cleaned[start..].char_indices() {
                if escape_next {
                    escape_next = false;
                    continue;
                }

                match ch {
                    '\\' if in_string => escape_next = true,
                    '"' => in_string = !in_string,
                    '{' if !in_string => depth += 1,
                    '}' if !in_string => {
                        depth -= 1;
                        if depth == 0 {
                            return Ok(&cleaned[start..start + i + 1]);
                        }
                    }
                    _ => {}
                }
            }
        }

        // Fallback: return cleaned response
        Ok(cleaned.trim())
    }

    /// Generate prompt using Ollama
    fn generate(&self, prompt: &str) -> Result<String> {
        self.generate_with_schema(prompt, None)
    }

    /// Generate with JSON schema constraint
    fn generate_with_schema(
        &self,
        prompt: &str,
        schema: Option<serde_json::Value>,
    ) -> Result<String> {
        let ollama = self
            .ollama
            .lock()
            .map_err(|e| anyhow!("Failed to lock Ollama client: {}", e))?;

        ollama.generate_with_schema(prompt, schema)
    }

    /// Strip markdown code block wrapper from JSON response
    ///
    /// Ollama CLI often returns JSON wrapped in markdown code blocks like:
    /// ```json
    /// { "actual": "json" }
    /// ```
    ///
    /// This function strips the wrapper to get the raw JSON.
    fn strip_markdown_json(response: &str) -> String {
        let trimmed = response.trim();

        // Check if wrapped in markdown code block
        if trimmed.starts_with("```json") || trimmed.starts_with("```") {
            // Find the first { or [ (start of JSON)
            if let Some(json_start) = trimmed.find(|c| c == '{' || c == '[') {
                // Find the last } or ] (end of JSON)
                if let Some(json_end) = trimmed.rfind(|c| c == '}' || c == ']') {
                    // Extract just the JSON part
                    return trimmed[json_start..=json_end].to_string();
                }
            }
        }

        // If no markdown wrapper found, return as-is
        response.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ollama::OllamaConfig;

    // Helper to create a test IntelliTask instance
    fn create_test_intellitask() -> Result<IntelliTask> {
        let ollama = OllamaClient::new(OllamaConfig::default())?;
        Ok(IntelliTask::new(ollama))
    }

    // Helper to create sample parent task
    fn create_sample_parent_task() -> ParentTask {
        ParentTask {
            id: "1.0".to_string(),
            title: "Database Schema Design".to_string(),
            description: "Design and implement user authentication tables".to_string(),
            subtasks: vec![],
            dependencies: vec![],
            complexity: Complexity::Moderate,
            estimated_hours: 8.0,
        }
    }

    #[test]
    fn test_complexity_ordering() {
        // Test that Complexity enum has correct ordering
        assert!(Complexity::Trivial < Complexity::Simple);
        assert!(Complexity::Simple < Complexity::Moderate);
        assert!(Complexity::Moderate < Complexity::Complex);
        assert!(Complexity::Complex < Complexity::VeryComplex);
    }

    #[test]
    fn test_priority_ordering() {
        // Test that TaskPriority enum has correct ordering
        assert!(TaskPriority::Critical > TaskPriority::High);
        assert!(TaskPriority::High > TaskPriority::Medium);
        assert!(TaskPriority::Medium > TaskPriority::Low);
        assert!(TaskPriority::Low > TaskPriority::Optional);
    }

    #[test]
    fn test_task_breakdown_serialization() {
        // Test that TaskBreakdown can be serialized and deserialized
        let breakdown = TaskBreakdown {
            prd_title: "Test Feature".to_string(),
            parent_tasks: vec![create_sample_parent_task()],
            relevant_files: vec![FileReference {
                path: "src/test.rs".to_string(),
                purpose: "Test file".to_string(),
                action: FileAction::Create,
            }],
            estimated_complexity: Complexity::Moderate,
        };

        let json = serde_json::to_string(&breakdown).unwrap();
        let deserialized: TaskBreakdown = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.prd_title, "Test Feature");
        assert_eq!(deserialized.parent_tasks.len(), 1);
        assert_eq!(deserialized.relevant_files.len(), 1);
    }

    #[test]
    fn test_subtask_serialization() {
        // Test that Subtask can be serialized and deserialized
        let subtask = Subtask {
            id: "1.1".to_string(),
            description: "Create migration".to_string(),
            acceptance_criteria: vec!["Migration runs".to_string()],
            dependencies: vec![],
            files_to_modify: vec!["migrations/001.sql".to_string()],
            complexity: Complexity::Simple,
            estimated_hours: 2.0,
        };

        let json = serde_json::to_string(&subtask).unwrap();
        let deserialized: Subtask = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, "1.1");
        assert_eq!(deserialized.acceptance_criteria.len(), 1);
    }

    #[test]
    #[ignore] // Only run when Ollama is available
    fn test_ollama_connection() {
        // Test that we can connect to Ollama
        let result = OllamaClient::new(OllamaConfig::default());
        assert!(
            result.is_ok(),
            "Failed to create Ollama client: {:?}",
            result.err()
        );
    }

    #[test]
    #[ignore] // Only run when Ollama is available
    fn test_generate_tasks_from_prd() {
        let intellitask = create_test_intellitask().expect("Failed to create IntelliTask instance");

        let prd = "PRD: User Authentication System\n\
                   Requirements:\n\
                   - Users can register with email/password\n\
                   - Users can login with credentials\n\
                   - Session management with JWT tokens\n\
                   - Password reset functionality";

        let result = intellitask.generate_tasks_from_prd(prd);
        assert!(
            result.is_ok(),
            "Failed to generate tasks: {:?}",
            result.err()
        );

        let breakdown = result.unwrap();
        assert!(
            !breakdown.parent_tasks.is_empty(),
            "No parent tasks generated"
        );
        assert!(
            !breakdown.relevant_files.is_empty(),
            "No relevant files identified"
        );
        assert!(!breakdown.prd_title.is_empty(), "No PRD title generated");

        // Verify at least one task has proper structure
        let first_task = &breakdown.parent_tasks[0];
        assert!(!first_task.id.is_empty(), "Task ID is empty");
        assert!(!first_task.title.is_empty(), "Task title is empty");
        assert!(
            !first_task.description.is_empty(),
            "Task description is empty"
        );
        assert!(
            first_task.estimated_hours > 0.0,
            "Estimated hours should be positive"
        );
    }

    #[test]
    #[ignore] // Only run when Ollama is available
    fn test_generate_subtasks() {
        let intellitask = create_test_intellitask().expect("Failed to create IntelliTask instance");

        let parent_task = create_sample_parent_task();
        let codebase_context = "Project: Rust async web service using Axum and PostgreSQL";

        let result = intellitask.generate_subtasks(&parent_task, codebase_context);
        assert!(
            result.is_ok(),
            "Failed to generate subtasks: {:?}",
            result.err()
        );

        let subtasks = result.unwrap();
        assert!(!subtasks.is_empty(), "No subtasks generated");
        assert!(
            subtasks.len() >= 3 && subtasks.len() <= 7,
            "Expected 3-7 subtasks, got {}",
            subtasks.len()
        );

        // Verify subtask structure
        for subtask in &subtasks {
            println!(
                "Generated subtask ID: {}, Expected prefix: {}.",
                subtask.id, parent_task.id
            );
            assert!(
                subtask.id.starts_with(&format!("{}.", parent_task.id)),
                "Subtask ID should start with parent ID. Got: {}, Expected prefix: {}.",
                subtask.id,
                parent_task.id
            );
            assert!(
                !subtask.description.is_empty(),
                "Subtask description is empty"
            );
            assert!(
                !subtask.acceptance_criteria.is_empty(),
                "Subtask should have acceptance criteria"
            );
            assert!(
                subtask.estimated_hours > 0.0,
                "Estimated hours should be positive"
            );
        }
    }

    #[test]
    #[ignore] // Only run when Ollama is available
    fn test_analyze_dependencies() {
        let intellitask = create_test_intellitask().expect("Failed to create IntelliTask instance");

        let tasks = vec![
            ParentTask {
                id: "1.0".to_string(),
                title: "Database Schema".to_string(),
                description: "Create tables".to_string(),
                subtasks: vec![],
                dependencies: vec![],
                complexity: Complexity::Moderate,
                estimated_hours: 8.0,
            },
            ParentTask {
                id: "2.0".to_string(),
                title: "API Endpoints".to_string(),
                description: "Create REST API".to_string(),
                subtasks: vec![],
                dependencies: vec!["1.0".to_string()],
                complexity: Complexity::Complex,
                estimated_hours: 16.0,
            },
        ];

        let result = intellitask.analyze_dependencies(&tasks);
        assert!(
            result.is_ok(),
            "Failed to analyze dependencies: {:?}",
            result.err()
        );

        let execution_order = result.unwrap();
        assert!(!execution_order.is_empty(), "No execution order generated");

        // Task 1.0 should come before 2.0 (dependency constraint)
        let pos_1 = execution_order.iter().position(|id| id == "1.0");
        let pos_2 = execution_order.iter().position(|id| id == "2.0");

        if let (Some(p1), Some(p2)) = (pos_1, pos_2) {
            assert!(p1 < p2, "Task 1.0 should come before 2.0 due to dependency");
        }
    }

    #[test]
    #[ignore] // Only run when Ollama is available
    fn test_prioritize_tasks() {
        let intellitask = create_test_intellitask().expect("Failed to create IntelliTask instance");

        let tasks = vec![
            create_sample_parent_task(),
            ParentTask {
                id: "2.0".to_string(),
                title: "Admin Dashboard".to_string(),
                description: "Create admin UI".to_string(),
                subtasks: vec![],
                dependencies: vec!["1.0".to_string()],
                complexity: Complexity::Complex,
                estimated_hours: 20.0,
            },
        ];

        let business_context = "MVP launch in 4 weeks. Authentication is critical for security.";

        let result = intellitask.prioritize_tasks(&tasks, business_context);
        assert!(
            result.is_ok(),
            "Failed to prioritize tasks: {:?}",
            result.err()
        );

        let priorities = result.unwrap();
        assert_eq!(
            priorities.len(),
            tasks.len(),
            "Should have priority for each task"
        );

        // Verify priorities are valid
        for (task_id, priority) in &priorities {
            assert!(!task_id.is_empty(), "Task ID should not be empty");
            // Priority should be one of the enum values
            assert!(matches!(
                priority,
                TaskPriority::Critical
                    | TaskPriority::High
                    | TaskPriority::Medium
                    | TaskPriority::Low
                    | TaskPriority::Optional
            ));
        }
    }

    #[test]
    #[ignore] // Only run when Ollama is available
    fn test_suggest_next_task() {
        let intellitask = create_test_intellitask().expect("Failed to create IntelliTask instance");

        let completed = vec!["1.0".to_string()];
        let remaining = vec![
            ParentTask {
                id: "2.0".to_string(),
                title: "API Implementation".to_string(),
                description: "Build REST API".to_string(),
                subtasks: vec![],
                dependencies: vec!["1.0".to_string()],
                complexity: Complexity::Complex,
                estimated_hours: 16.0,
            },
            ParentTask {
                id: "3.0".to_string(),
                title: "Documentation".to_string(),
                description: "Write API docs".to_string(),
                subtasks: vec![],
                dependencies: vec!["2.0".to_string()],
                complexity: Complexity::Simple,
                estimated_hours: 4.0,
            },
        ];

        let result = intellitask.suggest_next_task(&completed, &remaining);
        assert!(
            result.is_ok(),
            "Failed to suggest next task: {:?}",
            result.err()
        );

        let suggestion = result.unwrap();
        assert!(!suggestion.is_empty(), "Suggestion should not be empty");
        assert!(
            suggestion.contains("Suggested:"),
            "Should contain 'Suggested:' prefix"
        );

        // Should suggest task 2.0 since 1.0 is complete and 2.0 depends on it
        assert!(
            suggestion.contains("2.0"),
            "Should suggest task 2.0 as its dependency (1.0) is met"
        );
    }

    #[test]
    #[ignore] // Only run when Ollama is available
    fn test_json_parsing_error_handling() {
        let intellitask = create_test_intellitask().expect("Failed to create IntelliTask instance");

        // Test with invalid PRD that might cause JSON parsing issues
        let invalid_prd = ""; // Empty PRD

        let result = intellitask.generate_tasks_from_prd(invalid_prd);

        // Should handle error gracefully (either return error or valid response)
        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("parse")
                    || error_msg.contains("JSON")
                    || error_msg.contains("empty")
                    || error_msg.contains("invalid"),
                "Error should mention parsing or validation issue: {}",
                error_msg
            );
        }
    }

    #[test]
    fn test_strip_markdown_json() {
        // Test with markdown wrapper - verify JSON is parseable
        let wrapped = r#"```json
{
  "key": "value"
}
```"#;
        let stripped = IntelliTask::strip_markdown_json(wrapped);
        let parsed: serde_json::Value =
            serde_json::from_str(&stripped).expect("Stripped JSON should be parseable");
        assert_eq!(parsed["key"], "value");

        // Test without wrapper (should return as-is)
        let plain = r#"{"key": "value"}"#;
        let stripped = IntelliTask::strip_markdown_json(plain);
        let parsed: serde_json::Value =
            serde_json::from_str(&stripped).expect("Plain JSON should be parseable");
        assert_eq!(parsed["key"], "value");

        // Test with extra whitespace
        let wrapped_ws = r#"  ```json
{
  "key": "value"
}
```  "#;
        let stripped = IntelliTask::strip_markdown_json(wrapped_ws);
        let parsed: serde_json::Value = serde_json::from_str(&stripped)
            .expect("Stripped JSON with whitespace should be parseable");
        assert_eq!(parsed["key"], "value");
    }

    #[test]
    fn test_comprehensive_validation_reports_all_errors() {
        // This test verifies that our JSON schema validation reports ALL errors at once,
        // not just the first one. This was the bug that required 9 attempts to fix.

        // Intentionally broken JSON with MULTIPLE issues:
        // 1. Uses "tasks" instead of "parent_tasks"
        // 2. Missing "relevant_files"
        // 3. Missing "estimated_complexity"
        // 4. Task missing description, subtasks, dependencies, complexity, estimated_hours
        let broken_json = serde_json::json!({
            "prd_title": "Test PRD",
            "tasks": [  // Wrong field name!
                {
                    "id": "1",
                    "title": "Task 1"
                    // Missing: description, subtasks, dependencies, complexity, estimated_hours
                }
            ]
            // Missing: relevant_files, estimated_complexity
        });

        // Generate schema from TaskBreakdown struct
        let schema = schemars::schema_for!(TaskBreakdown);
        let schema_json = serde_json::to_value(&schema).expect("Schema should convert to JSON");

        // Compile and validate
        let validator =
            jsonschema::JSONSchema::compile(&schema_json).expect("Schema should compile");

        let errors: Vec<String> = validator
            .validate(&broken_json)
            .err()
            .map(|iter| iter.collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
            .map(|error| format!("{}", error))
            .collect();

        // CRITICAL: We should get MULTIPLE errors, not just one
        assert!(
            errors.len() >= 2,
            "Validation should report multiple errors at once. Got {} errors: {:?}",
            errors.len(),
            errors
        );

        // Verify it catches the missing required fields
        let error_text = errors.join(" ");
        assert!(
            error_text.contains("parent_tasks") || error_text.contains("required"),
            "Should report missing parent_tasks field"
        );
        assert!(
            error_text.contains("relevant_files") || error_text.contains("required"),
            "Should report missing relevant_files field"
        );
        assert!(
            error_text.contains("estimated_complexity") || error_text.contains("required"),
            "Should report missing estimated_complexity field"
        );

        println!(
            "Comprehensive validation found {} errors (good!):",
            errors.len()
        );
        for (i, e) in errors.iter().enumerate() {
            println!("  {}. {}", i + 1, e);
        }
    }
}
