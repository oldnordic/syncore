//! Unified error type for all SynCore MCP tools
//!
//! Provides consistent error handling across all 49 MCP tools with
//! standardized JSON error responses.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Unified error type for all SynCore MCP tools
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "details")]
pub enum SynCoreError {
    /// Invalid input parameters
    InvalidInput {
        message: String,
        field: Option<String>,
    },

    /// Resource not found (file, task, memory key, etc.)
    NotFound {
        resource_type: String,
        identifier: String,
    },

    /// Database operation failed
    DatabaseError {
        operation: String,
        message: String,
    },

    /// Vector store operation failed
    VectorError {
        operation: String,
        message: String,
    },

    /// Graph database operation failed
    GraphError {
        operation: String,
        message: String,
    },

    /// File system operation failed
    FileSystemError {
        operation: String,
        path: String,
        message: String,
    },

    /// Parser error (tree-sitter, ripgrep, etc.)
    ParserError {
        language: Option<String>,
        message: String,
    },

    /// Network/external service error (Ollama, Neo4j, etc.)
    ExternalServiceError {
        service: String,
        message: String,
    },

    /// Permission denied
    PermissionDenied {
        operation: String,
        resource: String,
    },

    /// Operation would violate constraints
    ConstraintViolation {
        constraint: String,
        message: String,
    },

    /// Internal server error (unexpected)
    Internal {
        message: String,
        context: Option<String>,
    },
}

impl fmt::Display for SynCoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput {
                message,
                field,
            } => {
                if let Some(field) = field {
                    write!(f, "Invalid input for field '{}': {}", field, message)
                } else {
                    write!(f, "Invalid input: {}", message)
                }
            }
            Self::NotFound {
                resource_type,
                identifier,
            } => {
                write!(f, "{} not found: {}", resource_type, identifier)
            }
            Self::DatabaseError {
                operation,
                message,
            } => {
                write!(f, "Database error during {}: {}", operation, message)
            }
            Self::VectorError {
                operation,
                message,
            } => {
                write!(f, "Vector store error during {}: {}", operation, message)
            }
            Self::GraphError {
                operation,
                message,
            } => {
                write!(f, "Graph error during {}: {}", operation, message)
            }
            Self::FileSystemError {
                operation,
                path,
                message,
            } => {
                write!(f, "File system error during {} on '{}': {}", operation, path, message)
            }
            Self::ParserError {
                language,
                message,
            } => {
                if let Some(lang) = language {
                    write!(f, "Parser error ({} parser): {}", lang, message)
                } else {
                    write!(f, "Parser error: {}", message)
                }
            }
            Self::ExternalServiceError {
                service,
                message,
            } => {
                write!(f, "{} service error: {}", service, message)
            }
            Self::PermissionDenied {
                operation,
                resource,
            } => {
                write!(f, "Permission denied for {} on {}", operation, resource)
            }
            Self::ConstraintViolation {
                constraint,
                message,
            } => {
                write!(f, "Constraint violation ({}): {}", constraint, message)
            }
            Self::Internal {
                message,
                context,
            } => {
                if let Some(ctx) = context {
                    write!(f, "Internal error ({}): {}", ctx, message)
                } else {
                    write!(f, "Internal error: {}", message)
                }
            }
        }
    }
}

impl std::error::Error for SynCoreError {}

impl From<anyhow::Error> for SynCoreError {
    fn from(err: anyhow::Error) -> Self {
        SynCoreError::Internal {
            message: err.to_string(),
            context: Some("anyhow conversion".to_string()),
        }
    }
}

impl From<std::io::Error> for SynCoreError {
    fn from(err: std::io::Error) -> Self {
        SynCoreError::FileSystemError {
            operation: "io".to_string(),
            path: "unknown".to_string(),
            message: err.to_string(),
        }
    }
}

impl From<rusqlite::Error> for SynCoreError {
    fn from(err: rusqlite::Error) -> Self {
        SynCoreError::DatabaseError {
            operation: "sqlite".to_string(),
            message: err.to_string(),
        }
    }
}

impl SynCoreError {
    /// Convert error to JSON value for MCP response
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_else(|_| {
            serde_json::json!({
                "type": "Internal",
                "details": {
                    "message": "Failed to serialize error",
                    "context": "error serialization"
                }
            })
        })
    }

    /// Create an invalid input error
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput {
            message: message.into(),
            field: None,
        }
    }

    /// Create an invalid input error for a specific field
    pub fn invalid_field(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InvalidInput {
            message: message.into(),
            field: Some(field.into()),
        }
    }

    /// Create a not found error
    pub fn not_found(resource_type: impl Into<String>, identifier: impl Into<String>) -> Self {
        Self::NotFound {
            resource_type: resource_type.into(),
            identifier: identifier.into(),
        }
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
            context: None,
        }
    }
}

/// Type alias for Results using SynCoreError
pub type SynCoreResult<T> = Result<T, SynCoreError>;
