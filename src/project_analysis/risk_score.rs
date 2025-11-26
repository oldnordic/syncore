//! Risk Score Computation Utilities
//!
//! Provides unified risk scoring for files based on:
//! - Hotspot metrics (code complexity and change frequency)
//! - Lines of code (LOC)
//! - Diagnostic counts weighted by severity

use std::collections::HashMap;

use super::diagnostics_severity::{severity_weight, NormalizedSeverity};

/// Input data for computing file risk score
#[derive(Debug, Clone)]
pub struct FileRiskInputs {
    /// Path to the file being analyzed
    pub file_path: String,
    /// Hotspot score from PAE analysis (0.0 if not available)
    pub hotspot_score: f32,
    /// Lines of code count (0 if unknown)
    pub loc: u32,
    /// Diagnostic counts grouped by normalized severity
    pub diagnostics_by_severity: HashMap<NormalizedSeverity, u32>,
}

/// Compute unified risk score for a file
///
/// Risk score formula:
/// ```
/// risk_score = hotspot_score
///     + 0.5 * loc_factor
///     + Σ (severity_weight(severity) * count)
/// ```
/// Where:
/// - `loc_factor = loc / 200.0` (so 200 LOC ≈ +1.0 risk)
/// - Severity weights: Info=1, Warning=2, Error=5, Unknown=1
///
/// This formula combines:
/// - Hotspot metrics (existing PAE complexity analysis)
/// - Code size (larger files have higher baseline risk)
/// - Diagnostic severity (errors weighted heavily, warnings moderately)
///
/// # Arguments
/// * `inputs` - File risk input data
///
/// # Returns
/// Computed risk score (higher = more risky)
pub fn compute_risk_score(inputs: &FileRiskInputs) -> f32 {
    // Base risk from hotspot analysis
    let mut risk_score = inputs.hotspot_score;

    // Add risk from code size (200 LOC = +1.0 risk)
    let loc_factor = inputs.loc as f32 / 200.0;
    risk_score += 0.5 * loc_factor;

    // Add risk from diagnostics weighted by severity
    for (severity, count) in &inputs.diagnostics_by_severity {
        let weight = severity_weight(severity);
        risk_score += (weight as f32) * (*count as f32);
    }

    risk_score
}

/// Compute risk score for multiple files
///
/// Convenience function to batch compute risk scores.
///
/// # Arguments
/// * `inputs_list` - List of file risk inputs
///
/// # Returns
/// Vector of (file_path, risk_score) tuples
pub fn compute_risk_scores_batch(inputs_list: &[FileRiskInputs]) -> Vec<(String, f32)> {
    inputs_list
        .iter()
        .map(|inputs| {
            let risk_score = compute_risk_score(inputs);
            (inputs.file_path.clone(), risk_score)
        })
        .collect()
}

/// Get risk category based on score
///
/// Categorizes risk scores for human interpretation:
/// - Low: < 5.0
/// - Medium: 5.0 - 15.0  
/// - High: 15.0 - 30.0
/// - Critical: >= 30.0
///
/// # Arguments
/// * `risk_score` - Computed risk score
///
/// # Returns
/// Risk category as string
pub fn risk_category(risk_score: f32) -> &'static str {
    match risk_score {
        score if score < 5.0 => "Low",
        score if score < 15.0 => "Medium",
        score if score < 30.0 => "High",
        _ => "Critical",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_risk_score_low_hotspot_low_diagnostics() {
        let mut diagnostics = HashMap::new();
        diagnostics.insert(NormalizedSeverity::Warning, 1);
        diagnostics.insert(NormalizedSeverity::Info, 2);

        let inputs = FileRiskInputs {
            file_path: "test.rs".to_string(),
            hotspot_score: 2.0,
            loc: 100,
            diagnostics_by_severity: diagnostics,
        };

        let risk = compute_risk_score(&inputs);
        // Expected: 2.0 (hotspot) + 0.5 * 0.5 (loc) + 2*1 + 1*2 = 2.0 + 0.25 + 2.0 + 2.0 = 6.25
        assert!((risk - 6.25).abs() < 0.01);
    }

    #[test]
    fn test_compute_risk_score_high_hotspot_few_errors() {
        let mut diagnostics = HashMap::new();
        diagnostics.insert(NormalizedSeverity::Error, 2);
        diagnostics.insert(NormalizedSeverity::Warning, 1);

        let inputs = FileRiskInputs {
            file_path: "complex.rs".to_string(),
            hotspot_score: 15.0,
            loc: 500,
            diagnostics_by_severity: diagnostics,
        };

        let risk = compute_risk_score(&inputs);
        // Expected: 15.0 + 0.5 * 2.5 + 2*5 + 1*2 = 15.0 + 1.25 + 10.0 + 2.0 = 28.25
        assert!((risk - 28.25).abs() < 0.01);
    }

    #[test]
    fn test_compute_risk_score_low_hotspot_many_warnings_errors() {
        let mut diagnostics = HashMap::new();
        diagnostics.insert(NormalizedSeverity::Error, 3);
        diagnostics.insert(NormalizedSeverity::Warning, 8);
        diagnostics.insert(NormalizedSeverity::Info, 5);

        let inputs = FileRiskInputs {
            file_path: "messy.rs".to_string(),
            hotspot_score: 1.0,
            loc: 300,
            diagnostics_by_severity: diagnostics,
        };

        let risk = compute_risk_score(&inputs);
        // Expected: 1.0 + 0.5 * 1.5 + 3*5 + 8*2 + 5*1 = 1.0 + 0.75 + 15.0 + 16.0 + 5.0 = 37.75
        assert!((risk - 37.75).abs() < 0.01);
    }

    #[test]
    fn test_compute_risk_scores_batch() {
        let inputs1 = FileRiskInputs {
            file_path: "file1.rs".to_string(),
            hotspot_score: 5.0,
            loc: 200,
            diagnostics_by_severity: HashMap::new(),
        };

        let inputs2 = FileRiskInputs {
            file_path: "file2.rs".to_string(),
            hotspot_score: 10.0,
            loc: 400,
            diagnostics_by_severity: HashMap::new(),
        };

        let results = compute_risk_scores_batch(&[inputs1, inputs2]);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "file1.rs");
        assert_eq!(results[1].0, "file2.rs");
        assert!(results[1].1 > results[0].1); // file2 should have higher risk
    }

    #[test]
    fn test_risk_category() {
        assert_eq!(risk_category(3.0), "Low");
        assert_eq!(risk_category(10.0), "Medium");
        assert_eq!(risk_category(20.0), "High");
        assert_eq!(risk_category(35.0), "Critical");
    }
}
