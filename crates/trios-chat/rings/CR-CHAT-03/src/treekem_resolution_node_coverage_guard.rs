//! # CR-CHAT-03 — TreeKEM resolution node coverage guard (Wave-129 Lane A)
//!
//! RATCHET TREE — all resolution nodes must be covered during a tree
//! resolution; uncovered nodes represent gaps in the group's shared
//! secret.
//!
//! During TreeKEM resolution, unmerged leaves are resolved to produce
//! a complete set of path secrets. If some resolution nodes are
//! missing:
//!
//! * **Incomplete resolution** — the group secret is derived from
//!   only a subset of members, excluding others from decryption.
//! * **Secret gap** — uncovered nodes mean some members cannot
//!   derive the necessary keys to decrypt group messages.
//! * **Membership violation** — MLS requires that all members can
//!   derive the group secret; uncovered nodes violate this.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. All declared node positions must be covered.
//! 2. Node position must be <= `TRNC_MAX_POSITION`.
//! 3. Resolution ID must not be zero.
//! 4. No duplicate node positions within a resolution.
//! 5. Covered count must equal declared count.
//! 6. Total entries <= `TRNC_MAX_ENTRIES`.
//!
//! Tests **TRNC-01..10**. Error enum [`CoverageError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * FULL-COVERAGE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum node position.
pub const TRNC_MAX_POSITION: u32 = 65535;

/// Maximum entries per batch.
pub const TRNC_MAX_ENTRIES: usize = 2048;

/// Resolution ID length.
pub const TRNC_RESOLUTION_ID_LEN: usize = 32;

/// A resolution node coverage entry.
#[derive(Debug, Clone)]
pub struct CoverageEntry {
    /// Resolution identifier.
    pub resolution_id: [u8; TRNC_RESOLUTION_ID_LEN],
    /// Total declared nodes in the resolution.
    pub declared_count: u32,
    /// Node positions that are covered.
    pub covered_positions: Vec<u32>,
}

/// All ways coverage validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CoverageError {
    /// Not all declared nodes are covered.
    IncompleteCoverage { declared: u32, covered: u32 },
    /// Node position exceeds maximum.
    PositionTooHigh { position: u32, max: u32 },
    /// Zero resolution ID.
    ZeroResolutionId(usize),
    /// Duplicate node position.
    DuplicatePosition { position: u32 },
    /// Covered count mismatch.
    CountMismatch { declared: u32, actual: u32 },
    /// Too many entries.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate TreeKEM resolution node coverage.
pub fn validate_resolution_coverage(
    entries: &[CoverageEntry],
) -> Result<(), CoverageError> {
    if entries.len() > TRNC_MAX_ENTRIES {
        return Err(CoverageError::TooMany {
            got: entries.len(),
            max: TRNC_MAX_ENTRIES,
        });
    }
    for (i, e) in entries.iter().enumerate() {
        if e.resolution_id == [0u8; TRNC_RESOLUTION_ID_LEN] {
            return Err(CoverageError::ZeroResolutionId(i));
        }
        let actual = e.covered_positions.len() as u32;
        if actual != e.declared_count {
            return Err(CoverageError::CountMismatch {
                declared: e.declared_count,
                actual,
            });
        }
        let mut seen: BTreeSet<u32> = BTreeSet::new();
        for &pos in &e.covered_positions {
            if pos > TRNC_MAX_POSITION {
                return Err(CoverageError::PositionTooHigh {
                    position: pos,
                    max: TRNC_MAX_POSITION,
                });
            }
            if !seen.insert(pos) {
                return Err(CoverageError::DuplicatePosition { position: pos });
            }
        }
        if actual < e.declared_count {
            return Err(CoverageError::IncompleteCoverage {
                declared: e.declared_count,
                covered: actual,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rid(byte: u8) -> [u8; TRNC_RESOLUTION_ID_LEN] {
        [byte; TRNC_RESOLUTION_ID_LEN]
    }

    fn coverage(id: u8, declared: u32, positions: Vec<u32>) -> CoverageEntry {
        CoverageEntry { resolution_id: rid(id), declared_count: declared, covered_positions: positions }
    }

    fn valid_entries() -> Vec<CoverageEntry> {
        vec![
            coverage(0x01, 3, vec![0, 1, 2]),
            coverage(0x02, 4, vec![10, 20, 30, 40]),
        ]
    }

    /// **TRNC-01** — incomplete coverage rejected.
    #[test]
    fn trnc_01_incomplete_rejected() {
        let e = coverage(0x01, 5, vec![0, 1, 2]);
        assert_eq!(
            validate_resolution_coverage(&[e]),
            Err(CoverageError::CountMismatch { declared: 5, actual: 3 })
        );
    }

    /// **TRNC-02** — position too high rejected.
    #[test]
    fn trnc_02_position_too_high_rejected() {
        let e = coverage(0x01, 1, vec![TRNC_MAX_POSITION + 1]);
        assert_eq!(
            validate_resolution_coverage(&[e]),
            Err(CoverageError::PositionTooHigh {
                position: TRNC_MAX_POSITION + 1,
                max: TRNC_MAX_POSITION,
            })
        );
    }

    /// **TRNC-03** — zero resolution ID rejected.
    #[test]
    fn trnc_03_zero_id_rejected() {
        let e = CoverageEntry { resolution_id: [0u8; TRNC_RESOLUTION_ID_LEN], declared_count: 1, covered_positions: vec![0] };
        assert_eq!(
            validate_resolution_coverage(&[e]),
            Err(CoverageError::ZeroResolutionId(0))
        );
    }

    /// **TRNC-04** — duplicate position rejected.
    #[test]
    fn trnc_04_duplicate_position_rejected() {
        let e = coverage(0x01, 2, vec![5, 5]);
        assert_eq!(
            validate_resolution_coverage(&[e]),
            Err(CoverageError::DuplicatePosition { position: 5 })
        );
    }

    /// **TRNC-05** — count mismatch rejected.
    #[test]
    fn trnc_05_count_mismatch_rejected() {
        let e = coverage(0x01, 10, vec![0, 1, 2]);
        assert_eq!(
            validate_resolution_coverage(&[e]),
            Err(CoverageError::CountMismatch { declared: 10, actual: 3 })
        );
    }

    /// **TRNC-06** — too many rejected.
    #[test]
    fn trnc_06_too_many_rejected() {
        let es: Vec<CoverageEntry> = (0..=TRNC_MAX_ENTRIES)
            .map(|i| {
                let mut id = [0u8; TRNC_RESOLUTION_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                CoverageEntry { resolution_id: id, declared_count: 1, covered_positions: vec![i as u32] }
            })
            .collect();
        assert_eq!(
            validate_resolution_coverage(&es),
            Err(CoverageError::TooMany {
                got: TRNC_MAX_ENTRIES + 1,
                max: TRNC_MAX_ENTRIES,
            })
        );
    }

    /// **TRNC-07** — valid accepted.
    #[test]
    fn trnc_07_valid_accepted() {
        assert_eq!(validate_resolution_coverage(&valid_entries()), Ok(()));
    }

    /// **TRNC-08** — empty accepted.
    #[test]
    fn trnc_08_empty_accepted() {
        assert_eq!(validate_resolution_coverage(&[]), Ok(()));
    }

    /// **TRNC-09** — zero declared with zero positions accepted.
    #[test]
    fn trnc_09_zero_declared_accepted() {
        let e = coverage(0x01, 0, vec![]);
        assert_eq!(validate_resolution_coverage(&[e]), Ok(()));
    }

    /// **TRNC-10** — large coverage accepted.
    #[test]
    fn trnc_10_large_coverage_accepted() {
        let positions: Vec<u32> = (0..256).collect();
        let e = coverage(0x01, 256, positions);
        assert_eq!(validate_resolution_coverage(&[e]), Ok(()));
    }
}
