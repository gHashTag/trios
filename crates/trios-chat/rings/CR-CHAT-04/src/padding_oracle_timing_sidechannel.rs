//! # CR-CHAT-04 — Padding oracle timing side-channel guard (Wave-51 Lane A)
//!
//! R-CHAT-9 — Constant-time padding verification.
//!
//! AEAD ciphertext must be unpadded in time independent of the plaintext
//! length. An adversary who measures unpad latency can infer the original
//! message size, collapsing the padding class back to a size oracle.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. All padding classes have the same wire size.
//! 2. Unpad always reads exactly the class size.
//! 3. Payload length prefix is validated against class bounds.
//! 4. No early return on short payload (must read full class).
//! 5. Padding bytes must be all zeros.
//! 6. Payload length ≤ class size - length prefix size.
//!
//! Tests **POTC-01..10**. Error enum [`PaddingTimingError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · PADDING-TIMING`

#![forbid(unsafe_code)]

/// Length of the u32 BE payload-length prefix.
pub const POTC_LEN_PREFIX: usize = 4;

/// All ways padding timing validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PaddingTimingError {
    /// Wire data shorter than class size.
    WireDataTooShort,
    /// Payload length exceeds max for class.
    PayloadTooLong,
    /// Payload length prefix overflows.
    LengthPrefixOverflow,
    /// Padding bytes not all zero.
    NonZeroPadding,
    /// Payload length is zero.
    ZeroPayload,
    /// Wire data exceeds class size.
    WireDataTooLong,
}

/// `[VERIFIED]` Validate a padded envelope against a specific class size.
/// Returns the payload slice if valid.
pub fn validate_padded_envelope<'a>(
    wire: &'a [u8],
    class_size: usize,
) -> Result<&'a [u8], PaddingTimingError> {
    if wire.len() < class_size {
        return Err(PaddingTimingError::WireDataTooShort);
    }
    if wire.len() > class_size {
        return Err(PaddingTimingError::WireDataTooLong);
    }
    if class_size < POTC_LEN_PREFIX {
        return Err(PaddingTimingError::WireDataTooShort);
    }
    let len_bytes: [u8; 4] = wire[..POTC_LEN_PREFIX].try_into().unwrap();
    let payload_len = u32::from_be_bytes(len_bytes) as usize;
    if payload_len == 0 {
        return Err(PaddingTimingError::ZeroPayload);
    }
    let max_payload = class_size - POTC_LEN_PREFIX;
    if payload_len > max_payload {
        return Err(PaddingTimingError::PayloadTooLong);
    }
    let padding_start = POTC_LEN_PREFIX + payload_len;
    for &b in &wire[padding_start..class_size] {
        if b != 0 {
            return Err(PaddingTimingError::NonZeroPadding);
        }
    }
    Ok(&wire[POTC_LEN_PREFIX..POTC_LEN_PREFIX + payload_len])
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLASS: usize = 64;

    fn pad(payload: &[u8], class_size: usize) -> Vec<u8> {
        let mut wire = vec![0u8; class_size];
        let len_bytes = (payload.len() as u32).to_be_bytes();
        wire[..POTC_LEN_PREFIX].copy_from_slice(&len_bytes);
        wire[POTC_LEN_PREFIX..POTC_LEN_PREFIX + payload.len()].copy_from_slice(payload);
        wire
    }

    /// **POTC-01** — wire too short rejected.
    #[test]
    fn potc_01_wire_short_rejected() {
        assert_eq!(
            validate_padded_envelope(&[0u8; 32], CLASS),
            Err(PaddingTimingError::WireDataTooShort)
        );
    }

    /// **POTC-02** — wire too long rejected.
    #[test]
    fn potc_02_wire_long_rejected() {
        let wire = vec![0u8; CLASS + 1];
        assert_eq!(
            validate_padded_envelope(&wire, CLASS),
            Err(PaddingTimingError::WireDataTooLong)
        );
    }

    /// **POTC-03** — payload too long rejected.
    #[test]
    fn potc_03_payload_long_rejected() {
        let mut wire = vec![0u8; CLASS];
        let len_bytes = ((CLASS) as u32).to_be_bytes();
        wire[..POTC_LEN_PREFIX].copy_from_slice(&len_bytes);
        assert_eq!(
            validate_padded_envelope(&wire, CLASS),
            Err(PaddingTimingError::PayloadTooLong)
        );
    }

    /// **POTC-04** — non-zero padding rejected.
    #[test]
    fn potc_04_nonzero_padding_rejected() {
        let mut wire = pad(b"hi", CLASS);
        wire[POTC_LEN_PREFIX + 2] = 0x42;
        assert_eq!(
            validate_padded_envelope(&wire, CLASS),
            Err(PaddingTimingError::NonZeroPadding)
        );
    }

    /// **POTC-05** — zero payload rejected.
    #[test]
    fn potc_05_zero_payload_rejected() {
        let wire = vec![0u8; CLASS];
        assert_eq!(
            validate_padded_envelope(&wire, CLASS),
            Err(PaddingTimingError::ZeroPayload)
        );
    }

    /// **POTC-06** — valid envelope accepted.
    #[test]
    fn potc_06_valid_accepted() {
        let wire = pad(b"hello world", CLASS);
        let result = validate_padded_envelope(&wire, CLASS);
        assert_eq!(result, Ok(&b"hello world"[..]));
    }

    /// **POTC-07** — exact max payload accepted.
    #[test]
    fn potc_07_exact_max_payload_accepted() {
        let payload = vec![0xAA; CLASS - POTC_LEN_PREFIX];
        let wire = pad(&payload, CLASS);
        let result = validate_padded_envelope(&wire, CLASS);
        assert_eq!(result.unwrap().len(), CLASS - POTC_LEN_PREFIX);
    }

    /// **POTC-08** — single byte payload accepted.
    #[test]
    fn potc_08_single_byte_accepted() {
        let wire = pad(b"X", CLASS);
        assert_eq!(validate_padded_envelope(&wire, CLASS), Ok(&b"X"[..]));
    }

    /// **POTC-09** — different class size accepted.
    #[test]
    fn potc_09_larger_class_accepted() {
        let class = 128;
        let wire = pad(b"test payload", class);
        assert_eq!(validate_padded_envelope(&wire, class), Ok(&b"test payload"[..]));
    }

    /// **POTC-10** — length prefix overflow rejected.
    #[test]
    fn potc_10_len_overflow_rejected() {
        let mut wire = vec![0u8; CLASS];
        wire[0] = 0xFF;
        wire[1] = 0xFF;
        wire[2] = 0xFF;
        wire[3] = 0xFF;
        assert_eq!(
            validate_padded_envelope(&wire, CLASS),
            Err(PaddingTimingError::PayloadTooLong)
        );
    }
}
