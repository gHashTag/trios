//! # CR-CHAT-01 — Prekey signature algorithm downgrade guard (Wave-50 Lane A)
//!
//! R-CHAT-4 — Signature algorithm downgrade prevention.
//!
//! An adversary who controls a KeyPackage can substitute a weaker
//! signature algorithm (e.g. Ed25519 → no signature, or RSA-1024)
//! to forge prekey bundles. This enables:
//!
//! * **Identity impersonation** — claim another user's identity key.
//! * **Bundle substitution** — replace PQ-hybrid KEM with classical-only.
//! * **Protocol downgrade** — force the group into a weaker crypto suite.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Signature algorithm is recognized.
//! 2. Only approved algorithms are allowed.
//! 3. Signature is non-empty.
//! 4. Signature length matches algorithm expectation.
//! 5. Public key length matches algorithm expectation.
//! 6. No duplicate algorithm entries in a bundle.
//!
//! Tests **PSAD-01..10**. Error enum [`SigAlgoDowngradeError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · SIG-ALGO-DOWNGRADE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Ed25519 signature algorithm.
pub const ALGO_ED25519: u16 = 0x0001;

/// Allowed signature algorithms.
pub const APPROVED_ALGOS: &[u16] = &[ALGO_ED25519];

/// Expected Ed25519 signature length.
pub const ED25519_SIG_LEN: usize = 64;

/// Expected Ed25519 public key length.
pub const ED25519_PK_LEN: usize = 32;

/// All ways signature algorithm downgrade detection can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SigAlgoDowngradeError {
    /// Unknown algorithm.
    UnknownAlgorithm,
    /// Algorithm not approved.
    AlgorithmNotApproved,
    /// Signature is empty.
    EmptySignature,
    /// Signature length mismatch.
    SigLengthMismatch,
    /// Public key length mismatch.
    PkLengthMismatch,
    /// Duplicate algorithm in bundle.
    DuplicateAlgorithm,
}

/// A signature entry in a prekey bundle.
#[derive(Debug, Clone)]
pub struct SignatureEntry {
    /// Algorithm identifier.
    pub algo: u16,
    /// Signature bytes.
    pub signature: Vec<u8>,
    /// Public key bytes.
    pub public_key: Vec<u8>,
}

/// `[VERIFIED]` Validate a single signature entry.
pub fn validate_signature_entry(
    entry: &SignatureEntry,
) -> Result<(), SigAlgoDowngradeError> {
    if entry.signature.is_empty() {
        return Err(SigAlgoDowngradeError::EmptySignature);
    }
    if !APPROVED_ALGOS.contains(&entry.algo) {
        return Err(SigAlgoDowngradeError::AlgorithmNotApproved);
    }
    match entry.algo {
        ALGO_ED25519 => {
            if entry.signature.len() != ED25519_SIG_LEN {
                return Err(SigAlgoDowngradeError::SigLengthMismatch);
            }
            if entry.public_key.len() != ED25519_PK_LEN {
                return Err(SigAlgoDowngradeError::PkLengthMismatch);
            }
        }
        _ => return Err(SigAlgoDowngradeError::UnknownAlgorithm),
    }
    Ok(())
}

/// `[VERIFIED]` Validate a bundle of signature entries for downgrade
/// attacks. Checks each entry individually and for duplicate algorithms.
pub fn validate_signature_bundle(
    entries: &[SignatureEntry],
) -> Result<(), SigAlgoDowngradeError> {
    if entries.is_empty() {
        return Ok(());
    }
    let mut seen_algos = BTreeSet::new();
    for entry in entries {
        validate_signature_entry(entry)?;
        if !seen_algos.insert(entry.algo) {
            return Err(SigAlgoDowngradeError::DuplicateAlgorithm);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_ed25519_entry() -> SignatureEntry {
        SignatureEntry {
            algo: ALGO_ED25519,
            signature: vec![0xAA; ED25519_SIG_LEN],
            public_key: vec![0xBB; ED25519_PK_LEN],
        }
    }

    /// **PSAD-01** — empty signature rejected.
    #[test]
    fn psad_01_empty_sig_rejected() {
        let e = SignatureEntry {
            algo: ALGO_ED25519,
            signature: vec![],
            public_key: vec![0xBB; ED25519_PK_LEN],
        };
        assert_eq!(
            validate_signature_entry(&e),
            Err(SigAlgoDowngradeError::EmptySignature)
        );
    }

    /// **PSAD-02** — unapproved algorithm rejected.
    #[test]
    fn psad_02_unapproved_rejected() {
        let e = SignatureEntry {
            algo: 0x9999,
            signature: vec![0xAA; 64],
            public_key: vec![0xBB; 32],
        };
        assert_eq!(
            validate_signature_entry(&e),
            Err(SigAlgoDowngradeError::AlgorithmNotApproved)
        );
    }

    /// **PSAD-03** — wrong signature length rejected.
    #[test]
    fn psad_03_wrong_sig_len_rejected() {
        let e = SignatureEntry {
            algo: ALGO_ED25519,
            signature: vec![0xAA; 32],
            public_key: vec![0xBB; ED25519_PK_LEN],
        };
        assert_eq!(
            validate_signature_entry(&e),
            Err(SigAlgoDowngradeError::SigLengthMismatch)
        );
    }

    /// **PSAD-04** — wrong public key length rejected.
    #[test]
    fn psad_04_wrong_pk_len_rejected() {
        let e = SignatureEntry {
            algo: ALGO_ED25519,
            signature: vec![0xAA; ED25519_SIG_LEN],
            public_key: vec![0xBB; 16],
        };
        assert_eq!(
            validate_signature_entry(&e),
            Err(SigAlgoDowngradeError::PkLengthMismatch)
        );
    }

    /// **PSAD-05** — duplicate algorithm in bundle rejected.
    #[test]
    fn psad_05_duplicate_algo_rejected() {
        let e = valid_ed25519_entry();
        assert_eq!(
            validate_signature_bundle(&[e.clone(), e]),
            Err(SigAlgoDowngradeError::DuplicateAlgorithm)
        );
    }

    /// **PSAD-06** — valid entry accepted.
    #[test]
    fn psad_06_valid_accepted() {
        assert_eq!(validate_signature_entry(&valid_ed25519_entry()), Ok(()));
    }

    /// **PSAD-07** — valid bundle accepted.
    #[test]
    fn psad_07_valid_bundle_accepted() {
        assert_eq!(validate_signature_bundle(&[valid_ed25519_entry()]), Ok(()));
    }

    /// **PSAD-08** — empty bundle accepted.
    #[test]
    fn psad_08_empty_bundle_accepted() {
        assert_eq!(validate_signature_bundle(&[]), Ok(()));
    }

    /// **PSAD-09** — exact boundary sig length accepted.
    #[test]
    fn psad_09_exact_sig_len_accepted() {
        let e = SignatureEntry {
            algo: ALGO_ED25519,
            signature: vec![0xCC; ED25519_SIG_LEN],
            public_key: vec![0xDD; ED25519_PK_LEN],
        };
        assert_eq!(validate_signature_entry(&e), Ok(()));
    }

    /// **PSAD-10** — exact boundary pk length accepted.
    #[test]
    fn psad_10_exact_pk_len_accepted() {
        let e = SignatureEntry {
            algo: ALGO_ED25519,
            signature: vec![0xCC; ED25519_SIG_LEN],
            public_key: vec![0xDD; ED25519_PK_LEN],
        };
        assert_eq!(validate_signature_entry(&e), Ok(()));
    }
}
