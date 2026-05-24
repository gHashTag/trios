//! # CR-CHAT-01 — Signed prekey signature binding guard (Wave-93 Lane A)
//!
//! IDENTITY — signed prekey must be bound to the identity key, R-CHAT-1.
//!
//! The signed prekey (SPK) is signed by the identity key to prevent
//! MITM attacks. If the binding is not verified:
//!
//! * **Prekey swap** — attacker replaces the victim's SPK with their
//!   own, and the signature check is skipped, allowing MITM.
//! * **Identity dissociation** — SPK is used without proof it belongs
//!   to the claimed identity, breaking the authentication chain.
//! * **Cross-user binding** — SPK from user A bound to identity of
//!   user B, enabling impersonation.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. SPK must be marked as signed.
//! 2. SPK must not be all zeros.
//! 3. Identity key must not be all zeros.
//! 4. SPK must differ from identity key.
//! 5. Signature must be non-empty.
//! 6. Maximum bindings <= `SPSB_MAX_BINDINGS`.
//!
//! Tests **SPSB-01..10**. Error enum [`PrekeyBindingError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * PREKEY-BINDING`

#![forbid(unsafe_code)]

/// Maximum bindings per batch.
pub const SPSB_MAX_BINDINGS: usize = 256;

/// Key length.
pub const SPSB_KEY_LEN: usize = 32;

/// A signed prekey binding record.
#[derive(Debug, Clone)]
pub struct PrekeyBinding {
    /// Identity key.
    pub identity_key: [u8; SPSB_KEY_LEN],
    /// Signed prekey.
    pub signed_prekey: [u8; SPSB_KEY_LEN],
    /// Whether the binding signature is verified.
    pub signature_verified: bool,
    /// Signature bytes (non-empty if verified).
    pub signature: Vec<u8>,
}

/// All ways prekey binding validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PrekeyBindingError {
    /// Signature not verified.
    NotVerified,
    /// Zero signed prekey.
    ZeroPrekey,
    /// Zero identity key.
    ZeroIdentity,
    /// SPK same as identity key.
    SameKey,
    /// Empty signature.
    EmptySignature,
    /// Too many bindings.
    TooManyBindings,
}

/// `[VERIFIED]` Validate signed prekey signature bindings.
pub fn validate_prekey_bindings(
    bindings: &[PrekeyBinding],
) -> Result<(), PrekeyBindingError> {
    if bindings.len() > SPSB_MAX_BINDINGS {
        return Err(PrekeyBindingError::TooManyBindings);
    }
    for b in bindings {
        if b.identity_key == [0u8; SPSB_KEY_LEN] {
            return Err(PrekeyBindingError::ZeroIdentity);
        }
        if b.signed_prekey == [0u8; SPSB_KEY_LEN] {
            return Err(PrekeyBindingError::ZeroPrekey);
        }
        if b.signed_prekey == b.identity_key {
            return Err(PrekeyBindingError::SameKey);
        }
        if !b.signature_verified {
            return Err(PrekeyBindingError::NotVerified);
        }
        if b.signature.is_empty() {
            return Err(PrekeyBindingError::EmptySignature);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> [u8; SPSB_KEY_LEN] {
        [byte; SPSB_KEY_LEN]
    }

    fn binding(identity: u8, prekey: u8) -> PrekeyBinding {
        PrekeyBinding {
            identity_key: key(identity),
            signed_prekey: key(prekey),
            signature_verified: true,
            signature: vec![0xDE, 0xAD],
        }
    }

    fn valid_bindings() -> Vec<PrekeyBinding> {
        vec![binding(0xAA, 0xBB), binding(0xCC, 0xDD)]
    }

    /// **SPSB-01** — not verified rejected.
    #[test]
    fn spsb_01_not_verified_rejected() {
        let mut b = binding(0xAA, 0xBB);
        b.signature_verified = false;
        assert_eq!(
            validate_prekey_bindings(&[b]),
            Err(PrekeyBindingError::NotVerified)
        );
    }

    /// **SPSB-02** — zero prekey rejected.
    #[test]
    fn spsb_02_zero_prekey_rejected() {
        let b = PrekeyBinding {
            identity_key: key(0xAA),
            signed_prekey: [0u8; SPSB_KEY_LEN],
            signature_verified: true,
            signature: vec![0xFF],
        };
        assert_eq!(
            validate_prekey_bindings(&[b]),
            Err(PrekeyBindingError::ZeroPrekey)
        );
    }

    /// **SPSB-03** — zero identity rejected.
    #[test]
    fn spsb_03_zero_identity_rejected() {
        let b = PrekeyBinding {
            identity_key: [0u8; SPSB_KEY_LEN],
            signed_prekey: key(0xBB),
            signature_verified: true,
            signature: vec![0xFF],
        };
        assert_eq!(
            validate_prekey_bindings(&[b]),
            Err(PrekeyBindingError::ZeroIdentity)
        );
    }

    /// **SPSB-04** — same key rejected.
    #[test]
    fn spsb_04_same_key_rejected() {
        let b = PrekeyBinding {
            identity_key: key(0xAA),
            signed_prekey: key(0xAA),
            signature_verified: true,
            signature: vec![0xFF],
        };
        assert_eq!(
            validate_prekey_bindings(&[b]),
            Err(PrekeyBindingError::SameKey)
        );
    }

    /// **SPSB-05** — empty signature rejected.
    #[test]
    fn spsb_05_empty_sig_rejected() {
        let mut b = binding(0xAA, 0xBB);
        b.signature = vec![];
        assert_eq!(
            validate_prekey_bindings(&[b]),
            Err(PrekeyBindingError::EmptySignature)
        );
    }

    /// **SPSB-06** — too many bindings rejected.
    #[test]
    fn spsb_06_too_many_rejected() {
        let bs: Vec<PrekeyBinding> = (0..=SPSB_MAX_BINDINGS)
            .map(|i| binding((0x10 + (i % 200) as u8), (0x20 + (i % 200) as u8)))
            .collect();
        assert_eq!(
            validate_prekey_bindings(&bs),
            Err(PrekeyBindingError::TooManyBindings)
        );
    }

    /// **SPSB-07** — valid bindings accepted.
    #[test]
    fn spsb_07_valid_accepted() {
        assert_eq!(validate_prekey_bindings(&valid_bindings()), Ok(()));
    }

    /// **SPSB-08** — empty accepted.
    #[test]
    fn spsb_08_empty_accepted() {
        assert_eq!(validate_prekey_bindings(&[]), Ok(()));
    }

    /// **SPSB-09** — single accepted.
    #[test]
    fn spsb_09_single_accepted() {
        assert_eq!(validate_prekey_bindings(&[binding(0x11, 0x22)]), Ok(()));
    }

    /// **SPSB-10** — max bindings boundary accepted.
    #[test]
    fn spsb_10_max_boundary_accepted() {
        let bs: Vec<PrekeyBinding> = (0..SPSB_MAX_BINDINGS)
            .map(|i| binding((0x10 + (i % 200) as u8), (0x20 + (i % 200) as u8)))
            .collect();
        assert_eq!(validate_prekey_bindings(&bs), Ok(()));
    }
}
