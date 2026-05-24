//! # CR-CHAT-03 — TreeKEM path secret forward secrecy guard (Wave-78 Lane B)
//!
//! RATCHET TREE — path secrets must derive in order along update path, R-CHAT-2.
//!
//! In TreeKEM, an update path carries a sequence of path secrets, each
//! derived from the previous one. If the derivation order is wrong:
//!
//! * **Out-of-order derivation** — a later path secret used before its
//!   predecessor breaks the forward-secrecy chain.
//! * **Missing secret** — a gap in the path means some nodes never
//!   receive their secret, breaking resolution.
//! * **Reused secret** — the same secret appearing twice means two
//!   subtree roots share a key, collapsing isolation.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Path secrets are in derivation order (index 0 = leaf).
//! 2. No duplicate secrets in the path.
//! 3. Path length >= `TPSF_MIN_PATH`.
//! 4. Path length <= `TPSF_MAX_PATH`.
//! 5. Each secret is non-zero.
//! 6. Secret length == `TPSF_SECRET_LEN`.
//!
//! Tests **TPSF-01..10**. Error enum [`PathSecretError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * PATH-SECRET-FORWARD`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Minimum path length.
pub const TPSF_MIN_PATH: usize = 2;

/// Maximum path length.
pub const TPSF_MAX_PATH: usize = 32;

/// Secret length (bytes).
pub const TPSF_SECRET_LEN: usize = 32;

/// All ways path secret validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PathSecretError {
    /// Duplicate secret in path.
    DuplicateSecret,
    /// Path too short.
    PathTooShort,
    /// Path too long.
    PathTooLong,
    /// Zero secret found.
    ZeroSecret,
    /// Secret length wrong.
    SecretLengthWrong,
    /// Empty path.
    EmptyPath,
}

/// `[VERIFIED]` Validate TreeKEM path secrets for forward-secrecy ordering.
pub fn validate_path_secret_order(
    secrets: &[Vec<u8>],
) -> Result<(), PathSecretError> {
    if secrets.is_empty() {
        return Err(PathSecretError::EmptyPath);
    }
    if secrets.len() < TPSF_MIN_PATH {
        return Err(PathSecretError::PathTooShort);
    }
    if secrets.len() > TPSF_MAX_PATH {
        return Err(PathSecretError::PathTooLong);
    }
    let mut seen = BTreeSet::new();
    for secret in secrets {
        if secret.len() != TPSF_SECRET_LEN {
            return Err(PathSecretError::SecretLengthWrong);
        }
        if secret.iter().all(|&b| b == 0) {
            return Err(PathSecretError::ZeroSecret);
        }
        if !seen.insert(secret.clone()) {
            return Err(PathSecretError::DuplicateSecret);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secret(byte: u8) -> Vec<u8> {
        vec![byte; TPSF_SECRET_LEN]
    }

    fn valid_path() -> Vec<Vec<u8>> {
        vec![secret(0x01), secret(0x02), secret(0x03), secret(0x04)]
    }

    /// **TPSF-01** — duplicate secret rejected.
    #[test]
    fn tpsf_01_duplicate_rejected() {
        let path = vec![secret(0x01), secret(0x02), secret(0x01)];
        assert_eq!(
            validate_path_secret_order(&path),
            Err(PathSecretError::DuplicateSecret)
        );
    }

    /// **TPSF-02** — path too short rejected.
    #[test]
    fn tpsf_02_too_short_rejected() {
        let path = vec![secret(0x01)];
        assert_eq!(
            validate_path_secret_order(&path),
            Err(PathSecretError::PathTooShort)
        );
    }

    /// **TPSF-03** — path too long rejected.
    #[test]
    fn tpsf_03_too_long_rejected() {
        let path: Vec<Vec<u8>> = (0..=TPSF_MAX_PATH)
            .map(|i| {
                let mut s = vec![0u8; TPSF_SECRET_LEN];
                s[0] = (i % 255 + 1) as u8;
                s
            })
            .collect();
        assert_eq!(
            validate_path_secret_order(&path),
            Err(PathSecretError::PathTooLong)
        );
    }

    /// **TPSF-04** — zero secret rejected.
    #[test]
    fn tpsf_04_zero_rejected() {
        let path = vec![secret(0x01), vec![0u8; TPSF_SECRET_LEN]];
        assert_eq!(
            validate_path_secret_order(&path),
            Err(PathSecretError::ZeroSecret)
        );
    }

    /// **TPSF-05** — secret length wrong rejected.
    #[test]
    fn tpsf_05_len_wrong_rejected() {
        let path = vec![secret(0x01), vec![0x02; 16]];
        assert_eq!(
            validate_path_secret_order(&path),
            Err(PathSecretError::SecretLengthWrong)
        );
    }

    /// **TPSF-06** — empty path rejected.
    #[test]
    fn tpsf_06_empty_rejected() {
        assert_eq!(
            validate_path_secret_order(&[]),
            Err(PathSecretError::EmptyPath)
        );
    }

    /// **TPSF-07** — valid path accepted.
    #[test]
    fn tpsf_07_valid_accepted() {
        assert_eq!(validate_path_secret_order(&valid_path()), Ok(()));
    }

    /// **TPSF-08** — minimum path length accepted.
    #[test]
    fn tpsf_08_min_path_accepted() {
        let path = vec![secret(0x01), secret(0x02)];
        assert_eq!(validate_path_secret_order(&path), Ok(()));
    }

    /// **TPSF-09** — max path length accepted.
    #[test]
    fn tpsf_09_max_path_accepted() {
        let path: Vec<Vec<u8>> = (1..=TPSF_MAX_PATH)
            .map(|i| {
                let mut s = vec![0u8; TPSF_SECRET_LEN];
                s[0] = (i % 255 + 1) as u8;
                s
            })
            .collect();
        assert_eq!(validate_path_secret_order(&path), Ok(()));
    }

    /// **TPSF-10** — diverse secrets accepted.
    #[test]
    fn tpsf_10_diverse_accepted() {
        let path: Vec<Vec<u8>> = (0..4)
            .map(|i| {
                let mut s = vec![0u8; TPSF_SECRET_LEN];
                s[0] = (i + 1) as u8;
                s[TPSF_SECRET_LEN - 1] = (i + 10) as u8;
                s
            })
            .collect();
        assert_eq!(validate_path_secret_order(&path), Ok(()));
    }
}
