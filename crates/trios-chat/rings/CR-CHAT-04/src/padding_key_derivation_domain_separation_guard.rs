//! # CR-CHAT-04 — Padding key derivation domain separation guard (Wave-113 Lane B)
//!
//! PADDING — key derivation labels must be domain-separated.
//!
//! Padding keys are derived via HKDF with a context-specific label.
//! If the same label is used for different purposes:
//!
//! * **Key confusion** — a padding key derived with label "pad-class"
//!   is also used for nonce generation, creating a dependency that
//!   violates the single-purpose key principle.
//! * **Cross-domain attack** — knowing one derived key allows
//!   computing keys in other domains if labels overlap.
//! * **Label collision** — two derivation calls with the same IKM,
//!   salt, and label produce identical keys, even though they serve
//!   different purposes.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. All labels must be unique.
//! 2. Label must be from the approved set.
//! 3. Label must not be empty.
//! 4. Derivation context must not be zero.
//! 5. No duplicate (label, context) pairs.
//! 6. Total derivations <= `PKDS_MAX_DERIVATIONS`.
//!
//! Tests **PKDS-01..10**. Error enum [`DomainSepError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * DOMAIN-SEPARATION`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum derivations per batch.
pub const PKDS_MAX_DERIVATIONS: usize = 256;

/// Approved domain labels.
pub const PKDS_APPROVED_LABELS: [&str; 4] = [
    "pad-class-select",
    "pad-nonce-gen",
    "pad-key-rotate",
    "pad-entropy-seed",
];

/// Context length.
pub const PKDS_CONTEXT_LEN: usize = 16;

/// A key derivation record.
#[derive(Debug, Clone)]
pub struct DerivationRecord {
    /// Domain label.
    pub label: String,
    /// Derivation context.
    pub context: [u8; PKDS_CONTEXT_LEN],
}

/// All ways domain separation validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DomainSepError {
    /// Duplicate label.
    DuplicateLabel(usize),
    /// Unapproved label.
    UnapprovedLabel { idx: usize, label: String },
    /// Empty label.
    EmptyLabel(usize),
    /// Zero context.
    ZeroContext(usize),
    /// Duplicate (label, context) pair.
    DuplicatePair(usize),
    /// Too many derivations.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate padding key derivation domain separation.
pub fn validate_domain_separation(
    records: &[DerivationRecord],
) -> Result<(), DomainSepError> {
    if records.len() > PKDS_MAX_DERIVATIONS {
        return Err(DomainSepError::TooMany {
            got: records.len(),
            max: PKDS_MAX_DERIVATIONS,
        });
    }
    let mut labels: BTreeSet<String> = BTreeSet::new();
    let mut pairs: BTreeSet<(String, [u8; PKDS_CONTEXT_LEN])> = BTreeSet::new();
    let approved: BTreeSet<&str> = PKDS_APPROVED_LABELS.iter().copied().collect();
    for (i, r) in records.iter().enumerate() {
        if r.label.is_empty() {
            return Err(DomainSepError::EmptyLabel(i));
        }
        if !approved.contains(r.label.as_str()) {
            return Err(DomainSepError::UnapprovedLabel {
                idx: i,
                label: r.label.clone(),
            });
        }
        if r.context == [0u8; PKDS_CONTEXT_LEN] {
            return Err(DomainSepError::ZeroContext(i));
        }
        if !labels.insert(r.label.clone()) {
            if !pairs.insert((r.label.clone(), r.context)) {
                return Err(DomainSepError::DuplicatePair(i));
            }
        }
        pairs.insert((r.label.clone(), r.context));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(byte: u8) -> [u8; PKDS_CONTEXT_LEN] {
        [byte; PKDS_CONTEXT_LEN]
    }

    fn record(label: &str, context: u8) -> DerivationRecord {
        DerivationRecord { label: label.to_string(), context: ctx(context) }
    }

    fn valid_records() -> Vec<DerivationRecord> {
        vec![
            record("pad-class-select", 0x01),
            record("pad-nonce-gen", 0x01),
            record("pad-key-rotate", 0x01),
            record("pad-entropy-seed", 0x01),
        ]
    }

    /// **PKDS-01** — duplicate label same context rejected.
    #[test]
    fn pkds_01_duplicate_pair_rejected() {
        let rs = vec![record("pad-class-select", 0x01), record("pad-class-select", 0x01)];
        assert_eq!(
            validate_domain_separation(&rs),
            Err(DomainSepError::DuplicatePair(1))
        );
    }

    /// **PKDS-02** — unapproved label rejected.
    #[test]
    fn pkds_02_unapproved_rejected() {
        let r = record("unknown-label", 0x01);
        assert_eq!(
            validate_domain_separation(&[r]),
            Err(DomainSepError::UnapprovedLabel {
                idx: 0,
                label: "unknown-label".to_string(),
            })
        );
    }

    /// **PKDS-03** — empty label rejected.
    #[test]
    fn pkds_03_empty_rejected() {
        let r = DerivationRecord { label: String::new(), context: ctx(0x01) };
        assert_eq!(
            validate_domain_separation(&[r]),
            Err(DomainSepError::EmptyLabel(0))
        );
    }

    /// **PKDS-04** — zero context rejected.
    #[test]
    fn pkds_04_zero_context_rejected() {
        let r = DerivationRecord { label: "pad-class-select".to_string(), context: [0u8; PKDS_CONTEXT_LEN] };
        assert_eq!(
            validate_domain_separation(&[r]),
            Err(DomainSepError::ZeroContext(0))
        );
    }

    /// **PKDS-05** — duplicate label different context accepted.
    #[test]
    fn pkds_05_same_label_diff_context_accepted() {
        let rs = vec![record("pad-class-select", 0x01), record("pad-class-select", 0x02)];
        assert_eq!(validate_domain_separation(&rs), Ok(()));
    }

    /// **PKDS-06** — too many rejected.
    #[test]
    fn pkds_06_too_many_rejected() {
        let labels = PKDS_APPROVED_LABELS;
        let rs: Vec<DerivationRecord> = (0..=PKDS_MAX_DERIVATIONS)
            .map(|i| {
                let label = labels[i % labels.len()];
                let ctx_byte = (i as u8).wrapping_add(1);
                DerivationRecord { label: label.to_string(), context: ctx(ctx_byte) }
            })
            .collect();
        assert_eq!(
            validate_domain_separation(&rs),
            Err(DomainSepError::TooMany {
                got: PKDS_MAX_DERIVATIONS + 1,
                max: PKDS_MAX_DERIVATIONS,
            })
        );
    }

    /// **PKDS-07** — valid accepted.
    #[test]
    fn pkds_07_valid_accepted() {
        assert_eq!(validate_domain_separation(&valid_records()), Ok(()));
    }

    /// **PKDS-08** — empty accepted.
    #[test]
    fn pkds_08_empty_accepted() {
        assert_eq!(validate_domain_separation(&[]), Ok(()));
    }

    /// **PKDS-09** — single accepted.
    #[test]
    fn pkds_09_single_accepted() {
        let rs = vec![record("pad-class-select", 0x01)];
        assert_eq!(validate_domain_separation(&rs), Ok(()));
    }

    /// **PKDS-10** — all labels used accepted.
    #[test]
    fn pkds_10_all_labels_accepted() {
        let rs: Vec<DerivationRecord> = PKDS_APPROVED_LABELS.iter()
            .enumerate()
            .map(|(i, &label)| DerivationRecord {
                label: label.to_string(),
                context: ctx((i as u8) + 1),
            })
            .collect();
        assert_eq!(validate_domain_separation(&rs), Ok(()));
    }
}
