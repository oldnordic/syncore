//! Diagnostics Severity Normalization
//!
//! Provides normalized severity representation for code diagnostics
//! across different tools (clippy, rustc, etc.) and converts them
//! to a consistent numeric representation for risk scoring.

use std::collections::HashMap;

/// Normalized severity levels for code diagnostics
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum NormalizedSeverity {
    /// Informational messages (notes, help, suggestions)
    Info,
    /// Warning level issues (potential problems, style issues)
    Warning,
    /// Error level issues (compilation errors, serious problems)
    Error,
    /// Unknown/unrecognized severity - treated as minimal impact
    Unknown,
}

/// Normalize raw severity strings from various tools into NormalizedSeverity
///
/// Maps common severity strings from different tools:
/// - "error" -> Error (compilation errors, serious issues)
/// - "warning" -> Warning (clippy warnings, potential problems)
/// - "note" -> Info (informational messages, suggestions)
/// - "help" -> Info (help messages, suggestions)
/// - "info" -> Info (general informational messages)
/// - Any other value -> Unknown (minimal impact)
///
/// # Arguments
/// * `raw` - Raw severity string from diagnostic tool
///
/// # Returns
/// Normalized severity enum variant
pub fn normalize_severity(raw: &str) -> NormalizedSeverity {
    match raw.to_lowercase().as_str() {
        "error" => NormalizedSeverity::Error,
        "warning" => NormalizedSeverity::Warning,
        "note" | "help" | "info" => NormalizedSeverity::Info,
        _ => NormalizedSeverity::Unknown,
    }
}

/// Convert normalized severity to numeric weight for risk scoring
///
/// Weight mapping:
/// - Info -> 1 (minimal impact)
/// - Warning -> 2 (moderate impact)
/// - Error -> 5 (high impact)
/// - Unknown -> 1 (minimal impact, treated as info)
///
/// # Arguments
/// * `severity` - Normalized severity enum
///
/// # Returns
/// Numeric weight for risk calculation
pub fn severity_weight(severity: &NormalizedSeverity) -> i32 {
    match severity {
        NormalizedSeverity::Info => 1,
        NormalizedSeverity::Warning => 2,
        NormalizedSeverity::Error => 5,
        NormalizedSeverity::Unknown => 1,
    }
}

/// Batch normalize multiple severity strings
///
/// Convenience function to convert a collection of raw severity strings
/// to normalized severity counts.
///
/// # Arguments
/// * `raw_severities` - Iterator of raw severity strings
///
/// # Returns
/// HashMap mapping NormalizedSeverity to count
pub fn normalize_severity_counts<I, S>(raw_severities: I) -> HashMap<NormalizedSeverity, u32>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut counts = HashMap::new();
    for raw in raw_severities {
        let normalized = normalize_severity(raw.as_ref());
        *counts.entry(normalized).or_insert(0) += 1;
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_severity() {
        assert_eq!(normalize_severity("error"), NormalizedSeverity::Error);
        assert_eq!(normalize_severity("warning"), NormalizedSeverity::Warning);
        assert_eq!(normalize_severity("note"), NormalizedSeverity::Info);
        assert_eq!(normalize_severity("help"), NormalizedSeverity::Info);
        assert_eq!(normalize_severity("info"), NormalizedSeverity::Info);
        assert_eq!(normalize_severity("unknown"), NormalizedSeverity::Unknown);
        assert_eq!(normalize_severity("ERROR"), NormalizedSeverity::Error);
        assert_eq!(normalize_severity("Warning"), NormalizedSeverity::Warning);
    }

    #[test]
    fn test_severity_weight() {
        assert_eq!(severity_weight(&NormalizedSeverity::Info), 1);
        assert_eq!(severity_weight(&NormalizedSeverity::Warning), 2);
        assert_eq!(severity_weight(&NormalizedSeverity::Error), 5);
        assert_eq!(severity_weight(&NormalizedSeverity::Unknown), 1);
    }

    #[test]
    fn test_normalize_severity_counts() {
        let severities = vec!["error", "warning", "note", "warning", "help"];
        let counts = normalize_severity_counts(&severities);

        assert_eq!(counts.get(&NormalizedSeverity::Error), Some(&1));
        assert_eq!(counts.get(&NormalizedSeverity::Warning), Some(&2));
        assert_eq!(counts.get(&NormalizedSeverity::Info), Some(&2));
        assert_eq!(counts.get(&NormalizedSeverity::Unknown), None);
    }
}
