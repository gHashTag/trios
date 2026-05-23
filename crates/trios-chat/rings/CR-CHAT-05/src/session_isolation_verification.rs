//! # CR-CHAT-05 — Session isolation verification guard (Wave-45 Lane B)
//!
//! R-CHAT-1 — At-rest session isolation enforcement.
//!
//! Persistence layers store envelopes from multiple sessions side-by-side.
//! An attacker with read access to the store can attempt cross-session
//! correlation by:
//!
//! * **Reusing ciphertexts** — copying an envelope from one session into
//!   another, causing decryption with wrong keys.
//! * **Merging sessions** — corrupting the session_id field to merge two
//!   independent sessions.
//! * **Counter collision** — forcing two sessions to share a counter value
//!   with the same ciphertext, enabling correlation analysis.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Each session has a unique session_id.
//! 2. No ciphertext is shared across sessions.
//! 3. Counter ranges per session do not overlap with identical ciphertexts.
//! 4. Session count is within bounds.
//! 5. No empty sessions (each has ≥ 1 envelope).
//! 6. All ciphertexts meet minimum AEAD length.
//!
//! Tests **SISO-01..10**. Error enum [`SessionIsolationError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · SESSION-ISOLATION`

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

/// Maximum number of sessions allowed in a store view.
pub const SISO_MAX_SESSIONS: usize = 256;

/// Minimum ciphertext length (AEAD tag).
pub const SISO_MIN_CT_LEN: usize = 16;

/// One session's data in the store.
#[derive(Debug, Clone)]
pub struct SessionData {
    /// Session identifier.
    pub session_id: [u8; 32],
    /// Envelope ciphertexts in counter order.
    pub ciphertexts: Vec<Vec<u8>>,
}

/// All ways session isolation can be violated.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SessionIsolationError {
    /// Duplicate session_id.
    DuplicateSessionId,
    /// Ciphertext shared across sessions.
    CrossSessionCiphertext,
    /// Session count exceeds maximum.
    TooManySessions,
    /// Empty session (no envelopes).
    EmptySession,
    /// Ciphertext below minimum AEAD length.
    CiphertextTooShort,
}

/// `[VERIFIED]` Verify session isolation across a set of sessions.
/// Returns `Ok(())` if all rules pass.
pub fn verify_session_isolation(
    sessions: &[SessionData],
) -> Result<(), SessionIsolationError> {
    if sessions.len() > SISO_MAX_SESSIONS {
        return Err(SessionIsolationError::TooManySessions);
    }
    let mut seen_ids = BTreeSet::new();
    let mut ct_to_session: BTreeMap<Vec<u8>, [u8; 32]> = BTreeMap::new();

    for session in sessions {
        if !seen_ids.insert(session.session_id) {
            return Err(SessionIsolationError::DuplicateSessionId);
        }
        if session.ciphertexts.is_empty() {
            return Err(SessionIsolationError::EmptySession);
        }
        for ct in &session.ciphertexts {
            if ct.len() < SISO_MIN_CT_LEN {
                return Err(SessionIsolationError::CiphertextTooShort);
            }
            if let Some(other_sid) = ct_to_session.get(ct) {
                if *other_sid != session.session_id {
                    return Err(SessionIsolationError::CrossSessionCiphertext);
                }
            } else {
                ct_to_session.insert(ct.clone(), session.session_id);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SID_A: [u8; 32] = [0xAA; 32];
    const SID_B: [u8; 32] = [0xBB; 32];

    fn session(sid: [u8; 32], cts: Vec<Vec<u8>>) -> SessionData {
        SessionData { session_id: sid, ciphertexts: cts }
    }

    fn ct(byte: u8) -> Vec<u8> {
        vec![byte; 32]
    }

    fn good_sessions() -> Vec<SessionData> {
        vec![
            session(SID_A, vec![ct(0x01), ct(0x02)]),
            session(SID_B, vec![ct(0x03), ct(0x04)]),
        ]
    }

    /// **SISO-01** — duplicate session_id rejected.
    #[test]
    fn siso_01_duplicate_session_id_rejected() {
        let sessions = vec![
            session(SID_A, vec![ct(0x01)]),
            session(SID_A, vec![ct(0x02)]),
        ];
        assert_eq!(
            verify_session_isolation(&sessions),
            Err(SessionIsolationError::DuplicateSessionId)
        );
    }

    /// **SISO-02** — cross-session ciphertext rejected.
    #[test]
    fn siso_02_cross_session_ciphertext_rejected() {
        let shared = ct(0xFF);
        let sessions = vec![
            session(SID_A, vec![shared.clone()]),
            session(SID_B, vec![shared]),
        ];
        assert_eq!(
            verify_session_isolation(&sessions),
            Err(SessionIsolationError::CrossSessionCiphertext)
        );
    }

    /// **SISO-03** — too many sessions rejected.
    #[test]
    fn siso_03_too_many_sessions_rejected() {
        let mut sessions = Vec::new();
        for i in 0..=SISO_MAX_SESSIONS {
            let mut sid = [0u8; 32];
            sid[0] = i as u8;
            sessions.push(session(sid, vec![ct(i as u8)]));
        }
        assert_eq!(
            verify_session_isolation(&sessions),
            Err(SessionIsolationError::TooManySessions)
        );
    }

    /// **SISO-04** — empty session rejected.
    #[test]
    fn siso_04_empty_session_rejected() {
        let sessions = vec![
            session(SID_A, vec![ct(0x01)]),
            session(SID_B, vec![]),
        ];
        assert_eq!(
            verify_session_isolation(&sessions),
            Err(SessionIsolationError::EmptySession)
        );
    }

    /// **SISO-05** — ciphertext too short rejected.
    #[test]
    fn siso_05_ciphertext_too_short_rejected() {
        let sessions = vec![
            session(SID_A, vec![vec![0x01; 8]]),
        ];
        assert_eq!(
            verify_session_isolation(&sessions),
            Err(SessionIsolationError::CiphertextTooShort)
        );
    }

    /// **SISO-06** — isolated sessions accepted.
    #[test]
    fn siso_06_isolated_accepted() {
        assert_eq!(verify_session_isolation(&good_sessions()), Ok(()));
    }

    /// **SISO-07** — single session accepted.
    #[test]
    fn siso_07_single_session_accepted() {
        let sessions = vec![session(SID_A, vec![ct(0x01)])];
        assert_eq!(verify_session_isolation(&sessions), Ok(()));
    }

    /// **SISO-08** — same ciphertext within same session accepted.
    #[test]
    fn siso_08_same_ct_same_session_accepted() {
        let sessions = vec![session(SID_A, vec![ct(0x01), ct(0x01)])];
        assert_eq!(verify_session_isolation(&sessions), Ok(()));
    }

    /// **SISO-09** — exact boundary ciphertext length accepted.
    #[test]
    fn siso_09_boundary_ct_len_accepted() {
        let sessions = vec![session(SID_A, vec![vec![0x01; SISO_MIN_CT_LEN]])];
        assert_eq!(verify_session_isolation(&sessions), Ok(()));
    }

    /// **SISO-10** — empty store accepted.
    #[test]
    fn siso_10_empty_store_accepted() {
        assert_eq!(verify_session_isolation(&[]), Ok(()));
    }
}
