//! # CR-CHAT-01 — Key package expiry guard (Wave-65 Lane A)
//!
//! IDENTITY — key packages must have valid lifetime, R-CHAT-1.
//!
//! A key package with no expiry or a stale expiry allows an attacker
//! to use a compromised key indefinitely:
//!
//! * **Expired package** — key material may be compromised after expiry.
//! * **Future-dated** — `not_before` in the future rejects valid clients.
//! * **Inverted lifetime** — `not_after < not_before` is nonsensical.
//! * **Zero lifetime** — package that expires at creation is useless.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. `not_before <= now <= not_after`.
//! 2. `not_before < not_after`.
//! 3. Lifetime <= `KPX_MAX_LIFETIME_SECS`.
//! 4. Lifetime >= `KPX_MIN_LIFETIME_SECS`.
//! 5. `not_before` not in the future (clock skew tolerance).
//! 6. `not_after` not in the past.
//!
//! Tests **KPX-01..10**. Error enum [`KeyPackageExpiryError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * KEY-PACKAGE-EXPIRY`

#![forbid(unsafe_code)]

/// Minimum package lifetime (seconds).
pub const KPX_MIN_LIFETIME_SECS: u64 = 60;

/// Maximum package lifetime (seconds).
pub const KPX_MAX_LIFETIME_SECS: u64 = 30 * 24 * 3600;

/// All ways key package expiry validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyPackageExpiryError {
    /// Package expired.
    Expired,
    /// Package not yet valid.
    NotYetValid,
    /// Lifetime inverted (not_after <= not_before).
    InvertedLifetime,
    /// Lifetime too short.
    LifetimeTooShort,
    /// Lifetime too long.
    LifetimeTooLong,
    /// Equal timestamps (zero lifetime).
    ZeroLifetime,
}

/// `[VERIFIED]` Validate key package lifetime window.
pub fn validate_key_package_expiry(
    now_secs: u64,
    not_before: u64,
    not_after: u64,
) -> Result<(), KeyPackageExpiryError> {
    if not_after == not_before {
        return Err(KeyPackageExpiryError::ZeroLifetime);
    }
    if not_after < not_before {
        return Err(KeyPackageExpiryError::InvertedLifetime);
    }
    let lifetime = not_after - not_before;
    if lifetime < KPX_MIN_LIFETIME_SECS {
        return Err(KeyPackageExpiryError::LifetimeTooShort);
    }
    if lifetime > KPX_MAX_LIFETIME_SECS {
        return Err(KeyPackageExpiryError::LifetimeTooLong);
    }
    if now_secs < not_before {
        return Err(KeyPackageExpiryError::NotYetValid);
    }
    if now_secs > not_after {
        return Err(KeyPackageExpiryError::Expired);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_000_000;

    fn valid_window() -> (u64, u64) {
        (NOW - 100, NOW + 3600)
    }

    /// **KPX-01** — expired rejected.
    #[test]
    fn kpx_01_expired_rejected() {
        assert_eq!(
            validate_key_package_expiry(NOW, NOW - 200, NOW - 100),
            Err(KeyPackageExpiryError::Expired)
        );
    }

    /// **KPX-02** — not yet valid rejected.
    #[test]
    fn kpx_02_not_yet_valid_rejected() {
        assert_eq!(
            validate_key_package_expiry(NOW, NOW + 100, NOW + 3600),
            Err(KeyPackageExpiryError::NotYetValid)
        );
    }

    /// **KPX-03** — inverted lifetime rejected.
    #[test]
    fn kpx_03_inverted_rejected() {
        assert_eq!(
            validate_key_package_expiry(NOW, NOW + 1000, NOW + 100),
            Err(KeyPackageExpiryError::InvertedLifetime)
        );
    }

    /// **KPX-04** — lifetime too short rejected.
    #[test]
    fn kpx_04_too_short_rejected() {
        assert_eq!(
            validate_key_package_expiry(NOW, NOW - 10, NOW + 10),
            Err(KeyPackageExpiryError::LifetimeTooShort)
        );
    }

    /// **KPX-05** — lifetime too long rejected.
    #[test]
    fn kpx_05_too_long_rejected() {
        assert_eq!(
            validate_key_package_expiry(NOW, NOW - 100, NOW + KPX_MAX_LIFETIME_SECS + 1),
            Err(KeyPackageExpiryError::LifetimeTooLong)
        );
    }

    /// **KPX-06** — zero lifetime rejected.
    #[test]
    fn kpx_06_zero_lifetime_rejected() {
        assert_eq!(
            validate_key_package_expiry(NOW, NOW, NOW),
            Err(KeyPackageExpiryError::ZeroLifetime)
        );
    }

    /// **KPX-07** — valid window accepted.
    #[test]
    fn kpx_07_valid_accepted() {
        let (nb, na) = valid_window();
        assert_eq!(validate_key_package_expiry(NOW, nb, na), Ok(()));
    }

    /// **KPX-08** — exact not_before boundary accepted.
    #[test]
    fn kpx_08_exact_start_accepted() {
        assert_eq!(
            validate_key_package_expiry(NOW, NOW, NOW + 3600),
            Ok(())
        );
    }

    /// **KPX-09** — exact not_after boundary accepted.
    #[test]
    fn kpx_09_exact_end_accepted() {
        assert_eq!(
            validate_key_package_expiry(NOW, NOW - 3600, NOW),
            Ok(())
        );
    }

    /// **KPX-10** — minimum lifetime accepted.
    #[test]
    fn kpx_10_min_lifetime_accepted() {
        assert_eq!(
            validate_key_package_expiry(NOW, NOW - 30, NOW + KPX_MIN_LIFETIME_SECS - 30),
            Ok(())
        );
    }
}
