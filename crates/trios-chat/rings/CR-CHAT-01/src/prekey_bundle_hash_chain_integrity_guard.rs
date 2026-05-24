//! # CR-CHAT-01 — Prekey bundle hash chain integrity guard (Wave-128 Lane B)
//!
//! IDENTITY — prekey bundle hashes must form a valid chain; a broken
//! chain indicates tampered or missing bundles.
//!
//! Prekey bundles are published in sequence, each containing a hash
//! of the previous bundle. This creates a tamper-evident chain:
//!
//! * **Tampering detection** — modifying a bundle's content changes
//!   its hash, breaking the chain for all subsequent bundles.
//! * **Missing bundle detection** — a gap in the chain means a
//!   bundle was suppressed or lost in transit.
//! * **Replay protection** — the chain sequence number prevents
//!   replaying an old bundle as the current one.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. First bundle must start at `PBHC_GENESIS_SEQ`.
//! 2. Sequence numbers must be contiguous.
//! 3. Hash must not be zero.
//! 4. Each bundle's prev_hash must equal the previous bundle's hash.
//! 5. No duplicate sequence numbers.
//! 6. Total bundles <= `PBHC_MAX_BUNDLES`.
//!
//! Tests **PBHC-01..10**. Error enum [`BundleChainError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * BUNDLE-CHAIN`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Genesis sequence number.
pub const PBHC_GENESIS_SEQ: u64 = 1;

/// Maximum bundles per batch.
pub const PBHC_MAX_BUNDLES: usize = 1024;

/// Hash length.
pub const PBHC_HASH_LEN: usize = 32;

/// Bundle ID length.
pub const PBHC_BUNDLE_ID_LEN: usize = 32;

/// A prekey bundle in the hash chain.
#[derive(Debug, Clone)]
pub struct BundleChainEntry {
    /// Bundle identifier.
    pub bundle_id: [u8; PBHC_BUNDLE_ID_LEN],
    /// Sequence number.
    pub seq: u64,
    /// Hash of this bundle.
    pub bundle_hash: [u8; PBHC_HASH_LEN],
    /// Hash of the previous bundle (zero for genesis).
    pub prev_hash: [u8; PBHC_HASH_LEN],
}

/// All ways bundle chain validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BundleChainError {
    /// Not at genesis.
    NotGenesis { idx: usize, seq: u64, expected: u64 },
    /// Gap in sequence.
    Gap { idx: usize, expected: u64, found: u64 },
    /// Zero hash.
    ZeroHash(usize),
    /// Chain broken.
    ChainBroken { idx: usize, expected_prev: [u8; PBHC_HASH_LEN], found_prev: [u8; PBHC_HASH_LEN] },
    /// Duplicate sequence.
    DuplicateSeq { idx: usize, seq: u64 },
    /// Too many bundles.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate prekey bundle hash chain integrity.
pub fn validate_bundle_chain(
    bundles: &[BundleChainEntry],
) -> Result<(), BundleChainError> {
    if bundles.len() > PBHC_MAX_BUNDLES {
        return Err(BundleChainError::TooMany {
            got: bundles.len(),
            max: PBHC_MAX_BUNDLES,
        });
    }
    let mut seen: BTreeSet<u64> = BTreeSet::new();
    let mut prev_hash: [u8; PBHC_HASH_LEN] = [0u8; PBHC_HASH_LEN];
    for (i, b) in bundles.iter().enumerate() {
        if b.bundle_hash == [0u8; PBHC_HASH_LEN] {
            return Err(BundleChainError::ZeroHash(i));
        }
        if i == 0 {
            if b.seq != PBHC_GENESIS_SEQ {
                return Err(BundleChainError::NotGenesis {
                    idx: 0,
                    seq: b.seq,
                    expected: PBHC_GENESIS_SEQ,
                });
            }
        } else {
            let expected_seq = bundles[i - 1].seq + 1;
            if b.seq != expected_seq {
                return Err(BundleChainError::Gap {
                    idx: i,
                    expected: expected_seq,
                    found: b.seq,
                });
            }
            if b.prev_hash != prev_hash {
                return Err(BundleChainError::ChainBroken {
                    idx: i,
                    expected_prev: prev_hash,
                    found_prev: b.prev_hash,
                });
            }
        }
        if !seen.insert(b.seq) {
            return Err(BundleChainError::DuplicateSeq { idx: i, seq: b.seq });
        }
        prev_hash = b.bundle_hash;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bid(byte: u8) -> [u8; PBHC_BUNDLE_ID_LEN] {
        [byte; PBHC_BUNDLE_ID_LEN]
    }

    fn hash(byte: u8) -> [u8; PBHC_HASH_LEN] {
        [byte; PBHC_HASH_LEN]
    }

    fn valid_chain() -> Vec<BundleChainEntry> {
        vec![
            BundleChainEntry { bundle_id: bid(0x01), seq: 1, bundle_hash: hash(0xA1), prev_hash: [0u8; PBHC_HASH_LEN] },
            BundleChainEntry { bundle_id: bid(0x02), seq: 2, bundle_hash: hash(0xA2), prev_hash: hash(0xA1) },
            BundleChainEntry { bundle_id: bid(0x03), seq: 3, bundle_hash: hash(0xA3), prev_hash: hash(0xA2) },
        ]
    }

    /// **PBHC-01** — not genesis rejected.
    #[test]
    fn pbhc_01_not_genesis_rejected() {
        let bs = vec![BundleChainEntry { bundle_id: bid(0x01), seq: 5, bundle_hash: hash(0xA1), prev_hash: [0u8; PBHC_HASH_LEN] }];
        assert_eq!(
            validate_bundle_chain(&bs),
            Err(BundleChainError::NotGenesis { idx: 0, seq: 5, expected: PBHC_GENESIS_SEQ })
        );
    }

    /// **PBHC-02** — gap rejected.
    #[test]
    fn pbhc_02_gap_rejected() {
        let bs = vec![
            BundleChainEntry { bundle_id: bid(0x01), seq: 1, bundle_hash: hash(0xA1), prev_hash: [0u8; PBHC_HASH_LEN] },
            BundleChainEntry { bundle_id: bid(0x02), seq: 5, bundle_hash: hash(0xA2), prev_hash: hash(0xA1) },
        ];
        assert_eq!(
            validate_bundle_chain(&bs),
            Err(BundleChainError::Gap { idx: 1, expected: 2, found: 5 })
        );
    }

    /// **PBHC-03** — zero hash rejected.
    #[test]
    fn pbhc_03_zero_hash_rejected() {
        let bs = vec![BundleChainEntry { bundle_id: bid(0x01), seq: 1, bundle_hash: [0u8; PBHC_HASH_LEN], prev_hash: [0u8; PBHC_HASH_LEN] }];
        assert_eq!(
            validate_bundle_chain(&bs),
            Err(BundleChainError::ZeroHash(0))
        );
    }

    /// **PBHC-04** — chain broken rejected.
    #[test]
    fn pbhc_04_chain_broken_rejected() {
        let bs = vec![
            BundleChainEntry { bundle_id: bid(0x01), seq: 1, bundle_hash: hash(0xA1), prev_hash: [0u8; PBHC_HASH_LEN] },
            BundleChainEntry { bundle_id: bid(0x02), seq: 2, bundle_hash: hash(0xA2), prev_hash: hash(0xBB) },
        ];
        assert_eq!(
            validate_bundle_chain(&bs),
            Err(BundleChainError::ChainBroken {
                idx: 1,
                expected_prev: hash(0xA1),
                found_prev: hash(0xBB),
            })
        );
    }

    /// **PBHC-05** — duplicate seq rejected.
    #[test]
    fn pbhc_05_duplicate_seq_rejected() {
        let bs = vec![
            BundleChainEntry { bundle_id: bid(0x01), seq: 1, bundle_hash: hash(0xA1), prev_hash: [0u8; PBHC_HASH_LEN] },
            BundleChainEntry { bundle_id: bid(0x02), seq: 2, bundle_hash: hash(0xA2), prev_hash: hash(0xA1) },
            BundleChainEntry { bundle_id: bid(0x03), seq: 2, bundle_hash: hash(0xA3), prev_hash: hash(0xA2) },
        ];
        assert_eq!(
            validate_bundle_chain(&bs),
            Err(BundleChainError::Gap { idx: 2, expected: 3, found: 2 })
        );
    }

    /// **PBHC-06** — too many rejected.
    #[test]
    fn pbhc_06_too_many_rejected() {
        let mut bs: Vec<BundleChainEntry> = Vec::with_capacity(PBHC_MAX_BUNDLES + 1);
        let mut prev = [0u8; PBHC_HASH_LEN];
        for i in 0..=PBHC_MAX_BUNDLES {
            let mut h = [0u8; PBHC_HASH_LEN];
            h[0] = (i as u8).wrapping_add(1);
            let mut id = [0u8; PBHC_BUNDLE_ID_LEN];
            id[0] = (i as u8).wrapping_add(1);
            bs.push(BundleChainEntry { bundle_id: id, seq: (i as u64) + 1, bundle_hash: h, prev_hash: prev });
            prev = h;
        }
        assert_eq!(
            validate_bundle_chain(&bs),
            Err(BundleChainError::TooMany {
                got: PBHC_MAX_BUNDLES + 1,
                max: PBHC_MAX_BUNDLES,
            })
        );
    }

    /// **PBHC-07** — valid accepted.
    #[test]
    fn pbhc_07_valid_accepted() {
        assert_eq!(validate_bundle_chain(&valid_chain()), Ok(()));
    }

    /// **PBHC-08** — empty accepted.
    #[test]
    fn pbhc_08_empty_accepted() {
        assert_eq!(validate_bundle_chain(&[]), Ok(()));
    }

    /// **PBHC-09** — single genesis accepted.
    #[test]
    fn pbhc_09_single_genesis_accepted() {
        let bs = vec![BundleChainEntry { bundle_id: bid(0x01), seq: 1, bundle_hash: hash(0xAA), prev_hash: [0u8; PBHC_HASH_LEN] }];
        assert_eq!(validate_bundle_chain(&bs), Ok(()));
    }

    /// **PBHC-10** — long chain accepted.
    #[test]
    fn pbhc_10_long_chain_accepted() {
        let mut bs = Vec::new();
        let mut prev = [0u8; PBHC_HASH_LEN];
        for i in 0..100u64 {
            let mut h = [0u8; PBHC_HASH_LEN];
            h[0..8].copy_from_slice(&(i + 1).to_be_bytes());
            let mut id = [0u8; PBHC_BUNDLE_ID_LEN];
            id[0] = (i as u8).wrapping_add(1);
            bs.push(BundleChainEntry { bundle_id: id, seq: i + 1, bundle_hash: h, prev_hash: prev });
            prev = h;
        }
        assert_eq!(validate_bundle_chain(&bs), Ok(()));
    }
}
