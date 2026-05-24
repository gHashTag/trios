//! # CR-CHAT-06 — Agent output redaction completeness guard (Wave-112 Lane A)
//!
//! AGENT SAFETY — sensitive data in agent output must be fully redacted.
//!
//! Agent responses may contain sensitive data (API keys, email
//! addresses, phone numbers). The redaction pipeline must catch all
//! occurrences:
//!
//! * **Secret leakage** — an unredacted API key in agent output is
//!   sent to all group members, exposing the secret.
//! * **PII exposure** — unredacted email addresses or phone numbers
//!   violate privacy regulations.
//! * **Pattern evasion** — adversaries craft inputs that produce
//!   output matching sensitive patterns in unexpected formats
//!   (e.g., base64-encoded secrets).
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. All declared sensitive patterns must be found and redacted.
//! 2. Redacted segments must be >= `AORC_MIN_REDACT_LEN`.
//! 3. Output must not contain unredacted sensitive markers.
//! 4. Redaction count must match expected count.
//! 5. Output ID must not be zero.
//! 6. Total outputs <= `AORC_MAX_OUTPUTS`.
//!
//! Tests **AORC-01..10**. Error enum [`RedactionError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * REDACTION-COMPLETE`

#![forbid(unsafe_code)]

/// Minimum redacted segment length.
pub const AORC_MIN_REDACT_LEN: usize = 3;

/// Maximum outputs per batch.
pub const AORC_MAX_OUTPUTS: usize = 256;

/// Output ID length.
pub const AORC_OUTPUT_ID_LEN: usize = 16;

/// A redaction verification record.
#[derive(Debug, Clone)]
pub struct RedactionCheck {
    /// Output identifier.
    pub output_id: [u8; AORC_OUTPUT_ID_LEN],
    /// Expected number of redactions.
    pub expected_redactions: usize,
    /// Actual number of redactions found.
    pub actual_redactions: usize,
    /// Whether all sensitive patterns were redacted.
    pub all_redacted: bool,
    /// Length of shortest redacted segment.
    pub min_redact_len: usize,
}

/// All ways redaction validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RedactionError {
    /// Not all patterns redacted.
    IncompleteRedaction { idx: usize, expected: usize, actual: usize },
    /// Redacted segment too short.
    ShortRedaction { idx: usize, len: usize, min: usize },
    /// Unredacted sensitive content.
    UnredactedContent(usize),
    /// Count mismatch.
    CountMismatch { idx: usize, expected: usize, actual: usize },
    /// Zero output ID.
    ZeroOutputId(usize),
    /// Too many outputs.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate agent output redaction completeness.
pub fn validate_redaction(
    checks: &[RedactionCheck],
) -> Result<(), RedactionError> {
    if checks.len() > AORC_MAX_OUTPUTS {
        return Err(RedactionError::TooMany {
            got: checks.len(),
            max: AORC_MAX_OUTPUTS,
        });
    }
    for (i, c) in checks.iter().enumerate() {
        if c.output_id == [0u8; AORC_OUTPUT_ID_LEN] {
            return Err(RedactionError::ZeroOutputId(i));
        }
        if !c.all_redacted {
            return Err(RedactionError::UnredactedContent(i));
        }
        if c.actual_redactions != c.expected_redactions {
            return Err(RedactionError::CountMismatch {
                idx: i,
                expected: c.expected_redactions,
                actual: c.actual_redactions,
            });
        }
        if c.expected_redactions > 0 && c.min_redact_len < AORC_MIN_REDACT_LEN {
            return Err(RedactionError::ShortRedaction {
                idx: i,
                len: c.min_redact_len,
                min: AORC_MIN_REDACT_LEN,
            });
        }
        if !c.all_redacted && c.expected_redactions > 0 {
            return Err(RedactionError::IncompleteRedaction {
                idx: i,
                expected: c.expected_redactions,
                actual: c.actual_redactions,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(byte: u8) -> [u8; AORC_OUTPUT_ID_LEN] {
        [byte; AORC_OUTPUT_ID_LEN]
    }

    fn check(id: u8, expected: usize, actual: usize, all: bool, min_len: usize) -> RedactionCheck {
        RedactionCheck {
            output_id: oid(id),
            expected_redactions: expected,
            actual_redactions: actual,
            all_redacted: all,
            min_redact_len: min_len,
        }
    }

    fn valid_checks() -> Vec<RedactionCheck> {
        vec![
            check(0x01, 3, 3, true, 8),
            check(0x02, 1, 1, true, 10),
            check(0x03, 0, 0, true, 0),
        ]
    }

    /// **AORC-01** — incomplete redaction rejected.
    #[test]
    fn aorc_01_incomplete_rejected() {
        let c = check(0x01, 3, 2, false, 8);
        assert_eq!(
            validate_redaction(&[c]),
            Err(RedactionError::UnredactedContent(0))
        );
    }

    /// **AORC-02** — short redaction rejected.
    #[test]
    fn aorc_02_short_rejected() {
        let c = check(0x01, 2, 2, true, 2);
        assert_eq!(
            validate_redaction(&[c]),
            Err(RedactionError::ShortRedaction {
                idx: 0,
                len: 2,
                min: AORC_MIN_REDACT_LEN,
            })
        );
    }

    /// **AORC-03** — unredacted content rejected.
    #[test]
    fn aorc_03_unredacted_rejected() {
        let c = RedactionCheck {
            output_id: oid(0x01),
            expected_redactions: 2,
            actual_redactions: 2,
            all_redacted: false,
            min_redact_len: 8,
        };
        assert_eq!(
            validate_redaction(&[c]),
            Err(RedactionError::UnredactedContent(0))
        );
    }

    /// **AORC-04** — count mismatch rejected.
    #[test]
    fn aorc_04_count_mismatch_rejected() {
        let c = check(0x01, 3, 2, true, 8);
        assert_eq!(
            validate_redaction(&[c]),
            Err(RedactionError::CountMismatch {
                idx: 0,
                expected: 3,
                actual: 2,
            })
        );
    }

    /// **AORC-05** — zero output ID rejected.
    #[test]
    fn aorc_05_zero_id_rejected() {
        let c = RedactionCheck {
            output_id: [0u8; AORC_OUTPUT_ID_LEN],
            expected_redactions: 1,
            actual_redactions: 1,
            all_redacted: true,
            min_redact_len: 8,
        };
        assert_eq!(
            validate_redaction(&[c]),
            Err(RedactionError::ZeroOutputId(0))
        );
    }

    /// **AORC-06** — too many rejected.
    #[test]
    fn aorc_06_too_many_rejected() {
        let cs: Vec<RedactionCheck> = (0..=AORC_MAX_OUTPUTS)
            .map(|i| {
                let b = (i as u8).wrapping_add(1);
                RedactionCheck {
                    output_id: oid(b),
                    expected_redactions: 0,
                    actual_redactions: 0,
                    all_redacted: true,
                    min_redact_len: 0,
                }
            })
            .collect();
        assert_eq!(
            validate_redaction(&cs),
            Err(RedactionError::TooMany {
                got: AORC_MAX_OUTPUTS + 1,
                max: AORC_MAX_OUTPUTS,
            })
        );
    }

    /// **AORC-07** — valid accepted.
    #[test]
    fn aorc_07_valid_accepted() {
        assert_eq!(validate_redaction(&valid_checks()), Ok(()));
    }

    /// **AORC-08** — empty accepted.
    #[test]
    fn aorc_08_empty_accepted() {
        assert_eq!(validate_redaction(&[]), Ok(()));
    }

    /// **AORC-09** — zero redactions accepted.
    #[test]
    fn aorc_09_zero_redactions_accepted() {
        let c = check(0x01, 0, 0, true, 0);
        assert_eq!(validate_redaction(&[c]), Ok(()));
    }

    /// **AORC-10** — boundary redaction length accepted.
    #[test]
    fn aorc_10_boundary_len_accepted() {
        let c = check(0x01, 1, 1, true, AORC_MIN_REDACT_LEN);
        assert_eq!(validate_redaction(&[c]), Ok(()));
    }
}
