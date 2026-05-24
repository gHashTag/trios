//! # CR-CHAT-02 — Root key derivation context label guard (Wave-140 Lane A)
//!
//! RATCHET — root key derivations must use unique context labels;
//! reusing labels across contexts enables cross-protocol attacks.
//!
//! The Double Ratchet derives keys using labeled KDF invocations.
//! Each derivation context (e.g. "initial-root", "dh-step",
//! "application", "ticket") must use a unique label:
//!
//! * **Cross-protocol attack** — if two derivation contexts share a
//!   label, key material from one context can be used in another.
//! * **Key confusion** — the same label with different inputs
//!   produces related outputs, leaking information about inputs.
//! * **Audit ambiguity** — without unique labels, auditors cannot
//!   determine which protocol step produced a given key.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. All labels in a batch must be unique.
//! 2. Label length must be >= `RKDL_MIN_LABEL_LEN`.
//! 3. Label length must be <= `RKDL_MAX_LABEL_LEN`.
//! 4. Context ID must not be zero.
//! 5. No duplicate context IDs.
//! 6. Batch size <= `RKDL_MAX_DERIVATIONS`.
//!
//! Tests **RKDL-01..10**. Error enum [`ContextLabelError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * LABEL-UNIQUE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Minimum label length.
pub const RKDL_MIN_LABEL_LEN: usize = 4;

/// Maximum label length.
pub const RKDL_MAX_LABEL_LEN: usize = 64;

/// Maximum derivations per batch.
pub const RKDL_MAX_DERIVATIONS: usize = 256;

/// Context ID length.
pub const RKDL_CONTEXT_ID_LEN: usize = 16;

/// A root key derivation context record.
#[derive(Debug, Clone)]
pub struct DerivationContextRecord {
    /// Context identifier.
    pub context_id: [u8; RKDL_CONTEXT_ID_LEN],
    /// Derivation label.
    pub label: Vec<u8>,
}

/// All ways context label validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContextLabelError {
    /// Duplicate label.
    DuplicateLabel {
        /// Index of the duplicate.
        idx: usize,
    },
    /// Label too short.
    TooShort {
        /// Index.
        idx: usize,
        /// Actual length.
        got: usize,
        /// Minimum length.
        min: usize,
    },
    /// Label too long.
    TooLong {
        /// Index.
        idx: usize,
        /// Actual length.
        got: usize,
        /// Maximum length.
        max: usize,
    },
    /// Zero context ID.
    ZeroContextId(usize),
    /// Duplicate context ID.
    DuplicateContextId {
        /// Index.
        idx: usize,
    },
    /// Batch too large.
    TooLarge {
        /// Actual count.
        got: usize,
        /// Maximum allowed.
        max: usize,
    },
}

/// `[VERIFIED]` Validate root key derivation context labels.
pub fn validate_context_labels(
    records: &[DerivationContextRecord],
) -> Result<(), ContextLabelError> {
    if records.len() > RKDL_MAX_DERIVATIONS {
        return Err(ContextLabelError::TooLarge {
            got: records.len(),
            max: RKDL_MAX_DERIVATIONS,
        });
    }
    let mut seen_contexts: BTreeSet<[u8; RKDL_CONTEXT_ID_LEN]> = BTreeSet::new();
    let mut seen_labels: BTreeSet<&[u8]> = BTreeSet::new();
    for (i, r) in records.iter().enumerate() {
        if r.context_id == [0u8; RKDL_CONTEXT_ID_LEN] {
            return Err(ContextLabelError::ZeroContextId(i));
        }
        if !seen_contexts.insert(r.context_id) {
            return Err(ContextLabelError::DuplicateContextId { idx: i });
        }
        if r.label.len() < RKDL_MIN_LABEL_LEN {
            return Err(ContextLabelError::TooShort {
                idx: i,
                got: r.label.len(),
                min: RKDL_MIN_LABEL_LEN,
            });
        }
        if r.label.len() > RKDL_MAX_LABEL_LEN {
            return Err(ContextLabelError::TooLong {
                idx: i,
                got: r.label.len(),
                max: RKDL_MAX_LABEL_LEN,
            });
        }
        if !seen_labels.insert(&r.label) {
            return Err(ContextLabelError::DuplicateLabel { idx: i });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cid(byte: u8) -> [u8; RKDL_CONTEXT_ID_LEN] {
        [byte; RKDL_CONTEXT_ID_LEN]
    }

    fn rec(id: u8, label: &[u8]) -> DerivationContextRecord {
        DerivationContextRecord { context_id: cid(id), label: label.to_vec() }
    }

    fn valid_records() -> Vec<DerivationContextRecord> {
        vec![
            rec(0x01, b"initial-root"),
            rec(0x02, b"dh-step"),
            rec(0x03, b"application"),
        ]
    }

    /// **RKDL-01** — duplicate label rejected.
    #[test]
    fn rkdl_01_duplicate_label_rejected() {
        let rs = vec![
            rec(0x01, b"initial-root"),
            rec(0x02, b"initial-root"),
        ];
        assert_eq!(
            validate_context_labels(&rs),
            Err(ContextLabelError::DuplicateLabel { idx: 1 })
        );
    }

    /// **RKDL-02** — label too short rejected.
    #[test]
    fn rkdl_02_too_short_rejected() {
        let r = rec(0x01, b"ab");
        assert_eq!(
            validate_context_labels(&[r]),
            Err(ContextLabelError::TooShort {
                idx: 0,
                got: 2,
                min: RKDL_MIN_LABEL_LEN,
            })
        );
    }

    /// **RKDL-03** — label too long rejected.
    #[test]
    fn rkdl_03_too_long_rejected() {
        let r = DerivationContextRecord {
            context_id: cid(0x01),
            label: vec![b'x'; RKDL_MAX_LABEL_LEN + 1],
        };
        assert_eq!(
            validate_context_labels(&[r]),
            Err(ContextLabelError::TooLong {
                idx: 0,
                got: RKDL_MAX_LABEL_LEN + 1,
                max: RKDL_MAX_LABEL_LEN,
            })
        );
    }

    /// **RKDL-04** — zero context ID rejected.
    #[test]
    fn rkdl_04_zero_context_rejected() {
        let r = DerivationContextRecord {
            context_id: [0u8; RKDL_CONTEXT_ID_LEN],
            label: b"test-label".to_vec(),
        };
        assert_eq!(
            validate_context_labels(&[r]),
            Err(ContextLabelError::ZeroContextId(0))
        );
    }

    /// **RKDL-05** — duplicate context ID rejected.
    #[test]
    fn rkdl_05_duplicate_context_rejected() {
        let rs = vec![
            rec(0x01, b"label-a"),
            rec(0x01, b"label-b"),
        ];
        assert_eq!(
            validate_context_labels(&rs),
            Err(ContextLabelError::DuplicateContextId { idx: 1 })
        );
    }

    /// **RKDL-06** — batch too large rejected.
    #[test]
    fn rkdl_06_too_large_rejected() {
        let rs: Vec<DerivationContextRecord> = (0..=RKDL_MAX_DERIVATIONS)
            .map(|i| {
                let mut id = [0u8; RKDL_CONTEXT_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                let mut label = b"lbl-".to_vec();
                label.extend_from_slice(&val.to_be_bytes()[..4]);
                DerivationContextRecord { context_id: id, label }
            })
            .collect();
        assert_eq!(
            validate_context_labels(&rs),
            Err(ContextLabelError::TooLarge {
                got: RKDL_MAX_DERIVATIONS + 1,
                max: RKDL_MAX_DERIVATIONS,
            })
        );
    }

    /// **RKDL-07** — valid accepted.
    #[test]
    fn rkdl_07_valid_accepted() {
        assert_eq!(validate_context_labels(&valid_records()), Ok(()));
    }

    /// **RKDL-08** — empty accepted.
    #[test]
    fn rkdl_08_empty_accepted() {
        assert_eq!(validate_context_labels(&[]), Ok(()));
    }

    /// **RKDL-09** — boundary label length accepted.
    #[test]
    fn rkdl_09_boundary_label_accepted() {
        let r = rec(0x01, &[b'x'; RKDL_MIN_LABEL_LEN]);
        assert_eq!(validate_context_labels(&[r]), Ok(()));
    }

    /// **RKDL-10** — many unique labels accepted.
    #[test]
    fn rkdl_10_many_unique_accepted() {
        let rs: Vec<DerivationContextRecord> = (0..20u8)
            .map(|i| {
                let label = format!("context-label-{:02}", i);
                rec(i + 1, label.as_bytes())
            })
            .collect();
        assert_eq!(validate_context_labels(&rs), Ok(()));
    }
}
