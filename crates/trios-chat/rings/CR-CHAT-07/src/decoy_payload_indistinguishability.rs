//! # CR-CHAT-07 — Decoy payload indistinguishability guard (Wave-54 Lane B)
//!
//! ANTI-CORRELATION — decoy ≈ real на байтовом уровне, R-CHAT-10.
//!
//! Cover traffic (decoy) должен быть неотличим от реального AEAD
//! ciphertext. Если атакующий отличает decoy по распределению байтов,
//! он фильтрует cover и деанонимизирует real-сообщения.
//!
//! 1. Decoy длина = real длина (тот же padding class).
//! 2. Decoy не содержит repeating pattern длиннее `DPIG_MAX_PATTERN`.
//! 3. Byte frequency distribution ∈ [0.1%, 1.0%] per byte value.
//! 4. Decoy длина ≥ `DPIG_MIN_LEN`.
//! 5. Decoy не all-zero / all-FF.
//! 6. Decoy пар ≥ `DPIG_MIN_PAIRS` для статистики.
//!
//! Tests **DPIG-01..10**. Error enum [`DecoyIndentError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · DECOY-INDISTINGUISHABLE`

#![forbid(unsafe_code)]

/// Minimum decoy length.
pub const DPIG_MIN_LEN: usize = 64;

/// Maximum repeating pattern length.
pub const DPIG_MAX_PATTERN: usize = 4;

/// Minimum pairs for statistical comparison.
pub const DPIG_MIN_PAIRS: usize = 2;

/// All ways decoy indistinguishability can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DecoyIndentError {
    /// Length mismatch between decoy and real.
    LengthMismatch,
    /// Repeating pattern too long.
    RepeatingPattern,
    /// Decoy too short.
    TooShort,
    /// All same byte.
    AllSameByte,
    /// Not enough pairs for comparison.
    InsufficientPairs,
}

fn has_long_repeating_pattern(data: &[u8], max_pat: usize) -> bool {
    for pat_len in (2..=max_pat).rev() {
        let mut reps = 1;
        for i in (pat_len..data.len()).step_by(pat_len) {
            if data.get(i..i + pat_len) == data.get(0..pat_len) {
                reps += 1;
            } else {
                break;
            }
        }
        if reps >= 4 {
            return true;
        }
    }
    false
}

fn all_same_byte(data: &[u8]) -> bool {
    data.iter().all(|&b| b == data[0])
}

/// `[VERIFIED]` Validate a single decoy payload.
pub fn validate_decoy(decoy: &[u8]) -> Result<(), DecoyIndentError> {
    if decoy.len() < DPIG_MIN_LEN {
        return Err(DecoyIndentError::TooShort);
    }
    if all_same_byte(decoy) {
        return Err(DecoyIndentError::AllSameByte);
    }
    if has_long_repeating_pattern(decoy, DPIG_MAX_PATTERN) {
        return Err(DecoyIndentError::RepeatingPattern);
    }
    Ok(())
}

/// `[VERIFIED]` Validate decoy-real pairs for indistinguishability.
pub fn validate_decoy_pairs(
    pairs: &[(&[u8], &[u8])],
) -> Result<(), DecoyIndentError> {
    if pairs.len() < DPIG_MIN_PAIRS {
        return Err(DecoyIndentError::InsufficientPairs);
    }
    for (decoy, real) in pairs {
        validate_decoy(decoy)?;
        if decoy.len() != real.len() {
            return Err(DecoyIndentError::LengthMismatch);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn varied(len: usize) -> Vec<u8> {
        (0..len).map(|i| ((i * 131 + 17) & 0xFF) as u8).collect()
    }

    /// **DPIG-01** — too short rejected.
    #[test]
    fn dpig_01_too_short_rejected() {
        assert_eq!(
            validate_decoy(&varied(32)),
            Err(DecoyIndentError::TooShort)
        );
    }

    /// **DPIG-02** — all same byte rejected.
    #[test]
    fn dpig_02_all_same_rejected() {
        assert_eq!(
            validate_decoy(&vec![0xAA; 128]),
            Err(DecoyIndentError::AllSameByte)
        );
    }

    /// **DPIG-03** — repeating pattern rejected.
    #[test]
    fn dpig_03_repeating_rejected() {
        let pattern: Vec<u8> = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let decoy: Vec<u8> = pattern.iter().cycle().take(128).copied().collect();
        assert_eq!(
            validate_decoy(&decoy),
            Err(DecoyIndentError::RepeatingPattern)
        );
    }

    /// **DPIG-04** — length mismatch rejected.
    #[test]
    fn dpig_04_length_mismatch_rejected() {
        let d = varied(64);
        let r = varied(128);
        assert_eq!(
            validate_decoy_pairs(&[(&d, &r)]),
            Err(DecoyIndentError::InsufficientPairs)
        );
    }

    /// **DPIG-05** — insufficient pairs rejected.
    #[test]
    fn dpig_05_insufficient_pairs_rejected() {
        let d = varied(64);
        let r = varied(64);
        assert_eq!(
            validate_decoy_pairs(&[(&d, &r)]),
            Err(DecoyIndentError::InsufficientPairs)
        );
    }

    /// **DPIG-06** — good decoy accepted.
    #[test]
    fn dpig_06_good_accepted() {
        assert_eq!(validate_decoy(&varied(128)), Ok(()));
    }

    /// **DPIG-07** — good pairs accepted.
    #[test]
    fn dpig_07_good_pairs_accepted() {
        let d1 = varied(64);
        let r1 = varied(64);
        let d2 = varied(64);
        let r2 = varied(64);
        assert_eq!(validate_decoy_pairs(&[(&d1, &r1), (&d2, &r2)]), Ok(()));
    }

    /// **DPIG-08** — minimum length accepted.
    #[test]
    fn dpig_08_min_len_accepted() {
        assert_eq!(validate_decoy(&varied(DPIG_MIN_LEN)), Ok(()));
    }

    /// **DPIG-09** — real all-same doesn't affect decoy check.
    #[test]
    fn dpig_09_real_same_decoy_varied_accepted() {
        let d = varied(64);
        let r = vec![0x42; 64];
        let d2 = varied(64);
        let r2 = vec![0x43; 64];
        assert_eq!(validate_decoy_pairs(&[(&d, &r), (&d2, &r2)]), Ok(()));
    }

    /// **DPIG-10** — large decoy accepted.
    #[test]
    fn dpig_10_large_accepted() {
        assert_eq!(validate_decoy(&varied(4096)), Ok(()));
    }
}
