//! Runtime Executor Selection
//!
//! Phase 7 Step 1 - Switch between RealExecutor and RealExecutorStub at runtime
//! via the SYNCORE_EXECUTOR environment variable.

use crate::macro_tools::executor_real::RealExecutor;
use crate::macro_tools::executor_stub::RealExecutorStub;
use crate::macro_tools::planner::ExecutionRecorder;
use crate::router::SynCoreState;
use std::sync::Arc;

/// Executor implementation kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutorKind {
    /// Production executor - executes real database operations
    Real,
    /// Stub executor - returns synthetic deterministic responses
    Stub,
}

impl ExecutorKind {
    /// Load executor kind from SYNCORE_EXECUTOR environment variable
    ///
    /// Valid values:
    /// - "real" or empty/missing -> ExecutorKind::Real (default)
    /// - "stub" -> ExecutorKind::Stub
    /// - Any other value -> ExecutorKind::Real (with warning log)
    ///
    /// # Examples
    /// ```
    /// std::env::set_var("SYNCORE_EXECUTOR", "stub");
    /// let kind = ExecutorKind::from_env();
    /// assert_eq!(kind, ExecutorKind::Stub);
    /// ```
    pub fn from_env() -> Self {
        match std::env::var("SYNCORE_EXECUTOR") {
            Ok(val) => {
                let val_lower = val.trim().to_lowercase();
                match val_lower.as_str() {
                    "real" | "" => ExecutorKind::Real,
                    "stub" => ExecutorKind::Stub,
                    unknown => {
                        eprintln!(
                            "WARNING: Unknown SYNCORE_EXECUTOR value '{}', falling back to Real",
                            unknown
                        );
                        ExecutorKind::Real
                    }
                }
            }
            Err(_) => ExecutorKind::Real, // Default
        }
    }
}

/// Create an executor instance based on the kind
///
/// # Arguments
/// - `kind`: The executor implementation to use
/// - `state`: Shared SynCore state (memory, tasks, vector store)
///
/// # Returns
/// Arc-wrapped trait object implementing ExecutionRecorder
///
/// # Examples
/// ```
/// let state = Arc::new(SynCoreState::new(...));
/// let executor = create_executor(ExecutorKind::Real, state);
/// executor.record_step("memory_store", json!({"key": "test"}));
/// ```
pub fn create_executor(
    kind: ExecutorKind,
    state: Arc<SynCoreState>,
) -> Arc<dyn ExecutionRecorder + Send + Sync> {
    match kind {
        ExecutorKind::Real => {
            let executor = RealExecutor::new(state);
            Arc::new(executor) as Arc<dyn ExecutionRecorder + Send + Sync>
        }
        ExecutorKind::Stub => {
            let stub = RealExecutorStub::new();
            Arc::new(stub) as Arc<dyn ExecutionRecorder + Send + Sync>
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_kind_from_env_default() {
        std::env::remove_var("SYNCORE_EXECUTOR");
        let kind = ExecutorKind::from_env();
        assert_eq!(kind, ExecutorKind::Real);
    }

    #[test]
    fn test_executor_kind_from_env_real() {
        std::env::set_var("SYNCORE_EXECUTOR", "real");
        let kind = ExecutorKind::from_env();
        assert_eq!(kind, ExecutorKind::Real);
        std::env::remove_var("SYNCORE_EXECUTOR");
    }

    #[test]
    fn test_executor_kind_from_env_stub() {
        std::env::set_var("SYNCORE_EXECUTOR", "stub");
        let kind = ExecutorKind::from_env();
        assert_eq!(kind, ExecutorKind::Stub);
        std::env::remove_var("SYNCORE_EXECUTOR");
    }

    #[test]
    fn test_executor_kind_from_env_invalid() {
        std::env::set_var("SYNCORE_EXECUTOR", "invalid_xyz");
        let kind = ExecutorKind::from_env();
        assert_eq!(kind, ExecutorKind::Real); // Fallback
        std::env::remove_var("SYNCORE_EXECUTOR");
    }

    #[test]
    fn test_executor_kind_from_env_case_insensitive() {
        std::env::set_var("SYNCORE_EXECUTOR", "STUB");
        let kind = ExecutorKind::from_env();
        assert_eq!(kind, ExecutorKind::Stub);
        std::env::remove_var("SYNCORE_EXECUTOR");

        std::env::set_var("SYNCORE_EXECUTOR", "Real");
        let kind = ExecutorKind::from_env();
        assert_eq!(kind, ExecutorKind::Real);
        std::env::remove_var("SYNCORE_EXECUTOR");
    }
}
