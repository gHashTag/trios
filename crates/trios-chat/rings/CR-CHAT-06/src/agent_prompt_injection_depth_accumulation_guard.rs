//! # CR-CHAT-06 — Agent prompt injection depth accumulation guard (Wave-134 Lane B)
//!
//! AGENT SAFETY — the cumulative injection depth across all reprompts
//! in a session must not exceed a maximum; deep accumulation enables
//! context poisoning.
//!
//! Each reprompt in an agent session can introduce nested injection
//! attempts. The cumulative depth across all reprompts must be bounded:
//!
//! * **Context poisoning** — each layer of injection adds adversarial
//!   context, compounding the attack surface.
//! * **Resource exhaustion** — deep injection chains cause the agent
//!   to process exponentially more tokens.
//! * **Safety bypass** — at sufficient depth, safety filters become
//!   unreliable as the original intent is buried under layers of
//!   adversarial wrapping.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Cumulative depth <= `APID_MAX_CUMULATIVE_DEPTH`.
//! 2. Single reprompt depth <= `APID_MAX_SINGLE_DEPTH`.
//! 3. Session ID must not be zero.
//! 4. No duplicate session IDs.
//! 5. Depth per reprompt must be > 0.
//! 6. Total entries <= `APID_MAX_ENTRIES`.
//!
//! Tests **APID-01..10**. Error enum [`InjectionDepthAccumError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * DEPTH-LIMITED`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum cumulative injection depth across session.
pub const APID_MAX_CUMULATIVE_DEPTH: u64 = 50;

/// Maximum single reprompt depth.
pub const APID_MAX_SINGLE_DEPTH: u64 = 10;

/// Maximum entries per batch.
pub const APID_MAX_ENTRIES: usize = 1024;

/// Session ID length.
pub const APID_SESSION_ID_LEN: usize = 32;

/// A reprompt injection depth record.
#[derive(Debug, Clone)]
pub struct RepromptDepthRecord {
    /// Session identifier.
    pub session_id: [u8; APID_SESSION_ID_LEN],
    /// Injection depth of this reprompt.
    pub depth: u64,
}

/// All ways injection depth accumulation validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum InjectionDepthAccumError {
    /// Cumulative depth exceeded.
    CumulativeExceeded { total: u64, max: u64 },
    /// Single reprompt depth exceeded.
    SingleExceeded { idx: usize, got: u64, max: u64 },
    /// Zero session ID.
    ZeroSessionId(usize),
    /// Duplicate session ID.
    DuplicateSessionId { idx: usize },
    /// Zero depth.
    ZeroDepth(usize),
    /// Too many entries.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate agent prompt injection depth accumulation.
pub fn validate_injection_depth_accum(
    records: &[RepromptDepthRecord],
) -> Result<(), InjectionDepthAccumError> {
    if records.len() > APID_MAX_ENTRIES {
        return Err(InjectionDepthAccumError::TooMany {
            got: records.len(),
            max: APID_MAX_ENTRIES,
        });
    }
    let mut seen: BTreeSet<[u8; APID_SESSION_ID_LEN]> = BTreeSet::new();
    let mut total: u64 = 0;
    for (i, r) in records.iter().enumerate() {
        if r.session_id == [0u8; APID_SESSION_ID_LEN] {
            return Err(InjectionDepthAccumError::ZeroSessionId(i));
        }
        if r.depth == 0 {
            return Err(InjectionDepthAccumError::ZeroDepth(i));
        }
        if !seen.insert(r.session_id) {
            return Err(InjectionDepthAccumError::DuplicateSessionId { idx: i });
        }
        if r.depth > APID_MAX_SINGLE_DEPTH {
            return Err(InjectionDepthAccumError::SingleExceeded {
                idx: i,
                got: r.depth,
                max: APID_MAX_SINGLE_DEPTH,
            });
        }
        total += r.depth;
    }
    if total > APID_MAX_CUMULATIVE_DEPTH {
        return Err(InjectionDepthAccumError::CumulativeExceeded {
            total,
            max: APID_MAX_CUMULATIVE_DEPTH,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sid(byte: u8) -> [u8; APID_SESSION_ID_LEN] {
        [byte; APID_SESSION_ID_LEN]
    }

    fn rec(session: u8, depth: u64) -> RepromptDepthRecord {
        RepromptDepthRecord { session_id: sid(session), depth }
    }

    fn valid_records() -> Vec<RepromptDepthRecord> {
        vec![
            rec(0x01, 3),
            rec(0x02, 5),
            rec(0x03, 2),
            rec(0x04, 4),
        ]
    }

    /// **APID-01** — cumulative exceeded rejected.
    #[test]
    fn apid_01_cumulative_exceeded_rejected() {
        let rs: Vec<RepromptDepthRecord> = (0..6u8)
            .map(|i| rec(i + 1, APID_MAX_SINGLE_DEPTH))
            .collect();
        assert_eq!(
            validate_injection_depth_accum(&rs),
            Err(InjectionDepthAccumError::CumulativeExceeded {
                total: 60,
                max: APID_MAX_CUMULATIVE_DEPTH,
            })
        );
    }

    /// **APID-02** — single exceeded rejected.
    #[test]
    fn apid_02_single_exceeded_rejected() {
        let r = rec(0x01, APID_MAX_SINGLE_DEPTH + 1);
        assert_eq!(
            validate_injection_depth_accum(&[r]),
            Err(InjectionDepthAccumError::SingleExceeded {
                idx: 0,
                got: APID_MAX_SINGLE_DEPTH + 1,
                max: APID_MAX_SINGLE_DEPTH,
            })
        );
    }

    /// **APID-03** — zero session ID rejected.
    #[test]
    fn apid_03_zero_session_rejected() {
        let r = RepromptDepthRecord { session_id: [0u8; APID_SESSION_ID_LEN], depth: 5 };
        assert_eq!(
            validate_injection_depth_accum(&[r]),
            Err(InjectionDepthAccumError::ZeroSessionId(0))
        );
    }

    /// **APID-04** — duplicate session ID rejected.
    #[test]
    fn apid_04_duplicate_rejected() {
        let rs = vec![
            rec(0x01, 3),
            rec(0x01, 4),
        ];
        assert_eq!(
            validate_injection_depth_accum(&rs),
            Err(InjectionDepthAccumError::DuplicateSessionId { idx: 1 })
        );
    }

    /// **APID-05** — zero depth rejected.
    #[test]
    fn apid_05_zero_depth_rejected() {
        let r = RepromptDepthRecord { session_id: sid(0x01), depth: 0 };
        assert_eq!(
            validate_injection_depth_accum(&[r]),
            Err(InjectionDepthAccumError::ZeroDepth(0))
        );
    }

    /// **APID-06** — too many rejected.
    #[test]
    fn apid_06_too_many_rejected() {
        let rs: Vec<RepromptDepthRecord> = (0..=APID_MAX_ENTRIES)
            .map(|i| {
                let mut s = [0u8; APID_SESSION_ID_LEN];
                let val = (i as u64) + 1;
                s[0..8].copy_from_slice(&val.to_be_bytes());
                RepromptDepthRecord { session_id: s, depth: 1 }
            })
            .collect();
        assert_eq!(
            validate_injection_depth_accum(&rs),
            Err(InjectionDepthAccumError::TooMany {
                got: APID_MAX_ENTRIES + 1,
                max: APID_MAX_ENTRIES,
            })
        );
    }

    /// **APID-07** — valid accepted.
    #[test]
    fn apid_07_valid_accepted() {
        assert_eq!(validate_injection_depth_accum(&valid_records()), Ok(()));
    }

    /// **APID-08** — empty accepted.
    #[test]
    fn apid_08_empty_accepted() {
        assert_eq!(validate_injection_depth_accum(&[]), Ok(()));
    }

    /// **APID-09** — single boundary depth accepted.
    #[test]
    fn apid_09_single_boundary_accepted() {
        let rs = vec![rec(0x01, APID_MAX_SINGLE_DEPTH)];
        assert_eq!(validate_injection_depth_accum(&rs), Ok(()));
    }

    /// **APID-10** — cumulative boundary accepted.
    #[test]
    fn apid_10_cumulative_boundary_accepted() {
        let rs: Vec<RepromptDepthRecord> = (0..5u8)
            .map(|i| rec(i + 1, APID_MAX_CUMULATIVE_DEPTH / 5))
            .collect();
        assert_eq!(validate_injection_depth_accum(&rs), Ok(()));
    }
}
