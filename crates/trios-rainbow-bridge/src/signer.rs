//! ed25519 signing / verification helpers for honey-channel events.
//!
//! A small wrapper so the rest of the crate can sign payload bytes without
//! pulling `ed25519_dalek` symbols into every module.

use ed25519_dalek::{
    Signature, SigningKey, VerifyingKey as DalekVerifyingKey, SECRET_KEY_LENGTH, Signer as _, Verifier as _,
};
use rand_core::OsRng;
use thiserror::Error;

/// Errors produced by [`HoneySigner`].
#[derive(Debug, Error)]
pub enum SigningError {
    /// The signature bytes are not a valid ed25519 signature.
    #[error("invalid signature bytes")]
    InvalidSignatureBytes,
    /// The signature is well-formed but does not verify against the
    /// given public key + message.
    #[error("signature verification failed")]
    VerifyFailed,
}

/// Opaque verifying key used to check honey-channel signatures.
#[derive(Debug, Clone)]
pub struct VerifyingKey(DalekVerifyingKey);

impl VerifyingKey {
    /// Raw 32-byte public key.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }
}

/// ed25519 signer. Holds a secret key — never expose outside the agent
/// process. In production this should be loaded from the OS keychain.
#[derive(Debug, Clone)]
pub struct HoneySigner {
    key: SigningKey,
}

impl HoneySigner {
    /// Generate a fresh signing key using the OS RNG.
    #[must_use]
    pub fn generate() -> Self {
        Self {
            key: SigningKey::generate(&mut OsRng),
        }
    }

    /// Deterministic signer for tests.
    #[must_use]
    pub fn from_seed(seed: &[u8; SECRET_KEY_LENGTH]) -> Self {
        Self {
            key: SigningKey::from_bytes(seed),
        }
    }

    /// Corresponding verifying key.
    #[must_use]
    pub fn verifying_key(&self) -> VerifyingKey {
        VerifyingKey(self.key.verifying_key())
    }

    /// Sign `bytes`. Returns the 64-byte ed25519 signature.
    #[must_use]
    pub fn sign(&self, bytes: &[u8]) -> Vec<u8> {
        self.key.sign(bytes).to_bytes().to_vec()
    }

    /// Verify `signature` on `bytes` against `key`.
    pub fn verify(
        key: &VerifyingKey,
        bytes: &[u8],
        signature: &[u8],
    ) -> Result<(), SigningError> {
        let sig_array: [u8; 64] = signature
            .try_into()
            .map_err(|_| SigningError::InvalidSignatureBytes)?;
        let sig = Signature::from_bytes(&sig_array);
        key.0
            .verify(bytes, &sig)
            .map_err(|_| SigningError::VerifyFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_then_verify_roundtrip() {
        let signer = HoneySigner::from_seed(&[7u8; SECRET_KEY_LENGTH]);
        let msg = b"rainbow bridge honey event";
        let sig = signer.sign(msg);
        HoneySigner::verify(&signer.verifying_key(), msg, &sig).expect("valid signature");
    }

    #[test]
    fn tampered_message_rejected() {
        let signer = HoneySigner::from_seed(&[9u8; SECRET_KEY_LENGTH]);
        let sig = signer.sign(b"original");
        let result = HoneySigner::verify(&signer.verifying_key(), b"tampered", &sig);
        assert!(matches!(result, Err(SigningError::VerifyFailed)));
    }

    #[test]
    fn malformed_signature_rejected() {
        let signer = HoneySigner::from_seed(&[1u8; SECRET_KEY_LENGTH]);
        let result = HoneySigner::verify(&signer.verifying_key(), b"m", &[0u8; 10]);
        assert!(matches!(result, Err(SigningError::InvalidSignatureBytes)));
    }
}
