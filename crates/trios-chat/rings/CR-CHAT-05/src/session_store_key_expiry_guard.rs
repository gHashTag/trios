//! # CR-CHAT-05 — Session store key expiry guard (Wave-84 Lane B)
//!
//! PERSISTENCE — expired session keys must not be retrievable, R-CHAT-5.
//!
//! Session keys have a TTL for forward-secrecy reasons. If expired keys
//! remain accessible:
//!
//! * **Session hijacking** — attacker retrieves an old session key from
//!   the store and impersonates a peer whose device was compromised.
//! * **Forward-secrecy bypass** — messages encrypted under an expired
//!   key can still be decrypted, violating the FS guarantee.
//! * **Key rotation evasion** — the ratchet rotates keys, but if the
//!   store keeps serving the old key, peers use stale material.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. No expired key has `is_active = true`.
//! 2. Key TTL must be <= `SSKG_MAX_TTL_SECS`.
//! 3. Key TTL must be > 0.
//! 4. No two active keys for the same session.
//! 5. Total keys <= `SSKG_MAX_KEYS`.
//! 6. Creation timestamp must be > 0.
//!
//! Tests **SSKG-01..10**. Error enum [`KeyExpiryError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * SESSION-KEY-EXPIRY`

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

/// Maximum TTL for a session key (seconds).
pub const SSKG_MAX_TTL_SECS: u64 = 86400;

/// Maximum keys in store.
pub const SSKG_MAX_KEYS: usize = 4096;

/// A session key entry.
#[derive(Debug, Clone)]
pub struct SessionKeyEntry {
    /// Session ID.
    pub session_id: u64,
    /// Creation timestamp (seconds since epoch).
    pub created_at: u64,
    /// TTL in seconds.
    pub ttl_secs: u64,
    /// Whether the key is marked active.
    pub is_active: bool,
}

impl SessionKeyEntry {
    /// Check if the key is expired at the given `now` timestamp.
    pub fn is_expired(&self, now: u64) -> bool {
        now > self.created_at + self.ttl_secs
    }
}

/// All ways key expiry validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyExpiryError {
    /// Expired key still active.
    ExpiredKeyActive(u64),
    /// TTL exceeds maximum.
    TtlExceeded(u64),
    /// Zero TTL.
    ZeroTtl(u64),
    /// Duplicate active keys for session.
    DuplicateActive(u64),
    /// Too many keys.
    TooManyKeys,
    /// Zero creation timestamp.
    ZeroCreated(u64),
}

/// `[VERIFIED]` Validate session store key expiry.
pub fn validate_session_key_expiry(
    keys: &[SessionKeyEntry],
    now: u64,
) -> Result<(), KeyExpiryError> {
    if keys.len() > SSKG_MAX_KEYS {
        return Err(KeyExpiryError::TooManyKeys);
    }
    let mut active_per_session: BTreeMap<u64, u32> = BTreeMap::new();
    for k in keys {
        if k.created_at == 0 {
            return Err(KeyExpiryError::ZeroCreated(k.session_id));
        }
        if k.ttl_secs == 0 {
            return Err(KeyExpiryError::ZeroTtl(k.session_id));
        }
        if k.ttl_secs > SSKG_MAX_TTL_SECS {
            return Err(KeyExpiryError::TtlExceeded(k.session_id));
        }
        if k.is_active && k.is_expired(now) {
            return Err(KeyExpiryError::ExpiredKeyActive(k.session_id));
        }
        if k.is_active {
            *active_per_session.entry(k.session_id).or_insert(0) += 1;
        }
    }
    for (sid, count) in &active_per_session {
        if *count > 1 {
            return Err(KeyExpiryError::DuplicateActive(*sid));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(session_id: u64, created_at: u64, ttl: u64, active: bool) -> SessionKeyEntry {
        SessionKeyEntry { session_id, created_at, ttl_secs: ttl, is_active: active }
    }

    fn valid_keys() -> Vec<SessionKeyEntry> {
        vec![
            key(1, 1000, 3600, true),
            key(2, 1000, 3600, true),
            key(3, 1000, 3600, false),
        ]
    }

    /// **SSKG-01** — expired key still active rejected.
    #[test]
    fn sskg_01_expired_active_rejected() {
        let keys = vec![key(1, 1000, 100, true)];
        assert_eq!(
            validate_session_key_expiry(&keys, 2000),
            Err(KeyExpiryError::ExpiredKeyActive(1))
        );
    }

    /// **SSKG-02** — TTL exceeded rejected.
    #[test]
    fn sskg_02_ttl_exceeded_rejected() {
        let keys = vec![key(1, 1000, SSKG_MAX_TTL_SECS + 1, true)];
        assert_eq!(
            validate_session_key_expiry(&keys, 1000),
            Err(KeyExpiryError::TtlExceeded(1))
        );
    }

    /// **SSKG-03** — zero TTL rejected.
    #[test]
    fn sskg_03_zero_ttl_rejected() {
        let keys = vec![key(1, 1000, 0, true)];
        assert_eq!(
            validate_session_key_expiry(&keys, 1000),
            Err(KeyExpiryError::ZeroTtl(1))
        );
    }

    /// **SSKG-04** — duplicate active keys rejected.
    #[test]
    fn sskg_04_duplicate_active_rejected() {
        let keys = vec![key(1, 1000, 3600, true), key(1, 1100, 3600, true)];
        assert_eq!(
            validate_session_key_expiry(&keys, 1200),
            Err(KeyExpiryError::DuplicateActive(1))
        );
    }

    /// **SSKG-05** — too many keys rejected.
    #[test]
    fn sskg_05_too_many_rejected() {
        let keys: Vec<SessionKeyEntry> = (0..=SSKG_MAX_KEYS as u64)
            .map(|i| key(i, 1000, 3600, false))
            .collect();
        assert_eq!(
            validate_session_key_expiry(&keys, 1000),
            Err(KeyExpiryError::TooManyKeys)
        );
    }

    /// **SSKG-06** — zero creation timestamp rejected.
    #[test]
    fn sskg_06_zero_created_rejected() {
        let keys = vec![key(1, 0, 3600, true)];
        assert_eq!(
            validate_session_key_expiry(&keys, 1000),
            Err(KeyExpiryError::ZeroCreated(1))
        );
    }

    /// **SSKG-07** — valid keys accepted.
    #[test]
    fn sskg_07_valid_accepted() {
        assert_eq!(validate_session_key_expiry(&valid_keys(), 2000), Ok(()));
    }

    /// **SSKG-08** — expired inactive key accepted (expired but not active).
    #[test]
    fn sskg_08_expired_inactive_accepted() {
        let keys = vec![key(1, 1000, 100, false)];
        assert_eq!(validate_session_key_expiry(&keys, 2000), Ok(()));
    }

    /// **SSKG-09** — empty accepted.
    #[test]
    fn sskg_09_empty_accepted() {
        assert_eq!(validate_session_key_expiry(&[], 1000), Ok(()));
    }

    /// **SSKG-10** — max TTL boundary accepted.
    #[test]
    fn sskg_10_max_ttl_accepted() {
        let keys = vec![key(1, 1000, SSKG_MAX_TTL_SECS, true)];
        assert_eq!(validate_session_key_expiry(&keys, 1000), Ok(()));
    }
}
