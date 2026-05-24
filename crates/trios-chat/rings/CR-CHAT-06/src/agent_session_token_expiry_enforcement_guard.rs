//! # CR-CHAT-06 — Agent session token expiry enforcement guard (Wave-141 Lane B)
//!
//! AGENT SAFETY — agent session tokens must not be expired; expired
//! tokens enable session hijacking.
//!
//! Each agent session carries a token with creation and expiry
//! timestamps. Using an expired token:
//!
//! * **Session hijacking** — an attacker who captures an expired
//!   token can replay it if the server doesn't validate expiry.
//! * **Privilege persistence** — permissions granted during a
//!   session should not persist beyond the token's lifetime.
//! * **Audit gap** — actions taken with expired tokens cannot be
//!   attributed to a valid session.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Token must not be expired (expiry > now).
//! 2. Token ID must not be zero.
//! 3. No duplicate token IDs.
//! 4. Created timestamp must be > 0.
//! 5. Expiry must be > created.
//! 6. Batch size <= `ASTE_MAX_TOKENS`.
//!
//! Tests **ASTX-01..10**. Error enum [`TokenExpiryError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * TOKEN-VALID`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum tokens per batch.
pub const ASTX_MAX_TOKENS: usize = 512;

/// Token ID length.
pub const ASTX_TOKEN_ID_LEN: usize = 32;

/// A session token expiry record.
#[derive(Debug, Clone)]
pub struct SessionTokenRecord {
    /// Token identifier.
    pub token_id: [u8; ASTX_TOKEN_ID_LEN],
    /// Creation timestamp (ms since epoch).
    pub created_ms: u64,
    /// Expiry timestamp (ms since epoch).
    pub expiry_ms: u64,
}

/// All ways token expiry validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TokenExpiryError {
    /// Token expired.
    Expired {
        /// Index.
        idx: usize,
        /// Expiry timestamp.
        expiry_ms: u64,
        /// Current time.
        now_ms: u64,
    },
    /// Zero token ID.
    ZeroTokenId(usize),
    /// Duplicate token ID.
    DuplicateTokenId {
        /// Index.
        idx: usize,
    },
    /// Zero created timestamp.
    ZeroCreated(usize),
    /// Expiry before created.
    ExpiryBeforeCreated {
        /// Index.
        idx: usize,
    },
    /// Too many tokens.
    TooMany {
        /// Actual count.
        got: usize,
        /// Maximum allowed.
        max: usize,
    },
}

/// `[VERIFIED]` Validate agent session token expiry.
pub fn validate_token_expiry(
    tokens: &[SessionTokenRecord],
    now_ms: u64,
) -> Result<(), TokenExpiryError> {
    if tokens.len() > ASTX_MAX_TOKENS {
        return Err(TokenExpiryError::TooMany {
            got: tokens.len(),
            max: ASTX_MAX_TOKENS,
        });
    }
    let mut seen: BTreeSet<[u8; ASTX_TOKEN_ID_LEN]> = BTreeSet::new();
    for (i, t) in tokens.iter().enumerate() {
        if t.token_id == [0u8; ASTX_TOKEN_ID_LEN] {
            return Err(TokenExpiryError::ZeroTokenId(i));
        }
        if !seen.insert(t.token_id) {
            return Err(TokenExpiryError::DuplicateTokenId { idx: i });
        }
        if t.created_ms == 0 {
            return Err(TokenExpiryError::ZeroCreated(i));
        }
        if t.expiry_ms <= t.created_ms {
            return Err(TokenExpiryError::ExpiryBeforeCreated { idx: i });
        }
        if t.expiry_ms <= now_ms {
            return Err(TokenExpiryError::Expired {
                idx: i,
                expiry_ms: t.expiry_ms,
                now_ms,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tid(byte: u8) -> [u8; ASTX_TOKEN_ID_LEN] {
        [byte; ASTX_TOKEN_ID_LEN]
    }

    fn token(id: u8, created: u64, expiry: u64) -> SessionTokenRecord {
        SessionTokenRecord { token_id: tid(id), created_ms: created, expiry_ms: expiry }
    }

    const NOW: u64 = 10_000_000;

    fn valid_tokens() -> Vec<SessionTokenRecord> {
        vec![
            token(0x01, NOW - 5000, NOW + 5000),
            token(0x02, NOW - 3000, NOW + 7000),
        ]
    }

    /// **ASTX-01** — expired rejected.
    #[test]
    fn astx_01_expired_rejected() {
        let t = token(0x01, NOW - 10000, NOW - 1000);
        assert_eq!(
            validate_token_expiry(&[t], NOW),
            Err(TokenExpiryError::Expired {
                idx: 0,
                expiry_ms: NOW - 1000,
                now_ms: NOW,
            })
        );
    }

    /// **ASTX-02** — zero token ID rejected.
    #[test]
    fn astx_02_zero_id_rejected() {
        let t = SessionTokenRecord { token_id: [0u8; ASTX_TOKEN_ID_LEN], created_ms: NOW - 1000, expiry_ms: NOW + 1000 };
        assert_eq!(
            validate_token_expiry(&[t], NOW),
            Err(TokenExpiryError::ZeroTokenId(0))
        );
    }

    /// **ASTX-03** — duplicate token ID rejected.
    #[test]
    fn astx_03_duplicate_rejected() {
        let ts = vec![
            token(0x01, NOW - 1000, NOW + 1000),
            token(0x01, NOW - 500, NOW + 1500),
        ];
        assert_eq!(
            validate_token_expiry(&ts, NOW),
            Err(TokenExpiryError::DuplicateTokenId { idx: 1 })
        );
    }

    /// **ASTX-04** — zero created rejected.
    #[test]
    fn astx_04_zero_created_rejected() {
        let t = SessionTokenRecord { token_id: tid(0x01), created_ms: 0, expiry_ms: NOW + 1000 };
        assert_eq!(
            validate_token_expiry(&[t], NOW),
            Err(TokenExpiryError::ZeroCreated(0))
        );
    }

    /// **ASTX-05** — expiry before created rejected.
    #[test]
    fn astx_05_expiry_before_created_rejected() {
        let t = token(0x01, NOW + 5000, NOW + 1000);
        assert_eq!(
            validate_token_expiry(&[t], NOW),
            Err(TokenExpiryError::ExpiryBeforeCreated { idx: 0 })
        );
    }

    /// **ASTX-06** — too many rejected.
    #[test]
    fn astx_06_too_many_rejected() {
        let ts: Vec<SessionTokenRecord> = (0..=ASTX_MAX_TOKENS)
            .map(|i| {
                let mut id = [0u8; ASTX_TOKEN_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                SessionTokenRecord { token_id: id, created_ms: NOW - 1000, expiry_ms: NOW + 1000 }
            })
            .collect();
        assert_eq!(
            validate_token_expiry(&ts, NOW),
            Err(TokenExpiryError::TooMany {
                got: ASTX_MAX_TOKENS + 1,
                max: ASTX_MAX_TOKENS,
            })
        );
    }

    /// **ASTX-07** — valid accepted.
    #[test]
    fn astx_07_valid_accepted() {
        assert_eq!(validate_token_expiry(&valid_tokens(), NOW), Ok(()));
    }

    /// **ASTX-08** — empty accepted.
    #[test]
    fn astx_08_empty_accepted() {
        assert_eq!(validate_token_expiry(&[], NOW), Ok(()));
    }

    /// **ASTX-09** — exactly at expiry boundary rejected.
    #[test]
    fn astx_09_exact_expiry_rejected() {
        let t = token(0x01, NOW - 1000, NOW);
        assert_eq!(
            validate_token_expiry(&[t], NOW),
            Err(TokenExpiryError::Expired {
                idx: 0,
                expiry_ms: NOW,
                now_ms: NOW,
            })
        );
    }

    /// **ASTX-10** — one ms after expiry boundary accepted.
    #[test]
    fn astx_10_one_ms_after_accepted() {
        let t = token(0x01, NOW - 1000, NOW + 1);
        assert_eq!(validate_token_expiry(&[t], NOW), Ok(()));
    }
}
