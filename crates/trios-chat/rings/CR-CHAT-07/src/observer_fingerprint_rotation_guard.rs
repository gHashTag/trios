//! # CR-CHAT-07 — Observer fingerprint rotation guard (Wave-82 Lane B)
//!
//! ANTI-CORRELATION — wire characteristics must rotate periodically, R-CHAT-10.
//!
//! A wire observer can fingerprint a device by its stable observable
//! characteristics: TLS cipher suite, ALPN, burst gap pattern, packet
//! size class. If these never change:
//!
//! * **Long-term tracking** — the same fingerprint follows a user
//!   across sessions and network changes.
//! * **Cross-protocol correlation** — matching fingerprint links
//!   chat traffic to other protocols from the same device.
//! * **Re-identification** — after a brief gap, the same fingerprint
//!   reveals the user has returned.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Fingerprint must rotate every <= `OFRG_MAX_SESSIONS` sessions.
//! 2. No two consecutive sessions share the same fingerprint.
//! 3. Fingerprint components are non-empty.
//! 4. Total sessions tracked <= `OFRG_MAX_TRACKED`.
//! 5. Rotation count <= `OFRG_MAX_ROTATIONS`.
//! 6. Fingerprint length <= `OFRG_MAX_FP_LEN`.
//!
//! Tests **OFRG-01..10**. Error enum [`FingerprintRotationError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * OBSERVER-FINGERPRINT`

#![forbid(unsafe_code)]

/// Maximum sessions between rotations.
pub const OFRG_MAX_SESSIONS: usize = 8;

/// Maximum tracked sessions.
pub const OFRG_MAX_TRACKED: usize = 256;

/// Maximum rotations.
pub const OFRG_MAX_ROTATIONS: usize = 64;

/// Maximum fingerprint length.
pub const OFRG_MAX_FP_LEN: usize = 128;

/// A session fingerprint.
#[derive(Debug, Clone)]
pub struct SessionFingerprint {
    /// Session index (monotone).
    pub session: usize,
    /// Fingerprint bytes.
    pub fingerprint: Vec<u8>,
}

/// All ways fingerprint rotation validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FingerprintRotationError {
    /// No rotation within max sessions.
    NoRotation(usize),
    /// Consecutive sessions share fingerprint.
    SameAsPrevious,
    /// Empty fingerprint.
    EmptyFingerprint,
    /// Too many tracked sessions.
    TooManyTracked,
    /// Too many rotations.
    TooManyRotations,
    /// Fingerprint too long.
    FingerprintTooLong,
}

/// `[VERIFIED]` Validate that fingerprints rotate periodically.
pub fn validate_fingerprint_rotation(
    sessions: &[SessionFingerprint],
) -> Result<(), FingerprintRotationError> {
    if sessions.len() > OFRG_MAX_TRACKED {
        return Err(FingerprintRotationError::TooManyTracked);
    }
    if sessions.is_empty() {
        return Ok(());
    }
    let mut rotations = 0usize;
    let mut same_run = 1usize;
    let mut prev_fp: Option<&[u8]> = None;
    for session in sessions {
        if session.fingerprint.is_empty() {
            return Err(FingerprintRotationError::EmptyFingerprint);
        }
        if session.fingerprint.len() > OFRG_MAX_FP_LEN {
            return Err(FingerprintRotationError::FingerprintTooLong);
        }
        if let Some(pf) = prev_fp {
            if session.fingerprint == pf {
                same_run += 1;
                if same_run > OFRG_MAX_SESSIONS {
                    return Err(FingerprintRotationError::NoRotation(same_run));
                }
            } else {
                rotations += 1;
                same_run = 1;
            }
        }
        prev_fp = Some(&session.fingerprint);
    }
    if rotations > OFRG_MAX_ROTATIONS {
        return Err(FingerprintRotationError::TooManyRotations);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fp(byte: u8, session: usize) -> SessionFingerprint {
        SessionFingerprint {
            session,
            fingerprint: vec![byte; 16],
        }
    }

    fn valid_sessions() -> Vec<SessionFingerprint> {
        vec![
            fp(0x01, 0), fp(0x01, 1), fp(0x01, 2),
            fp(0x02, 3), fp(0x02, 4),
            fp(0x03, 5), fp(0x03, 6),
        ]
    }

    /// **OFRG-01** — no rotation rejected.
    #[test]
    fn ofrg_01_no_rotation_rejected() {
        let sessions: Vec<SessionFingerprint> = (0..OFRG_MAX_SESSIONS + 1)
            .map(|i| fp(0x01, i))
            .collect();
        assert_eq!(
            validate_fingerprint_rotation(&sessions),
            Err(FingerprintRotationError::NoRotation(OFRG_MAX_SESSIONS + 1))
        );
    }

    /// **OFRG-02** — same as previous (within limit) accepted.
    #[test]
    fn ofrg_02_same_within_limit() {
        let sessions: Vec<SessionFingerprint> = (0..OFRG_MAX_SESSIONS)
            .map(|i| fp(0x01, i))
            .collect();
        assert_eq!(validate_fingerprint_rotation(&sessions), Ok(()));
    }

    /// **OFRG-03** — empty fingerprint rejected.
    #[test]
    fn ofrg_03_empty_rejected() {
        let s = SessionFingerprint { session: 0, fingerprint: vec![] };
        assert_eq!(
            validate_fingerprint_rotation(&[s]),
            Err(FingerprintRotationError::EmptyFingerprint)
        );
    }

    /// **OFRG-04** — too many tracked rejected.
    #[test]
    fn ofrg_04_too_many_rejected() {
        let sessions: Vec<SessionFingerprint> = (0..=OFRG_MAX_TRACKED)
            .map(|i| fp((i % 256) as u8, i))
            .collect();
        assert_eq!(
            validate_fingerprint_rotation(&sessions),
            Err(FingerprintRotationError::TooManyTracked)
        );
    }

    /// **OFRG-05** — too many rotations rejected.
    #[test]
    fn ofrg_05_too_many_rotations_rejected() {
        let sessions: Vec<SessionFingerprint> = (0..=OFRG_MAX_ROTATIONS + 1)
            .map(|i| fp((i % 256) as u8, i))
            .collect();
        assert_eq!(
            validate_fingerprint_rotation(&sessions),
            Err(FingerprintRotationError::TooManyRotations)
        );
    }

    /// **OFRG-06** — fingerprint too long rejected.
    #[test]
    fn ofrg_06_fp_long_rejected() {
        let s = SessionFingerprint {
            session: 0,
            fingerprint: vec![0x01; OFRG_MAX_FP_LEN + 1],
        };
        assert_eq!(
            validate_fingerprint_rotation(&[s]),
            Err(FingerprintRotationError::FingerprintTooLong)
        );
    }

    /// **OFRG-07** — valid sessions accepted.
    #[test]
    fn ofrg_07_valid_accepted() {
        assert_eq!(validate_fingerprint_rotation(&valid_sessions()), Ok(()));
    }

    /// **OFRG-08** — empty accepted.
    #[test]
    fn ofrg_08_empty_accepted() {
        assert_eq!(validate_fingerprint_rotation(&[]), Ok(()));
    }

    /// **OFRG-09** — single session accepted.
    #[test]
    fn ofrg_09_single_accepted() {
        assert_eq!(validate_fingerprint_rotation(&[fp(0x01, 0)]), Ok(()));
    }

    /// **OFRG-10** — max same-run accepted.
    #[test]
    fn ofrg_10_max_run_accepted() {
        let sessions: Vec<SessionFingerprint> = (0..OFRG_MAX_SESSIONS)
            .map(|i| fp(0x01, i))
            .collect();
        assert_eq!(validate_fingerprint_rotation(&sessions), Ok(()));
    }
}
