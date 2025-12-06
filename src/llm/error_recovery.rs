//! Candle Error Containment and Safe Recovery
//!
//! Provides robust error handling and recovery mechanisms for Candle inference
//! operations. Ensures that Candle failures are gracefully contained and don't
//! crash the entire system.
//!
//! ## Features
//!
//! - Circuit breaker pattern for repeated failures
//! - Exponential backoff with jitter
//! - Fallback to safe responses
//! - Comprehensive error classification
//! - Metrics collection for monitoring

use anyhow::{anyhow, Result};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{error, info, warn};

use crate::llm::{Completion, LanguageModel, Prompt};

/// Candle-specific error types
#[derive(Debug, Clone)]
pub enum CandleError {
    /// Model loading failed
    ModelLoadFailed(String),
    /// Tokenization error
    TokenizationFailed(String),
    /// Inference error (CUDA, CPU, memory)
    InferenceFailed(String),
    /// Out of memory error
    OutOfMemory(String),
    /// CUDA device error
    CudaError(String),
    /// Model file not found
    ModelNotFound(String),
    /// Timeout during inference
    Timeout(String),
    /// Unknown Candle error
    Unknown(String),
}

impl CandleError {
    /// Determine if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            CandleError::InferenceFailed(_)
                | CandleError::CudaError(_)
                | CandleError::Timeout(_)
                | CandleError::Unknown(_)
        )
    }

    /// Determine if error indicates model corruption
    pub fn indicates_model_corruption(&self) -> bool {
        matches!(self, CandleError::ModelLoadFailed(_) | CandleError::TokenizationFailed(_))
    }

    /// Get error severity level
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            CandleError::ModelLoadFailed(_) => ErrorSeverity::Critical,
            CandleError::TokenizationFailed(_) => ErrorSeverity::Critical,
            CandleError::OutOfMemory(_) => ErrorSeverity::High,
            CandleError::CudaError(_) => ErrorSeverity::High,
            CandleError::ModelNotFound(_) => ErrorSeverity::High,
            CandleError::InferenceFailed(_) => ErrorSeverity::Medium,
            CandleError::Timeout(_) => ErrorSeverity::Medium,
            CandleError::Unknown(_) => ErrorSeverity::Low,
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Error recovery configuration
#[derive(Debug, Clone)]
pub struct ErrorRecoveryConfig {
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Initial backoff duration
    pub initial_backoff: Duration,
    /// Maximum backoff duration
    pub max_backoff: Duration,
    /// Backoff multiplier
    pub backoff_multiplier: f64,
    /// Enable jitter
    pub enable_jitter: bool,
    /// Circuit breaker failure threshold
    pub circuit_breaker_threshold: u32,
    /// Circuit breaker recovery timeout
    pub circuit_breaker_timeout: Duration,
    /// Fallback response for critical failures
    pub fallback_response: Option<String>,
}

impl Default for ErrorRecoveryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(5),
            backoff_multiplier: 2.0,
            enable_jitter: true,
            circuit_breaker_threshold: 5,
            circuit_breaker_timeout: Duration::from_secs(30),
            fallback_response: Some(
                "I apologize, but I'm experiencing technical difficulties. Please try again later."
                    .to_string(),
            ),
        }
    }
}

/// Error recovery state
#[derive(Debug)]
pub struct ErrorRecoveryState {
    config: ErrorRecoveryConfig,
    failure_count: std::sync::atomic::AtomicU32,
    last_failure: std::sync::Mutex<Option<Instant>>,
    circuit_open_time: std::sync::Mutex<Option<Instant>>,
    total_requests: std::sync::atomic::AtomicU32,
    successful_requests: std::sync::atomic::AtomicU32,
}

impl ErrorRecoveryState {
    /// Create new error recovery state
    pub fn new(config: ErrorRecoveryConfig) -> Self {
        Self {
            config,
            failure_count: std::sync::atomic::AtomicU32::new(0),
            last_failure: std::sync::Mutex::new(None),
            circuit_open_time: std::sync::Mutex::new(None),
            total_requests: std::sync::atomic::AtomicU32::new(0),
            successful_requests: std::sync::atomic::AtomicU32::new(0),
        }
    }

    /// Check if circuit breaker is open
    pub fn is_circuit_open(&self) -> bool {
        if let Some(open_time) = *self.circuit_open_time.lock().unwrap() {
            if open_time.elapsed() < self.config.circuit_breaker_timeout {
                return true;
            } else {
                // Circuit timeout expired, reset
                *self.circuit_open_time.lock().unwrap() = None;
                self.failure_count.store(0, std::sync::atomic::Ordering::Relaxed);
                return false;
            }
        }
        false
    }

    /// Record a failure
    pub fn record_failure(&self) {
        self.failure_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        *self.last_failure.lock().unwrap() = Some(Instant::now());

        // Check if circuit should be opened
        let failure_count = self.failure_count.load(std::sync::atomic::Ordering::Relaxed);
        if failure_count >= self.config.circuit_breaker_threshold {
            *self.circuit_open_time.lock().unwrap() = Some(Instant::now());
            warn!("Circuit breaker opened after {} failures", failure_count);
        }
    }

    /// Record a success
    pub fn record_success(&self) {
        self.failure_count.store(0, std::sync::atomic::Ordering::Relaxed);
        self.successful_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        let total = self.total_requests.load(std::sync::atomic::Ordering::Relaxed);
        let successful = self.successful_requests.load(std::sync::atomic::Ordering::Relaxed);

        if total == 0 {
            0.0
        } else {
            successful as f64 / total as f64
        }
    }

    /// Get current failure count
    pub fn failure_count(&self) -> u32 {
        self.failure_count.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// Safe language model wrapper with error recovery
pub struct SafeLanguageModel {
    inner: Arc<dyn LanguageModel>,
    state: Arc<ErrorRecoveryState>,
}

impl SafeLanguageModel {
    /// Create new safe language model wrapper
    pub fn new(model: Arc<dyn LanguageModel>, config: ErrorRecoveryConfig) -> Self {
        Self {
            inner: model,
            state: Arc::new(ErrorRecoveryState::new(config)),
        }
    }

    /// Get completion with error recovery
    pub async fn complete_with_recovery(&self, prompt: &Prompt) -> Result<Completion> {
        self.state.total_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Check circuit breaker
        if self.state.is_circuit_open() {
            warn!("Circuit breaker is open, returning fallback response");
            return self.get_fallback_response(prompt);
        }

        let mut last_error = None;

        for attempt in 0..=self.state.config.max_retries {
            // Exponential backoff with jitter
            if attempt > 0 {
                let backoff = self.calculate_backoff(attempt);
                info!("Retrying LLM call attempt {} after {:?}", attempt, backoff);
                sleep(backoff).await;
            }

            match self.inner.complete(prompt) {
                Ok(completion) => {
                    self.state.record_success();
                    return Ok(completion);
                }
                Err(e) => {
                    let candle_error = self.classify_error(&e);
                    error!("LLM call failed on attempt {}: {:?} - {}", attempt, candle_error, e);

                    // Don't retry unrecoverable errors
                    if !candle_error.is_recoverable() {
                        break;
                    }

                    last_error = Some(e);
                }
            }
        }

        // All retries failed
        self.state.record_failure();

        match last_error {
            Some(e) => {
                // Try fallback response
                if let Ok(fallback) = self.get_fallback_response(prompt) {
                    warn!("Using fallback response after all retries failed: {}", e);
                    Ok(fallback)
                } else {
                    Err(anyhow!("All LLM retries failed and fallback unavailable: {}", e))
                }
            }
            None => Err(anyhow!("LLM call failed with unknown error")),
        }
    }

    /// Classify error into CandleError type
    fn classify_error(&self, error: &anyhow::Error) -> CandleError {
        let error_str = error.to_string().to_lowercase();

        if error_str.contains("model") && error_str.contains("load") {
            CandleError::ModelLoadFailed(error_str)
        } else if error_str.contains("token") {
            CandleError::TokenizationFailed(error_str)
        } else if error_str.contains("cuda") || error_str.contains("gpu") {
            CandleError::CudaError(error_str)
        } else if error_str.contains("memory") || error_str.contains("oom") {
            CandleError::OutOfMemory(error_str)
        } else if error_str.contains("not found") || error_str.contains("no such file") {
            CandleError::ModelNotFound(error_str)
        } else if error_str.contains("timeout") || error_str.contains("timed out") {
            CandleError::Timeout(error_str)
        } else if error_str.contains("inference") || error_str.contains("generation") {
            CandleError::InferenceFailed(error_str)
        } else {
            CandleError::Unknown(error_str)
        }
    }

    /// Calculate exponential backoff with jitter
    fn calculate_backoff(&self, attempt: u32) -> Duration {
        let base_backoff = self.state.config.initial_backoff;
        let multiplier = self.state.config.backoff_multiplier;

        let backoff_ms = base_backoff.as_millis() as f64 * multiplier.powi(attempt as i32);
        let backoff_ms = backoff_ms.min(self.state.config.max_backoff.as_millis() as f64);

        // Add jitter if enabled
        let jitter = if self.state.config.enable_jitter {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            rng.gen_range(0.0..=0.1) * backoff_ms // 0-10% jitter
        } else {
            0.0
        };

        Duration::from_millis((backoff_ms + jitter) as u64)
    }

    /// Get fallback response
    fn get_fallback_response(&self, prompt: &Prompt) -> Result<Completion> {
        if let Some(fallback_text) = &self.state.config.fallback_response {
            Ok(Completion::new(fallback_text.clone()))
        } else {
            Err(anyhow!("No fallback response configured"))
        }
    }

    /// Get recovery state metrics
    pub fn get_metrics(&self) -> RecoveryMetrics {
        RecoveryMetrics {
            total_requests: self.state.total_requests.load(std::sync::atomic::Ordering::Relaxed),
            successful_requests: self
                .state
                .successful_requests
                .load(std::sync::atomic::Ordering::Relaxed),
            failure_count: self.state.failure_count(),
            success_rate: self.state.success_rate(),
            circuit_open: self.state.is_circuit_open(),
        }
    }

    /// Reset recovery state
    pub fn reset_state(&self) {
        self.state.failure_count.store(0, std::sync::atomic::Ordering::Relaxed);
        self.state.successful_requests.store(0, std::sync::atomic::Ordering::Relaxed);
        self.state.total_requests.store(0, std::sync::atomic::Ordering::Relaxed);
        *self.state.circuit_open_time.lock().unwrap() = None;
        *self.state.last_failure.lock().unwrap() = None;
    }
}

/// Recovery metrics
#[derive(Debug, Clone)]
pub struct RecoveryMetrics {
    pub total_requests: u32,
    pub successful_requests: u32,
    pub failure_count: u32,
    pub success_rate: f64,
    pub circuit_open: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::test::TestLanguageModel;

    #[tokio::test]
    async fn test_circuit_breaker() {
        let config = ErrorRecoveryConfig {
            circuit_breaker_threshold: 2,
            circuit_breaker_timeout: Duration::from_millis(100),
            ..Default::default()
        };

        let state = ErrorRecoveryState::new(config);

        // Initially closed
        assert!(!state.is_circuit_open());

        // Record failures
        state.record_failure();
        state.record_failure();

        // Should be open now
        assert!(state.is_circuit_open());

        // Wait for timeout
        sleep(Duration::from_millis(150)).await;

        // Should be closed again
        assert!(!state.is_circuit_open());
    }

    #[test]
    fn test_error_classification() {
        let model = TestLanguageModel::predefined("test");
        let safe_model = SafeLanguageModel::new(Arc::new(model), ErrorRecoveryConfig::default());

        let error = anyhow!("Model failed to load: file not found");
        let candle_error = safe_model.classify_error(&error);

        assert!(matches!(candle_error, CandleError::ModelLoadFailed(_)));
        assert!(!candle_error.is_recoverable());
    }

    #[test]
    fn test_backoff_calculation() {
        let config = ErrorRecoveryConfig {
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_millis(1000),
            backoff_multiplier: 2.0,
            enable_jitter: false,
            ..Default::default()
        };

        let state = ErrorRecoveryState::new(config);
        let safe_model = SafeLanguageModel::new(
            Arc::new(TestLanguageModel::predefined("test")),
            ErrorRecoveryConfig::default(),
        );

        // Test exponential backoff
        let backoff1 = safe_model.calculate_backoff(1);
        let backoff2 = safe_model.calculate_backoff(2);
        let backoff3 = safe_model.calculate_backoff(3);

        assert_eq!(backoff1, Duration::from_millis(200)); // 100 * 2^1
        assert_eq!(backoff2, Duration::from_millis(400)); // 100 * 2^2
        assert_eq!(backoff3, Duration::from_millis(800)); // 100 * 2^3
    }
}
