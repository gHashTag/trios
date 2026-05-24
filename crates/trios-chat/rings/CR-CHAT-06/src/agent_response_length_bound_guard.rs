//! # CR-CHAT-06 — Agent response length bound guard (Wave-100 Lane A)
//!
//! AGENT SAFETY — agent responses must not exceed length limits.
//!
//! Without length bounds on agent responses:
//!
//! * **Buffer overflow** — clients with fixed receive buffers crash when
//!   receiving responses larger than expected.
//! * **Resource exhaustion** — a compromised agent can generate
//!   arbitrarily large responses, consuming memory and bandwidth.
//! * **Denial of service** — oversized responses block the encrypted
//!   channel, preventing legitimate messages.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Response length <= `ARLB_MAX_RESPONSE_LEN`.
//! 2. Response length > 0 (empty responses are invalid).
//! 3. Total responses in batch <= `ARLB_MAX_BATCH`.
//! 4. Cumulative batch length <= `ARLB_MAX_CUMULATIVE`.
//! 5. No duplicate response IDs.
//! 6. Response ID must not be all zeros.
//!
//! Tests **ARLB-01..10**. Error enum [`ResponseLengthError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * RESPONSE-BOUND`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum single response length.
pub const ARLB_MAX_RESPONSE_LEN: usize = 65_536;

/// Maximum responses per batch.
pub const ARLB_MAX_BATCH: usize = 256;

/// Maximum cumulative response length per batch.
pub const ARLB_MAX_CUMULATIVE: usize = 1_048_576;

/// Response ID length.
pub const ARLB_ID_LEN: usize = 16;

/// A single agent response.
#[derive(Debug, Clone)]
pub struct AgentResponse {
    /// Unique response ID.
    pub id: [u8; ARLB_ID_LEN],
    /// Response payload.
    pub payload: Vec<u8>,
}

/// All ways response length validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResponseLengthError {
    /// Response exceeds max length.
    TooLong { idx: usize, len: usize, max: usize },
    /// Empty response.
    Empty(usize),
    /// Batch too large.
    BatchTooLarge { got: usize, max: usize },
    /// Cumulative length exceeded.
    CumulativeExceeded { total: usize, max: usize },
    /// Duplicate ID.
    DuplicateId(usize),
    /// Zero ID.
    ZeroId(usize),
}

/// `[VERIFIED]` Validate agent response length bounds.
pub fn validate_response_lengths(
    responses: &[AgentResponse],
) -> Result<(), ResponseLengthError> {
    if responses.len() > ARLB_MAX_BATCH {
        return Err(ResponseLengthError::BatchTooLarge {
            got: responses.len(),
            max: ARLB_MAX_BATCH,
        });
    }
    let mut total: usize = 0;
    let mut seen: BTreeSet<[u8; ARLB_ID_LEN]> = BTreeSet::new();
    for (i, r) in responses.iter().enumerate() {
        if r.id == [0u8; ARLB_ID_LEN] {
            return Err(ResponseLengthError::ZeroId(i));
        }
        if r.payload.is_empty() {
            return Err(ResponseLengthError::Empty(i));
        }
        if r.payload.len() > ARLB_MAX_RESPONSE_LEN {
            return Err(ResponseLengthError::TooLong {
                idx: i,
                len: r.payload.len(),
                max: ARLB_MAX_RESPONSE_LEN,
            });
        }
        if !seen.insert(r.id) {
            return Err(ResponseLengthError::DuplicateId(i));
        }
        total += r.payload.len();
        if total > ARLB_MAX_CUMULATIVE {
            return Err(ResponseLengthError::CumulativeExceeded {
                total,
                max: ARLB_MAX_CUMULATIVE,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(byte: u8) -> [u8; ARLB_ID_LEN] {
        [byte; ARLB_ID_LEN]
    }

    fn response(id_byte: u8, len: usize) -> AgentResponse {
        AgentResponse { id: id(id_byte), payload: vec![0xAA; len] }
    }

    fn valid_batch() -> Vec<AgentResponse> {
        vec![
            response(0x01, 100),
            response(0x02, 200),
            response(0x03, 300),
        ]
    }

    /// **ARLB-01** — too long rejected.
    #[test]
    fn arlb_01_too_long_rejected() {
        let rs = vec![response(0x01, ARLB_MAX_RESPONSE_LEN + 1)];
        assert_eq!(
            validate_response_lengths(&rs),
            Err(ResponseLengthError::TooLong {
                idx: 0,
                len: ARLB_MAX_RESPONSE_LEN + 1,
                max: ARLB_MAX_RESPONSE_LEN,
            })
        );
    }

    /// **ARLB-02** — empty rejected.
    #[test]
    fn arlb_02_empty_rejected() {
        let r = AgentResponse { id: id(0x01), payload: vec![] };
        assert_eq!(
            validate_response_lengths(&[r]),
            Err(ResponseLengthError::Empty(0))
        );
    }

    /// **ARLB-03** — batch too large rejected.
    #[test]
    fn arlb_03_batch_too_large_rejected() {
        let rs: Vec<AgentResponse> = (0..=ARLB_MAX_BATCH)
            .map(|i| {
                let b = (i % 254 + 1) as u8;
                AgentResponse { id: id(b), payload: vec![0x42; 10] }
            })
            .collect();
        assert!(matches!(
            validate_response_lengths(&rs),
            Err(ResponseLengthError::BatchTooLarge { .. })
        ));
    }

    /// **ARLB-04** — cumulative exceeded rejected.
    #[test]
    fn arlb_04_cumulative_exceeded_rejected() {
        let count = (ARLB_MAX_CUMULATIVE / ARLB_MAX_RESPONSE_LEN) + 1;
        let rs: Vec<AgentResponse> = (0..=count)
            .map(|i| response((i as u8).wrapping_add(1), ARLB_MAX_RESPONSE_LEN))
            .collect();
        assert!(matches!(
            validate_response_lengths(&rs),
            Err(ResponseLengthError::CumulativeExceeded { .. })
        ));
    }

    /// **ARLB-05** — duplicate ID rejected.
    #[test]
    fn arlb_05_duplicate_rejected() {
        let rs = vec![response(0x01, 10), response(0x01, 20)];
        assert_eq!(
            validate_response_lengths(&rs),
            Err(ResponseLengthError::DuplicateId(1))
        );
    }

    /// **ARLB-06** — zero ID rejected.
    #[test]
    fn arlb_06_zero_id_rejected() {
        let r = AgentResponse { id: [0u8; ARLB_ID_LEN], payload: vec![0x42; 10] };
        assert_eq!(
            validate_response_lengths(&[r]),
            Err(ResponseLengthError::ZeroId(0))
        );
    }

    /// **ARLB-07** — valid batch accepted.
    #[test]
    fn arlb_07_valid_accepted() {
        assert_eq!(validate_response_lengths(&valid_batch()), Ok(()));
    }

    /// **ARLB-08** — single max-length accepted.
    #[test]
    fn arlb_08_max_length_accepted() {
        let rs = vec![response(0x01, ARLB_MAX_RESPONSE_LEN)];
        assert_eq!(validate_response_lengths(&rs), Ok(()));
    }

    /// **ARLB-09** — empty batch accepted.
    #[test]
    fn arlb_09_empty_batch_accepted() {
        assert_eq!(validate_response_lengths(&[]), Ok(()));
    }

    /// **ARLB-10** — boundary cumulative accepted.
    #[test]
    fn arlb_10_boundary_cumulative_accepted() {
        let count = ARLB_MAX_CUMULATIVE / ARLB_MAX_RESPONSE_LEN;
        let rs: Vec<AgentResponse> = (0..count)
            .map(|i| response((i as u8) + 1, ARLB_MAX_RESPONSE_LEN))
            .collect();
        assert_eq!(validate_response_lengths(&rs), Ok(()));
    }
}
