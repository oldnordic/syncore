//! Reasoning Metrics - PHASE ST-9
//!
//! Tree-of-Thought level metrics collection and reporting.
//! Provides comprehensive metrics for reasoning operations, safety events,
//! and performance monitoring integrated with ToTEngine and BranchManager.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// Core reasoning metrics for Tree-of-Thought operations
///
/// Tracks expansion performance, safety events, pruning operations,
/// and tree topology metrics. Thread-safe for concurrent access.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReasoningMetrics {
    /// Total number of node expansions attempted
    pub nodes_expanded_total: u64,

    /// Number of successful node expansions
    pub nodes_expanded_success: u64,

    /// Number of failed node expansions
    pub nodes_expanded_failed: u64,

    /// Total number of nodes pruned
    pub nodes_pruned_total: u64,

    /// Number of nodes pruned due to low scores
    pub pruned_low_score: u64,

    /// Number of nodes pruned due to safety violations
    pub pruned_safety: u64,

    /// Maximum tree depth reached
    pub max_tree_depth: u32,

    /// Maximum tree breadth reached
    pub max_tree_breadth: u32,

    /// Current tree depth
    pub current_depth: u32,

    /// Current tree breadth
    pub current_breadth: u32,

    /// Total reasoning steps taken
    pub reasoning_steps: u64,

    /// Number of LLM failures encountered
    pub llm_failures: u64,

    /// Number of identical expansions detected
    pub identical_expansions_detected: u64,

    /// Total number of safety violations
    pub safety_violations_total: u64,

    /// Total time spent on expansions in milliseconds
    pub total_expansion_time_ms: f64,
}

/// Immutable snapshot of reasoning metrics
///
/// Copy-only value type for safe sharing across threads
/// and MCP response serialization.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReasoningMetricsSnapshot {
    pub nodes_expanded_total: u64,
    pub nodes_expanded_success: u64,
    pub nodes_expanded_failed: u64,
    pub nodes_pruned_total: u64,
    pub pruned_low_score: u64,
    pub pruned_safety: u64,
    pub max_tree_depth: u32,
    pub max_tree_breadth: u32,
    pub current_depth: u32,
    pub current_breadth: u32,
    pub reasoning_steps: u64,
    pub llm_failures: u64,
    pub identical_expansions_detected: u64,
    pub safety_violations_total: u64,
    pub total_expansion_time_ms: f64,
    pub average_expansion_time_ms: f64,
}

impl ReasoningMetrics {
    /// Create new reasoning metrics with all counters at zero
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a node expansion attempt
    ///
    /// Updates expansion counters, depth/breadth tracking, and timing.
    /// Thread-safe for concurrent access.
    pub fn record_expand(&mut self, depth: u32, breadth: u32, ok: bool, duration_ms: f64) {
        // Update expansion counters
        self.nodes_expanded_total += 1;
        if ok {
            self.nodes_expanded_success += 1;
        } else {
            self.nodes_expanded_failed += 1;
        }

        // Update reasoning steps
        self.reasoning_steps += 1;

        // Update depth and breadth tracking
        self.current_depth = depth;
        self.current_breadth = breadth;

        if depth > self.max_tree_depth {
            self.max_tree_depth = depth;
        }

        if breadth > self.max_tree_breadth {
            self.max_tree_breadth = breadth;
        }

        // Update timing
        self.total_expansion_time_ms += duration_ms;
    }

    /// Record pruning due to low scores
    pub fn record_prune_low_score(&mut self, count: u64) {
        self.nodes_pruned_total += count;
        self.pruned_low_score += count;
    }

    /// Record pruning due to safety violations
    pub fn record_prune_safety(&mut self, count: u64) {
        self.nodes_pruned_total += count;
        self.pruned_safety += count;
    }

    /// Record a safety violation
    ///
    /// Tracks both total violations and identical expansions separately.
    pub fn record_safety_violation(&mut self, identical_expansion: bool) {
        self.safety_violations_total += 1;

        if identical_expansion {
            self.identical_expansions_detected += 1;
        }
    }

    /// Record an LLM failure
    pub fn record_llm_failure(&mut self) {
        self.llm_failures += 1;
    }

    /// Calculate average expansion time in milliseconds
    ///
    /// Returns 0.0 if no expansions have been recorded.
    pub fn average_expansion_time_ms(&self) -> f64 {
        if self.nodes_expanded_total == 0 {
            0.0
        } else {
            self.total_expansion_time_ms / self.nodes_expanded_total as f64
        }
    }

    /// Merge another ReasoningMetrics into this one
    ///
    /// Aggregates counters and recalculates derived metrics.
    /// Useful for combining metrics from multiple sessions or time periods.
    pub fn merge(&mut self, other: &ReasoningMetrics) {
        // Aggregate counters
        self.nodes_expanded_total += other.nodes_expanded_total;
        self.nodes_expanded_success += other.nodes_expanded_success;
        self.nodes_expanded_failed += other.nodes_expanded_failed;
        self.nodes_pruned_total += other.nodes_pruned_total;
        self.pruned_low_score += other.pruned_low_score;
        self.pruned_safety += other.pruned_safety;
        self.reasoning_steps += other.reasoning_steps;
        self.llm_failures += other.llm_failures;
        self.identical_expansions_detected += other.identical_expansions_detected;
        self.safety_violations_total += other.safety_violations_total;
        self.total_expansion_time_ms += other.total_expansion_time_ms;

        // Update max values
        self.max_tree_depth = self.max_tree_depth.max(other.max_tree_depth);
        self.max_tree_breadth = self.max_tree_breadth.max(other.max_tree_breadth);

        // Use other's current values as they might be more recent
        self.current_depth = other.current_depth;
        self.current_breadth = other.current_breadth;
    }

    /// Create an immutable snapshot of current metrics
    ///
    /// Returns a copy-only value type safe for serialization
    /// and sharing across threads without locks.
    pub fn snapshot(&self) -> ReasoningMetricsSnapshot {
        ReasoningMetricsSnapshot {
            nodes_expanded_total: self.nodes_expanded_total,
            nodes_expanded_success: self.nodes_expanded_success,
            nodes_expanded_failed: self.nodes_expanded_failed,
            nodes_pruned_total: self.nodes_pruned_total,
            pruned_low_score: self.pruned_low_score,
            pruned_safety: self.pruned_safety,
            max_tree_depth: self.max_tree_depth,
            max_tree_breadth: self.max_tree_breadth,
            current_depth: self.current_depth,
            current_breadth: self.current_breadth,
            reasoning_steps: self.reasoning_steps,
            llm_failures: self.llm_failures,
            identical_expansions_detected: self.identical_expansions_detected,
            safety_violations_total: self.safety_violations_total,
            total_expansion_time_ms: self.total_expansion_time_ms,
            average_expansion_time_ms: self.average_expansion_time_ms(),
        }
    }

    /// Reset all metrics to zero
    ///
    /// Useful for testing or starting fresh measurement periods.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

impl ReasoningMetricsSnapshot {
    /// Create empty snapshot
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any reasoning activity has occurred
    pub fn has_activity(&self) -> bool {
        self.nodes_expanded_total > 0 || self.reasoning_steps > 0
    }

    /// Get success rate as percentage (0.0 to 100.0)
    pub fn success_rate(&self) -> f64 {
        if self.nodes_expanded_total == 0 {
            0.0
        } else {
            (self.nodes_expanded_success as f64 / self.nodes_expanded_total as f64) * 100.0
        }
    }

    /// Get safety violation rate as percentage (0.0 to 100.0)
    pub fn safety_violation_rate(&self) -> f64 {
        if self.reasoning_steps == 0 {
            0.0
        } else {
            (self.safety_violations_total as f64 / self.reasoning_steps as f64) * 100.0
        }
    }
}

/// Thread-safe container for reasoning metrics
pub type ThreadSafeReasoningMetrics = Arc<Mutex<ReasoningMetrics>>;

/// Create new thread-safe reasoning metrics instance
pub fn new_reasoning_metrics() -> ThreadSafeReasoningMetrics {
    Arc::new(Mutex::new(ReasoningMetrics::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reasoning_metrics_default() {
        let metrics = ReasoningMetrics::new();

        assert_eq!(metrics.nodes_expanded_total, 0);
        assert_eq!(metrics.nodes_expanded_success, 0);
        assert_eq!(metrics.nodes_expanded_failed, 0);
        assert_eq!(metrics.max_tree_depth, 0);
        assert_eq!(metrics.max_tree_breadth, 0);
        assert_eq!(metrics.reasoning_steps, 0);
        assert_eq!(metrics.llm_failures, 0);
        assert_eq!(metrics.total_expansion_time_ms, 0.0);
        assert_eq!(metrics.average_expansion_time_ms(), 0.0);
    }

    #[test]
    fn test_reasoning_metrics_snapshot_default() {
        let snapshot = ReasoningMetricsSnapshot::new();

        assert_eq!(snapshot.nodes_expanded_total, 0);
        assert_eq!(snapshot.average_expansion_time_ms, 0.0);
        assert_eq!(snapshot.success_rate(), 0.0);
        assert_eq!(snapshot.safety_violation_rate(), 0.0);
        assert!(!snapshot.has_activity());
    }

    #[test]
    fn test_snapshot_activity_detection() {
        let mut snapshot = ReasoningMetricsSnapshot::new();
        assert!(!snapshot.has_activity());

        snapshot.nodes_expanded_total = 1;
        assert!(snapshot.has_activity());

        snapshot.nodes_expanded_total = 0;
        snapshot.reasoning_steps = 1;
        assert!(snapshot.has_activity());
    }

    #[test]
    fn test_snapshot_rates() {
        let mut snapshot = ReasoningMetricsSnapshot::new();

        // Test success rate
        snapshot.nodes_expanded_total = 10;
        snapshot.nodes_expanded_success = 8;
        assert_eq!(snapshot.success_rate(), 80.0);

        // Test safety violation rate
        snapshot.reasoning_steps = 20;
        snapshot.safety_violations_total = 2;
        assert_eq!(snapshot.safety_violation_rate(), 10.0);

        // Test edge cases
        snapshot.nodes_expanded_total = 0;
        assert_eq!(snapshot.success_rate(), 0.0);

        snapshot.reasoning_steps = 0;
        assert_eq!(snapshot.safety_violation_rate(), 0.0);
    }

    #[test]
    fn test_thread_safe_metrics_creation() {
        let metrics = new_reasoning_metrics();

        // Should be able to lock and access
        let locked = metrics.lock().unwrap();
        assert_eq!(locked.nodes_expanded_total, 0);

        // Should be cloneable
        let metrics_clone = metrics.clone();
        let locked_clone = metrics_clone.lock().unwrap();
        assert_eq!(locked_clone.nodes_expanded_total, 0);
    }

    #[test]
    fn test_metrics_reset() {
        let mut metrics = ReasoningMetrics::new();

        // Add some data
        metrics.record_expand(1, 2, true, 50.0);
        metrics.record_prune_low_score(1);
        metrics.record_safety_violation(true);

        // Verify data exists
        assert_eq!(metrics.nodes_expanded_total, 1);
        assert_eq!(metrics.nodes_pruned_total, 1);
        assert_eq!(metrics.safety_violations_total, 1);

        // Reset
        metrics.reset();

        // Verify reset to defaults
        assert_eq!(metrics.nodes_expanded_total, 0);
        assert_eq!(metrics.nodes_pruned_total, 0);
        assert_eq!(metrics.safety_violations_total, 0);
    }
}
