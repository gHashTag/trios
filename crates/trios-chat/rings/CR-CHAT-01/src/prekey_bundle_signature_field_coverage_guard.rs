//! # CR-CHAT-01 — Prekey bundle signature field coverage guard (Wave-155 Lane A)
//!
//! IDENTITY — every field in a prekey bundle must be covered by the
//! signature; uncovered fields can be tampered with.
//!
//! In PQXDH, the prekey bundle contains identity key, signed prekey,
//! and one-time prekeys. If not all fields are covered by the
//! signature:
//!
//! * **Field tampering** — an attacker can modify uncovered fields
//!   without invalidating the signature.
//! * **Key substitution** — uncovered one-time prekeys can be
//!   replaced with attacker-controlled keys.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. All required field tags must be present.
//! 2. Bundle ID must not be zero.
//! 3. No duplicate bundle IDs.
//! 4. Field tag must not be zero.
//! 5. Coverage count <= `PBSF_MAX_FIELDS`.
//! 6. Batch size <= `PBSF_MAX_BUNDLES`.
//!
//! Tests **PBSF-01..10**. Error enum [`FieldCoverageError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * FIELD-COVERED`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum bundles per batch.
pub const PBSF_MAX_BUNDLES: usize = 256;

/// Maximum fields per bundle.
pub const PBSF_MAX_FIELDS: usize = 32;

/// Required field tags that must be covered.
pub const PBSF_REQUIRED_TAGS: &[u8] = &[0x01, 0x02, 0x03];

/// Bundle ID length.
pub const PBSF_BUNDLE_ID_LEN: usize = 32;

/// A field coverage record.
#[derive(Debug, Clone)]
pub struct CoverageRecord {
    /// Bundle identifier.
    pub bundle_id: [u8; PBSF_BUNDLE_ID_LEN],
    /// Field tags covered by the signature.
    pub covered_tags: Vec<u8>,
}

/// All ways field coverage validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FieldCoverageError {
    /// Missing required tag.
    MissingTag {
        idx: usize,
        missing: u8,
    },
    /// Zero bundle ID.
    ZeroBundleId(usize),
    /// Duplicate bundle ID.
    DuplicateBundleId {
        idx: usize,
    },
    /// Zero field tag.
    ZeroTag(usize),
    /// Too many fields.
    TooManyFields {
        idx: usize,
        got: usize,
        max: usize,
    },
    /// Too many bundles.
    TooMany {
        got: usize,
        max: usize,
    },
}

/// `[VERIFIED]` Validate prekey bundle signature field coverage.
pub fn validate_field_coverage(
    bundles: &[CoverageRecord],
) -> Result<(), FieldCoverageError> {
    if bundles.len() > PBSF_MAX_BUNDLES {
        return Err(FieldCoverageError::TooMany {
            got: bundles.len(),
            max: PBSF_MAX_BUNDLES,
        });
    }
    let mut seen: BTreeSet<[u8; PBSF_BUNDLE_ID_LEN]> = BTreeSet::new();
    for (i, b) in bundles.iter().enumerate() {
        if b.bundle_id == [0u8; PBSF_BUNDLE_ID_LEN] {
            return Err(FieldCoverageError::ZeroBundleId(i));
        }
        if !seen.insert(b.bundle_id) {
            return Err(FieldCoverageError::DuplicateBundleId { idx: i });
        }
        if b.covered_tags.len() > PBSF_MAX_FIELDS {
            return Err(FieldCoverageError::TooManyFields {
                idx: i,
                got: b.covered_tags.len(),
                max: PBSF_MAX_FIELDS,
            });
        }
        for &tag in &b.covered_tags {
            if tag == 0 {
                return Err(FieldCoverageError::ZeroTag(i));
            }
        }
        for &required in PBSF_REQUIRED_TAGS {
            if !b.covered_tags.contains(&required) {
                return Err(FieldCoverageError::MissingTag {
                    idx: i,
                    missing: required,
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bid(byte: u8) -> [u8; PBSF_BUNDLE_ID_LEN] {
        [byte; PBSF_BUNDLE_ID_LEN]
    }

    fn full_coverage(id: u8) -> CoverageRecord {
        CoverageRecord { bundle_id: bid(id), covered_tags: vec![0x01, 0x02, 0x03] }
    }

    fn valid_bundles() -> Vec<CoverageRecord> {
        vec![
            full_coverage(0x01),
            full_coverage(0x02),
            full_coverage(0x03),
        ]
    }

    /// **PBSF-01** — missing tag rejected.
    #[test]
    fn pbsf_01_missing_tag_rejected() {
        let b = CoverageRecord { bundle_id: bid(0x01), covered_tags: vec![0x01, 0x02] };
        assert_eq!(
            validate_field_coverage(&[b]),
            Err(FieldCoverageError::MissingTag { idx: 0, missing: 0x03 })
        );
    }

    /// **PBSF-02** — zero bundle ID rejected.
    #[test]
    fn pbsf_02_zero_bundle_rejected() {
        let b = CoverageRecord { bundle_id: [0u8; PBSF_BUNDLE_ID_LEN], covered_tags: vec![0x01, 0x02, 0x03] };
        assert_eq!(
            validate_field_coverage(&[b]),
            Err(FieldCoverageError::ZeroBundleId(0))
        );
    }

    /// **PBSF-03** — duplicate bundle ID rejected.
    #[test]
    fn pbsf_03_duplicate_rejected() {
        let bs = vec![full_coverage(0x01), full_coverage(0x01)];
        assert_eq!(
            validate_field_coverage(&bs),
            Err(FieldCoverageError::DuplicateBundleId { idx: 1 })
        );
    }

    /// **PBSF-04** — zero tag rejected.
    #[test]
    fn pbsf_04_zero_tag_rejected() {
        let b = CoverageRecord { bundle_id: bid(0x01), covered_tags: vec![0x00, 0x01, 0x02, 0x03] };
        assert_eq!(
            validate_field_coverage(&[b]),
            Err(FieldCoverageError::ZeroTag(0))
        );
    }

    /// **PBSF-05** — too many fields rejected.
    #[test]
    fn pbsf_05_too_many_fields_rejected() {
        let mut tags: Vec<u8> = (1..=PBSF_MAX_FIELDS as u8).collect();
        tags.push(0xFF);
        let b = CoverageRecord { bundle_id: bid(0x01), covered_tags: tags };
        assert_eq!(
            validate_field_coverage(&[b]),
            Err(FieldCoverageError::TooManyFields { idx: 0, got: PBSF_MAX_FIELDS + 1, max: PBSF_MAX_FIELDS })
        );
    }

    /// **PBSF-06** — too many bundles rejected.
    #[test]
    fn pbsf_06_too_many_rejected() {
        let bs: Vec<CoverageRecord> = (0..=PBSF_MAX_BUNDLES)
            .map(|i| {
                let mut id = [0u8; PBSF_BUNDLE_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                CoverageRecord { bundle_id: id, covered_tags: vec![0x01, 0x02, 0x03] }
            })
            .collect();
        assert_eq!(
            validate_field_coverage(&bs),
            Err(FieldCoverageError::TooMany {
                got: PBSF_MAX_BUNDLES + 1,
                max: PBSF_MAX_BUNDLES,
            })
        );
    }

    /// **PBSF-07** — valid accepted.
    #[test]
    fn pbsf_07_valid_accepted() {
        assert_eq!(validate_field_coverage(&valid_bundles()), Ok(()));
    }

    /// **PBSF-08** — empty accepted.
    #[test]
    fn pbsf_08_empty_accepted() {
        assert_eq!(validate_field_coverage(&[]), Ok(()));
    }

    /// **PBSF-09** — extra tags accepted.
    #[test]
    fn pbsf_09_extra_tags_accepted() {
        let b = CoverageRecord { bundle_id: bid(0x01), covered_tags: vec![0x01, 0x02, 0x03, 0x04, 0x05] };
        assert_eq!(validate_field_coverage(&[b]), Ok(()));
    }

    /// **PBSF-10** — many valid accepted.
    #[test]
    fn pbsf_10_many_valid_accepted() {
        let bs: Vec<CoverageRecord> = (0..20u8)
            .map(|i| full_coverage(i + 1))
            .collect();
        assert_eq!(validate_field_coverage(&bs), Ok(()));
    }
}
