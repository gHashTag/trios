//! # CR-CHAT-03 — TreeKEM group membership epoch continuity guard (Wave-156 Lane A)
//!
//! RATCHET TREE — group membership changes must follow strict epoch
//! ordering; gaps enable membership injection.
//!
//! In MLS, every membership change (add/remove/update) advances the
//! epoch counter by exactly 1. If epochs are skipped or reordered:
//!
//! * **Membership injection** — an attacker can inject unauthorized
//!   members during epoch gaps.
//! * **State desynchronization** — group members with inconsistent
//!   epoch views cannot agree on membership.
//! * **Replay attacks** — old membership operations can be replayed
//!   if epochs are not strictly monotonic.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Epoch must be strictly increasing (+1 each step).
//! 2. First epoch must be > 0.
//! 3. Operation ID must not be zero.
//! 4. No duplicate operation IDs.
//! 5. Operation type must be valid (Add=1, Remove=2, Update=3).
//! 6. Batch size <= `TGMC_MAX_OPS`.
//!
//! Tests **TGMC-01..10**. Error enum [`EpochContinuityError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * EPOCH-CONTINUOUS`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum operations per batch.
pub const TGMC_MAX_OPS: usize = 512;

/// Operation ID length.
pub const TGMC_OP_ID_LEN: usize = 16;

/// Valid operation types.
pub const TGMC_VALID_OP_TYPES: &[u8] = &[1, 2, 3];

/// A membership operation record.
#[derive(Debug, Clone)]
pub struct MembershipOp {
    /// Operation identifier.
    pub op_id: [u8; TGMC_OP_ID_LEN],
    /// Epoch number.
    pub epoch: u64,
    /// Operation type (1=Add, 2=Remove, 3=Update).
    pub op_type: u8,
}

/// All ways epoch continuity validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EpochContinuityError {
    /// Epoch not strictly incrementing by 1.
    NonContinuous {
        idx: usize,
        prev: u64,
        got: u64,
    },
    /// Zero epoch.
    ZeroEpoch(usize),
    /// Zero operation ID.
    ZeroOpId(usize),
    /// Duplicate operation ID.
    DuplicateOpId {
        idx: usize,
    },
    /// Invalid operation type.
    InvalidOpType {
        idx: usize,
        got: u8,
    },
    /// Too many operations.
    TooMany {
        got: usize,
        max: usize,
    },
}

/// `[VERIFIED]` Validate group membership epoch continuity.
pub fn validate_epoch_continuity(
    ops: &[MembershipOp],
) -> Result<(), EpochContinuityError> {
    if ops.len() > TGMC_MAX_OPS {
        return Err(EpochContinuityError::TooMany {
            got: ops.len(),
            max: TGMC_MAX_OPS,
        });
    }
    let mut seen: BTreeSet<[u8; TGMC_OP_ID_LEN]> = BTreeSet::new();
    let mut prev_epoch: Option<u64> = None;
    for (i, op) in ops.iter().enumerate() {
        if op.op_id == [0u8; TGMC_OP_ID_LEN] {
            return Err(EpochContinuityError::ZeroOpId(i));
        }
        if !seen.insert(op.op_id) {
            return Err(EpochContinuityError::DuplicateOpId { idx: i });
        }
        if op.epoch == 0 {
            return Err(EpochContinuityError::ZeroEpoch(i));
        }
        if let Some(pe) = prev_epoch {
            if op.epoch != pe + 1 {
                return Err(EpochContinuityError::NonContinuous {
                    idx: i,
                    prev: pe,
                    got: op.epoch,
                });
            }
        }
        if !TGMC_VALID_OP_TYPES.contains(&op.op_type) {
            return Err(EpochContinuityError::InvalidOpType {
                idx: i,
                got: op.op_type,
            });
        }
        prev_epoch = Some(op.epoch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(byte: u8) -> [u8; TGMC_OP_ID_LEN] {
        [byte; TGMC_OP_ID_LEN]
    }

    fn op(id: u8, epoch: u64, op_type: u8) -> MembershipOp {
        MembershipOp { op_id: oid(id), epoch, op_type }
    }

    fn valid_ops() -> Vec<MembershipOp> {
        vec![
            op(0x01, 1, 1),
            op(0x02, 2, 3),
            op(0x03, 3, 2),
            op(0x04, 4, 1),
        ]
    }

    /// **TGMC-01** — non-continuous epoch rejected.
    #[test]
    fn tgmc_01_non_continuous_rejected() {
        let ops = vec![
            op(0x01, 1, 1),
            op(0x02, 3, 1),
        ];
        assert_eq!(
            validate_epoch_continuity(&ops),
            Err(EpochContinuityError::NonContinuous { idx: 1, prev: 1, got: 3 })
        );
    }

    /// **TGMC-02** — zero epoch rejected.
    #[test]
    fn tgmc_02_zero_epoch_rejected() {
        let op = MembershipOp { op_id: oid(0x01), epoch: 0, op_type: 1 };
        assert_eq!(
            validate_epoch_continuity(&[op]),
            Err(EpochContinuityError::ZeroEpoch(0))
        );
    }

    /// **TGMC-03** — zero op ID rejected.
    #[test]
    fn tgmc_03_zero_op_id_rejected() {
        let op = MembershipOp { op_id: [0u8; TGMC_OP_ID_LEN], epoch: 1, op_type: 1 };
        assert_eq!(
            validate_epoch_continuity(&[op]),
            Err(EpochContinuityError::ZeroOpId(0))
        );
    }

    /// **TGMC-04** — duplicate op ID rejected.
    #[test]
    fn tgmc_04_duplicate_rejected() {
        let ops = vec![
            op(0x01, 1, 1),
            op(0x01, 2, 1),
        ];
        assert_eq!(
            validate_epoch_continuity(&ops),
            Err(EpochContinuityError::DuplicateOpId { idx: 1 })
        );
    }

    /// **TGMC-05** — invalid op type rejected.
    #[test]
    fn tgmc_05_invalid_op_type_rejected() {
        let op = MembershipOp { op_id: oid(0x01), epoch: 1, op_type: 99 };
        assert_eq!(
            validate_epoch_continuity(&[op]),
            Err(EpochContinuityError::InvalidOpType { idx: 0, got: 99 })
        );
    }

    /// **TGMC-06** — too many rejected.
    #[test]
    fn tgmc_06_too_many_rejected() {
        let ops: Vec<MembershipOp> = (0..=TGMC_MAX_OPS)
            .map(|i| {
                let mut id = [0u8; TGMC_OP_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                MembershipOp { op_id: id, epoch: val, op_type: 1 }
            })
            .collect();
        assert_eq!(
            validate_epoch_continuity(&ops),
            Err(EpochContinuityError::TooMany {
                got: TGMC_MAX_OPS + 1,
                max: TGMC_MAX_OPS,
            })
        );
    }

    /// **TGMC-07** — valid accepted.
    #[test]
    fn tgmc_07_valid_accepted() {
        assert_eq!(validate_epoch_continuity(&valid_ops()), Ok(()));
    }

    /// **TGMC-08** — empty accepted.
    #[test]
    fn tgmc_08_empty_accepted() {
        assert_eq!(validate_epoch_continuity(&[]), Ok(()));
    }

    /// **TGMC-09** — single accepted.
    #[test]
    fn tgmc_09_single_accepted() {
        assert_eq!(validate_epoch_continuity(&[op(0x01, 5, 2)]), Ok(()));
    }

    /// **TGMC-10** — long continuous chain accepted.
    #[test]
    fn tgmc_10_long_chain_accepted() {
        let ops: Vec<MembershipOp> = (0..20u8)
            .map(|i| op(i + 1, (i as u64) + 1, (i % 3) + 1))
            .collect();
        assert_eq!(validate_epoch_continuity(&ops), Ok(()));
    }
}
