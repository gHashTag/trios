//! Wave-38 / L-CHAT-2-wisklc (R-CHAT-2 / CR-CHAT-02) — Welcome
//! `init_secret` KDF label confusion defence per RFC 9420 §8.4
//! "Key Schedule" and §9.2 "Welcome Messages".
//!
//! The MLS key schedule is anchored by the per-epoch `init_secret`,
//! derived from the previous epoch's `epoch_secret` via:
//!
//!     init_secret = ExpandWithLabel(epoch_secret, "init", "", KDF.Nh)
//!
//! The literal ASCII label `"init"` (4 bytes) is mandatory. RFC 9420
//! §8.4 explicitly enumerates the allowed labels: `"init"`,
//! `"sender data"`, `"encryption"`, `"exporter"`, `"external"`,
//! `"confirm"`, `"membership"`, `"resumption"`, `"authentication"`,
//! `"epoch"`. Using the wrong label here produces a different
//! `init_secret`, which propagates to every key derived in the new
//! epoch \u2014 with `encryption_secret`, `sender_data_secret`, and
//! `confirmation_key` all silently diverging from spec.
//!
//! Mainstream MLS stacks (early OpenMLS PR drafts, several
//! interop-only forks) have been caught with:
//!   * label byte-for-byte mismatch (`"INIT"`, `"Init"`, `"init\0"`),
//!   * UTF-8 multibyte impostors that visually resemble `"init"`
//!     (homoglyph attack),
//!   * empty label (skipping the ExpandWithLabel altogether and
//!     using the raw epoch_secret as init_secret),
//!   * wrong but spec-listed labels (e.g. using `"epoch"` instead
//!     of `"init"`).
//!
//! In two-party interop tests this produces a clean cross-stack DoS:
//! one side derives epoch keys under `"init"`, the other under
//! `"epoch"`, and every application packet decrypts to garbage.
//! Under an active attacker who can dictate either side's
//! implementation choice (e.g. a malicious vendor SDK), this is a
//! covert mis-key oracle.
//!
//! This lane is the consumption-side guard at the joiner processing
//! a Welcome. A single deny wins.
//!
//! Six rules enforced in fixed order:
//!   1. NonCanonicalLabelLength \u2014 `frame.label.len()` must equal
//!      `WISKLC_INIT_LABEL.len()` (4 bytes).
//!   2. NonAsciiLabelByte \u2014 every byte in `frame.label` must be in
//!      the printable ASCII range `0x20..=0x7e` (rejects null bytes,
//!      UTF-8 multibyte impostors, control characters).
//!   3. LabelMismatch \u2014 `frame.label` must equal the constant
//!      `WISKLC_INIT_LABEL` byte-for-byte.
//!   4. NonCanonicalEpochSecretLength \u2014 `frame.epoch_secret.len()`
//!      must equal `WISKLC_KDF_NH` (32 bytes for HKDF-SHA-256, the
//!      MLS-128 ciphersuite default).
//!   5. ContextNotEmpty \u2014 ExpandWithLabel for `init_secret` MUST
//!      use an empty context (`""`), per RFC 9420 §8.4.
//!   6. NonCanonicalOutputLength \u2014 `frame.declared_output_len`
//!      must equal `WISKLC_KDF_NH` (32 bytes \u2014 KDF.Nh).
//!
//! Anchor: `\u03c6\u00b2 + \u03c6\u207b\u00b2 = 3 \u00b7 TRINITY \u00b7 CHAT \u00b7 WELCOME-INIT-SECRET-LABEL`

#![forbid(unsafe_code)]

/// Canonical RFC 9420 §8.4 KDF label for `init_secret`: ASCII `"init"`.
pub const WISKLC_INIT_LABEL: &[u8; 4] = b"init";

/// HKDF-SHA-256 output length \u2014 the MLS-128 ciphersuite default
/// (`KDF.Nh = 32` bytes per RFC 9420 §5.1.1).
pub const WISKLC_KDF_NH: usize = 32;

/// One MLS init-secret-derivation frame as visible to the joiner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InitSecretDerivationFrame {
    /// The KDF label bytes (must be `b"init"` \u2014 4 bytes).
    pub label: Vec<u8>,
    /// Previous epoch's `epoch_secret` (must be `WISKLC_KDF_NH` bytes).
    pub epoch_secret: Vec<u8>,
    /// The ExpandWithLabel context (must be empty).
    pub context: Vec<u8>,
    /// Declared length of the produced `init_secret`.
    pub declared_output_len: usize,
}

/// Typed errors for `validate_init_secret_kdf_label`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum InitSecretKdfLabelError {
    /// Rule 1 \u2014 label length is not 4.
    NonCanonicalLabelLength,
    /// Rule 2 \u2014 label contains non-printable-ASCII bytes.
    NonAsciiLabelByte,
    /// Rule 3 \u2014 label is not byte-equal to `WISKLC_INIT_LABEL`.
    LabelMismatch,
    /// Rule 4 \u2014 epoch_secret length is not `WISKLC_KDF_NH`.
    NonCanonicalEpochSecretLength,
    /// Rule 5 \u2014 context is non-empty.
    ContextNotEmpty,
    /// Rule 6 \u2014 declared output length is not `WISKLC_KDF_NH`.
    NonCanonicalOutputLength,
}

/// Constructive guard for one Welcome `init_secret` derivation frame.
/// Returns `Ok(())` iff every rule (1)..(6) holds.
///
/// `[VERIFIED]` against the 10 unit tests `WISKLC-01..10` below and
/// the Coq theorems `INV-CHAT-248..252` in the W38 Section of
/// `proofs/chat/Trinity_Chat.v`.
pub fn validate_init_secret_kdf_label(
    frame: &InitSecretDerivationFrame,
) -> Result<(), InitSecretKdfLabelError> {
    // Rule 1: label length canonical.
    if frame.label.len() != WISKLC_INIT_LABEL.len() {
        return Err(InitSecretKdfLabelError::NonCanonicalLabelLength);
    }
    // Rule 2: every label byte printable ASCII.
    for &b in frame.label.iter() {
        if !(0x20..=0x7e).contains(&b) {
            return Err(InitSecretKdfLabelError::NonAsciiLabelByte);
        }
    }
    // Rule 3: label byte-for-byte match.
    if frame.label.as_slice() != WISKLC_INIT_LABEL.as_slice() {
        return Err(InitSecretKdfLabelError::LabelMismatch);
    }
    // Rule 4: epoch_secret length canonical.
    if frame.epoch_secret.len() != WISKLC_KDF_NH {
        return Err(InitSecretKdfLabelError::NonCanonicalEpochSecretLength);
    }
    // Rule 5: context empty.
    if !frame.context.is_empty() {
        return Err(InitSecretKdfLabelError::ContextNotEmpty);
    }
    // Rule 6: declared output length canonical.
    if frame.declared_output_len != WISKLC_KDF_NH {
        return Err(InitSecretKdfLabelError::NonCanonicalOutputLength);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_frame() -> InitSecretDerivationFrame {
        InitSecretDerivationFrame {
            label: WISKLC_INIT_LABEL.to_vec(),
            epoch_secret: vec![0xAB_u8; WISKLC_KDF_NH],
            context: Vec::new(),
            declared_output_len: WISKLC_KDF_NH,
        }
    }

    /// WISKLC-01 \u2014 short label (3 bytes) rejected \u2014 Rule 1.
    #[test]
    fn wisklc_01_short_label_rejected() {
        let mut f = ok_frame();
        f.label = b"ini".to_vec();
        assert_eq!(
            validate_init_secret_kdf_label(&f),
            Err(InitSecretKdfLabelError::NonCanonicalLabelLength)
        );
    }

    /// WISKLC-02 \u2014 long label (5 bytes, trailing null) rejected \u2014 Rule 1.
    #[test]
    fn wisklc_02_long_label_rejected() {
        let mut f = ok_frame();
        f.label = b"init\0".to_vec();
        assert_eq!(
            validate_init_secret_kdf_label(&f),
            Err(InitSecretKdfLabelError::NonCanonicalLabelLength)
        );
    }

    /// WISKLC-03 \u2014 non-ASCII label byte rejected \u2014 Rule 2.
    #[test]
    fn wisklc_03_non_ascii_label_byte_rejected() {
        let mut f = ok_frame();
        // 4-byte UTF-8 multibyte sequence imitating "init" length but
        // containing high bytes.
        f.label = vec![0xE2, 0x80, 0x90, 0x91];
        assert_eq!(
            validate_init_secret_kdf_label(&f),
            Err(InitSecretKdfLabelError::NonAsciiLabelByte)
        );
    }

    /// WISKLC-04 \u2014 wrong-case label "INIT" rejected \u2014 Rule 3.
    #[test]
    fn wisklc_04_uppercase_label_rejected() {
        let mut f = ok_frame();
        f.label = b"INIT".to_vec();
        assert_eq!(
            validate_init_secret_kdf_label(&f),
            Err(InitSecretKdfLabelError::LabelMismatch)
        );
    }

    /// WISKLC-05 \u2014 wrong-but-listed label "epoch" rejected \u2014 Rule 3.
    /// (Length 5 != 4, so really Rule 1 fires \u2014 we use a 4-byte alt.)
    #[test]
    fn wisklc_05_alt_label_same_length_rejected() {
        let mut f = ok_frame();
        f.label = b"seed".to_vec(); // 4 bytes, valid ASCII, not "init"
        assert_eq!(
            validate_init_secret_kdf_label(&f),
            Err(InitSecretKdfLabelError::LabelMismatch)
        );
    }

    /// WISKLC-06 \u2014 short epoch_secret (16 bytes) rejected \u2014 Rule 4.
    #[test]
    fn wisklc_06_short_epoch_secret_rejected() {
        let mut f = ok_frame();
        f.epoch_secret = vec![0xAB_u8; 16];
        assert_eq!(
            validate_init_secret_kdf_label(&f),
            Err(InitSecretKdfLabelError::NonCanonicalEpochSecretLength)
        );
    }

    /// WISKLC-07 \u2014 non-empty context rejected \u2014 Rule 5.
    #[test]
    fn wisklc_07_non_empty_context_rejected() {
        let mut f = ok_frame();
        f.context = b"extra".to_vec();
        assert_eq!(
            validate_init_secret_kdf_label(&f),
            Err(InitSecretKdfLabelError::ContextNotEmpty)
        );
    }

    /// WISKLC-08 \u2014 non-canonical declared output length rejected \u2014 Rule 6.
    #[test]
    fn wisklc_08_non_canonical_output_len_rejected() {
        let mut f = ok_frame();
        f.declared_output_len = 64;
        assert_eq!(
            validate_init_secret_kdf_label(&f),
            Err(InitSecretKdfLabelError::NonCanonicalOutputLength)
        );
    }

    /// WISKLC-09 \u2014 zero-length declared output rejected \u2014 Rule 6.
    #[test]
    fn wisklc_09_zero_output_len_rejected() {
        let mut f = ok_frame();
        f.declared_output_len = 0;
        assert_eq!(
            validate_init_secret_kdf_label(&f),
            Err(InitSecretKdfLabelError::NonCanonicalOutputLength)
        );
    }

    /// WISKLC-10 \u2014 canonical init-secret derivation accepted.
    #[test]
    fn wisklc_10_canonical_frame_accepted() {
        assert_eq!(validate_init_secret_kdf_label(&ok_frame()), Ok(()));
    }
}
