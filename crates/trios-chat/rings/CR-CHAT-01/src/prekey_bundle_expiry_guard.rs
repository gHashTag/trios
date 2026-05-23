//! # CR-CHAT-01 — Prekey bundle expiry guard (Wave-40 Lane A)
//!
//! RFC 9420 §8.1 — KeyPackage / prekey bundle freshness validation.
//!
//! Prekey bundles carry a creation timestamp. An adversary who can inject
//! a stale bundle can:
//!
//! * **Replay compromised keys** — if a prekey was compromised at time T,
//!   re-offering the same bundle after the victim has rotated lets the
//!   attacker decrypt new sessions.
//! * **Future-dated bundle injection** — a bundle with a timestamp far in
//!   the future causes the victim to accept it for an unreasonably long
//!   time, widening the replay window.
//! * **Version downgrade** — an old bundle from a previous protocol
//!   version might lack security-critical fields.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Bundle creation timestamp is present (non-zero).
//! 2. Timestamp is not in the future (within clock-skew tolerance).
//! 3. Timestamp is within maximum bundle age (≤ 30 days).
//! 4. Protocol version matches current version.
//! 5. Bundle has not been superseded by a newer bundle from the same
//!    identity.
//! 6. Bundle public key length is canonical (32 bytes for X25519).
//!
//! Tests **PKBE-01..10**. Error enum [`PrekeyBundleExpiryError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · PREKEY-BUNDLE-EXPIRY`

#![forbid(unsafe_code)]

/// Maximum bundle age in seconds (30 days).
pub const PKBE_MAX_AGE_SECS: u64 = 30 * 24 * 3600;

/// Maximum clock skew tolerance in seconds (5 minutes).
pub const PKBE_CLOCK_SKEW_SECS: u64 = 300;

/// Canonical public key length for X25519.
pub const PKBE_PUB_KEY_LEN: usize = 32;

/// Current protocol version.
pub const PKBE_PROTOCOL_VERSION: u16 = 1;

/// A prekey bundle with metadata for freshness validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrekeyBundleExpiry {
    /// Creation timestamp (seconds since UNIX epoch).
    pub created_at: u64,
    /// Protocol version of the bundle.
    pub protocol_version: u16,
    /// X25519 public key bytes.
    pub pub_key: Vec<u8>,
    /// Identity fingerprint (for supersession checks).
    pub identity_fingerprint: [u8; 32],
}

/// Receiver's view for validation.
#[derive(Debug, Clone)]
pub struct PrekeyBundleView {
    /// Current wall-clock time (seconds since UNIX epoch).
    pub now_secs: u64,
    /// Latest known bundle timestamp for this identity (if any).
    pub latest_known_timestamp: Option<u64>,
}

/// All ways a prekey bundle can fail freshness validation.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PrekeyBundleExpiryError {
    /// `created_at` is zero (timestamp missing).
    MissingTimestamp,
    /// Bundle timestamp is in the future beyond clock-skew tolerance.
    FutureDated,
    /// Bundle has exceeded maximum age.
    Expired,
    /// Protocol version mismatch.
    VersionMismatch,
    /// Bundle has been superseded by a newer bundle.
    Superseded,
    /// Public key length is not canonical (32 bytes).
    NonCanonicalPubKeyLength,
}

/// `[VERIFIED]` Validate a prekey bundle's freshness against the
/// receiver's view. Returns `Ok(())` if all rules pass, else the
/// first failing rule.
///
/// Rules enforced in fixed order:
///
/// 1. `created_at != 0`.
/// 2. `created_at <= now + CLOCK_SKEW`.
/// 3. `now - created_at <= MAX_AGE`.
/// 4. `protocol_version == PKBE_PROTOCOL_VERSION`.
/// 5. `created_at >= latest_known_timestamp` (if set).
/// 6. `pub_key.len() == 32`.
pub fn validate_prekey_bundle_expiry(
    bundle: &PrekeyBundleExpiry,
    view: &PrekeyBundleView,
) -> Result<(), PrekeyBundleExpiryError> {
    if bundle.created_at == 0 {
        return Err(PrekeyBundleExpiryError::MissingTimestamp);
    }
    if bundle.created_at > view.now_secs + PKBE_CLOCK_SKEW_SECS {
        return Err(PrekeyBundleExpiryError::FutureDated);
    }
    if view.now_secs > bundle.created_at + PKBE_MAX_AGE_SECS {
        return Err(PrekeyBundleExpiryError::Expired);
    }
    if bundle.protocol_version != PKBE_PROTOCOL_VERSION {
        return Err(PrekeyBundleExpiryError::VersionMismatch);
    }
    if let Some(latest) = view.latest_known_timestamp {
        if bundle.created_at < latest {
            return Err(PrekeyBundleExpiryError::Superseded);
        }
    }
    if bundle.pub_key.len() != PKBE_PUB_KEY_LEN {
        return Err(PrekeyBundleExpiryError::NonCanonicalPubKeyLength);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_700_000_000;

    fn good_view() -> PrekeyBundleView {
        PrekeyBundleView {
            now_secs: NOW,
            latest_known_timestamp: None,
        }
    }

    fn good_bundle() -> PrekeyBundleExpiry {
        PrekeyBundleExpiry {
            created_at: NOW - 3600,
            protocol_version: PKBE_PROTOCOL_VERSION,
            pub_key: vec![0xAA; 32],
            identity_fingerprint: [0xBB; 32],
        }
    }

    /// **PKBE-01** — zero timestamp rejected.
    #[test]
    fn pkbe_01_missing_timestamp_rejected() {
        let mut b = good_bundle();
        b.created_at = 0;
        assert_eq!(
            validate_prekey_bundle_expiry(&b, &good_view()),
            Err(PrekeyBundleExpiryError::MissingTimestamp)
        );
    }

    /// **PKBE-02** — future-dated bundle (1 hour ahead) rejected.
    #[test]
    fn pkbe_02_future_dated_rejected() {
        let mut b = good_bundle();
        b.created_at = NOW + 3600;
        assert_eq!(
            validate_prekey_bundle_expiry(&b, &good_view()),
            Err(PrekeyBundleExpiryError::FutureDated)
        );
    }

    /// **PKBE-03** — expired bundle (> 30 days old) rejected.
    #[test]
    fn pkbe_03_expired_rejected() {
        let mut b = good_bundle();
        b.created_at = NOW - PKBE_MAX_AGE_SECS - 1;
        assert_eq!(
            validate_prekey_bundle_expiry(&b, &good_view()),
            Err(PrekeyBundleExpiryError::Expired)
        );
    }

    /// **PKBE-04** — protocol version mismatch rejected.
    #[test]
    fn pkbe_04_version_mismatch_rejected() {
        let mut b = good_bundle();
        b.protocol_version = 99;
        assert_eq!(
            validate_prekey_bundle_expiry(&b, &good_view()),
            Err(PrekeyBundleExpiryError::VersionMismatch)
        );
    }

    /// **PKBE-05** — superseded bundle rejected.
    #[test]
    fn pkbe_05_superseded_rejected() {
        let mut view = good_view();
        view.latest_known_timestamp = Some(NOW - 1800);
        let b = PrekeyBundleExpiry {
            created_at: NOW - 3600,
            protocol_version: PKBE_PROTOCOL_VERSION,
            pub_key: vec![0xAA; 32],
            identity_fingerprint: [0xBB; 32],
        };
        assert_eq!(
            validate_prekey_bundle_expiry(&b, &view),
            Err(PrekeyBundleExpiryError::Superseded)
        );
    }

    /// **PKBE-06** — non-canonical public key length rejected.
    #[test]
    fn pkbe_06_non_canonical_pubkey_rejected() {
        let mut b = good_bundle();
        b.pub_key = vec![0xAA; 16];
        assert_eq!(
            validate_prekey_bundle_expiry(&b, &good_view()),
            Err(PrekeyBundleExpiryError::NonCanonicalPubKeyLength)
        );
    }

    /// **PKBE-07** — fresh bundle accepted.
    #[test]
    fn pkbe_07_fresh_bundle_accepted() {
        assert_eq!(validate_prekey_bundle_expiry(&good_bundle(), &good_view()), Ok(()));
    }

    /// **PKBE-08** — bundle within clock-skew tolerance accepted.
    #[test]
    fn pkbe_08_clock_skew_tolerance_accepted() {
        let b = PrekeyBundleExpiry {
            created_at: NOW + PKBE_CLOCK_SKEW_SECS,
            protocol_version: PKBE_PROTOCOL_VERSION,
            pub_key: vec![0xAA; 32],
            identity_fingerprint: [0xBB; 32],
        };
        assert_eq!(validate_prekey_bundle_expiry(&b, &good_view()), Ok(()));
    }

    /// **PKBE-09** — bundle at exact max age boundary accepted.
    #[test]
    fn pkbe_09_max_age_boundary_accepted() {
        let b = PrekeyBundleExpiry {
            created_at: NOW - PKBE_MAX_AGE_SECS,
            protocol_version: PKBE_PROTOCOL_VERSION,
            pub_key: vec![0xAA; 32],
            identity_fingerprint: [0xBB; 32],
        };
        assert_eq!(validate_prekey_bundle_expiry(&b, &good_view()), Ok(()));
    }

    /// **PKBE-10** — bundle matching latest timestamp accepted (not superseded).
    #[test]
    fn pkbe_10_matching_latest_accepted() {
        let mut view = good_view();
        view.latest_known_timestamp = Some(NOW - 3600);
        assert_eq!(validate_prekey_bundle_expiry(&good_bundle(), &view), Ok(()));
    }
}
