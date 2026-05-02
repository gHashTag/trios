//! Verifier — Welch t-test and L14 enforcement.
//!
//! Provides statistical verification for trial results and
//! enforces L14 trailer validation.

use super::{victory::{TtestReport, welch_ttest, VictoryError}, invariants::TrialConfig};

/// Verifier for statistical validation.
pub struct Verifier {
    baseline_bpbs: Vec<f64>,
    alpha: f64,
}

impl Verifier {
    /// Create a new verifier with baseline BPBs.
    pub fn new(baseline_bpbs: Vec<f64>) -> Self {
        Self {
            baseline_bpbs,
            alpha: 0.05, // 95% confidence
        }
    }

    /// Create a new verifier with custom alpha.
    pub fn with_alpha(baseline_bpbs: Vec<f64>, alpha: f64) -> Self {
        Self { baseline_bpbs, alpha }
    }

    /// Verify a trial result.
    ///
    /// Performs Welch t-test and returns TtestReport.
    pub fn verify(&self, bpb: f64) -> Result<TtestReport, VictoryError> {
        welch_ttest(bpb, &self.baseline_bpbs)
    }

    /// Add a BPB to the baseline.
    pub fn add_baseline(&mut self, bpb: f64) {
        self.baseline_bpbs.push(bpb);
    }

    /// Get current baseline size.
    pub fn baseline_size(&self) -> usize {
        self.baseline_bpbs.len()
    }

    /// Calculate baseline mean.
    pub fn baseline_mean(&self) -> Option<f64> {
        if self.baseline_bpbs.is_empty() {
            return None;
        }
        let sum: f64 = self.baseline_bpbs.iter().sum();
        Some(sum / self.baseline_bpbs.len() as f64)
    }
}

/// L14 trailer validator.
///
/// Validates that trial results include required L14 trailer:
/// `Agent: <CODENAME>`
pub struct L14Validator {
    required_agent: Option<String>,
}

impl L14Validator {
    /// Create a new validator with required agent codename.
    pub fn new(required_agent: String) -> Self {
        Self {
            required_agent: Some(required_agent),
        }
    }

    /// Create a validator without required agent (any agent OK).
    pub fn any_agent() -> Self {
        Self {
            required_agent: None,
        }
    }

    /// Validate L14 trailer in commit message.
    ///
    /// Checks for `Agent: <CODENAME>` trailer.
    pub fn validate_commit(&self, message: &str) -> Result<(), L14Error> {
        let trailers = extract_trailers(message);
        let agent_trailer = trailers.iter()
            .find(|t| t.key.eq_ignore_ascii_case("Agent"));

        match (&self.required_agent, agent_trailer) {
            (None, Some(_)) => Ok(()),  // Any agent OK
            (Some(expected), Some(found)) => {
                if found.value.eq_ignore_ascii_case(expected) {
                    Ok(())
                } else {
                    Err(L14Error::WrongAgent {
                        expected: expected.clone(),
                        found: found.value.clone(),
                    })
                }
            }
            (Some(expected), None) => {
                Err(L14Error::MissingAgent(expected.clone()))
            }
            (None, None) => {
                Err(L14Error::MissingTrailer("Agent".into()))
            }
        }
    }
}

/// L14 validation error.
#[derive(Debug, thiserror::Error)]
pub enum L14Error {
    #[error("Missing L14 trailer: {0}")]
    MissingTrailer(String),

    #[error("Missing required agent in L14 trailer: expected '{0}'")]
    MissingAgent(String),

    #[error("Wrong agent in L14 trailer: expected '{expected}', found '{found}'")]
    WrongAgent {
        expected: String,
        found: String,
    },
}

/// Git trailer.
#[derive(Debug, Clone)]
struct Trailer {
    key: String,
    value: String,
}

/// Extract trailers from commit message.
///
/// Looks for `Key: Value` patterns at the end of message.
fn extract_trailers(message: &str) -> Vec<Trailer> {
    message.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if let Some((key, value)) = trimmed.split_once(':') {
                // Only count as trailer if key is a single word and has value
                if key.split_whitespace().count() == 1 && !value.trim().is_empty() {
                    Some(Trailer {
                        key: key.trim().into(),
                        value: value.trim().into(),
                    })
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_trailers() {
        let message = "Add feature\n\nAgent: ALPHA\nReviewed-by: BETA";
        let trailers = extract_trailers(message);
        assert_eq!(trailers.len(), 2);
        assert_eq!(trailers[0].key, "Agent");
        assert_eq!(trailers[0].value, "ALPHA");
    }

    #[test]
    fn test_l14_validator_ok() {
        let validator = L14Validator::new("ALPHA".into());
        let message = "Fix bug\nAgent: ALPHA";
        assert!(validator.validate_commit(message).is_ok());
    }

    #[test]
    fn test_l14_validator_wrong_agent() {
        let validator = L14Validator::new("ALPHA".into());
        let message = "Fix bug\nAgent: BETA";
        assert!(matches!(
            validator.validate_commit(message).unwrap_err(),
            L14Error::WrongAgent { .. }
        ));
    }

    #[test]
    fn test_l14_validator_missing() {
        let validator = L14Validator::new("ALPHA".into());
        let message = "Fix bug";
        assert!(matches!(
            validator.validate_commit(message).unwrap_err(),
            L14Error::MissingTrailer(_)
        ));
    }

    #[test]
    fn test_verifier_baseline() {
        let mut verifier = Verifier::new(vec![3.0, 3.1, 2.9]);
        assert_eq!(verifier.baseline_size(), 3);
        assert_eq!(verifier.baseline_mean(), Some(3.0));

        verifier.add_baseline(3.2);
        assert_eq!(verifier.baseline_size(), 4);
    }
}
