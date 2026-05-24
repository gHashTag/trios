//! # CR-CHAT-02 — Double ratchet chain key rotation bound guard (Wave-136 Lane B)
//!
//! RATCHET — the number of chain key rotations without a DH ratchet
//! step must be bounded; excessive rotations weaken forward secrecy.
//!
//! In the Double Ratchet algorithm, the sending/receiving chain
//! advances by hashing the chain key for each message. A DH ratchet
//! step introduces fresh entropy. If too many messages are sent
//! without a DH step:
//!
//! * **Forward secrecy gap** — compromise of one chain key reveals
//!   all subsequent chain keys until the next DH step.
//! * **Key wear** — repeatedly hashing the same chain key reduces
//!   the entropy budget over time.
//! * **Compounding risk** — each additional rotation without DH
//!   increases the window of vulnerability.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Rotations since last DH <= `DRCB_MAX_ROTATIONS`.
//! 2. Session ID must not be zero.
//! 3. No duplicate session IDs.
//! 4. Rotation count must be > 0.
//! 5. DH step flag must be set for first entry in session.
//! 6. Batch size <= `DRCB_MAX_BATCH`.
//!
//! Tests **DRCB-01..10**. Error enum [`RotationBoundError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * ROTATION-BOUNDED`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum rotations without DH step.
pub const DRCB_MAX_ROTATIONS: u64 = 1000;

/// Maximum batch size.
pub const DRCB_MAX_BATCH: usize = 1024;

/// Session ID length.
pub const DRCB_SESSION_ID_LEN: usize = 32;

/// A chain key rotation record.
#[derive(Debug, Clone)]
pub struct RotationRecord {
    /// Session identifier.
    pub session_id: [u8; DRCB_SESSION_ID_LEN],
    /// Number of rotations since last DH step.
    pub rotations_since_dh: u64,
    /// Whether a DH step has occurred at least once.
    pub dh_step_occurred: bool,
}

/// All ways rotation bound validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RotationBoundError {
    /// Too many rotations without DH.
    TooManyRotations { idx: usize, got: u64, max: u64 },
    /// Zero session ID.
    ZeroSessionId(usize),
    /// Duplicate session ID.
    DuplicateSessionId { idx: usize },
    /// Zero rotation count.
    ZeroRotations(usize),
    /// First entry must have DH step.
    FirstMissingDh(usize),
    /// Batch too large.
    TooLarge { got: usize, max: usize },
}

/// `[VERIFIED]` Validate double ratchet chain key rotation bound.
pub fn validate_rotation_bound(
    records: &[RotationRecord],
) -> Result<(), RotationBoundError> {
    if records.len() > DRCB_MAX_BATCH {
        return Err(RotationBoundError::TooLarge {
            got: records.len(),
            max: DRCB_MAX_BATCH,
        });
    }
    let mut seen: BTreeSet<[u8; DRCB_SESSION_ID_LEN]> = BTreeSet::new();
    for (i, r) in records.iter().enumerate() {
        if r.session_id == [0u8; DRCB_SESSION_ID_LEN] {
            return Err(RotationBoundError::ZeroSessionId(i));
        }
        if r.rotations_since_dh == 0 {
            return Err(RotationBoundError::ZeroRotations(i));
        }
        if !seen.insert(r.session_id) {
            return Err(RotationBoundError::DuplicateSessionId { idx: i });
        }
        if !r.dh_step_occurred {
            return Err(RotationBoundError::FirstMissingDh(i));
        }
        if r.rotations_since_dh > DRCB_MAX_ROTATIONS {
            return Err(RotationBoundError::TooManyRotations {
                idx: i,
                got: r.rotations_since_dh,
                max: DRCB_MAX_ROTATIONS,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sid(byte: u8) -> [u8; DRCB_SESSION_ID_LEN] {
        [byte; DRCB_SESSION_ID_LEN]
    }

    fn rec(session: u8, rotations: u64, dh: bool) -> RotationRecord {
        RotationRecord { session_id: sid(session), rotations_since_dh: rotations, dh_step_occurred: dh }
    }

    fn valid_records() -> Vec<RotationRecord> {
        vec![
            rec(0x01, 50, true),
            rec(0x02, 100, true),
            rec(0x03, 200, true),
        ]
    }

    /// **DRCB-01** — too many rotations rejected.
    #[test]
    fn drcb_01_too_many_rejected() {
        let r = rec(0x01, DRCB_MAX_ROTATIONS + 1, true);
        assert_eq!(
            validate_rotation_bound(&[r]),
            Err(RotationBoundError::TooManyRotations {
                idx: 0,
                got: DRCB_MAX_ROTATIONS + 1,
                max: DRCB_MAX_ROTATIONS,
            })
        );
    }

    /// **DRCB-02** — zero session ID rejected.
    #[test]
    fn drcb_02_zero_session_rejected() {
        let r = RotationRecord { session_id: [0u8; DRCB_SESSION_ID_LEN], rotations_since_dh: 10, dh_step_occurred: true };
        assert_eq!(
            validate_rotation_bound(&[r]),
            Err(RotationBoundError::ZeroSessionId(0))
        );
    }

    /// **DRCB-03** — duplicate session ID rejected.
    #[test]
    fn drcb_03_duplicate_rejected() {
        let rs = vec![
            rec(0x01, 10, true),
            rec(0x01, 20, true),
        ];
        assert_eq!(
            validate_rotation_bound(&rs),
            Err(RotationBoundError::DuplicateSessionId { idx: 1 })
        );
    }

    /// **DRCB-04** — zero rotations rejected.
    #[test]
    fn drcb_04_zero_rotations_rejected() {
        let r = RotationRecord { session_id: sid(0x01), rotations_since_dh: 0, dh_step_occurred: true };
        assert_eq!(
            validate_rotation_bound(&[r]),
            Err(RotationBoundError::ZeroRotations(0))
        );
    }

    /// **DRCB-05** — missing DH step rejected.
    #[test]
    fn drcb_05_missing_dh_rejected() {
        let r = RotationRecord { session_id: sid(0x01), rotations_since_dh: 10, dh_step_occurred: false };
        assert_eq!(
            validate_rotation_bound(&[r]),
            Err(RotationBoundError::FirstMissingDh(0))
        );
    }

    /// **DRCB-06** — batch too large rejected.
    #[test]
    fn drcb_06_too_large_rejected() {
        let rs: Vec<RotationRecord> = (0..=DRCB_MAX_BATCH)
            .map(|i| {
                let mut s = [0u8; DRCB_SESSION_ID_LEN];
                let val = (i as u64) + 1;
                s[0..8].copy_from_slice(&val.to_be_bytes());
                RotationRecord { session_id: s, rotations_since_dh: 1, dh_step_occurred: true }
            })
            .collect();
        assert_eq!(
            validate_rotation_bound(&rs),
            Err(RotationBoundError::TooLarge {
                got: DRCB_MAX_BATCH + 1,
                max: DRCB_MAX_BATCH,
            })
        );
    }

    /// **DRCB-07** — valid accepted.
    #[test]
    fn drcb_07_valid_accepted() {
        assert_eq!(validate_rotation_bound(&valid_records()), Ok(()));
    }

    /// **DRCB-08** — empty accepted.
    #[test]
    fn drcb_08_empty_accepted() {
        assert_eq!(validate_rotation_bound(&[]), Ok(()));
    }

    /// **DRCB-09** — boundary rotations accepted.
    #[test]
    fn drcb_09_boundary_accepted() {
        let r = rec(0x01, DRCB_MAX_ROTATIONS, true);
        assert_eq!(validate_rotation_bound(&[r]), Ok(()));
    }

    /// **DRCB-10** — many sessions accepted.
    #[test]
    fn drcb_10_many_sessions_accepted() {
        let rs: Vec<RotationRecord> = (0..20u8)
            .map(|i| rec(i + 1, (i as u64) * 10 + 1, true))
            .collect();
        assert_eq!(validate_rotation_bound(&rs), Ok(()));
    }
}
