//! Code Graph Refactoring Suggestion Engine
//! Analyzes CodeGraphStore to detect code smells and generate refactoring plans.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use super::code_graph_store::CodeGraphStore;

/// Long function detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongFunction {
    pub name: String,
    pub lines: usize,
    pub suggestion: String,
}

/// Dead code detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadCode {
    pub name: String,
    pub kind: String,
    pub reason: String,
}

/// Duplicate function detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateFunction {
    pub function1: String,
    pub function2: String,
    pub similarity: f32,
}

/// Refactoring step in a plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactoringStep {
    pub step_number: usize,
    pub action: String,
    pub description: String,
    pub files_affected: Vec<String>,
}

/// Complete refactoring plan for a symbol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactoringPlan {
    pub priority: String,
    pub steps: Vec<RefactoringStep>,
    pub estimated_effort: String,
    pub risks: Vec<String>,
}

/// Comprehensive refactoring check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactoringCheckResult {
    pub long_functions: Vec<LongFunction>,
    pub dead_code: Vec<DeadCode>,
    pub duplicate_functions: Vec<DuplicateFunction>,
    pub total_issues: usize,
}

/// Refactoring Suggestion Engine
pub struct RefactoringSuggestionEngine<'a> {
    store: &'a CodeGraphStore,
}

impl<'a> RefactoringSuggestionEngine<'a> {
    /// Create a new engine with a reference to the code graph store
    pub fn new(store: &'a CodeGraphStore) -> Self {
        Self { store }
    }

    /// Detect functions that exceed the maximum line threshold
    pub fn detect_long_functions(&self, max_lines: usize) -> Result<Vec<LongFunction>> {
        let functions = self.store.get_all_functions()?;
        let mut long_functions = Vec::new();

        for func in functions {
            let line_count = func.line_end.saturating_sub(func.line_start) + 1;
            if line_count > max_lines {
                let suggestion = if line_count > max_lines * 3 {
                    format!(
                        "Critical: Function has {} lines ({}x over limit). Consider extracting multiple helper functions.",
                        line_count,
                        line_count / max_lines
                    )
                } else if line_count > max_lines * 2 {
                    format!(
                        "High priority: Function has {} lines. Extract logical sections into separate functions.",
                        line_count
                    )
                } else {
                    format!(
                        "Function has {} lines (limit: {}). Consider splitting into smaller functions.",
                        line_count, max_lines
                    )
                };

                long_functions.push(LongFunction {
                    name: func.name.clone(),
                    lines: line_count,
                    suggestion,
                });
            }
        }

        // Sort by line count descending
        long_functions.sort_by(|a, b| b.lines.cmp(&a.lines));
        Ok(long_functions)
    }

    /// Detect dead code (functions with no callers and not public)
    pub fn detect_dead_code(&self) -> Result<Vec<DeadCode>> {
        let functions = self.store.get_all_functions()?;
        let mut dead_code = Vec::new();

        // Build a set of all called functions
        let mut called_functions: HashSet<String> = HashSet::new();
        for func in &functions {
            let callees = self.store.get_callees(&func.name)?;
            for callee in callees {
                called_functions.insert(callee);
            }
        }

        // Check each function
        for func in &functions {
            // Skip public functions (they may be called externally)
            if func.is_public {
                continue;
            }

            // Skip main entry points
            if func.name == "main" || func.name == "new" {
                continue;
            }

            let callers = self.store.get_callers(&func.name)?;
            if callers.is_empty() && !called_functions.contains(&func.name) {
                dead_code.push(DeadCode {
                    name: func.name.clone(),
                    kind: "function".to_string(),
                    reason: "No callers found and not public".to_string(),
                });
            }
        }

        Ok(dead_code)
    }

    /// Detect duplicate functions based on semantic similarity
    pub fn detect_duplicate_functions(
        &self,
        similarity_threshold: f32,
    ) -> Result<Vec<DuplicateFunction>> {
        let functions = self.store.get_all_functions()?;
        let mut duplicates = Vec::new();
        let mut checked_pairs: HashSet<(String, String)> = HashSet::new();

        for func in &functions {
            // Search for similar functions
            let similar = self.store.search_similar_functions(&func.name, 10)?;

            // Estimate similarity based on ranking (closer = higher similarity)
            for (index, similar_name) in similar.iter().enumerate() {
                // Skip self-comparison
                if similar_name == &func.name {
                    continue;
                }

                // Estimate similarity (1.0 for first, decreasing)
                let estimated_similarity = 1.0 - (index as f32 * 0.05);

                // Skip if below threshold
                if estimated_similarity < similarity_threshold {
                    continue;
                }

                // Create ordered pair to avoid duplicates
                let pair = if func.name < *similar_name {
                    (func.name.clone(), similar_name.clone())
                } else {
                    (similar_name.clone(), func.name.clone())
                };

                // Skip if already checked
                if checked_pairs.contains(&pair) {
                    continue;
                }

                checked_pairs.insert(pair.clone());
                duplicates.push(DuplicateFunction {
                    function1: pair.0,
                    function2: pair.1,
                    similarity: estimated_similarity,
                });
            }
        }

        // Sort by similarity descending
        duplicates.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(duplicates)
    }

    /// Run comprehensive refactoring check
    pub fn check_all(
        &self,
        max_lines: usize,
        similarity_threshold: f32,
    ) -> Result<RefactoringCheckResult> {
        let long_functions = self.detect_long_functions(max_lines)?;
        let dead_code = self.detect_dead_code()?;
        let duplicate_functions = self.detect_duplicate_functions(similarity_threshold)?;

        let total_issues = long_functions.len() + dead_code.len() + duplicate_functions.len();

        Ok(RefactoringCheckResult {
            long_functions,
            dead_code,
            duplicate_functions,
            total_issues,
        })
    }

    /// Generate a detailed refactoring plan for a function
    pub fn suggest_refactor_plan(&self, function_name: &str) -> Result<RefactoringPlan> {
        let functions = self.store.get_all_functions()?;
        let target_func = functions
            .iter()
            .find(|f| f.name == function_name)
            .ok_or_else(|| anyhow!("Function '{}' not found", function_name))?;

        let callers = self.store.get_callers(function_name)?;
        let callees = self.store.get_callees(function_name)?;
        let line_count = target_func.line_end.saturating_sub(target_func.line_start) + 1;

        // Determine priority based on multiple factors
        let priority =
            self.calculate_refactor_priority(line_count, callers.len(), target_func.is_public);

        // Build refactoring steps
        let mut steps = Vec::new();
        let mut step_number = 1;

        // Step 1: Analyze dependencies
        steps.push(RefactoringStep {
            step_number,
            action: "analyze_dependencies".to_string(),
            description: format!(
                "Analyze {} callers and {} callees to understand function context",
                callers.len(),
                callees.len()
            ),
            files_affected: vec![target_func.qualified_path.clone()],
        });
        step_number += 1;

        // Step 2: Create tests if needed
        if target_func.is_public {
            steps.push(RefactoringStep {
                step_number,
                action: "ensure_test_coverage".to_string(),
                description:
                    "Ensure comprehensive test coverage before refactoring public function"
                        .to_string(),
                files_affected: vec![format!("tests/{}_test.rs", function_name)],
            });
            step_number += 1;
        }

        // Step 3: Extract helper functions if long
        if line_count > 50 {
            let num_helpers = (line_count / 25).max(2);
            steps.push(RefactoringStep {
                step_number,
                action: "extract_helpers".to_string(),
                description: format!(
                    "Extract approximately {} helper functions to reduce complexity",
                    num_helpers
                ),
                files_affected: vec![target_func.qualified_path.clone()],
            });
            step_number += 1;
        }

        // Step 4: Update callers if signature changes
        if !callers.is_empty() {
            steps.push(RefactoringStep {
                step_number,
                action: "update_callers".to_string(),
                description: format!(
                    "Update {} calling sites if signature changes",
                    callers.len()
                ),
                files_affected: callers.clone(),
            });
            step_number += 1;
        }

        // Step 5: Verify
        steps.push(RefactoringStep {
            step_number,
            action: "verify_refactoring".to_string(),
            description: "Run all tests and verify behavior preservation".to_string(),
            files_affected: vec!["tests/".to_string()],
        });

        // Calculate effort
        let estimated_effort = self.estimate_effort(line_count, callers.len());

        // Identify risks
        let risks = self.identify_risks(target_func.is_public, callers.len(), line_count);

        Ok(RefactoringPlan {
            priority,
            steps,
            estimated_effort,
            risks,
        })
    }

    fn calculate_refactor_priority(
        &self,
        line_count: usize,
        caller_count: usize,
        is_public: bool,
    ) -> String {
        let mut score = 0;

        // Line count impact
        if line_count > 200 {
            score += 40;
        } else if line_count > 100 {
            score += 35;
        } else if line_count > 50 {
            score += 20;
        }

        // Caller count impact (more callers = higher risk but also higher value)
        if caller_count > 10 {
            score += 30;
        } else if caller_count > 5 {
            score += 25;
        } else if caller_count > 0 {
            score += 10;
        }

        // Public API impact
        if is_public {
            score += 20;
        }

        match score {
            0..=25 => "low".to_string(),
            26..=50 => "medium".to_string(),
            51..=75 => "high".to_string(),
            _ => "critical".to_string(),
        }
    }

    fn estimate_effort(&self, line_count: usize, caller_count: usize) -> String {
        let base_hours = (line_count as f32 / 50.0).ceil() as usize;
        let caller_hours = (caller_count as f32 / 5.0).ceil() as usize;
        let total_hours = base_hours + caller_hours;

        if total_hours <= 2 {
            "Small (~2 hours)".to_string()
        } else if total_hours <= 8 {
            "Medium (~1 day)".to_string()
        } else if total_hours <= 16 {
            "Large (~3 days)".to_string()
        } else {
            format!("Very Large (~{} days)", total_hours / 8)
        }
    }

    fn identify_risks(
        &self,
        is_public: bool,
        caller_count: usize,
        line_count: usize,
    ) -> Vec<String> {
        let mut risks = Vec::new();

        if is_public {
            risks.push("Public API: Changes may break external consumers".to_string());
        }

        if caller_count > 10 {
            risks.push(format!(
                "High coupling: {} callers may be affected",
                caller_count
            ));
        }

        if line_count > 100 {
            risks.push(
                "Complex function: High risk of introducing bugs during refactoring".to_string(),
            );
        }

        if caller_count == 0 && !is_public {
            risks
                .push("Potentially dead code: Consider removal instead of refactoring".to_string());
        }

        if risks.is_empty() {
            risks.push("Low risk refactoring".to_string());
        }

        risks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portfolio::code_graph_extractor::{CodeGraph, FunctionNode};
    use tempfile::TempDir;

    #[test]
    fn test_refactoring_priority_calculation() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let vectors_dir = temp_dir.path().join("vectors");

        let store =
            CodeGraphStore::new_with_paths(&db_path, &vectors_dir).expect("Should create store");
        let engine = RefactoringSuggestionEngine::new(&store);

        // Test low priority
        let priority = engine.calculate_refactor_priority(30, 1, false);
        assert_eq!(priority, "low");

        // Test medium priority
        let priority = engine.calculate_refactor_priority(60, 3, false);
        assert_eq!(priority, "medium");

        // Test high priority
        let priority = engine.calculate_refactor_priority(120, 7, false);
        assert_eq!(priority, "high");

        // Test critical priority
        let priority = engine.calculate_refactor_priority(250, 15, true);
        assert_eq!(priority, "critical");
    }

    #[test]
    fn test_effort_estimation() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let vectors_dir = temp_dir.path().join("vectors");

        let store =
            CodeGraphStore::new_with_paths(&db_path, &vectors_dir).expect("Should create store");
        let engine = RefactoringSuggestionEngine::new(&store);

        let effort = engine.estimate_effort(25, 2);
        assert!(effort.contains("Small"));

        let effort = engine.estimate_effort(100, 10);
        assert!(effort.contains("Medium") || effort.contains("Large"));

        let effort = engine.estimate_effort(500, 50);
        assert!(effort.contains("Very Large"));
    }
}
