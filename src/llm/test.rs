//! Test backend for LanguageModel - deterministic responses without network
//!
//! This is NOT a mock - it's a real in-process implementation that returns
//! deterministic responses for offline testing. No network calls are made.

use super::{Completion, LanguageModel, Prompt};
use anyhow::Result;
use std::sync::{Arc, Mutex};

/// Test mode for deterministic language model behavior
#[derive(Debug, Clone)]
pub enum TestMode {
    /// Echo mode: returns the user prompt text as-is
    Echo,
    /// Predefined mode: always returns the same constant string
    Predefined(String),
    /// Scripted mode: returns responses from a predefined sequence
    /// After sequence exhausts, repeats the last response
    Scripted(Vec<String>),
    /// Simulate failure: always returns an error
    SimulateFailure(String),
}

/// Deterministic test implementation of LanguageModel
///
/// This is a real backend that runs in-process without network calls.
/// Perfect for TDD and CI/CD environments where external services are unavailable.
pub struct TestLanguageModel {
    mode: TestMode,
    /// For Scripted mode: tracks current position in sequence
    script_index: Arc<Mutex<usize>>,
}

impl TestLanguageModel {
    /// Create a test model in Echo mode
    pub fn echo() -> Self {
        Self {
            mode: TestMode::Echo,
            script_index: Arc::new(Mutex::new(0)),
        }
    }

    /// Create a test model that always returns the same response
    pub fn predefined(response: impl Into<String>) -> Self {
        Self {
            mode: TestMode::Predefined(response.into()),
            script_index: Arc::new(Mutex::new(0)),
        }
    }

    /// Create a test model with a scripted sequence of responses
    ///
    /// After the sequence exhausts, the last response is repeated indefinitely.
    pub fn scripted(responses: Vec<String>) -> Self {
        Self {
            mode: TestMode::Scripted(responses),
            script_index: Arc::new(Mutex::new(0)),
        }
    }

    /// Create a test model that simulates backend failure
    pub fn simulate_failure(error_msg: impl Into<String>) -> Self {
        Self {
            mode: TestMode::SimulateFailure(error_msg.into()),
            script_index: Arc::new(Mutex::new(0)),
        }
    }

    /// Reset script index (useful for test setup)
    pub fn reset_script(&self) {
        let mut index = self.script_index.lock().unwrap();
        *index = 0;
    }
}

impl LanguageModel for TestLanguageModel {
    fn complete(&self, prompt: &Prompt) -> Result<Completion> {
        match &self.mode {
            TestMode::Echo => {
                // Return user prompt as response
                Ok(Completion::new(prompt.user.clone()))
            }
            TestMode::Predefined(response) => {
                // Always return the same response
                Ok(Completion::new(response.clone()))
            }
            TestMode::Scripted(responses) => {
                if responses.is_empty() {
                    return Ok(Completion::new(""));
                }

                let mut index = self.script_index.lock().unwrap();
                let current_index = *index;

                // Get response at current index, or last response if exhausted
                let response = if current_index < responses.len() {
                    responses[current_index].clone()
                } else {
                    responses.last().unwrap().clone()
                };

                // Advance index (but don't go past end)
                if current_index < responses.len() {
                    *index += 1;
                }

                Ok(Completion::new(response))
            }
            TestMode::SimulateFailure(error_msg) => Err(anyhow::anyhow!(
                "Test backend simulated failure: {}",
                error_msg
            )),
        }
    }

    fn backend_name(&self) -> &str {
        "test"
    }

    fn health_check(&self) -> Result<bool> {
        match &self.mode {
            TestMode::SimulateFailure(_) => Ok(false),
            _ => Ok(true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echo_mode_returns_user_text() {
        let model = TestLanguageModel::echo();
        let prompt = Prompt::new("System instruction", "Hello world");

        let result = model.complete(&prompt).unwrap();
        assert_eq!(result.text, "Hello world");
    }

    #[test]
    fn test_predefined_mode_returns_constant() {
        let model = TestLanguageModel::predefined("Fixed response");
        let prompt1 = Prompt::new("", "First prompt");
        let prompt2 = Prompt::new("", "Second prompt");

        let result1 = model.complete(&prompt1).unwrap();
        let result2 = model.complete(&prompt2).unwrap();

        assert_eq!(result1.text, "Fixed response");
        assert_eq!(result2.text, "Fixed response");
    }

    #[test]
    fn test_scripted_mode_returns_sequence_then_repeats_last() {
        let model = TestLanguageModel::scripted(vec![
            "First".to_string(),
            "Second".to_string(),
            "Third".to_string(),
        ]);

        let prompt = Prompt::new("", "test");

        // Get first three responses
        assert_eq!(model.complete(&prompt).unwrap().text, "First");
        assert_eq!(model.complete(&prompt).unwrap().text, "Second");
        assert_eq!(model.complete(&prompt).unwrap().text, "Third");

        // After sequence exhausts, should repeat last
        assert_eq!(model.complete(&prompt).unwrap().text, "Third");
        assert_eq!(model.complete(&prompt).unwrap().text, "Third");
    }

    #[test]
    fn test_simulate_failure_returns_error() {
        let model = TestLanguageModel::simulate_failure("Backend unreachable");
        let prompt = Prompt::new("", "test");

        let result = model.complete(&prompt);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Backend unreachable"));
    }

    #[test]
    fn test_health_check_for_different_modes() {
        let echo_model = TestLanguageModel::echo();
        assert!(echo_model.health_check().unwrap());

        let failure_model = TestLanguageModel::simulate_failure("down");
        assert!(!failure_model.health_check().unwrap());
    }

    #[test]
    fn test_backend_name() {
        let model = TestLanguageModel::echo();
        assert_eq!(model.backend_name(), "test");
    }
}
