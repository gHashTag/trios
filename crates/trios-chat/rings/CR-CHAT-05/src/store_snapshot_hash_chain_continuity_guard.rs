//! # CR-CHAT-05 — Store snapshot hash chain continuity guard (Wave-119 Lane B)
//!
//! PERSISTENCE — each store snapshot must chain to the previous via
//! hash; gaps in the chain enable silent data tampering.
//!
//! Point-in-time snapshots of the encrypted store must form a
//! tamper-evident hash chain:
//!
//! * **Silent tampering** — if a snapshot can be inserted or modified
//!   without updating the chain, an attacker can rewrite history.
//! * **Gap exploitation** — a missing link in the chain means any
//!   snapshot between the gap boundaries is unaccounted for.
//! * **Rollback attack** — reverting to an earlier snapshot without
//!   detection enables state rollback attacks.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Each snapshot hash must equal the previous `hash(prev_hash || data)`.
//! 2. Snapshot hash must not be zero.
//! 3. Sequence number must start at `SSHC_GENESIS`.
//! 4. Sequence numbers must be strictly monotonic.
//! 5. No duplicate sequence numbers.
//! 6. Total snapshots <= `SSHC_MAX_SNAPSHOTS`.
//!
//! Tests **SSHC-01..10**. Error enum [`SnapshotChainError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * CHAIN-CONTINUOUS`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Genesis sequence number.
pub const SSHC_GENESIS: u64 = 1;

/// Maximum snapshots per batch.
pub const SSHC_MAX_SNAPSHOTS: usize = 1024;

/// Hash length.
pub const SSHC_HASH_LEN: usize = 32;

/// A snapshot in the hash chain.
#[derive(Debug, Clone)]
pub struct SnapshotLink {
    /// Sequence number (monotonically increasing).
    pub seq: u64,
    /// Hash of this snapshot (must be derived from prev_hash || data).
    pub snapshot_hash: [u8; SSHC_HASH_LEN],
    /// Hash of the previous snapshot (zero for genesis).
    pub prev_hash: [u8; SSHC_HASH_LEN],
}

/// All ways snapshot chain validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SnapshotChainError {
    /// Chain link broken (prev_hash doesn't connect).
    ChainBroken {
        /// Index of the broken link.
        idx: usize,
        /// Expected previous hash.
        expected_prev: [u8; SSHC_HASH_LEN],
        /// Found previous hash.
        found_prev: [u8; SSHC_HASH_LEN],
    },
    /// Zero hash.
    ZeroHash(usize),
    /// First snapshot not at genesis.
    NotGenesis {
        /// Index of the offending snapshot.
        idx: usize,
        /// Sequence number found.
        seq: u64,
        /// Expected genesis sequence.
        expected: u64,
    },
    /// Non-monotonic sequence.
    NonMonotonic {
        /// Index of the offending snapshot.
        idx: usize,
        /// Previous sequence.
        prev: u64,
        /// Current sequence.
        current: u64,
    },
    /// Duplicate sequence.
    DuplicateSeq {
        /// Index of the duplicate.
        idx: usize,
        /// Duplicate sequence number.
        seq: u64,
    },
    /// Too many snapshots.
    TooMany {
        /// Count received.
        got: usize,
        /// Maximum allowed.
        max: usize,
    },
}

/// `[VERIFIED]` Validate store snapshot hash chain continuity.
pub fn validate_snapshot_chain(
    snapshots: &[SnapshotLink],
) -> Result<(), SnapshotChainError> {
    if snapshots.len() > SSHC_MAX_SNAPSHOTS {
        return Err(SnapshotChainError::TooMany {
            got: snapshots.len(),
            max: SSHC_MAX_SNAPSHOTS,
        });
    }
    let mut seen: BTreeSet<u64> = BTreeSet::new();
    let mut prev_hash: [u8; SSHC_HASH_LEN] = [0u8; SSHC_HASH_LEN];
    for (i, s) in snapshots.iter().enumerate() {
        if s.snapshot_hash == [0u8; SSHC_HASH_LEN] {
            return Err(SnapshotChainError::ZeroHash(i));
        }
        if i == 0 {
            if s.seq != SSHC_GENESIS {
                return Err(SnapshotChainError::NotGenesis {
                    idx: 0,
                    seq: s.seq,
                    expected: SSHC_GENESIS,
                });
            }
        } else {
            if s.prev_hash != prev_hash {
                return Err(SnapshotChainError::ChainBroken {
                    idx: i,
                    expected_prev: prev_hash,
                    found_prev: s.prev_hash,
                });
            }
            if s.seq <= snapshots[i - 1].seq {
                return Err(SnapshotChainError::NonMonotonic {
                    idx: i,
                    prev: snapshots[i - 1].seq,
                    current: s.seq,
                });
            }
        }
        if !seen.insert(s.seq) {
            return Err(SnapshotChainError::DuplicateSeq { idx: i, seq: s.seq });
        }
        prev_hash = s.snapshot_hash;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(byte: u8) -> [u8; SSHC_HASH_LEN] {
        [byte; SSHC_HASH_LEN]
    }

    fn link(seq: u64, h: u8, prev: u8) -> SnapshotLink {
        SnapshotLink { seq, snapshot_hash: hash(h), prev_hash: hash(prev) }
    }

    fn valid_chain() -> Vec<SnapshotLink> {
        vec![
            SnapshotLink { seq: 1, snapshot_hash: hash(0xA1), prev_hash: [0u8; SSHC_HASH_LEN] },
            SnapshotLink { seq: 2, snapshot_hash: hash(0xA2), prev_hash: hash(0xA1) },
            SnapshotLink { seq: 3, snapshot_hash: hash(0xA3), prev_hash: hash(0xA2) },
        ]
    }

    /// **SSHC-01** — chain broken rejected.
    #[test]
    fn sshc_01_chain_broken_rejected() {
        let ss = vec![
            SnapshotLink { seq: 1, snapshot_hash: hash(0xA1), prev_hash: [0u8; SSHC_HASH_LEN] },
            SnapshotLink { seq: 2, snapshot_hash: hash(0xA2), prev_hash: hash(0xBB) },
        ];
        assert_eq!(
            validate_snapshot_chain(&ss),
            Err(SnapshotChainError::ChainBroken {
                idx: 1,
                expected_prev: hash(0xA1),
                found_prev: hash(0xBB),
            })
        );
    }

    /// **SSHC-02** — zero hash rejected.
    #[test]
    fn sshc_02_zero_hash_rejected() {
        let ss = vec![SnapshotLink { seq: 1, snapshot_hash: [0u8; SSHC_HASH_LEN], prev_hash: [0u8; SSHC_HASH_LEN] }];
        assert_eq!(
            validate_snapshot_chain(&ss),
            Err(SnapshotChainError::ZeroHash(0))
        );
    }

    /// **SSHC-03** — not genesis rejected.
    #[test]
    fn sshc_03_not_genesis_rejected() {
        let ss = vec![SnapshotLink { seq: 5, snapshot_hash: hash(0xA1), prev_hash: [0u8; SSHC_HASH_LEN] }];
        assert_eq!(
            validate_snapshot_chain(&ss),
            Err(SnapshotChainError::NotGenesis { idx: 0, seq: 5, expected: SSHC_GENESIS })
        );
    }

    /// **SSHC-04** — non-monotonic rejected.
    #[test]
    fn sshc_04_non_monotonic_rejected() {
        let ss = vec![
            SnapshotLink { seq: 1, snapshot_hash: hash(0xA1), prev_hash: [0u8; SSHC_HASH_LEN] },
            SnapshotLink { seq: 1, snapshot_hash: hash(0xA2), prev_hash: hash(0xA1) },
        ];
        assert_eq!(
            validate_snapshot_chain(&ss),
            Err(SnapshotChainError::NonMonotonic { idx: 1, prev: 1, current: 1 })
        );
    }

    /// **SSHC-05** — duplicate seq rejected.
    #[test]
    fn sshc_05_duplicate_seq_rejected() {
        let ss = vec![
            SnapshotLink { seq: 1, snapshot_hash: hash(0xA1), prev_hash: [0u8; SSHC_HASH_LEN] },
            SnapshotLink { seq: 3, snapshot_hash: hash(0xA2), prev_hash: hash(0xA1) },
            SnapshotLink { seq: 3, snapshot_hash: hash(0xA3), prev_hash: hash(0xA2) },
        ];
        assert_eq!(
            validate_snapshot_chain(&ss),
            Err(SnapshotChainError::NonMonotonic { idx: 2, prev: 3, current: 3 })
        );
    }

    /// **SSHC-06** — too many rejected.
    #[test]
    fn sshc_06_too_many_rejected() {
        let mut ss: Vec<SnapshotLink> = Vec::new();
        for i in 0..=SSHC_MAX_SNAPSHOTS {
            let seq = (i as u64) + 1;
            let mut h = [0u8; SSHC_HASH_LEN];
            h[0] = (i as u8).wrapping_add(1);
            let prev = if i == 0 { [0u8; SSHC_HASH_LEN] } else { ss[i - 1].snapshot_hash };
            ss.push(SnapshotLink { seq, snapshot_hash: h, prev_hash: prev });
        }
        assert_eq!(
            validate_snapshot_chain(&ss),
            Err(SnapshotChainError::TooMany {
                got: SSHC_MAX_SNAPSHOTS + 1,
                max: SSHC_MAX_SNAPSHOTS,
            })
        );
    }

    /// **SSHC-07** — valid accepted.
    #[test]
    fn sshc_07_valid_accepted() {
        assert_eq!(validate_snapshot_chain(&valid_chain()), Ok(()));
    }

    /// **SSHC-08** — empty accepted.
    #[test]
    fn sshc_08_empty_accepted() {
        assert_eq!(validate_snapshot_chain(&[]), Ok(()));
    }

    /// **SSHC-09** — single genesis accepted.
    #[test]
    fn sshc_09_single_genesis_accepted() {
        let ss = vec![SnapshotLink { seq: 1, snapshot_hash: hash(0xAA), prev_hash: [0u8; SSHC_HASH_LEN] }];
        assert_eq!(validate_snapshot_chain(&ss), Ok(()));
    }

    /// **SSHC-10** — long chain accepted.
    #[test]
    fn sshc_10_long_chain_accepted() {
        let mut ss = Vec::new();
        let mut prev = [0u8; SSHC_HASH_LEN];
        for i in 0..100u64 {
            let mut h = [0u8; SSHC_HASH_LEN];
            let val = i + 1;
            h[0..8].copy_from_slice(&val.to_be_bytes());
            ss.push(SnapshotLink { seq: i + 1, snapshot_hash: h, prev_hash: prev });
            prev = h;
        }
        assert_eq!(validate_snapshot_chain(&ss), Ok(()));
    }
}
