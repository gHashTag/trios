//! # CR-CHAT-07 — Cover traffic replay detection guard (Wave-113 Lane A)
//!
//! ANTI-CORRELATION — cover traffic patterns must not repeat.
//!
//! Cover traffic emissions follow a deterministic schedule. If the
//! same emission pattern repeats across observation windows:
//!
//! * **Pattern fingerprinting** — the adversary builds a database of
//!   cover patterns and matches them to identify cover epochs.
//! * **Real vs. cover separation** — repeating patterns that don't
//!   match any known cover schedule are flagged as real traffic.
//! * **Temporal correlation** — matching patterns across windows
//!   reveals the cover schedule period.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. No duplicate emission pattern hashes across windows.
//! 2. Window index must be strictly increasing.
//! 3. Window index must not be zero.
//! 4. Pattern hash must not be zero.
//! 5. Emission count per window must be >= `CTRD_MIN_EMISSIONS`.
//! 6. Total windows <= `CTRD_MAX_WINDOWS`.
//!
//! Tests **CTRD-01..10**. Error enum [`CoverReplayError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * COVER-NO-REPLAY`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Minimum emissions per window.
pub const CTRD_MIN_EMISSIONS: usize = 10;

/// Maximum windows per batch.
pub const CTRD_MAX_WINDOWS: usize = 256;

/// Pattern hash length.
pub const CTRD_HASH_LEN: usize = 32;

/// A cover traffic window pattern.
#[derive(Debug, Clone)]
pub struct CoverWindow {
    /// Window index.
    pub index: u64,
    /// Hash of the emission pattern in this window.
    pub pattern_hash: [u8; CTRD_HASH_LEN],
    /// Number of emissions in this window.
    pub emission_count: usize,
}

/// All ways cover replay validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CoverReplayError {
    /// Duplicate pattern hash.
    DuplicatePattern(usize),
    /// Not increasing.
    NotIncreasing { idx: usize, prev: u64, current: u64 },
    /// Zero index.
    ZeroIndex(usize),
    /// Zero hash.
    ZeroHash(usize),
    /// Below minimum emissions.
    BelowMin { idx: usize, count: usize, min: usize },
    /// Too many windows.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate cover traffic replay detection.
pub fn validate_cover_replay(
    windows: &[CoverWindow],
) -> Result<(), CoverReplayError> {
    if windows.len() > CTRD_MAX_WINDOWS {
        return Err(CoverReplayError::TooMany {
            got: windows.len(),
            max: CTRD_MAX_WINDOWS,
        });
    }
    let mut seen: BTreeSet<[u8; CTRD_HASH_LEN]> = BTreeSet::new();
    let mut prev: u64 = 0;
    for (i, w) in windows.iter().enumerate() {
        if w.index == 0 {
            return Err(CoverReplayError::ZeroIndex(i));
        }
        if w.pattern_hash == [0u8; CTRD_HASH_LEN] {
            return Err(CoverReplayError::ZeroHash(i));
        }
        if w.emission_count < CTRD_MIN_EMISSIONS {
            return Err(CoverReplayError::BelowMin {
                idx: i,
                count: w.emission_count,
                min: CTRD_MIN_EMISSIONS,
            });
        }
        if i > 0 && w.index <= prev {
            return Err(CoverReplayError::NotIncreasing {
                idx: i,
                prev,
                current: w.index,
            });
        }
        if !seen.insert(w.pattern_hash) {
            return Err(CoverReplayError::DuplicatePattern(i));
        }
        prev = w.index;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(byte: u8) -> [u8; CTRD_HASH_LEN] {
        [byte; CTRD_HASH_LEN]
    }

    fn window(index: u64, hash_byte: u8, count: usize) -> CoverWindow {
        CoverWindow { index, pattern_hash: hash(hash_byte), emission_count: count }
    }

    fn valid_windows() -> Vec<CoverWindow> {
        vec![
            window(1, 0x01, 20),
            window(2, 0x02, 25),
            window(3, 0x03, 22),
        ]
    }

    /// **CTRD-01** — duplicate pattern rejected.
    #[test]
    fn ctrd_01_duplicate_rejected() {
        let ws = vec![window(1, 0xAA, 20), window(2, 0xAA, 25)];
        assert_eq!(
            validate_cover_replay(&ws),
            Err(CoverReplayError::DuplicatePattern(1))
        );
    }

    /// **CTRD-02** — not increasing rejected.
    #[test]
    fn ctrd_02_not_increasing_rejected() {
        let ws = vec![window(5, 0x01, 20), window(3, 0x02, 20)];
        assert_eq!(
            validate_cover_replay(&ws),
            Err(CoverReplayError::NotIncreasing {
                idx: 1,
                prev: 5,
                current: 3,
            })
        );
    }

    /// **CTRD-03** — zero index rejected.
    #[test]
    fn ctrd_03_zero_index_rejected() {
        let w = CoverWindow { index: 0, pattern_hash: hash(0x01), emission_count: 20 };
        assert_eq!(
            validate_cover_replay(&[w]),
            Err(CoverReplayError::ZeroIndex(0))
        );
    }

    /// **CTRD-04** — zero hash rejected.
    #[test]
    fn ctrd_04_zero_hash_rejected() {
        let w = CoverWindow { index: 1, pattern_hash: [0u8; CTRD_HASH_LEN], emission_count: 20 };
        assert_eq!(
            validate_cover_replay(&[w]),
            Err(CoverReplayError::ZeroHash(0))
        );
    }

    /// **CTRD-05** — below min rejected.
    #[test]
    fn ctrd_05_below_min_rejected() {
        let w = window(1, 0x01, 5);
        assert_eq!(
            validate_cover_replay(&[w]),
            Err(CoverReplayError::BelowMin {
                idx: 0,
                count: 5,
                min: CTRD_MIN_EMISSIONS,
            })
        );
    }

    /// **CTRD-06** — too many rejected.
    #[test]
    fn ctrd_06_too_many_rejected() {
        let ws: Vec<CoverWindow> = (0..=CTRD_MAX_WINDOWS)
            .map(|i| {
                let b = (i as u8).wrapping_add(1);
                CoverWindow { index: (i as u64) + 1, pattern_hash: hash(b), emission_count: 20 }
            })
            .collect();
        assert_eq!(
            validate_cover_replay(&ws),
            Err(CoverReplayError::TooMany {
                got: CTRD_MAX_WINDOWS + 1,
                max: CTRD_MAX_WINDOWS,
            })
        );
    }

    /// **CTRD-07** — valid accepted.
    #[test]
    fn ctrd_07_valid_accepted() {
        assert_eq!(validate_cover_replay(&valid_windows()), Ok(()));
    }

    /// **CTRD-08** — empty accepted.
    #[test]
    fn ctrd_08_empty_accepted() {
        assert_eq!(validate_cover_replay(&[]), Ok(()));
    }

    /// **CTRD-09** — single accepted.
    #[test]
    fn ctrd_09_single_accepted() {
        let ws = vec![window(1, 0x01, 20)];
        assert_eq!(validate_cover_replay(&ws), Ok(()));
    }

    /// **CTRD-10** — boundary emissions accepted.
    #[test]
    fn ctrd_10_boundary_accepted() {
        let ws = vec![window(1, 0x01, CTRD_MIN_EMISSIONS)];
        assert_eq!(validate_cover_replay(&ws), Ok(()));
    }
}
