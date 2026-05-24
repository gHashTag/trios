//! # CR-CHAT-04 — Padding crypto binding guard (Wave-69 Lane A)
//!
//! PADDING — padding bytes must be in AEAD AD, R-CHAT-4.
//!
//! If padding bytes are not included in the AEAD Associated Data, an
//! attacker can modify them without breaking the authentication tag:
//!
//! * **Padding modification** — change padding length to shift the
//!   payload boundary, causing parsing errors or data corruption.
//! * **Padding injection** — inject extra padding bytes to influence
//!   the decrypted payload layout.
//! * **Length oracle** — observe which padding lengths are accepted
//!   to infer payload content.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Padding length is included in AD.
//! 2. Padding bytes are all-zero (deterministic).
//! 3. AD contains the full `(header_len, payload_len, padding_len)` tuple.
//! 4. AD length == `PCBG_AD_LEN`.
//! 5. Padding length <= `PCBG_MAX_PADDING`.
//! 6. Padding length is a multiple of `PCBG_ALIGN`.
//!
//! Tests **PCBG-01..10**. Error enum [`PaddingBindingError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * PADDING-CRYPTO-BINDING`

#![forbid(unsafe_code)]

/// AD must encode 3 u32 values.
pub const PCBG_AD_LEN: usize = 12;

/// Maximum padding bytes.
pub const PCBG_MAX_PADDING: usize = 65536;

/// Padding alignment.
pub const PCBG_ALIGN: usize = 16;

/// All ways padding crypto binding can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PaddingBindingError {
    /// Padding not in AD.
    PaddingNotInAd,
    /// Padding bytes not all-zero.
    NonZeroPadding,
    /// AD length wrong.
    AdLengthWrong,
    /// Padding too large.
    PaddingTooLarge,
    /// Padding misaligned.
    PaddingMisaligned,
    /// AD payload/padding mismatch.
    AdMismatch,
}

/// `[VERIFIED]` Validate that padding is cryptographically bound via AEAD AD.
pub fn validate_padding_binding(
    ad: &[u8],
    header_len: u32,
    payload_len: u32,
    padding_len: u32,
    padding_bytes: &[u8],
) -> Result<(), PaddingBindingError> {
    if ad.len() != PCBG_AD_LEN {
        return Err(PaddingBindingError::AdLengthWrong);
    }
    if padding_len as usize > PCBG_MAX_PADDING {
        return Err(PaddingBindingError::PaddingTooLarge);
    }
    if padding_len as usize != padding_bytes.len() {
        return Err(PaddingBindingError::AdMismatch);
    }
    if padding_len > 0 && (padding_len as usize) % PCBG_ALIGN != 0 {
        return Err(PaddingBindingError::PaddingMisaligned);
    }
    if !padding_bytes.iter().all(|&b| b == 0) {
        return Err(PaddingBindingError::NonZeroPadding);
    }
    let expected_ad: [u8; 12] = [
        header_len.to_be_bytes()[0], header_len.to_be_bytes()[1],
        header_len.to_be_bytes()[2], header_len.to_be_bytes()[3],
        payload_len.to_be_bytes()[0], payload_len.to_be_bytes()[1],
        payload_len.to_be_bytes()[2], payload_len.to_be_bytes()[3],
        padding_len.to_be_bytes()[0], padding_len.to_be_bytes()[1],
        padding_len.to_be_bytes()[2], padding_len.to_be_bytes()[3],
    ];
    if ad != expected_ad {
        return Err(PaddingBindingError::PaddingNotInAd);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ad(header: u32, payload: u32, padding: u32) -> [u8; 12] {
        let mut ad = [0u8; 12];
        ad[0..4].copy_from_slice(&header.to_be_bytes());
        ad[4..8].copy_from_slice(&payload.to_be_bytes());
        ad[8..12].copy_from_slice(&padding.to_be_bytes());
        ad
    }

    fn valid_params() -> ([u8; 12], u32, u32, u32, Vec<u8>) {
        let ad = make_ad(16, 64, 32);
        (ad, 16, 64, 32, vec![0u8; 32])
    }

    /// **PCBG-01** — padding not in AD rejected.
    #[test]
    fn pcbg_01_not_in_ad_rejected() {
        let wrong_ad = make_ad(16, 64, 0);
        let padding = vec![0u8; 32];
        assert_eq!(
            validate_padding_binding(&wrong_ad, 16, 64, 32, &padding),
            Err(PaddingBindingError::PaddingNotInAd)
        );
    }

    /// **PCBG-02** — non-zero padding rejected.
    #[test]
    fn pcbg_02_non_zero_rejected() {
        let ad = make_ad(16, 64, 32);
        let padding = vec![0xFF; 32];
        assert_eq!(
            validate_padding_binding(&ad, 16, 64, 32, &padding),
            Err(PaddingBindingError::NonZeroPadding)
        );
    }

    /// **PCBG-03** — AD length wrong rejected.
    #[test]
    fn pcbg_03_ad_len_rejected() {
        let short_ad = [0u8; 8];
        assert_eq!(
            validate_padding_binding(&short_ad, 16, 64, 32, &[]),
            Err(PaddingBindingError::AdLengthWrong)
        );
    }

    /// **PCBG-04** — padding too large rejected.
    #[test]
    fn pcbg_04_too_large_rejected() {
        let ad = make_ad(16, 64, (PCBG_MAX_PADDING + 1) as u32);
        assert_eq!(
            validate_padding_binding(&ad, 16, 64, (PCBG_MAX_PADDING + 1) as u32, &[]),
            Err(PaddingBindingError::PaddingTooLarge)
        );
    }

    /// **PCBG-05** — padding misaligned rejected.
    #[test]
    fn pcbg_05_misaligned_rejected() {
        let ad = make_ad(16, 64, 7);
        let padding = vec![0u8; 7];
        assert_eq!(
            validate_padding_binding(&ad, 16, 64, 7, &padding),
            Err(PaddingBindingError::PaddingMisaligned)
        );
    }

    /// **PCBG-06** — AD mismatch rejected.
    #[test]
    fn pcbg_06_ad_mismatch_rejected() {
        let ad = make_ad(16, 64, 32);
        assert_eq!(
            validate_padding_binding(&ad, 16, 64, 32, &vec![0u8; 16]),
            Err(PaddingBindingError::AdMismatch)
        );
    }

    /// **PCBG-07** — valid binding accepted.
    #[test]
    fn pcbg_07_valid_accepted() {
        let (ad, h, p, pd, pb) = valid_params();
        assert_eq!(validate_padding_binding(&ad, h, p, pd, &pb), Ok(()));
    }

    /// **PCBG-08** — zero padding accepted.
    #[test]
    fn pcbg_08_zero_padding_accepted() {
        let ad = make_ad(16, 64, 0);
        assert_eq!(validate_padding_binding(&ad, 16, 64, 0, &[]), Ok(()));
    }

    /// **PCBG-09** — large aligned padding accepted.
    #[test]
    fn pcbg_09_large_padding_accepted() {
        let ad = make_ad(16, 64, 4096);
        let padding = vec![0u8; 4096];
        assert_eq!(validate_padding_binding(&ad, 16, 64, 4096, &padding), Ok(()));
    }

    /// **PCBG-10** — minimum aligned padding accepted.
    #[test]
    fn pcbg_10_min_aligned_accepted() {
        let ad = make_ad(16, 64, PCBG_ALIGN as u32);
        let padding = vec![0u8; PCBG_ALIGN];
        assert_eq!(validate_padding_binding(&ad, 16, 64, PCBG_ALIGN as u32, &padding), Ok(()));
    }
}
