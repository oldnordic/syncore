//! Failure Analysis
//!
//! Implements root cause analysis and recovery action generation

use super::classifier::FailureClassifier;
use super::types::{FailureCategory, RootCause, RecoveryAction, FailureAnalysis};

/// Failure analysis engine for generating root causes and recovery actions
#[derive(Debug)]
pub struct FailureAnalysisEngine {
    classifier: FailureClassifier,
}

impl FailureAnalysisEngine {
    /// Create a new failure analysis engine
    pub fn new() -> Self {
        Self {
            classifier: FailureClassifier::new(),
        }
    }

    /// Perform comprehensive failure analysis
    pub fn analyze_failure(
        &self,
        action_description: &str,
        error_message: &str,
        _context: &str,
    ) -> FailureAnalysis {
        // Classify the failure category
        let category = self.classifier.classify_failure_category(error_message);

        // Generate root causes
        let root_causes = self.generate_root_causes(action_description, error_message, &category);

        // Generate recovery actions
        let recovery_actions = self.generate_recovery_actions(&category, &root_causes);

        // Assess severity
        let severity = self.classifier.assess_failure_severity(error_message, &category);

        // Determine recoverability
        let is_recoverable = self.classifier.assess_recoverability(&category, severity);

        // Estimate recovery time
        let recovery_time = self.classifier.estimate_recovery_time(&category, severity);

        FailureAnalysis {
            original_action: action_description.to_string(),
            error_message: error_message.to_string(),
            category,
            root_causes,
            recovery_actions,
            severity,
            is_recoverable,
            estimated_recovery_time: recovery_time,
            timestamp: crate::agent::current_timestamp_ms(),
        }
    }

    /// Generate root causes for failure
    fn generate_root_causes(
        &self,
        _action: &str,
        error_message: &str,
        category: &FailureCategory,
    ) -> Vec<RootCause> {
        let mut causes = Vec::new();

        match category {
            FailureCategory::Network => {
                causes.push(RootCause {
                    description: "Network connectivity issue".to_string(),
                    confidence: 0.8,
                    evidence: vec![error_message.to_string()],
                    recommendations: vec![
                        "Check network connection".to_string(),
                        "Verify endpoint availability".to_string(),
                        "Implement retry logic".to_string(),
                    ],
                });
            }
            FailureCategory::Database => {
                causes.push(RootCause {
                    description: "Database operation failure".to_string(),
                    confidence: 0.9,
                    evidence: vec![error_message.to_string()],
                    recommendations: vec![
                        "Check database connection".to_string(),
                        "Verify query syntax".to_string(),
                        "Handle transaction conflicts".to_string(),
                    ],
                });
            }
            FailureCategory::Authentication => {
                causes.push(RootCause {
                    description: "Authentication/authorization failure".to_string(),
                    confidence: 0.95,
                    evidence: vec![error_message.to_string()],
                    recommendations: vec![
                        "Verify credentials".to_string(),
                        "Check token validity".to_string(),
                        "Review permission settings".to_string(),
                    ],
                });
            }
            FailureCategory::Resource => {
                causes.push(RootCause {
                    description: "Resource constraint or exhaustion".to_string(),
                    confidence: 0.85,
                    evidence: vec![error_message.to_string()],
                    recommendations: vec![
                        "Monitor resource usage".to_string(),
                        "Implement resource limits".to_string(),
                        "Scale resources if needed".to_string(),
                    ],
                });
            }
            FailureCategory::Logic => {
                causes.push(RootCause {
                    description: "Logic error or invalid input".to_string(),
                    confidence: 0.9,
                    evidence: vec![error_message.to_string()],
                    recommendations: vec![
                        "Review algorithm logic".to_string(),
                        "Add input validation".to_string(),
                        "Improve error handling".to_string(),
                    ],
                });
            }
            FailureCategory::Performance => {
                causes.push(RootCause {
                    description: "Performance degradation".to_string(),
                    confidence: 0.75,
                    evidence: vec![error_message.to_string()],
                    recommendations: vec![
                        "Optimize algorithms".to_string(),
                        "Add caching".to_string(),
                        "Consider parallel processing".to_string(),
                    ],
                });
            }
            FailureCategory::ExternalService => {
                causes.push(RootCause {
                    description: "External service dependency failure".to_string(),
                    confidence: 0.8,
                    evidence: vec![error_message.to_string()],
                    recommendations: vec![
                        "Check service availability".to_string(),
                        "Implement circuit breaker".to_string(),
                        "Add fallback mechanisms".to_string(),
                    ],
                });
            }
            FailureCategory::Unknown => {
                causes.push(RootCause {
                    description: "Uncategorized failure".to_string(),
                    confidence: 0.5,
                    evidence: vec![error_message.to_string()],
                    recommendations: vec![
                        "Investigate error details".to_string(),
                        "Add logging for debugging".to_string(),
                        "Review error handling patterns".to_string(),
                    ],
                });
            }
        }

        causes
    }

    /// Generate recovery actions
    fn generate_recovery_actions(
        &self,
        category: &FailureCategory,
        _root_causes: &[RootCause],
    ) -> Vec<RecoveryAction> {
        let mut actions = Vec::new();

        match category {
            FailureCategory::Network => {
                actions.push(RecoveryAction {
                    action: "Retry with exponential backoff".to_string(),
                    priority: 8,
                    success_probability: 0.7,
                    resources: vec!["Network connection".to_string()],
                    prerequisites: vec!["Network connectivity".to_string()],
                });
                actions.push(RecoveryAction {
                    action: "Switch to backup endpoint".to_string(),
                    priority: 6,
                    success_probability: 0.6,
                    resources: vec!["Backup endpoint".to_string()],
                    prerequisites: vec!["Backup configuration".to_string()],
                });
            }
            FailureCategory::Database => {
                actions.push(RecoveryAction {
                    action: "Retry database operation".to_string(),
                    priority: 7,
                    success_probability: 0.8,
                    resources: vec!["Database connection".to_string()],
                    prerequisites: vec!["Database availability".to_string()],
                });
                actions.push(RecoveryAction {
                    action: "Use cached data".to_string(),
                    priority: 5,
                    success_probability: 0.5,
                    resources: vec!["Data cache".to_string()],
                    prerequisites: vec!["Cache availability".to_string()],
                });
            }
            FailureCategory::Authentication => {
                actions.push(RecoveryAction {
                    action: "Refresh authentication token".to_string(),
                    priority: 9,
                    success_probability: 0.9,
                    resources: vec!["Token refresh endpoint".to_string()],
                    prerequisites: vec!["Valid refresh token".to_string()],
                });
            }
            FailureCategory::Resource => {
                actions.push(RecoveryAction {
                    action: "Free up resources".to_string(),
                    priority: 7,
                    success_probability: 0.6,
                    resources: vec!["Memory management".to_string()],
                    prerequisites: vec!["Resource access permissions".to_string()],
                });
            }
            FailureCategory::Logic => {
                actions.push(RecoveryAction {
                    action: "Skip invalid operation".to_string(),
                    priority: 6,
                    success_probability: 0.8,
                    resources: vec![],
                    prerequisites: vec!["Graceful degradation".to_string()],
                });
            }
            FailureCategory::Performance => {
                actions.push(RecoveryAction {
                    action: "Reduce operation scope".to_string(),
                    priority: 5,
                    success_probability: 0.7,
                    resources: vec![],
                    prerequisites: vec!["Configurable parameters".to_string()],
                });
            }
            FailureCategory::ExternalService => {
                actions.push(RecoveryAction {
                    action: "Use cached results".to_string(),
                    priority: 7,
                    success_probability: 0.6,
                    resources: vec!["Response cache".to_string()],
                    prerequisites: vec!["Recent cache data".to_string()],
                });
            }
            FailureCategory::Unknown => {
                actions.push(RecoveryAction {
                    action: "Log and continue".to_string(),
                    priority: 4,
                    success_probability: 0.5,
                    resources: vec!["Logging system".to_string()],
                    prerequisites: vec![],
                });
            }
        }

        actions
    }

    /// Get reference to the classifier
    pub fn classifier(&self) -> &FailureClassifier {
        &self.classifier
    }
}

impl Default for FailureAnalysisEngine {
    fn default() -> Self {
        Self::new()
    }
}