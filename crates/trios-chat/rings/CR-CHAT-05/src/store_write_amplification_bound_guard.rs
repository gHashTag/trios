//! # CR-CHAT-05 — Store write amplification bound guard (Wave-149 Lane B)
//!
//! PERSISTENCE — store write amplification must be bounded; excessive
//! amplification indicates write storms or corruption.
//!
//! Write amplification is the ratio of physical writes to logical
//! writes. If amplification exceeds the expected bound:
//!
//! * **Write storm** — a burst of writes can overwhelm the storage
//!   subsystem, causing latency spikes.
//! * **Compaction thrashing** — excessive compaction creates a
//!   feedback loop of more writes.
//! * **SSD wear** — high amplification accelerates SSD wear,
//!   reducing device lifetime.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Amplification factor <= `SWAB_MAX_AMP_NUM / SWAB_MAX_AMP_DEN`.
//! 2. Session ID must not be zero.
//! 3. No duplicate session IDs.
//! 4. Logical writes must be > 0.
//! 5. Physical writes must be >= logical writes.
//! 6. Batch size <= `SWAB_MAX_SESSIONS`.
//!
//! Tests **SWAB-01..10**. Error enum [`WriteAmpError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * AMP-BOUNDED`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum amplification numerator.
pub const SWAB_MAX_AMP_NUM: u64 = 10;

/// Maximum amplification denominator.
pub const SWAB_MAX_AMP_DEN: u64 = 1;

/// Maximum sessions per batch.
pub const SWAB_MAX_SESSIONS: usize = 256;

/// Session ID length.
pub const SWAB_SESSION_ID_LEN: usize = 32;

/// A write amplification record.
#[derive(Debug, Clone)]
pub struct WriteAmpRecord {
    /// Session identifier.
    pub session_id: [u8; SWAB_SESSION_ID_LEN],
    /// Logical write count.
    pub logical_writes: u64,
    /// Physical write count.
    pub physical_writes: u64,
}

/// All ways write amplification validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WriteAmpError {
    /// Amplification too high.
    TooHigh {
        /// Index.
        idx: usize,
        /// Computed amplification ×1000.
        amp_x1000: u64,
        /// Maximum ×1000.
        max_x1000: u64,
    },
    /// Zero session ID.
    ZeroSessionId(
        /// Index.
        usize,
    ),
    /// Duplicate session ID.
    DuplicateSessionId {
        /// Index.
        idx: usize,
    },
    /// Zero logical writes.
    ZeroLogical(
        /// Index.
        usize,
    ),
    /// Physical < logical.
    PhysicalLessThanLogical {
        /// Index.
        idx: usize,
    },
    /// Too many sessions.
    TooMany {
        /// Actual count.
        got: usize,
        /// Maximum allowed.
        max: usize,
    },
}

/// `[VERIFIED]` Validate store write amplification bound.
pub fn validate_write_amplification(
    records: &[WriteAmpRecord],
) -> Result<(), WriteAmpError> {
    if records.len() > SWAB_MAX_SESSIONS {
        return Err(WriteAmpError::TooMany {
            got: records.len(),
            max: SWAB_MAX_SESSIONS,
        });
    }
    let mut seen: BTreeSet<[u8; SWAB_SESSION_ID_LEN]> = BTreeSet::new();
    let max_x1000 = SWAB_MAX_AMP_NUM * 1000 / SWAB_MAX_AMP_DEN;
    for (i, r) in records.iter().enumerate() {
        if r.session_id == [0u8; SWAB_SESSION_ID_LEN] {
            return Err(WriteAmpError::ZeroSessionId(i));
        }
        if !seen.insert(r.session_id) {
            return Err(WriteAmpError::DuplicateSessionId { idx: i });
        }
        if r.logical_writes == 0 {
            return Err(WriteAmpError::ZeroLogical(i));
        }
        if r.physical_writes < r.logical_writes {
            return Err(WriteAmpError::PhysicalLessThanLogical { idx: i });
        }
        let amp_x1000 = (r.physical_writes * 1000) / r.logical_writes;
        if amp_x1000 > max_x1000 {
            return Err(WriteAmpError::TooHigh {
                idx: i,
                amp_x1000,
                max_x1000,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sid(byte: u8) -> [u8; SWAB_SESSION_ID_LEN] {
        [byte; SWAB_SESSION_ID_LEN]
    }

    fn rec(session: u8, logical: u64, physical: u64) -> WriteAmpRecord {
        WriteAmpRecord { session_id: sid(session), logical_writes: logical, physical_writes: physical }
    }

    fn valid_records() -> Vec<WriteAmpRecord> {
        vec![
            rec(0x01, 100, 300),
            rec(0x02, 200, 800),
        ]
    }

    /// **SWAB-01** — too high rejected.
    #[test]
    fn swab_01_too_high_rejected() {
        let r = rec(0x01, 10, 200);
        let max_x1000 = SWAB_MAX_AMP_NUM * 1000 / SWAB_MAX_AMP_DEN;
        assert_eq!(
            validate_write_amplification(&[r]),
            Err(WriteAmpError::TooHigh {
                idx: 0,
                amp_x1000: 20000,
                max_x1000,
            })
        );
    }

    /// **SWAB-02** — zero session ID rejected.
    #[test]
    fn swab_02_zero_session_rejected() {
        let r = WriteAmpRecord { session_id: [0u8; SWAB_SESSION_ID_LEN], logical_writes: 100, physical_writes: 300 };
        assert_eq!(
            validate_write_amplification(&[r]),
            Err(WriteAmpError::ZeroSessionId(0))
        );
    }

    /// **SWAB-03** — duplicate session ID rejected.
    #[test]
    fn swab_03_duplicate_rejected() {
        let rs = vec![
            rec(0x01, 100, 300),
            rec(0x01, 200, 800),
        ];
        assert_eq!(
            validate_write_amplification(&rs),
            Err(WriteAmpError::DuplicateSessionId { idx: 1 })
        );
    }

    /// **SWAB-04** — zero logical rejected.
    #[test]
    fn swab_04_zero_logical_rejected() {
        let r = WriteAmpRecord { session_id: sid(0x01), logical_writes: 0, physical_writes: 100 };
        assert_eq!(
            validate_write_amplification(&[r]),
            Err(WriteAmpError::ZeroLogical(0))
        );
    }

    /// **SWAB-05** — physical < logical rejected.
    #[test]
    fn swab_05_physical_less_rejected() {
        let r = rec(0x01, 100, 50);
        assert_eq!(
            validate_write_amplification(&[r]),
            Err(WriteAmpError::PhysicalLessThanLogical { idx: 0 })
        );
    }

    /// **SWAB-06** — too many rejected.
    #[test]
    fn swab_06_too_many_rejected() {
        let rs: Vec<WriteAmpRecord> = (0..=SWAB_MAX_SESSIONS)
            .map(|i| {
                let mut s = [0u8; SWAB_SESSION_ID_LEN];
                let val = (i as u64) + 1;
                s[0..8].copy_from_slice(&val.to_be_bytes());
                WriteAmpRecord { session_id: s, logical_writes: 100, physical_writes: 300 }
            })
            .collect();
        assert_eq!(
            validate_write_amplification(&rs),
            Err(WriteAmpError::TooMany {
                got: SWAB_MAX_SESSIONS + 1,
                max: SWAB_MAX_SESSIONS,
            })
        );
    }

    /// **SWAB-07** — valid accepted.
    #[test]
    fn swab_07_valid_accepted() {
        assert_eq!(validate_write_amplification(&valid_records()), Ok(()));
    }

    /// **SWAB-08** — empty accepted.
    #[test]
    fn swab_08_empty_accepted() {
        assert_eq!(validate_write_amplification(&[]), Ok(()));
    }

    /// **SWAB-09** — boundary amplification accepted.
    #[test]
    fn swab_09_boundary_accepted() {
        let r = rec(0x01, 1, SWAB_MAX_AMP_NUM / SWAB_MAX_AMP_DEN);
        assert_eq!(validate_write_amplification(&[r]), Ok(()));
    }

    /// **SWAB-10** — 1:1 ratio accepted.
    #[test]
    fn swab_10_one_to_one_accepted() {
        let r = rec(0x01, 100, 100);
        assert_eq!(validate_write_amplification(&[r]), Ok(()));
    }
}
