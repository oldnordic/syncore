//! Failure Classification
//!
//! Implements failure category classification and basic analysis

use super::types::FailureCategory;

/// Failure classifier for categorizing and analyzing failures
#[derive(Debug)]
pub struct FailureClassifier;

impl FailureClassifier {
    /// Create a new failure classifier
    pub fn new() -> Self {
        Self
    }

    /// Classify failure category from error message
    pub fn classify_failure_category(&self, error_message: &str) -> FailureCategory {
        let error_lower = error_message.to_lowercase();

        if error_lower.contains("network")
            || error_lower.contains("connection")
            || error_lower.contains("timeout")
            || error_lower.contains("dns")
        {
            FailureCategory::Network
        } else if error_lower.contains("database")
            || error_lower.contains("sql")
            || error_lower.contains("connection refused")
        {
            FailureCategory::Database
        } else if error_lower.contains("auth")
            || error_lower.contains("unauthorized")
            || error_lower.contains("forbidden")
            || error_lower.contains("login")
        {
            FailureCategory::Authentication
        } else if error_lower.contains("memory")
            || error_lower.contains("cpu")
            || error_lower.contains("disk space")
            || error_lower.contains("resource")
        {
            FailureCategory::Resource
        } else if error_lower.contains("invalid")
            || error_lower.contains("logic")
            || error_lower.contains("algorithm")
            || error_lower.contains("parse")
        {
            FailureCategory::Logic
        } else if error_lower.contains("performance")
            || error_lower.contains("slow")
            || error_lower.contains("timeout")
        {
            FailureCategory::Performance
        } else if error_lower.contains("api")
            || error_lower.contains("service")
            || error_lower.contains("external")
        {
            FailureCategory::ExternalService
        } else {
            FailureCategory::Unknown
        }
    }

    /// Assess failure severity
    pub fn assess_failure_severity(&self, error_message: &str, category: &FailureCategory) -> i32 {
        let base_severity = match category {
            FailureCategory::Resource => 8,
            FailureCategory::Authentication => 7,
            FailureCategory::Database => 6,
            FailureCategory::Network => 5,
            FailureCategory::Performance => 4,
            FailureCategory::ExternalService => 4,
            FailureCategory::Logic => 3,
            FailureCategory::Unknown => 2,
        };

        // Adjust based on error message content
        let error_lower = error_message.to_lowercase();
        let adjustment = if error_lower.contains("critical") || error_lower.contains("fatal") {
            2
        } else if error_lower.contains("warning") {
            -1
        } else {
            0
        };

        // Ensure severity stays within bounds
        (base_severity + adjustment).max(1).min(10)
    }

    /// Assess recoverability
    pub fn assess_recoverability(&self, category: &FailureCategory, severity: i32) -> bool {
        match category {
            FailureCategory::Network => true,
            FailureCategory::Database => severity < 8,
            FailureCategory::Authentication => true,
            FailureCategory::Resource => severity < 9,
            FailureCategory::Performance => true,
            FailureCategory::ExternalService => true,
            FailureCategory::Logic => severity < 7,
            FailureCategory::Unknown => severity < 5,
        }
    }

    /// Estimate recovery time in seconds
    pub fn estimate_recovery_time(&self, category: &FailureCategory, severity: i32) -> i32 {
        let base_time = match category {
            FailureCategory::Network => 30,
            FailureCategory::Database => 10,
            FailureCategory::Authentication => 5,
            FailureCategory::Resource => 300,
            FailureCategory::Performance => 60,
            FailureCategory::ExternalService => 45,
            FailureCategory::Logic => 20,
            FailureCategory::Unknown => 60,
        };

        // Adjust based on severity (higher severity = longer recovery)
        let multiplier = (severity as f64 / 5.0).max(1.0);
        (base_time as f64 * multiplier) as i32
    }
}

impl Default for FailureClassifier {
    fn default() -> Self {
        Self::new()
    }
}