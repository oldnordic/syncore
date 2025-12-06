//! Reflection Engine Implementation
//!
//! Main reflection engine with failure analysis and pattern detection

use crate::agent::{ApreError, ApreResult};
use crate::memory::Memory;
use crate::raggraph::HopGraphTransformer;
use crate::reasoning::ToTEngine;
use serde_json;
use std::sync::Arc;

use super::analysis::FailureAnalysisEngine;
use super::types::{ReflectionReport, FailureAnalysis, RetryPlan, EmergentBehavior};

/// Reflection Engine for analyzing failures and generating insights
#[derive(Debug)]
pub struct ReflectionEngine {
    /// Memory service for storing reflections
    memory: Arc<Memory>,

    /// Graph transformer for semantic reasoning
    hop_graph: HopGraphTransformer,

    /// Reasoning engine for generating insights
    reasoning_engine: Arc<ToTEngine>,

    /// Analysis engine for failure classification and analysis
    analysis_engine: FailureAnalysisEngine,
}

impl ReflectionEngine {
    /// Create a new reflection engine
    pub fn new(
        memory: Arc<Memory>,
        hop_graph: HopGraphTransformer,
        reasoning_engine: Arc<ToTEngine>,
    ) -> Self {
        Self {
            memory,
            hop_graph,
            reasoning_engine,
            analysis_engine: FailureAnalysisEngine::new(),
        }
    }

    /// Analyze a failure and generate reflection report
    pub async fn analyze_failure(
        &self,
        action_description: &str,
        error_message: &str,
        context: &str,
    ) -> ApreResult<ReflectionReport> {
        // Extract plan ID from context
        let plan_id = self.extract_plan_id(context);

        // Perform detailed failure analysis
        let failure_analysis = self.perform_failure_analysis(action_description, error_message, context)?;

        let failure_detected = true;

        // Generate insights
        let insights = self.generate_insights(&failure_analysis, context).await?;

        // Create reflection report
        let mut report = ReflectionReport::new(plan_id);
        report.failure_detected = failure_detected;
        report.failure_analysis = Some(failure_analysis.clone());
        report.action_description = action_description.to_string();
        report.error_summary = error_message.to_string();
        report.failure_category = Some(format!("{:?}", failure_analysis.category));
        report.root_causes = failure_analysis.root_causes.iter().map(|rc| rc.description.clone()).collect();
        report.recovery_actions = failure_analysis.recovery_actions.iter().map(|ra| ra.action.clone()).collect();
        report.insights = insights;
        report.summary = format!("Analysis of failure in '{}'", action_description);
        report.recommendations = self.generate_recommendations(&failure_analysis);

        // Store reflection
        self.store_reflection(&report).await?;

        Ok(report)
    }

    /// Store reflection report in memory
    pub async fn store_reflection(&self, report: &ReflectionReport) -> ApreResult<()> {
        let report_json = serde_json::to_string(report).map_err(|e| {
            ApreError::MemoryError(anyhow::anyhow!("Failed to serialize reflection: {}", e))
        })?;

        let key = format!("reflection:{}", report.id);
        self.memory.store(&key, &report_json).map_err(|e| ApreError::MemoryError(e))?;

        // Also store by plan ID for retrieval
        let plan_key = format!("plan_reflections:{}", report.plan_id);
        let existing_reflections = self.memory.query(&plan_key).unwrap_or(None).unwrap_or_default();
        let mut reflections: Vec<String> = serde_json::from_str(&existing_reflections).unwrap_or_default();
        reflections.push(report.id.clone());

        let reflections_json = serde_json::to_string(&reflections).map_err(|e| {
            ApreError::MemoryError(anyhow::anyhow!("Failed to serialize reflection list: {}", e))
        })?;

        self.memory.store(&plan_key, &reflections_json).map_err(|e| ApreError::MemoryError(e))?;

        Ok(())
    }

    /// Detect infinite loops from repetitive failure patterns
    pub async fn detect_infinite_loop(
        &self,
        action: &str,
        context: &str,
    ) -> ApreResult<bool> {
        // Query recent reflections for this plan/action
        let recent_reflections = self.query_recent_reflections(10).await?;

        // Count occurrences of the same action
        let action_count = recent_reflections.iter()
            .filter(|r| r.action_description.contains(action))
            .count();

        // Detect thrashing behavior
        let thrashing_detected = self.detect_thrashing_behavior(&recent_reflections);

        Ok(action_count > 5 || thrashing_detected)
    }

    /// Generate retry plan from reflection
    pub fn generate_retry_plan(&self, report: &ReflectionReport) -> Option<RetryPlan> {
        let failure_analysis = report.failure_analysis.as_ref()?;

        // Only generate retry plan if failure is recoverable
        if !failure_analysis.is_recoverable {
            return None;
        }

        let retry_plan = RetryPlan {
            id: format!("retry_{}", report.id),
            max_retries: 3,
            initial_delay_ms: 1000,
            backoff_multiplier: 2.0,
            max_delay_ms: 30000,
            retry_actions: failure_analysis.recovery_actions.iter()
                .filter(|ra| ra.priority >= 6)
                .map(|ra| ra.action.clone())
                .collect(),
            abort_conditions: vec![
                "Max retries exceeded".to_string(),
                "Severity > 8".to_string(),
                "Resource exhausted".to_string(),
            ],
            created_at: crate::agent::current_timestamp_ms(),
        };

        Some(retry_plan)
    }

    /// Analyze emergent behaviors from reflection history
    pub async fn analyze_emergent_behaviors(&self) -> ApreResult<Vec<EmergentBehavior>> {
        let behaviors = self.analyze_emergent_behaviors_internal().await?;
        Ok(behaviors)
    }

    /// Perform detailed failure analysis
    fn perform_failure_analysis(
        &self,
        action_description: &str,
        error_message: &str,
        context: &str,
    ) -> ApreResult<FailureAnalysis> {
        let analysis = self.analysis_engine.analyze_failure(action_description, error_message, context);
        Ok(analysis)
    }

    /// Generate insights using reasoning engine
    async fn generate_insights(
        &self,
        failure_analysis: &FailureAnalysis,
        _context: &str,
    ) -> ApreResult<Vec<String>> {
        let mut insights = Vec::new();

        // Add basic insights based on analysis
        insights.push(format!(
            "Failure category: {:?}",
            failure_analysis.category
        ));
        insights.push(format!(
            "Severity level: {}",
            failure_analysis.severity
        ));
        insights.push(format!(
            "Recoverable: {}",
            failure_analysis.is_recoverable
        ));

        // In a real implementation, this would use the reasoning engine
        // For now, add some basic reasoning insights
        if failure_analysis.severity > 7 {
            insights.push("High severity failure requires immediate attention".to_string());
        }

        if !failure_analysis.is_recoverable {
            insights.push("Failure may require manual intervention".to_string());
        }

        if failure_analysis.root_causes.len() > 2 {
            insights.push("Multiple root causes identified - comprehensive review needed".to_string());
        }

        Ok(insights)
    }

    /// Extract plan ID from context
    fn extract_plan_id(&self, context: &str) -> String {
        // Simple extraction - look for plan: prefix
        if let Some(start) = context.find("plan:") {
            let start_idx = start + 5; // Skip "plan:"
            if let Some(end) = context[start_idx..].find(char::is_whitespace) {
                context[start_idx..start_idx + end].to_string()
            } else {
                context[start_idx..].to_string()
            }
        } else {
            "unknown_plan".to_string()
        }
    }

    /// Generate recommendations from failure analysis
    fn generate_recommendations(&self, failure_analysis: &FailureAnalysis) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Add recommendations from recovery actions
        for action in &failure_analysis.recovery_actions {
            if action.priority >= 7 {
                recommendations.push(action.action.clone());
            }
        }

        // Add category-specific recommendations
        match failure_analysis.category {
            super::types::FailureCategory::Network => {
                recommendations.push("Implement circuit breaker pattern".to_string());
                recommendations.push("Add network monitoring and alerts".to_string());
            }
            super::types::FailureCategory::Database => {
                recommendations.push("Review database connection pooling".to_string());
                recommendations.push("Implement retry with transaction handling".to_string());
            }
            super::types::FailureCategory::Resource => {
                recommendations.push("Monitor resource usage metrics".to_string());
                recommendations.push("Implement resource usage limits".to_string());
            }
            _ => {
                recommendations.push("Review error handling and logging".to_string());
            }
        }

        recommendations
    }

    /// Query recent reflections from memory
    async fn query_recent_reflections(&self, limit: usize) -> ApreResult<Vec<ReflectionReport>> {
        // In a real implementation, this would query memory with proper pagination
        // For now, return empty list
        Ok(Vec::new())
    }

    /// Detect thrashing behavior from reflection patterns
    fn detect_thrashing_behavior(&self, _reflections: &[ReflectionReport]) -> bool {
        // In a real implementation, this would analyze patterns
        // For now, return false
        false
    }

    /// Internal emergent behavior analysis
    async fn analyze_emergent_behaviors_internal(&self) -> ApreResult<Vec<EmergentBehavior>> {
        // In a real implementation, this would analyze reflection history
        // For now, return empty list
        Ok(Vec::new())
    }
}