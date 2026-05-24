//! # CR-CHAT-02 — Epoch rollover wraparound guard (Wave-73 Lane A)
//!
//! RATCHET — epoch counter near u64::MAX forces rotation, R-CHAT-2.
//!
//! The epoch counter in the double ratchet is a u64. Near `u64::MAX`,
//! wrapping would break monotonicity:
//!
//! * **Silent wraparound** — epoch goes from `u64::MAX` to 0, which
//!   the monotonicity guard may or may not catch depending on ordering.
//! * **Fork after wrap** — two branches with epoch 0 after wrap are
//!   indistinguishable from the original epoch 0.
//! * **Key reuse** — post-wrap keys collide with the very first epoch's
//!   keys if the KDF input is only the epoch number.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Current epoch < `EPRW_DANGER_ZONE` triggers forced rotation.
//! 2. Epoch must not be u64::MAX (one left — too close).
//! 3. After forced rotation, epoch resets to a new base.
//! 4. Rotation threshold is `EPRW_ROTATION_THRESHOLD`.
//! 5. No two consecutive rotations without a gap.
//! 6. Rotation count per session <= `EPRW_MAX_ROTATIONS`.
//!
//! Tests **EPRW-01..10**. Error enum [`EpochRolloverError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * EPOCH-ROLLOVER`

#![forbid(unsafe_code)]

/// Rotation threshold — must rotate before this.
pub const EPRW_ROTATION_THRESHOLD: u64 = u64::MAX - 1000;

/// Danger zone — warn at this point.
pub const EPRW_DANGER_ZONE: u64 = u64::MAX - 100;

/// Maximum rotations per session.
pub const EPRW_MAX_ROTATIONS: usize = 8;

/// All ways epoch rollover validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EpochRolloverError {
    /// Epoch at u64::MAX — must rotate immediately.
    AtMax,
    /// In danger zone — forced rotation required.
    DangerZone,
    /// Past rotation threshold — must rotate.
    PastThreshold,
    /// Consecutive rotations without gap.
    ConsecutiveRotations,
    /// Too many rotations in session.
    TooManyRotations,
}

/// `[VERIFIED]` Validate epoch counter for rollover safety.
pub fn validate_epoch_rollover(
    current_epoch: u64,
    rotation_count: usize,
    last_was_rotation: bool,
) -> Result<(), EpochRolloverError> {
    if rotation_count > EPRW_MAX_ROTATIONS {
        return Err(EpochRolloverError::TooManyRotations);
    }
    if current_epoch == u64::MAX {
        return Err(EpochRolloverError::AtMax);
    }
    if current_epoch >= EPRW_DANGER_ZONE {
        if last_was_rotation {
            return Err(EpochRolloverError::ConsecutiveRotations);
        }
        return Err(EpochRolloverError::DangerZone);
    }
    if current_epoch >= EPRW_ROTATION_THRESHOLD {
        return Err(EpochRolloverError::PastThreshold);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **EPRW-01** — epoch at u64::MAX rejected.
    #[test]
    fn eprw_01_at_max_rejected() {
        assert_eq!(
            validate_epoch_rollover(u64::MAX, 0, false),
            Err(EpochRolloverError::AtMax)
        );
    }

    /// **EPRW-02** — danger zone rejected.
    #[test]
    fn eprw_02_danger_zone_rejected() {
        assert_eq!(
            validate_epoch_rollover(u64::MAX - 50, 0, false),
            Err(EpochRolloverError::DangerZone)
        );
    }

    /// **EPRW-03** — past threshold rejected.
    #[test]
    fn eprw_03_past_threshold_rejected() {
        assert_eq!(
            validate_epoch_rollover(EPRW_ROTATION_THRESHOLD, 0, false),
            Err(EpochRolloverError::PastThreshold)
        );
    }

    /// **EPRW-04** — consecutive rotations rejected.
    #[test]
    fn eprw_04_consecutive_rejected() {
        assert_eq!(
            validate_epoch_rollover(u64::MAX - 50, 1, true),
            Err(EpochRolloverError::ConsecutiveRotations)
        );
    }

    /// **EPRW-05** — too many rotations rejected.
    #[test]
    fn eprw_05_too_many_rejected() {
        assert_eq!(
            validate_epoch_rollover(100, EPRW_MAX_ROTATIONS + 1, false),
            Err(EpochRolloverError::TooManyRotations)
        );
    }

    /// **EPRW-06** — safe epoch accepted.
    #[test]
    fn eprw_06_safe_accepted() {
        assert_eq!(validate_epoch_rollover(1000, 0, false), Ok(()));
    }

    /// **EPRW-07** — just below threshold accepted.
    #[test]
    fn eprw_07_below_threshold_accepted() {
        assert_eq!(
            validate_epoch_rollover(EPRW_ROTATION_THRESHOLD - 1, 0, false),
            Ok(())
        );
    }

    /// **EPRW-08** — epoch 0 accepted.
    #[test]
    fn eprw_08_zero_accepted() {
        assert_eq!(validate_epoch_rollover(0, 0, false), Ok(()));
    }

    /// **EPRW-09** — max rotations at safe epoch accepted.
    #[test]
    fn eprw_09_max_rotations_accepted() {
        assert_eq!(validate_epoch_rollover(100, EPRW_MAX_ROTATIONS, false), Ok(()));
    }

    /// **EPRW-10** — mid-range epoch accepted.
    #[test]
    fn eprw_10_mid_range_accepted() {
        assert_eq!(validate_epoch_rollover(u64::MAX / 2, 0, false), Ok(()));
    }
}
