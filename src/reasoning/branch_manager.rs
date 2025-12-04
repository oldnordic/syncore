//! Branch Manager - PHASE ST-6 Circuit Breaker
//!
//! Safety enforcement layer for Tree-of-Thought reasoning.
//! Implements circuit breaker pattern to prevent infinite loops,
//! excessive branching, and repetitive thought patterns.

use crate::reasoning::{ReasoningError, ReasoningResult};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// Configuration limits for branch management
#[derive(Debug, Clone)]
pub struct BranchLimits {
    pub max_nodes: i64,
    pub max_depth: i64,
    pub max_breadth: i64,
    pub max_identical_expansions: i64,
    pub max_consecutive_errors: i64,
}

impl Default for BranchLimits {
    fn default() -> Self {
        Self {
            max_nodes: 200,
            max_depth: 10,
            max_breadth: 5,
            max_identical_expansions: 3,
            max_consecutive_errors: 5,
        }
    }
}

/// Diagnostics for branch safety monitoring
#[derive(Debug, Clone, Default)]
pub struct BranchDiagnostics {
    pub total_nodes: i64,
    pub depth: i64,
    pub breadth: i64,
    pub identical_expansions: i64,
    pub consecutive_errors: i64,
    pub last_safety_violation: Option<String>,
    pub breaker_status: String,
}

/// Branch Manager for circuit breaker safety enforcement
pub struct BranchManager {
    limits: BranchLimits,
    session_diagnostics: HashMap<String, BranchDiagnostics>,
    content_counts: HashMap<String, HashMap<String, i64>>, // session_id -> content_hash -> count
    parent_child_counts: HashMap<String, i64>,             // parent_id -> child count
    node_depths: HashMap<String, i64>,                     // node_id -> depth
}

impl Default for BranchManager {
    fn default() -> Self {
        Self::new(BranchLimits::default())
    }
}

impl BranchManager {
    /// Create new BranchManager with specified limits
    pub fn new(limits: BranchLimits) -> Self {
        Self {
            limits,
            session_diagnostics: HashMap::new(),
            content_counts: HashMap::new(),
            parent_child_counts: HashMap::new(),
            node_depths: HashMap::new(),
        }
    }

    /// Check safety constraints before expansion
    pub fn check_before_expand(&mut self, session_id: &str, node: &Value) -> ReasoningResult<()> {
        // Get current diagnostics
        let diagnostics = self.get_diagnostics(session_id);

        // Check total node limit
        if diagnostics.total_nodes >= self.limits.max_nodes {
            return Err(ReasoningError::BranchLimitExceeded(format!(
                "Branch limit exceeded: Session {} has {} nodes, limit is {}",
                session_id, diagnostics.total_nodes, self.limits.max_nodes
            )));
        }

        // Check depth limit
        if let Some(parent_id) = node.get("parent_id").and_then(|v| v.as_str()) {
            if !parent_id.is_empty() {
                let depth = self.calculate_depth(session_id, parent_id) + 1;
                if depth > self.limits.max_depth {
                    return Err(ReasoningError::DepthLimitExceeded(format!(
                        "Depth limit exceeded: Node depth {} exceeds limit {}",
                        depth, self.limits.max_depth
                    )));
                }
            }
        }

        // Check breadth limit
        if let Some(parent_id) = node.get("parent_id").and_then(|v| v.as_str()) {
            if !parent_id.is_empty() {
                let child_count = self.parent_child_counts.get(parent_id).unwrap_or(&0);
                if *child_count >= self.limits.max_breadth {
                    return Err(ReasoningError::BreadthLimitExceeded(format!(
                        "Breadth limit exceeded: Parent {} has {} children, limit is {}",
                        parent_id, child_count, self.limits.max_breadth
                    )));
                }
            }
        }

        // Check identical expansion limit
        if self.detect_identical_expansion(session_id, node) {
            return Err(ReasoningError::RepetitiveThoughtPattern(
                "Identical expansion detected".to_string(),
            ));
        }

        // Check loop detection
        if self.detect_loop_via_hash(session_id, node) {
            return Err(ReasoningError::LoopDetected("Loop detected".to_string()));
        }

        // Check consecutive error limit
        if diagnostics.consecutive_errors >= self.limits.max_consecutive_errors {
            return Err(ReasoningError::TooManyErrors(format!(
                "Too many consecutive errors: {}",
                diagnostics.consecutive_errors
            )));
        }

        Ok(())
    }

    /// Record successful expansion
    pub fn record_success(&mut self, session_id: &str, node: &Value) -> ReasoningResult<()> {
        let diagnostics = self.get_diagnostics_mut(session_id);

        // Update counters
        diagnostics.total_nodes += 1;
        diagnostics.consecutive_errors = 0; // Reset on success
        diagnostics.breaker_status = "active".to_string();
        diagnostics.last_safety_violation = None;

        // Track content hash for identical expansion detection
        if let Some(content) = node.get("content").and_then(|v| v.as_str()) {
            // Extract base content by removing common suffixes like " - 0", " - 1", etc.
            let base_content = content.split(" - ").next().unwrap_or(content);
            let content_hash = self.hash_content(base_content);
            let counts =
                self.content_counts.entry(session_id.to_string()).or_insert_with(HashMap::new);
            *counts.entry(content_hash).or_insert(0) += 1;
        }

        // Track parent-child relationships for breadth calculation
        if let Some(parent_id) = node.get("parent_id").and_then(|v| v.as_str()) {
            if !parent_id.is_empty() {
                *self.parent_child_counts.entry(parent_id.to_string()).or_insert(0) += 1;
            }
        }

        // Track node depth
        if let Some(node_id) = node.get("id").and_then(|v| v.as_str()) {
            let depth = if let Some(parent_id) = node.get("parent_id").and_then(|v| v.as_str()) {
                if !parent_id.is_empty() {
                    self.node_depths.get(parent_id).unwrap_or(&0) + 1
                } else {
                    0
                }
            } else {
                0
            };
            self.node_depths.insert(node_id.to_string(), depth);
        }

        Ok(())
    }

    /// Record failed expansion
    pub fn record_failure(
        &mut self,
        session_id: &str,
        _node: &Value,
        error: &str,
    ) -> ReasoningResult<()> {
        let diagnostics = self.get_diagnostics_mut(session_id);

        diagnostics.consecutive_errors += 1;
        diagnostics.breaker_status = "warning".to_string();
        diagnostics.last_safety_violation = Some(error.to_string());

        Ok(())
    }

    /// Get diagnostics for a session
    pub fn get_diagnostics(&self, session_id: &str) -> BranchDiagnostics {
        self.session_diagnostics.get(session_id).cloned().unwrap_or_default()
    }

    /// Get mutable diagnostics for a session
    fn get_diagnostics_mut(&mut self, session_id: &str) -> &mut BranchDiagnostics {
        self.session_diagnostics
            .entry(session_id.to_string())
            .or_insert_with(BranchDiagnostics::default)
    }

    /// Detect identical expansion via content hash
    fn detect_identical_expansion(&self, session_id: &str, node: &Value) -> bool {
        if let Some(content) = node.get("content").and_then(|v| v.as_str()) {
            // Extract base content by removing common suffixes like " - 0", " - 1", etc.
            let base_content = content.split(" - ").next().unwrap_or(content);
            let content_hash = self.hash_content(base_content);
            if let Some(counts) = self.content_counts.get(session_id) {
                // Get count of this base content hash
                let count = counts.get(&content_hash).unwrap_or(&0);
                return *count >= self.limits.max_identical_expansions;
            }
        }
        false
    }

    /// Detect loops via content hash comparison
    fn detect_loop_via_hash(&self, session_id: &str, node: &Value) -> bool {
        if let Some(content) = node.get("content").and_then(|v| v.as_str()) {
            // Extract base content by removing common suffixes like " - 0", " - 1", etc.
            let base_content = content.split(" - ").next().unwrap_or(content);
            let content_hash = self.hash_content(base_content);
            if let Some(counts) = self.content_counts.get(session_id) {
                return counts.contains_key(&content_hash);
            }
        }
        false
    }

    /// Calculate depth of a node by traversing up the tree
    fn calculate_depth(&self, _session_id: &str, parent_id: &str) -> i64 {
        // Get depth of parent node
        self.node_depths.get(parent_id).unwrap_or(&0) + 1
    }

    /// Hash content for comparison
    fn hash_content(&self, content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Export safety counters for metrics integration
    ///
    /// Returns (identical_expansions, total_safety_violations) for the current session.
    /// For now, returns global counters since BranchManager tracks per-session.
    pub fn export_safety_counters(&self) -> (u64, u64) {
        // Calculate total identical expansions across all sessions
        let total_identical: u64 = self
            .content_counts
            .values()
            .flat_map(|counts| counts.values())
            .filter(|&&count| count >= self.limits.max_identical_expansions)
            .count() as u64;

        // Calculate total safety violations from diagnostics
        let total_violations: u64 = self
            .session_diagnostics
            .values()
            .map(|d| {
                let mut violations = 0;
                if d.consecutive_errors > 0 {
                    violations += d.consecutive_errors as u64;
                }
                if d.last_safety_violation.is_some() {
                    violations += 1;
                }
                violations
            })
            .sum();

        (total_identical, total_violations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_branch_limits_default() {
        let limits = BranchLimits::default();
        assert_eq!(limits.max_nodes, 200);
        assert_eq!(limits.max_depth, 10);
        assert_eq!(limits.max_breadth, 5);
        assert_eq!(limits.max_identical_expansions, 3);
        assert_eq!(limits.max_consecutive_errors, 5);
    }

    #[test]
    fn test_branch_manager_creation() {
        let limits = BranchLimits::default();
        let manager = BranchManager::new(limits);
        assert_eq!(manager.limits.max_nodes, 200);
    }

    #[test]
    fn test_content_hashing() {
        let manager = BranchManager::new(BranchLimits::default());
        let hash1 = manager.hash_content("test content");
        let hash2 = manager.hash_content("test content");
        let hash3 = manager.hash_content("different content");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_diagnostics_default() {
        let diagnostics = BranchDiagnostics::default();
        assert_eq!(diagnostics.total_nodes, 0);
        assert_eq!(diagnostics.depth, 0);
        assert_eq!(diagnostics.breadth, 0);
        assert_eq!(diagnostics.identical_expansions, 0);
        assert_eq!(diagnostics.consecutive_errors, 0);
        assert!(diagnostics.last_safety_violation.is_none());
        assert_eq!(diagnostics.breaker_status, "");
    }
}
