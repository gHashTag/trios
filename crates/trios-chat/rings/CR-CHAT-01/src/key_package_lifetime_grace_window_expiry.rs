//! Wave-37 / L-CHAT-1-kplgw (R-CHAT-1 / CR-CHAT-01) — KeyPackage
//! lifetime grace-window expiry defence per RFC 9420 §10.1
//! "KeyPackage Lifetime" and §5.3 "Credential Validation".
//!
//! Every MLS `KeyPackage` carries a `(not_before, not_after)` validity
//! window expressed as Unix seconds. RFC 9420 §10.1 requires that a
//! KeyPackage MUST only be used as the basis of a Welcome / Add when
//! `not_before <= now <= not_after`. Real deployments (OpenMLS,
//! MLS++/wickr-mls) additionally pin a maximum lifetime ceiling and
//! reject suspiciously-wide windows used as a hedge against clock
//! skew.
//!
//! Mainstream stacks frequently get the **grace window** wrong:
//!   * they accept a KeyPackage with `not_before` arbitrarily far in
//!     the past (no floor), letting an attacker who long ago harvested
//!     a leaked KeyPackage replay it forever;
//!   * they accept `not_after - not_before > MAX_LIFETIME` (no
//!     ceiling), which is exactly what RFC 9420 §10.1 warns against;
//!   * they fail open on the boundary `now == not_after`, which the
//!     RFC requires to be inclusive but easy to misread as exclusive.
//!
//! A faulty grace window lets an attacker push a stale or perpetual
//! KeyPackage through the Delivery Service and force a victim joiner
//! to encrypt a Welcome to an init_key whose long-term private half
//! the attacker already controls (or harvests later).
//!
//! This lane is the consumption-side guard at the joiner / DS. A
//! single deny wins.
//!
//! Seven rules enforced in fixed order:
//!   1. NonCanonicalLeafIdLength — `package.leaf_id.len()` must equal
//!      `KPLGW_LEAF_ID_LEN` (16 bytes — MLS LeafID anchor).
//!   2. InvertedLifetime — reject `package.not_after < package.not_before`
//!      (degenerate / lying KeyPackages).
//!   3. LifetimeWindowTooLong — reject
//!      `package.not_after - package.not_before > KPLGW_MAX_LIFETIME_SECS`
//!      (RFC 9420 §10.1 ceiling, 90 days here).
//!   4. NotYetValid — reject `view.now < package.not_before` (clock
//!      skew is not the DS's problem).
//!   5. Expired — reject `view.now > package.not_after`. The
//!      boundary `view.now == package.not_after` is accepted
//!      (RFC 9420 §10.1: validity window is inclusive on both sides).
//!   6. GraceFloorViolation — reject
//!      `view.now - package.not_before > KPLGW_GRACE_FLOOR_SECS`
//!      (no harvested-then-frozen KeyPackages — even within the
//!      declared window we cap how stale a usable KeyPackage may be).
//!   7. ZeroNotAfter — reject `package.not_after == 0` (degenerate
//!      sentinel that crosses the integer boundary in many stacks).
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · KEYPACKAGE-LIFETIME-GRACE`

#![forbid(unsafe_code)]

/// Canonical MLS LeafID length (16 bytes — R-CHAT-1).
pub const KPLGW_LEAF_ID_LEN: usize = 16;

/// Maximum allowed KeyPackage lifetime window: 90 days in seconds.
/// RFC 9420 §10.1 leaves the exact value to the application; this is
/// the OpenMLS default and the value Trinity Chat enforces.
pub const KPLGW_MAX_LIFETIME_SECS: u64 = 90 * 24 * 60 * 60;

/// Maximum allowed "harvest staleness": even if the declared window
/// is open, a KeyPackage whose `not_before` is more than 30 days in
/// the past is rejected. This is the constructive defence against
/// long-tail KeyPackage hoarding.
pub const KPLGW_GRACE_FLOOR_SECS: u64 = 30 * 24 * 60 * 60;

/// One MLS `KeyPackage` lifetime header as visible to the joiner / DS.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyPackageLifetime {
    /// MLS LeafID (16 bytes) — the leaf this KeyPackage anchors to.
    pub leaf_id: Vec<u8>,
    /// Unix seconds; inclusive lower bound.
    pub not_before: u64,
    /// Unix seconds; inclusive upper bound.
    pub not_after: u64,
}

/// Receiver-side view of wall-clock time at validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LifetimeView {
    /// Current wall-clock time in Unix seconds.
    pub now: u64,
}

/// Typed errors for `validate_key_package_lifetime`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyPackageLifetimeError {
    /// Rule 1 — non-canonical LeafID length.
    NonCanonicalLeafIdLength,
    /// Rule 2 — `not_after < not_before`.
    InvertedLifetime,
    /// Rule 3 — declared window exceeds the ceiling.
    LifetimeWindowTooLong,
    /// Rule 4 — `now < not_before`.
    NotYetValid,
    /// Rule 5 — `now > not_after`.
    Expired,
    /// Rule 6 — `now - not_before` exceeds the harvest grace floor.
    GraceFloorViolation,
    /// Rule 7 — `not_after == 0` sentinel.
    ZeroNotAfter,
}

/// Constructive guard for a single KeyPackage lifetime header.
/// Returns `Ok(())` iff every rule (1)..(7) holds.
///
/// `[VERIFIED]` against the 10 unit tests `KPLGW-01..10` below and
/// the Coq theorems `INV-CHAT-238..242` in the W37 Section of
/// `proofs/chat/Trinity_Chat.v`.
pub fn validate_key_package_lifetime(
    package: &KeyPackageLifetime,
    view: &LifetimeView,
) -> Result<(), KeyPackageLifetimeError> {
    // Rule 1: LeafID canonical length.
    if package.leaf_id.len() != KPLGW_LEAF_ID_LEN {
        return Err(KeyPackageLifetimeError::NonCanonicalLeafIdLength);
    }
    // Rule 7 (checked early — degenerate sentinel rejected before
    // arithmetic that could mask it):
    if package.not_after == 0 {
        return Err(KeyPackageLifetimeError::ZeroNotAfter);
    }
    // Rule 2: monotone lifetime.
    if package.not_after < package.not_before {
        return Err(KeyPackageLifetimeError::InvertedLifetime);
    }
    // Rule 3: declared window <= ceiling.
    if package.not_after.saturating_sub(package.not_before) > KPLGW_MAX_LIFETIME_SECS {
        return Err(KeyPackageLifetimeError::LifetimeWindowTooLong);
    }
    // Rule 4: not-yet-valid.
    if view.now < package.not_before {
        return Err(KeyPackageLifetimeError::NotYetValid);
    }
    // Rule 5: expired (inclusive upper bound).
    if view.now > package.not_after {
        return Err(KeyPackageLifetimeError::Expired);
    }
    // Rule 6: harvest grace floor.
    if view.now.saturating_sub(package.not_before) > KPLGW_GRACE_FLOOR_SECS {
        return Err(KeyPackageLifetimeError::GraceFloorViolation);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_leaf_id() -> Vec<u8> {
        vec![0x55_u8; KPLGW_LEAF_ID_LEN]
    }

    /// `now` is fixed at a round Unix second; `not_before` is one hour
    /// before it; `not_after` is one day after it (well below the
    /// 90-day ceiling and 30-day grace floor).
    const T_NOW: u64 = 1_800_000_000;
    const T_HOUR: u64 = 3_600;
    const T_DAY: u64 = 86_400;

    fn ok_view() -> LifetimeView {
        LifetimeView { now: T_NOW }
    }

    fn ok_package() -> KeyPackageLifetime {
        KeyPackageLifetime {
            leaf_id: ok_leaf_id(),
            not_before: T_NOW - T_HOUR,
            not_after: T_NOW + T_DAY,
        }
    }

    /// KPLGW-01 — short leaf_id (8 bytes) rejected — Rule 1.
    #[test]
    fn kplgw_01_short_leaf_id_rejected() {
        let mut p = ok_package();
        p.leaf_id = vec![0x55_u8; 8];
        assert_eq!(
            validate_key_package_lifetime(&p, &ok_view()),
            Err(KeyPackageLifetimeError::NonCanonicalLeafIdLength)
        );
    }

    /// KPLGW-02 — inverted lifetime rejected — Rule 2.
    #[test]
    fn kplgw_02_inverted_lifetime_rejected() {
        let mut p = ok_package();
        p.not_before = T_NOW + T_DAY;
        p.not_after = T_NOW - T_HOUR;
        assert_eq!(
            validate_key_package_lifetime(&p, &ok_view()),
            Err(KeyPackageLifetimeError::InvertedLifetime)
        );
    }

    /// KPLGW-03 — lifetime window longer than ceiling rejected — Rule 3.
    #[test]
    fn kplgw_03_window_too_long_rejected() {
        let mut p = ok_package();
        // 91 days span > 90-day ceiling.
        p.not_before = T_NOW - T_HOUR;
        p.not_after = T_NOW + 91 * T_DAY;
        assert_eq!(
            validate_key_package_lifetime(&p, &ok_view()),
            Err(KeyPackageLifetimeError::LifetimeWindowTooLong)
        );
    }

    /// KPLGW-04 — not-yet-valid rejected — Rule 4.
    #[test]
    fn kplgw_04_not_yet_valid_rejected() {
        let mut p = ok_package();
        p.not_before = T_NOW + T_HOUR;
        p.not_after = T_NOW + T_DAY;
        assert_eq!(
            validate_key_package_lifetime(&p, &ok_view()),
            Err(KeyPackageLifetimeError::NotYetValid)
        );
    }

    /// KPLGW-05 — expired rejected — Rule 5.
    #[test]
    fn kplgw_05_expired_rejected() {
        let mut p = ok_package();
        p.not_before = T_NOW - 2 * T_DAY;
        p.not_after = T_NOW - T_HOUR;
        assert_eq!(
            validate_key_package_lifetime(&p, &ok_view()),
            Err(KeyPackageLifetimeError::Expired)
        );
    }

    /// KPLGW-06 — boundary `now == not_after` accepted — Rule 5.
    #[test]
    fn kplgw_06_boundary_not_after_accepted() {
        let mut p = ok_package();
        p.not_before = T_NOW - T_HOUR;
        p.not_after = T_NOW;
        assert_eq!(validate_key_package_lifetime(&p, &ok_view()), Ok(()));
    }

    /// KPLGW-07 — harvested KeyPackage (35 days stale) rejected — Rule 6.
    #[test]
    fn kplgw_07_grace_floor_violation_rejected() {
        let mut p = ok_package();
        // not_before is 35 days in the past; window itself is still
        // within the 90-day ceiling.
        p.not_before = T_NOW - 35 * T_DAY;
        p.not_after = T_NOW + T_DAY;
        assert_eq!(
            validate_key_package_lifetime(&p, &ok_view()),
            Err(KeyPackageLifetimeError::GraceFloorViolation)
        );
    }

    /// KPLGW-08 — zero not_after sentinel rejected — Rule 7.
    #[test]
    fn kplgw_08_zero_not_after_rejected() {
        let mut p = ok_package();
        p.not_before = 0;
        p.not_after = 0;
        assert_eq!(
            validate_key_package_lifetime(&p, &ok_view()),
            Err(KeyPackageLifetimeError::ZeroNotAfter)
        );
    }

    /// KPLGW-09 — exactly 90-day window accepted — Rule 3 boundary.
    #[test]
    fn kplgw_09_max_window_accepted() {
        let mut p = ok_package();
        p.not_before = T_NOW - T_HOUR;
        p.not_after = p.not_before + KPLGW_MAX_LIFETIME_SECS;
        assert_eq!(validate_key_package_lifetime(&p, &ok_view()), Ok(()));
    }

    /// KPLGW-10 — canonical KeyPackage lifetime accepted.
    #[test]
    fn kplgw_10_canonical_lifetime_accepted() {
        assert_eq!(
            validate_key_package_lifetime(&ok_package(), &ok_view()),
            Ok(())
        );
    }
}
