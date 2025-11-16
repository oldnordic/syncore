//! Agent Circuit Breaker - Prevent AI agents from getting stuck in unproductive loops
//!
//! This module implements a circuit breaker pattern to detect and prevent:
//! - Repeated tool calls with no progress
//! - Identical tool calls with same parameters
//! - Tools that consistently return no output
//! - Excessive sequential tool usage without user interaction
//!
//! Inspired by GPU safety guards, this ensures AI agents don't waste compute cycles.

use std::sync::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{warn, info};

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation
    Closed,
    /// Temporary failure threshold reached - next call will be blocked
    Open,
    /// Testing if system recovered
    HalfOpen,
}

/// Track tool call patterns
#[derive(Debug, Clone)]
struct ToolCallRecord {
    tool_name: String,
    _params_hash: u64,  // Reserved for future debugging/logging
    timestamp: Instant,
    had_output: bool,
}

/// Configuration for circuit breaker thresholds
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Maximum identical calls before tripping
    pub max_identical_calls: usize,
    /// Maximum no-output calls before tripping
    pub max_no_output_calls: usize,
    /// Maximum calls in time window
    pub max_calls_per_window: usize,
    /// Time window for rate limiting
    pub time_window: Duration,
    /// How long to keep circuit open
    pub open_duration: Duration,
    /// Reset threshold for half-open state
    pub reset_threshold: usize,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            max_identical_calls: 3,
            max_no_output_calls: 5,
            max_calls_per_window: 10,
            time_window: Duration::from_secs(30),
            open_duration: Duration::from_secs(60),
            reset_threshold: 3,
        }
    }
}

/// Circuit breaker for AI agent tool usage
pub struct AgentCircuitBreaker {
    config: CircuitBreakerConfig,
    state: Arc<Mutex<CircuitState>>,
    history: Arc<Mutex<Vec<ToolCallRecord>>>,
    identical_call_count: Arc<Mutex<HashMap<u64, usize>>>,
    no_output_streak: Arc<Mutex<usize>>,
    last_trip_time: Arc<Mutex<Option<Instant>>>,
    successful_calls: Arc<Mutex<usize>>,
}

impl AgentCircuitBreaker {
    /// Create a new circuit breaker with default config
    pub fn new() -> Self {
        Self::with_config(CircuitBreakerConfig::default())
    }

    /// Create a new circuit breaker with custom config
    pub fn with_config(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(CircuitState::Closed)),
            history: Arc::new(Mutex::new(Vec::new())),
            identical_call_count: Arc::new(Mutex::new(HashMap::new())),
            no_output_streak: Arc::new(Mutex::new(0)),
            last_trip_time: Arc::new(Mutex::new(None)),
            successful_calls: Arc::new(Mutex::new(0)),
        }
    }

    /// Check if a tool call should be allowed
    pub fn check_tool_call(&self, tool_name: &str, params: &str) -> Result<(), CircuitBreakerError> {
        let mut state = self.state.lock().unwrap();

        // Check circuit state
        match *state {
            CircuitState::Open => {
                // Check if enough time has passed to try half-open
                if let Some(trip_time) = *self.last_trip_time.lock().unwrap() {
                    if trip_time.elapsed() >= self.config.open_duration {
                        *state = CircuitState::HalfOpen;
                        *self.successful_calls.lock().unwrap() = 0;
                        info!("Circuit breaker entering half-open state");
                    } else {
                        return Err(CircuitBreakerError::CircuitOpen {
                            reason: "Too many repeated or unproductive tool calls".to_string(),
                            retry_after: self.config.open_duration - trip_time.elapsed(),
                        });
                    }
                }
            }
            CircuitState::HalfOpen => {
                // In half-open, allow call but track success
            }
            CircuitState::Closed => {
                // Normal operation
            }
        }

        // Hash parameters for duplicate detection
        let params_hash = Self::hash_params(tool_name, params);

        // Check for identical calls
        let mut identical_counts = self.identical_call_count.lock().unwrap();
        let count = identical_counts.entry(params_hash).or_insert(0);
        *count += 1;

        if *count >= self.config.max_identical_calls {
            self.trip_circuit("Identical tool calls exceeded threshold");
            return Err(CircuitBreakerError::TooManyIdenticalCalls {
                tool: tool_name.to_string(),
                count: *count,
            });
        }

        // Check rate limiting
        let mut history = self.history.lock().unwrap();
        let now = Instant::now();

        // Clean old history
        history.retain(|record| now.duration_since(record.timestamp) < self.config.time_window);

        if history.len() >= self.config.max_calls_per_window {
            self.trip_circuit("Call rate limit exceeded");
            return Err(CircuitBreakerError::RateLimitExceeded {
                window: self.config.time_window,
                count: history.len(),
            });
        }

        Ok(())
    }

    /// Record the result of a tool call
    pub fn record_result(&self, tool_name: &str, params: &str, had_output: bool) {
        let params_hash = Self::hash_params(tool_name, params);

        let mut history = self.history.lock().unwrap();
        history.push(ToolCallRecord {
            tool_name: tool_name.to_string(),
            _params_hash: params_hash,
            timestamp: Instant::now(),
            had_output,
        });

        // Track no-output streak
        let mut streak = self.no_output_streak.lock().unwrap();
        if had_output {
            *streak = 0;

            // Handle half-open success
            if *self.state.lock().unwrap() == CircuitState::HalfOpen {
                let mut success_count = self.successful_calls.lock().unwrap();
                *success_count += 1;

                if *success_count >= self.config.reset_threshold {
                    *self.state.lock().unwrap() = CircuitState::Closed;
                    self.reset();
                    info!("Circuit breaker reset to closed state");
                }
            }
        } else {
            *streak += 1;

            if *streak >= self.config.max_no_output_calls {
                self.trip_circuit("Too many no-output calls");
            }
        }
    }

    /// Manually trip the circuit breaker
    fn trip_circuit(&self, reason: &str) {
        *self.state.lock().unwrap() = CircuitState::Open;
        *self.last_trip_time.lock().unwrap() = Some(Instant::now());
        warn!("Circuit breaker tripped: {}", reason);
    }

    /// Reset circuit breaker state
    pub fn reset(&self) {
        *self.state.lock().unwrap() = CircuitState::Closed;
        self.history.lock().unwrap().clear();
        self.identical_call_count.lock().unwrap().clear();
        *self.no_output_streak.lock().unwrap() = 0;
        *self.last_trip_time.lock().unwrap() = None;
        *self.successful_calls.lock().unwrap() = 0;
        info!("Circuit breaker reset");
    }

    /// Get current circuit state
    pub fn state(&self) -> CircuitState {
        *self.state.lock().unwrap()
    }

    /// Get statistics
    pub fn stats(&self) -> CircuitBreakerStats {
        let history = self.history.lock().unwrap();
        let no_output_count = history.iter().filter(|r| !r.had_output).count();

        CircuitBreakerStats {
            state: *self.state.lock().unwrap(),
            total_calls: history.len(),
            no_output_calls: no_output_count,
            unique_tools: history.iter()
                .map(|r| r.tool_name.clone())
                .collect::<std::collections::HashSet<_>>()
                .len(),
        }
    }

    /// Hash parameters for duplicate detection
    fn hash_params(tool_name: &str, params: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        tool_name.hash(&mut hasher);
        params.hash(&mut hasher);
        hasher.finish()
    }
}

impl Default for AgentCircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about circuit breaker state
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    pub state: CircuitState,
    pub total_calls: usize,
    pub no_output_calls: usize,
    pub unique_tools: usize,
}

/// Circuit breaker errors
#[derive(Debug)]
pub enum CircuitBreakerError {
    CircuitOpen {
        reason: String,
        retry_after: Duration,
    },
    TooManyIdenticalCalls {
        tool: String,
        count: usize,
    },
    RateLimitExceeded {
        window: Duration,
        count: usize,
    },
}

impl std::fmt::Display for CircuitBreakerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitBreakerError::CircuitOpen { reason, retry_after } => {
                write!(f, "Circuit breaker is open: {}. Retry after {:?}", reason, retry_after)
            }
            CircuitBreakerError::TooManyIdenticalCalls { tool, count } => {
                write!(f, "Too many identical calls to {}: {} times", tool, count)
            }
            CircuitBreakerError::RateLimitExceeded { window, count } => {
                write!(f, "Rate limit exceeded: {} calls in {:?}", count, window)
            }
        }
    }
}

impl std::error::Error for CircuitBreakerError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_calls_trip_breaker() {
        let breaker = AgentCircuitBreaker::new();

        // First 2 calls should succeed
        assert!(breaker.check_tool_call("test_tool", "param1").is_ok());
        breaker.record_result("test_tool", "param1", false);

        assert!(breaker.check_tool_call("test_tool", "param1").is_ok());
        breaker.record_result("test_tool", "param1", false);

        // Third identical call should trip
        let result = breaker.check_tool_call("test_tool", "param1");
        assert!(result.is_err());
        assert_eq!(breaker.state(), CircuitState::Open);
    }

    #[test]
    fn test_no_output_streak_trips_breaker() {
        let config = CircuitBreakerConfig {
            max_no_output_calls: 3,
            ..Default::default()
        };
        let breaker = AgentCircuitBreaker::with_config(config);

        // Make 3 calls with no output
        for i in 0..3 {
            assert!(breaker.check_tool_call("tool", &format!("param{}", i)).is_ok());
            breaker.record_result("tool", &format!("param{}", i), false);
        }

        assert_eq!(breaker.state(), CircuitState::Open);
    }

    #[test]
    fn test_successful_calls_prevent_trip() {
        let breaker = AgentCircuitBreaker::new();

        for i in 0..10 {
            assert!(breaker.check_tool_call("tool", &format!("param{}", i)).is_ok());
            breaker.record_result("tool", &format!("param{}", i), true);
        }

        assert_eq!(breaker.state(), CircuitState::Closed);
    }

    #[test]
    fn test_half_open_recovery() {
        let config = CircuitBreakerConfig {
            open_duration: Duration::from_millis(10),
            reset_threshold: 2,
            ..Default::default()
        };
        let breaker = AgentCircuitBreaker::with_config(config);

        // Trip the breaker
        for _ in 0..3 {
            let _ = breaker.check_tool_call("tool", "param");
            breaker.record_result("tool", "param", false);
        }
        assert_eq!(breaker.state(), CircuitState::Open);

        // Wait for open duration
        std::thread::sleep(Duration::from_millis(15));

        // Should enter half-open
        assert!(breaker.check_tool_call("tool2", "param2").is_ok());
        assert_eq!(breaker.state(), CircuitState::HalfOpen);

        // Successful calls should close circuit
        breaker.record_result("tool2", "param2", true);
        breaker.check_tool_call("tool3", "param3").unwrap();
        breaker.record_result("tool3", "param3", true);

        assert_eq!(breaker.state(), CircuitState::Closed);
    }
}
