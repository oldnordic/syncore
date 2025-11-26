//! Utility functions for code graph operations

use super::types::CodeEntity;
use crate::parser::FunctionInfo;

/// Format function signature for display
pub(super) fn format_function_signature(func: &FunctionInfo) -> String {
    if func.parameters.is_empty() {
        format!("{}()", func.name)
    } else {
        format!("{}({})", func.name, func.parameters.join(", "))
    }
}

/// Format entity as text for embedding
pub(super) fn format_entity_for_embedding(entity: &CodeEntity) -> String {
    let mut parts = vec![entity.entity_type.as_str().to_string(), entity.name.clone()];

    if let Some(sig) = &entity.signature {
        parts.push(sig.clone());
    }

    if let Some(doc) = &entity.docstring {
        parts.push(doc.clone());
    }

    // APEX v1.7 Phase 3: Include body_snippet in embedding text
    // This allows semantic search to match against function implementation details
    if let Some(body) = &entity.body_snippet {
        parts.push(body.clone());
    }

    parts.join(" ")
}
