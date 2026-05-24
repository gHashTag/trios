//! # CR-CHAT-06 — Agent output format validation guard (Wave-124 Lane B)
//!
//! AGENT SAFETY — agent outputs must conform to declared content type;
//! format mismatches enable injection attacks via content type confusion.
//!
//! When an agent declares an output content type (e.g., "text/plain"),
//! the actual output must conform:
//!
//! * **Content type confusion** — declaring "text/plain" but emitting
//!   HTML/JS enables cross-site scripting through the agent output.
//! * **MIME sniffing** — browsers may interpret declared text as HTML
//!   if it contains angle brackets, enabling script injection.
//! * **Format downgrade** — switching from structured (JSON) to
//!   unstructured output mid-session bypasses output validators.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Content type must be in `AOFV_APPROVED_TYPES`.
//! 2. Output must not contain forbidden patterns for its type.
//! 3. Output ID must not be zero.
//! 4. No duplicate output IDs.
//! 5. Output length must be <= `AOFV_MAX_LEN`.
//! 6. Total outputs <= `AOFV_MAX_OUTPUTS`.
//!
//! Tests **AOFV-01..10**. Error enum [`FormatValidationError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * FORMAT-VALID`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Approved content types.
pub const AOFV_APPROVED_TYPES: &[&str] = &[
    "text/plain",
    "application/json",
    "text/markdown",
    "application/octet-stream",
];

/// Forbidden patterns for text/plain output.
pub const AOFV_TEXT_FORBIDDEN: &[&str] = &["<script", "<iframe", "javascript:", "onerror="];

/// Maximum output length.
pub const AOFV_MAX_LEN: usize = 65536;

/// Maximum outputs per batch.
pub const AOFV_MAX_OUTPUTS: usize = 1024;

/// Output ID length.
pub const AOFV_OUTPUT_ID_LEN: usize = 32;

/// An agent output record.
#[derive(Debug, Clone)]
pub struct OutputRecord {
    /// Output identifier.
    pub output_id: [u8; AOFV_OUTPUT_ID_LEN],
    /// Declared content type.
    pub content_type: String,
    /// Output payload.
    pub payload: String,
}

/// All ways format validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FormatValidationError {
    /// Content type not approved.
    UnapprovedType { idx: usize, content_type: String },
    /// Forbidden pattern found in text output.
    ForbiddenPattern { idx: usize, pattern: String },
    /// Zero output ID.
    ZeroOutputId(usize),
    /// Duplicate output ID.
    DuplicateOutputId { idx: usize },
    /// Output too long.
    TooLong { idx: usize, got: usize, max: usize },
    /// Too many outputs.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate agent output format.
pub fn validate_output_format(
    outputs: &[OutputRecord],
) -> Result<(), FormatValidationError> {
    if outputs.len() > AOFV_MAX_OUTPUTS {
        return Err(FormatValidationError::TooMany {
            got: outputs.len(),
            max: AOFV_MAX_OUTPUTS,
        });
    }
    let mut seen: BTreeSet<[u8; AOFV_OUTPUT_ID_LEN]> = BTreeSet::new();
    for (i, o) in outputs.iter().enumerate() {
        if o.output_id == [0u8; AOFV_OUTPUT_ID_LEN] {
            return Err(FormatValidationError::ZeroOutputId(i));
        }
        if !seen.insert(o.output_id) {
            return Err(FormatValidationError::DuplicateOutputId { idx: i });
        }
        if !AOFV_APPROVED_TYPES.contains(&o.content_type.as_str()) {
            return Err(FormatValidationError::UnapprovedType {
                idx: i,
                content_type: o.content_type.clone(),
            });
        }
        if o.payload.len() > AOFV_MAX_LEN {
            return Err(FormatValidationError::TooLong {
                idx: i,
                got: o.payload.len(),
                max: AOFV_MAX_LEN,
            });
        }
        if o.content_type == "text/plain" {
            let lower = o.payload.to_lowercase();
            for &pat in AOFV_TEXT_FORBIDDEN {
                if lower.contains(pat) {
                    return Err(FormatValidationError::ForbiddenPattern {
                        idx: i,
                        pattern: pat.to_string(),
                    });
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(byte: u8) -> [u8; AOFV_OUTPUT_ID_LEN] {
        [byte; AOFV_OUTPUT_ID_LEN]
    }

    fn output(id: u8, ct: &str, payload: &str) -> OutputRecord {
        OutputRecord { output_id: oid(id), content_type: ct.to_string(), payload: payload.to_string() }
    }

    fn valid_outputs() -> Vec<OutputRecord> {
        vec![
            output(0x01, "text/plain", "Hello, world!"),
            output(0x02, "application/json", r#"{"status":"ok"}"#),
            output(0x03, "text/markdown", "# Header\nContent"),
        ]
    }

    /// **AOFV-01** — unapproved type rejected.
    #[test]
    fn aofv_01_unapproved_type_rejected() {
        let os = vec![output(0x01, "text/html", "<p>hello</p>")];
        assert_eq!(
            validate_output_format(&os),
            Err(FormatValidationError::UnapprovedType { idx: 0, content_type: "text/html".to_string() })
        );
    }

    /// **AOFV-02** — forbidden pattern rejected.
    #[test]
    fn aofv_02_forbidden_pattern_rejected() {
        let os = vec![output(0x01, "text/plain", "click <script>alert(1)</script>")];
        assert_eq!(
            validate_output_format(&os),
            Err(FormatValidationError::ForbiddenPattern { idx: 0, pattern: "<script".to_string() })
        );
    }

    /// **AOFV-03** — zero output ID rejected.
    #[test]
    fn aofv_03_zero_id_rejected() {
        let o = OutputRecord { output_id: [0u8; AOFV_OUTPUT_ID_LEN], content_type: "text/plain".to_string(), payload: "ok".to_string() };
        assert_eq!(
            validate_output_format(&[o]),
            Err(FormatValidationError::ZeroOutputId(0))
        );
    }

    /// **AOFV-04** — duplicate output ID rejected.
    #[test]
    fn aofv_04_duplicate_rejected() {
        let os = vec![
            output(0x01, "text/plain", "a"),
            output(0x01, "text/plain", "b"),
        ];
        assert_eq!(
            validate_output_format(&os),
            Err(FormatValidationError::DuplicateOutputId { idx: 1 })
        );
    }

    /// **AOFV-05** — too long rejected.
    #[test]
    fn aofv_05_too_long_rejected() {
        let os = vec![OutputRecord { output_id: oid(0x01), content_type: "text/plain".to_string(), payload: "x".repeat(AOFV_MAX_LEN + 1) }];
        assert_eq!(
            validate_output_format(&os),
            Err(FormatValidationError::TooLong { idx: 0, got: AOFV_MAX_LEN + 1, max: AOFV_MAX_LEN })
        );
    }

    /// **AOFV-06** — too many rejected.
    #[test]
    fn aofv_06_too_many_rejected() {
        let os: Vec<OutputRecord> = (0..=AOFV_MAX_OUTPUTS)
            .map(|i| {
                let mut id = [0u8; AOFV_OUTPUT_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                OutputRecord { output_id: id, content_type: "text/plain".to_string(), payload: "ok".to_string() }
            })
            .collect();
        assert_eq!(
            validate_output_format(&os),
            Err(FormatValidationError::TooMany {
                got: AOFV_MAX_OUTPUTS + 1,
                max: AOFV_MAX_OUTPUTS,
            })
        );
    }

    /// **AOFV-07** — valid accepted.
    #[test]
    fn aofv_07_valid_accepted() {
        assert_eq!(validate_output_format(&valid_outputs()), Ok(()));
    }

    /// **AOFV-08** — empty accepted.
    #[test]
    fn aofv_08_empty_accepted() {
        assert_eq!(validate_output_format(&[]), Ok(()));
    }

    /// **AOFV-09** — iframe forbidden rejected.
    #[test]
    fn aofv_09_iframe_rejected() {
        let os = vec![output(0x01, "text/plain", "<iframe src=evil>")];
        assert_eq!(
            validate_output_format(&os),
            Err(FormatValidationError::ForbiddenPattern { idx: 0, pattern: "<iframe".to_string() })
        );
    }

    /// **AOFV-10** — JSON with angle brackets accepted.
    #[test]
    fn aofv_10_json_with_brackets_accepted() {
        let os = vec![output(0x01, "application/json", r#"{"html":"<div>ok</div>"}"#)];
        assert_eq!(validate_output_format(&os), Ok(()));
    }
}
