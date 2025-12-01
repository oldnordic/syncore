//! Error Types for MCP Tool Responses
//!
//! Phase 7 Step 3 - Standardized error categorization

use serde::{Deserialize, Serialize};

/// Standardized error types for MCP tool failures
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum ErrorType {
    /// Required parameter is missing
    MissingParameter,
    /// I/O operation failed (file, network, database)
    IoError,
    /// Invalid action or operation requested
    InvalidAction,
    /// Feature or resource not available
    NotAvailable,
    /// Operation exceeded time limit
    Timeout,
    /// Internal system error
    Internal,
}

impl std::fmt::Display for ErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorType::MissingParameter => write!(f, "MissingParameter"),
            ErrorType::IoError => write!(f, "IoError"),
            ErrorType::InvalidAction => write!(f, "InvalidAction"),
            ErrorType::NotAvailable => write!(f, "NotAvailable"),
            ErrorType::Timeout => write!(f, "Timeout"),
            ErrorType::Internal => write!(f, "Internal"),
        }
    }
}

impl ErrorType {
    /// Infer error type from error message
    pub fn from_message(msg: &str) -> Self {
        let msg_lower = msg.to_lowercase();

        if msg_lower.contains("missing")
            || msg_lower.contains("required")
            || msg_lower.contains("parameter")
        {
            ErrorType::MissingParameter
        } else if msg_lower.contains("timeout") || msg_lower.contains("exceeded") {
            ErrorType::Timeout
        } else if msg_lower.contains("io")
            || msg_lower.contains("file")
            || msg_lower.contains("database")
        {
            ErrorType::IoError
        } else if msg_lower.contains("invalid")
            || msg_lower.contains("unknown")
            || msg_lower.contains("unsupported")
        {
            ErrorType::InvalidAction
        } else if msg_lower.contains("unavailable")
            || msg_lower.contains("not available")
            || msg_lower.contains("disabled")
        {
            ErrorType::NotAvailable
        } else {
            ErrorType::Internal
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_type_inference() {
        assert_eq!(ErrorType::from_message("Missing 'key' parameter"), ErrorType::MissingParameter);
        assert_eq!(ErrorType::from_message("Timeout exceeded"), ErrorType::Timeout);
        assert_eq!(ErrorType::from_message("IO error reading file"), ErrorType::IoError);
        assert_eq!(ErrorType::from_message("Invalid action"), ErrorType::InvalidAction);
        assert_eq!(ErrorType::from_message("Feature not available"), ErrorType::NotAvailable);
        assert_eq!(ErrorType::from_message("Something went wrong"), ErrorType::Internal);
    }

    #[test]
    fn test_error_type_display() {
        assert_eq!(ErrorType::MissingParameter.to_string(), "MissingParameter");
        assert_eq!(ErrorType::Timeout.to_string(), "Timeout");
        assert_eq!(ErrorType::IoError.to_string(), "IoError");
    }
}
