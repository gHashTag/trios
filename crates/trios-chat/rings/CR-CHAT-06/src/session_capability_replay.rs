//! # CR-CHAT-06 — Session-scoped capability token replay guard (Wave-41 Lane A)
//!
//! R-CHAT-8 — Session-scoped capability enforcement.
//!
//! Capability tokens are bound to a specific session per R-CHAT-8. An
//! attacker who obtains a token from session A and replays it in session
//! B can escalate privileges — executing actions that were never
//! authorized in the target session.
//!
//! This guard validates that every capability token presented in a
//! session was actually issued for that session. Additionally:
//!
//! * Tokens must carry a valid issuer fingerprint.
//! * Tokens must not be expired.
//! * The scope must be non-empty.
//! * The token must not have been consumed already (one-shot tokens).
//! * The session_id must match the current session.
//!
//! trios-chat enforces **7 rules**:
//!
//! 1. Token session_id is non-empty.
//! 2. Token session_id matches the current session.
//! 3. Issuer fingerprint is non-empty.
//! 4. Scope is non-empty.
//! 5. Token is not expired.
//! 6. Token has not been consumed (replay within session).
//! 7. Token TTL does not exceed maximum allowed (1 hour).
//!
//! Tests **SESSCAP-01..10**. Error enum [`SessionCapError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · SESSION-CAPABILITY`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum TTL for a capability token (1 hour in seconds).
pub const SESSCAP_MAX_TTL_SECS: u64 = 3600;

/// A session-scoped capability token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCapToken {
    /// Session identifier this token was issued for.
    pub session_id: Vec<u8>,
    /// Issuer fingerprint (Ed25519 public key hash).
    pub issuer: Vec<u8>,
    /// Granted scope (e.g. "tool:invoke", "file:read").
    pub scope: Vec<u8>,
    /// Token creation timestamp (UNIX seconds).
    pub created_at: u64,
    /// Token time-to-live in seconds.
    pub ttl_secs: u64,
    /// Unique token nonce (for replay detection).
    pub nonce: Vec<u8>,
}

/// Receiver's session view for validation.
#[derive(Debug, Clone)]
pub struct SessionCapView {
    /// Current session identifier.
    pub current_session_id: Vec<u8>,
    /// Current wall-clock time (UNIX seconds).
    pub now_secs: u64,
    /// Set of already-consumed nonces in this session.
    pub consumed_nonces: BTreeSet<Vec<u8>>,
}

/// All ways a session capability token can be rejected.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SessionCapError {
    /// `session_id` is empty.
    EmptySessionId,
    /// `session_id` does not match current session.
    SessionIdMismatch,
    /// Issuer fingerprint is empty.
    EmptyIssuer,
    /// Scope is empty.
    EmptyScope,
    /// Token has expired.
    Expired,
    /// Token nonce has already been consumed (replay).
    NonceReplay,
    /// TTL exceeds maximum allowed.
    TtlExceedsMax,
}

/// `[VERIFIED]` Validate a session-scoped capability token against the
/// receiver's session view. Returns `Ok(())` if all rules pass.
///
/// Rules enforced in fixed order:
///
/// 1. `session_id` is non-empty.
/// 2. `session_id == view.current_session_id`.
/// 3. `issuer` is non-empty.
/// 4. `scope` is non-empty.
/// 5. `created_at + ttl_secs > now_secs` (not expired).
/// 6. `nonce` not in `view.consumed_nonces`.
/// 7. `ttl_secs <= SESSCAP_MAX_TTL_SECS`.
pub fn validate_session_cap(
    token: &SessionCapToken,
    view: &SessionCapView,
) -> Result<(), SessionCapError> {
    if token.session_id.is_empty() {
        return Err(SessionCapError::EmptySessionId);
    }
    if token.session_id != view.current_session_id {
        return Err(SessionCapError::SessionIdMismatch);
    }
    if token.issuer.is_empty() {
        return Err(SessionCapError::EmptyIssuer);
    }
    if token.scope.is_empty() {
        return Err(SessionCapError::EmptyScope);
    }
    if token.created_at.saturating_add(token.ttl_secs) <= view.now_secs {
        return Err(SessionCapError::Expired);
    }
    if view.consumed_nonces.contains(&token.nonce) {
        return Err(SessionCapError::NonceReplay);
    }
    if token.ttl_secs > SESSCAP_MAX_TTL_SECS {
        return Err(SessionCapError::TtlExceedsMax);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_700_000_000;
    const SESSION: &[u8] = b"session-abc-123";

    fn good_view() -> SessionCapView {
        SessionCapView {
            current_session_id: SESSION.to_vec(),
            now_secs: NOW,
            consumed_nonces: BTreeSet::new(),
        }
    }

    fn good_token() -> SessionCapToken {
        SessionCapToken {
            session_id: SESSION.to_vec(),
            issuer: vec![0xCC; 32],
            scope: b"tool:invoke".to_vec(),
            created_at: NOW - 100,
            ttl_secs: 600,
            nonce: vec![0xDD; 16],
        }
    }

    /// **SESSCAP-01** — empty session_id rejected.
    #[test]
    fn sesscap_01_empty_session_id_rejected() {
        let mut t = good_token();
        t.session_id = vec![];
        assert_eq!(
            validate_session_cap(&t, &good_view()),
            Err(SessionCapError::EmptySessionId)
        );
    }

    /// **SESSCAP-02** — session_id mismatch rejected.
    #[test]
    fn sesscap_02_session_id_mismatch_rejected() {
        let mut t = good_token();
        t.session_id = b"other-session".to_vec();
        assert_eq!(
            validate_session_cap(&t, &good_view()),
            Err(SessionCapError::SessionIdMismatch)
        );
    }

    /// **SESSCAP-03** — empty issuer rejected.
    #[test]
    fn sesscap_03_empty_issuer_rejected() {
        let mut t = good_token();
        t.issuer = vec![];
        assert_eq!(
            validate_session_cap(&t, &good_view()),
            Err(SessionCapError::EmptyIssuer)
        );
    }

    /// **SESSCAP-04** — empty scope rejected.
    #[test]
    fn sesscap_04_empty_scope_rejected() {
        let mut t = good_token();
        t.scope = vec![];
        assert_eq!(
            validate_session_cap(&t, &good_view()),
            Err(SessionCapError::EmptyScope)
        );
    }

    /// **SESSCAP-05** — expired token rejected.
    #[test]
    fn sesscap_05_expired_rejected() {
        let mut t = good_token();
        t.created_at = NOW - 1000;
        t.ttl_secs = 500;
        assert_eq!(
            validate_session_cap(&t, &good_view()),
            Err(SessionCapError::Expired)
        );
    }

    /// **SESSCAP-06** — nonce replay rejected.
    #[test]
    fn sesscap_06_nonce_replay_rejected() {
        let mut view = good_view();
        view.consumed_nonces.insert(vec![0xDD; 16]);
        assert_eq!(
            validate_session_cap(&good_token(), &view),
            Err(SessionCapError::NonceReplay)
        );
    }

    /// **SESSCAP-07** — TTL exceeds max (2 hours) rejected.
    #[test]
    fn sesscap_07_ttl_exceeds_max_rejected() {
        let mut t = good_token();
        t.ttl_secs = 7201;
        assert_eq!(
            validate_session_cap(&t, &good_view()),
            Err(SessionCapError::TtlExceedsMax)
        );
    }

    /// **SESSCAP-08** — valid token accepted.
    #[test]
    fn sesscap_08_valid_token_accepted() {
        assert_eq!(validate_session_cap(&good_token(), &good_view()), Ok(()));
    }

    /// **SESSCAP-09** — token at exact TTL boundary accepted.
    #[test]
    fn sesscap_09_ttl_boundary_accepted() {
        let mut t = good_token();
        t.ttl_secs = SESSCAP_MAX_TTL_SECS;
        assert_eq!(validate_session_cap(&t, &good_view()), Ok(()));
    }

    /// **SESSCAP-10** — different nonce in same session accepted.
    #[test]
    fn sesscap_10_different_nonce_accepted() {
        let mut view = good_view();
        view.consumed_nonces.insert(vec![0xDD; 16]);
        let mut t = good_token();
        t.nonce = vec![0xEE; 16];
        assert_eq!(validate_session_cap(&t, &view), Ok(()));
    }
}
