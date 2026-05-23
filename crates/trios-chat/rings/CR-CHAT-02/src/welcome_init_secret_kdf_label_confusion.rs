//! # CR-CHAT-02 · CR-CHAT-02 — Welcome init-secret KDF label confusion guard
//!
//! Wave-38 Lane A — `welcome_init_secret_kdf_label_confusion` (CR-CHAT-02).
//!
//! Constructive guard at the joiner against KDF label confusion in the
//! `init_secret` derivation. Per RFC 9420 §8.4:
//!
//! ```text
//! init_secret = ExpandWithLabel(epoch_secret, "init", "", KDF.Nh)
//! ```
//!
//! The literal 4-byte ASCII string `"init"` is mandatory. Loose stacks have
//! been caught using `"INIT"`, `"Init"`, `"epoch"`, `"init\0"`, UTF-8
//! homoglyph variants, or skipping ExpandWithLabel entirely. Result: silent
//! two-party key divergence — every application packet decrypts to garbage
//! and the failure mode masquerades as "decryption error" without any obvious
//! attribution.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Canonical 4-byte label length.
//! 2. Every label byte in printable ASCII range `0x20..=0x7e`.
//! 3. Byte-for-byte equality with `b"init"`.
//! 4. Canonical 32-byte `epoch_secret` length.
//! 5. Empty context (per RFC §8.4).
//! 6. Canonical 32-byte declared output length.
//!
//! Tests **WISKLC-01..10**. Error enum [`InitSecretKdfLabelError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · WISKLC`

#![forbid(unsafe_code)]

/// Canonical label for `init_secret` derivation per RFC 9420 §8.4.
pub const WISKLC_INIT_LABEL: &[u8; 4] = b"init";

/// KDF output length in bytes (HKDF-SHA-256 = 32).
pub const WISKLC_KDF_NH: usize = 32;

/// Canonical epoch_secret length in bytes.
pub const WISKLC_EPOCH_SECRET_LEN: usize = 32;

/// Parameters for an `ExpandWithLabel("init", ...)` call that the joiner
/// must validate before deriving the `init_secret`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitSecretKdfParams {
    /// KDF label — MUST be exactly `b"init"` (4 bytes).
    pub label: Vec<u8>,
    /// Epoch secret input — MUST be exactly 32 bytes.
    pub epoch_secret: Vec<u8>,
    /// KDF context — MUST be empty per RFC 9420 §8.4.
    pub context: Vec<u8>,
    /// Declared KDF output length — MUST be exactly 32.
    pub output_len: usize,
}

/// All ways an `ExpandWithLabel("init", ...)` call can be rejected.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum InitSecretKdfLabelError {
    /// `label.len() != 4`.
    NonCanonicalLabelLength,
    /// A label byte falls outside printable ASCII `0x20..=0x7e`.
    NonPrintableLabelByte,
    /// `label != b"init"` byte-for-byte.
    LabelMismatch,
    /// `epoch_secret.len() != 32`.
    NonCanonicalEpochSecretLength,
    /// `context` is non-empty (RFC 9420 §8.4 requires empty context).
    NonEmptyContext,
    /// `output_len != 32`.
    NonCanonicalOutputLength,
}

/// `[VERIFIED]` Validate an `ExpandWithLabel("init", ...)` call against the
/// 6 canonical rules from RFC 9420 §8.4. Returns `Ok(())` if all rules
/// pass, else the first failing rule as an [`InitSecretKdfLabelError`].
///
/// Rules enforced in fixed order:
///
/// 1. `label.len() == 4`.
/// 2. Every byte of `label` in `0x20..=0x7e`.
/// 3. `label == b"init"`.
/// 4. `epoch_secret.len() == 32`.
/// 5. `context.is_empty()`.
/// 6. `output_len == 32`.
pub fn validate_init_secret_kdf_label(
    params: &InitSecretKdfParams,
) -> Result<(), InitSecretKdfLabelError> {
    if params.label.len() != WISKLC_INIT_LABEL.len() {
        return Err(InitSecretKdfLabelError::NonCanonicalLabelLength);
    }
    if !params.label.iter().all(|&b| (0x20..=0x7e).contains(&b)) {
        return Err(InitSecretKdfLabelError::NonPrintableLabelByte);
    }
    if params.label != WISKLC_INIT_LABEL.as_slice() {
        return Err(InitSecretKdfLabelError::LabelMismatch);
    }
    if params.epoch_secret.len() != WISKLC_EPOCH_SECRET_LEN {
        return Err(InitSecretKdfLabelError::NonCanonicalEpochSecretLength);
    }
    if !params.context.is_empty() {
        return Err(InitSecretKdfLabelError::NonEmptyContext);
    }
    if params.output_len != WISKLC_KDF_NH {
        return Err(InitSecretKdfLabelError::NonCanonicalOutputLength);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good_params() -> InitSecretKdfParams {
        InitSecretKdfParams {
            label: b"init".to_vec(),
            epoch_secret: vec![0xAA; 32],
            context: vec![],
            output_len: 32,
        }
    }

    /// **WISKLC-01** — label too short (3 bytes) rejected.
    #[test]
    fn wisklc_01_short_label_rejected() {
        let mut p = good_params();
        p.label = b"ini".to_vec();
        assert_eq!(
            validate_init_secret_kdf_label(&p),
            Err(InitSecretKdfLabelError::NonCanonicalLabelLength)
        );
    }

    /// **WISKLC-02** — label too long (5 bytes) rejected.
    #[test]
    fn wisklc_02_long_label_rejected() {
        let mut p = good_params();
        p.label = b"initx".to_vec();
        assert_eq!(
            validate_init_secret_kdf_label(&p),
            Err(InitSecretKdfLabelError::NonCanonicalLabelLength)
        );
    }

    /// **WISKLC-03** — non-printable byte in label rejected.
    #[test]
    fn wisklc_03_non_printable_label_byte_rejected() {
        let mut p = good_params();
        p.label = vec![0x01, 0x6e, 0x69, 0x74];
        assert_eq!(
            validate_init_secret_kdf_label(&p),
            Err(InitSecretKdfLabelError::NonPrintableLabelByte)
        );
    }

    /// **WISKLC-04** — wrong label content ("INIT") rejected.
    #[test]
    fn wisklc_04_label_mismatch_uppercase_rejected() {
        let mut p = good_params();
        p.label = b"INIT".to_vec();
        assert_eq!(
            validate_init_secret_kdf_label(&p),
            Err(InitSecretKdfLabelError::LabelMismatch)
        );
    }

    /// **WISKLC-05** — wrong label content ("Init") rejected.
    #[test]
    fn wisklc_05_label_mismatch_mixed_case_rejected() {
        let mut p = good_params();
        p.label = b"Init".to_vec();
        assert_eq!(
            validate_init_secret_kdf_label(&p),
            Err(InitSecretKdfLabelError::LabelMismatch)
        );
    }

    /// **WISKLC-06** — non-canonical epoch_secret length (16 bytes) rejected.
    #[test]
    fn wisklc_06_short_epoch_secret_rejected() {
        let mut p = good_params();
        p.epoch_secret = vec![0xBB; 16];
        assert_eq!(
            validate_init_secret_kdf_label(&p),
            Err(InitSecretKdfLabelError::NonCanonicalEpochSecretLength)
        );
    }

    /// **WISKLC-07** — non-empty context rejected.
    #[test]
    fn wisklc_07_non_empty_context_rejected() {
        let mut p = good_params();
        p.context = vec![0x00];
        assert_eq!(
            validate_init_secret_kdf_label(&p),
            Err(InitSecretKdfLabelError::NonEmptyContext)
        );
    }

    /// **WISKLC-08** — non-canonical output length rejected.
    #[test]
    fn wisklc_08_wrong_output_len_rejected() {
        let mut p = good_params();
        p.output_len = 64;
        assert_eq!(
            validate_init_secret_kdf_label(&p),
            Err(InitSecretKdfLabelError::NonCanonicalOutputLength)
        );
    }

    /// **WISKLC-09** — null-terminated label "init\0" rejected.
    #[test]
    fn wisklc_09_null_terminated_label_rejected() {
        let mut p = good_params();
        p.label = b"init\x00".to_vec();
        assert_eq!(
            validate_init_secret_kdf_label(&p),
            Err(InitSecretKdfLabelError::NonCanonicalLabelLength)
        );
    }

    /// **WISKLC-10** — canonical params accepted.
    #[test]
    fn wisklc_10_canonical_params_accepted() {
        assert_eq!(validate_init_secret_kdf_label(&good_params()), Ok(()));
    }
}
