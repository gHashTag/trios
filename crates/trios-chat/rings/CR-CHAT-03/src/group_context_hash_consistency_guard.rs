//! # CR-CHAT-03 — Group context hash consistency guard (Wave-59 Lane B)
//!
//! RATCHET TREE — all members must agree on group context hash, R-CHAT-4.
//!
//! Group context hash = hash(tree, epoch, members). If two members have
//! different context hashes, the group is in split-brain. An attacker can:
//!
//! * **Tamper Welcome** — inject a different tree into a Welcome for a
//!   new member.
//! * **Withhold Update** — not forward an UpdatePath to part of the group.
//! * **Inject divergent Commit** — create two versions of the same epoch.
//!
//! 1. All members report the same context hash.
//! 2. Hash length = `GCHC_HASH_LEN`.
//! 3. Member count >= `GCHC_MIN_MEMBERS`.
//! 4. No duplicate member IDs.
//! 5. Epoch > 0.
//! 6. Hash != all-zeros.
//!
//! Tests **GCHC-01..10**. Error enum [`GroupContextError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · GROUP-CONTEXT-HASH`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Expected hash length (SHA-256).
pub const GCHC_HASH_LEN: usize = 32;

/// Minimum members for consensus.
pub const GCHC_MIN_MEMBERS: usize = 2;

/// All ways group context consistency can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum GroupContextError {
    /// Hash mismatch between members.
    HashMismatch,
    /// Wrong hash length.
    WrongHashLength,
    /// Too few members.
    TooFewMembers,
    /// Duplicate member ID.
    DuplicateMember,
    /// Zero epoch.
    ZeroEpoch,
    /// All-zero hash.
    ZeroHash,
}

/// A member's reported group context.
#[derive(Debug, Clone)]
pub struct MemberContext {
    /// Member leaf index.
    pub member_id: u32,
    /// Epoch number.
    pub epoch: u64,
    /// Reported group context hash.
    pub context_hash: [u8; GCHC_HASH_LEN],
}

/// `[VERIFIED]` Validate that all members report the same group context.
pub fn validate_group_context_consistency(
    reports: &[MemberContext],
) -> Result<(), GroupContextError> {
    if reports.len() < GCHC_MIN_MEMBERS {
        return Err(GroupContextError::TooFewMembers);
    }
    let mut seen_ids = BTreeSet::new();
    let mut reference_hash: Option<&[u8; GCHC_HASH_LEN]> = None;
    for r in reports {
        if r.epoch == 0 {
            return Err(GroupContextError::ZeroEpoch);
        }
        if r.context_hash == [0u8; GCHC_HASH_LEN] {
            return Err(GroupContextError::ZeroHash);
        }
        if !seen_ids.insert(r.member_id) {
            return Err(GroupContextError::DuplicateMember);
        }
        match reference_hash {
            None => reference_hash = Some(&r.context_hash),
            Some(ref_hash) => {
                if r.context_hash != *ref_hash {
                    return Err(GroupContextError::HashMismatch);
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH_A: [u8; GCHC_HASH_LEN] = [0xAA; GCHC_HASH_LEN];
    const HASH_B: [u8; GCHC_HASH_LEN] = [0xBB; GCHC_HASH_LEN];

    fn ctx(id: u32, epoch: u64, hash: [u8; GCHC_HASH_LEN]) -> MemberContext {
        MemberContext { member_id: id, epoch, context_hash: hash }
    }

    fn good_reports() -> Vec<MemberContext> {
        vec![ctx(0, 5, HASH_A), ctx(1, 5, HASH_A), ctx(2, 5, HASH_A)]
    }

    /// **GCHC-01** — hash mismatch rejected.
    #[test]
    fn gchc_01_mismatch_rejected() {
        let r = vec![ctx(0, 5, HASH_A), ctx(1, 5, HASH_B)];
        assert_eq!(
            validate_group_context_consistency(&r),
            Err(GroupContextError::HashMismatch)
        );
    }

    /// **GCHC-02** — too few members rejected.
    #[test]
    fn gchc_02_too_few_rejected() {
        assert_eq!(
            validate_group_context_consistency(&[ctx(0, 1, HASH_A)]),
            Err(GroupContextError::TooFewMembers)
        );
    }

    /// **GCHC-03** — duplicate member rejected.
    #[test]
    fn gchc_03_duplicate_rejected() {
        let r = vec![ctx(0, 5, HASH_A), ctx(0, 5, HASH_A)];
        assert_eq!(
            validate_group_context_consistency(&r),
            Err(GroupContextError::DuplicateMember)
        );
    }

    /// **GCHC-04** — zero epoch rejected.
    #[test]
    fn gchc_04_zero_epoch_rejected() {
        let r = vec![ctx(0, 0, HASH_A), ctx(1, 0, HASH_A)];
        assert_eq!(
            validate_group_context_consistency(&r),
            Err(GroupContextError::ZeroEpoch)
        );
    }

    /// **GCHC-05** — zero hash rejected.
    #[test]
    fn gchc_05_zero_hash_rejected() {
        let r = vec![ctx(0, 1, [0u8; 32]), ctx(1, 1, [0u8; 32])];
        assert_eq!(
            validate_group_context_consistency(&r),
            Err(GroupContextError::ZeroHash)
        );
    }

    /// **GCHC-06** — consistent accepted.
    #[test]
    fn gchc_06_consistent_accepted() {
        assert_eq!(validate_group_context_consistency(&good_reports()), Ok(()));
    }

    /// **GCHC-07** — two members accepted.
    #[test]
    fn gchc_07_two_accepted() {
        let r = vec![ctx(0, 1, HASH_A), ctx(1, 1, HASH_A)];
        assert_eq!(validate_group_context_consistency(&r), Ok(()));
    }

    /// **GCHC-08** — large group accepted.
    #[test]
    fn gchc_08_large_accepted() {
        let r: Vec<MemberContext> = (0..50)
            .map(|i| ctx(i, 10, HASH_A))
            .collect();
        assert_eq!(validate_group_context_consistency(&r), Ok(()));
    }

    /// **GCHC-09** — mismatch at end rejected.
    #[test]
    fn gchc_09_mismatch_end_rejected() {
        let mut r: Vec<MemberContext> = (0..9)
            .map(|i| ctx(i, 5, HASH_A))
            .collect();
        r.push(ctx(9, 5, HASH_B));
        assert_eq!(
            validate_group_context_consistency(&r),
            Err(GroupContextError::HashMismatch)
        );
    }

    /// **GCHC-10** — empty reports → too few rejected.
    #[test]
    fn gchc_10_empty_rejected() {
        assert_eq!(
            validate_group_context_consistency(&[]),
            Err(GroupContextError::TooFewMembers)
        );
    }
}
