//! # CR-CHAT-06 — Agent response size burst bound guard (Wave-150 Lane B)
//!
//! AGENT SAFETY — agent responses must be bounded in size; oversized
//! responses indicate resource exhaustion or data exfiltration.
//!
//! Each agent response has an expected size range. If responses
//! exceed the maximum allowed size:
//!
//! * **Resource exhaustion** — processing oversized responses
//!   consumes excessive memory and CPU.
//! * **Data exfiltration** — an attacker-controlled agent can
//!   exfiltrate data by embedding it in oversized responses.
//! * **Denial of service** — flooding a client with oversized
//!   responses overwhelms the client's processing capacity.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Response size <= `ARSB_MAX_SIZE`.
//! 2. Burst total (sum of sizes) <= `ARSB_MAX_BURST_TOTAL`.
//! 3. Session ID must not be zero.
//! 4. No duplicate session IDs.
//! 5. Response ID must not be zero.
//! 6. Batch size <= `ARSB_MAX_RESPONSES`.
//!
//! Tests **ARSB-01..10**. Error enum [`ResponseBurstError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * RESPONSE-BOUNDED`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum single response size in bytes.
pub const ARSB_MAX_SIZE: usize = 1_048_576;

/// Maximum burst total in bytes.
pub const ARSB_MAX_BURST_TOTAL: usize = 10_485_760;

/// Maximum responses per batch.
pub const ARSB_MAX_RESPONSES: usize = 256;

/// Session ID length.
pub const ARSB_SESSION_ID_LEN: usize = 32;

/// Response ID length.
pub const ARSB_RESPONSE_ID_LEN: usize = 16;

/// A response size record.
#[derive(Debug, Clone)]
pub struct ResponseSizeRecord {
    /// Session identifier.
    pub session_id: [u8; ARSB_SESSION_ID_LEN],
    /// Response identifier.
    pub response_id: [u8; ARSB_RESPONSE_ID_LEN],
    /// Response size in bytes.
    pub size: usize,
}

/// All ways response burst validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResponseBurstError {
    /// Single response too large.
    TooLarge { idx: usize, got: usize, max: usize },
    /// Burst total exceeded.
    BurstTotal { total: usize, max: usize },
    /// Zero session ID.
    ZeroSessionId(usize),
    /// Duplicate session ID.
    DuplicateSessionId { idx: usize },
    /// Zero response ID.
    ZeroResponseId(usize),
    /// Too many responses.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate agent response size burst bound.
pub fn validate_response_burst(
    responses: &[ResponseSizeRecord],
) -> Result<(), ResponseBurstError> {
    if responses.len() > ARSB_MAX_RESPONSES {
        return Err(ResponseBurstError::TooMany {
            got: responses.len(),
            max: ARSB_MAX_RESPONSES,
        });
    }
    let mut seen_sessions: BTreeSet<[u8; ARSB_SESSION_ID_LEN]> = BTreeSet::new();
    let mut total: usize = 0;
    for (i, r) in responses.iter().enumerate() {
        if r.session_id == [0u8; ARSB_SESSION_ID_LEN] {
            return Err(ResponseBurstError::ZeroSessionId(i));
        }
        if r.response_id == [0u8; ARSB_RESPONSE_ID_LEN] {
            return Err(ResponseBurstError::ZeroResponseId(i));
        }
        if !seen_sessions.insert(r.session_id) {
            return Err(ResponseBurstError::DuplicateSessionId { idx: i });
        }
        if r.size > ARSB_MAX_SIZE {
            return Err(ResponseBurstError::TooLarge {
                idx: i,
                got: r.size,
                max: ARSB_MAX_SIZE,
            });
        }
        total += r.size;
    }
    if total > ARSB_MAX_BURST_TOTAL {
        return Err(ResponseBurstError::BurstTotal {
            total,
            max: ARSB_MAX_BURST_TOTAL,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sid(byte: u8) -> [u8; ARSB_SESSION_ID_LEN] {
        [byte; ARSB_SESSION_ID_LEN]
    }

    fn rid(byte: u8) -> [u8; ARSB_RESPONSE_ID_LEN] {
        [byte; ARSB_RESPONSE_ID_LEN]
    }

    fn resp(session: u8, response: u8, size: usize) -> ResponseSizeRecord {
        ResponseSizeRecord { session_id: sid(session), response_id: rid(response), size }
    }

    fn valid_responses() -> Vec<ResponseSizeRecord> {
        vec![
            resp(0x01, 0xA1, 1024),
            resp(0x02, 0xA2, 4096),
            resp(0x03, 0xA3, 8192),
        ]
    }

    /// **ARSB-01** — too large rejected.
    #[test]
    fn arsb_01_too_large_rejected() {
        let r = resp(0x01, 0xA1, ARSB_MAX_SIZE + 1);
        assert_eq!(
            validate_response_burst(&[r]),
            Err(ResponseBurstError::TooLarge {
                idx: 0,
                got: ARSB_MAX_SIZE + 1,
                max: ARSB_MAX_SIZE,
            })
        );
    }

    /// **ARSB-02** — burst total exceeded.
    #[test]
    fn arsb_02_burst_total_rejected() {
        let rs: Vec<ResponseSizeRecord> = (0..20u8)
            .map(|i| {
                let mut s = [0u8; ARSB_SESSION_ID_LEN];
                let val = (i as u64) + 1;
                s[0..8].copy_from_slice(&val.to_be_bytes());
                ResponseSizeRecord {
                    session_id: s,
                    response_id: rid(i + 1),
                    size: ARSB_MAX_SIZE,
                }
            })
            .collect();
        assert_eq!(
            validate_response_burst(&rs),
            Err(ResponseBurstError::BurstTotal {
                total: 20 * ARSB_MAX_SIZE,
                max: ARSB_MAX_BURST_TOTAL,
            })
        );
    }

    /// **ARSB-03** — zero session ID rejected.
    #[test]
    fn arsb_03_zero_session_rejected() {
        let r = ResponseSizeRecord {
            session_id: [0u8; ARSB_SESSION_ID_LEN],
            response_id: rid(0xA1),
            size: 1024,
        };
        assert_eq!(
            validate_response_burst(&[r]),
            Err(ResponseBurstError::ZeroSessionId(0))
        );
    }

    /// **ARSB-04** — duplicate session ID rejected.
    #[test]
    fn arsb_04_duplicate_rejected() {
        let rs = vec![
            resp(0x01, 0xA1, 1024),
            resp(0x01, 0xA2, 2048),
        ];
        assert_eq!(
            validate_response_burst(&rs),
            Err(ResponseBurstError::DuplicateSessionId { idx: 1 })
        );
    }

    /// **ARSB-05** — zero response ID rejected.
    #[test]
    fn arsb_05_zero_response_rejected() {
        let r = ResponseSizeRecord {
            session_id: sid(0x01),
            response_id: [0u8; ARSB_RESPONSE_ID_LEN],
            size: 1024,
        };
        assert_eq!(
            validate_response_burst(&[r]),
            Err(ResponseBurstError::ZeroResponseId(0))
        );
    }

    /// **ARSB-06** — too many rejected.
    #[test]
    fn arsb_06_too_many_rejected() {
        let rs: Vec<ResponseSizeRecord> = (0..=ARSB_MAX_RESPONSES)
            .map(|i| {
                let mut s = [0u8; ARSB_SESSION_ID_LEN];
                let val = (i as u64) + 1;
                s[0..8].copy_from_slice(&val.to_be_bytes());
                let mut r = [0u8; ARSB_RESPONSE_ID_LEN];
                r[0..8].copy_from_slice(&val.to_be_bytes());
                ResponseSizeRecord { session_id: s, response_id: r, size: 100 }
            })
            .collect();
        assert_eq!(
            validate_response_burst(&rs),
            Err(ResponseBurstError::TooMany {
                got: ARSB_MAX_RESPONSES + 1,
                max: ARSB_MAX_RESPONSES,
            })
        );
    }

    /// **ARSB-07** — valid accepted.
    #[test]
    fn arsb_07_valid_accepted() {
        assert_eq!(validate_response_burst(&valid_responses()), Ok(()));
    }

    /// **ARSB-08** — empty accepted.
    #[test]
    fn arsb_08_empty_accepted() {
        assert_eq!(validate_response_burst(&[]), Ok(()));
    }

    /// **ARSB-09** — boundary size accepted.
    #[test]
    fn arsb_09_boundary_size_accepted() {
        let r = resp(0x01, 0xA1, ARSB_MAX_SIZE);
        assert_eq!(validate_response_burst(&[r]), Ok(()));
    }

    /// **ARSB-10** — boundary burst total accepted.
    #[test]
    fn arsb_10_boundary_burst_accepted() {
        let count = ARSB_MAX_BURST_TOTAL / ARSB_MAX_SIZE;
        let rs: Vec<ResponseSizeRecord> = (0..count as u8)
            .map(|i| {
                let mut s = [0u8; ARSB_SESSION_ID_LEN];
                s[0] = i + 1;
                ResponseSizeRecord { session_id: s, response_id: rid(i + 1), size: ARSB_MAX_SIZE }
            })
            .collect();
        assert_eq!(validate_response_burst(&rs), Ok(()));
    }
}
