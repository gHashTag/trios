//! # CR-CHAT-01 — Identity key signature algorithm consistency guard (Wave-152 Lane A)
//!
//! IDENTITY — all signatures in a batch must use the same algorithm;
//! mixing algorithms enables downgrade attacks.
//!
//! In MLS/Signal, identity keys sign protocol messages. If a batch
//! contains signatures using different algorithms:
//!
//! * **Downgrade attack** — an attacker can substitute a weaker
//!   algorithm for a stronger one if mixing is allowed.
//! * **Algorithm confusion** — different algorithms have different
//!   security properties; mixing them creates inconsistent guarantees.
//! * **Cross-algorithm forgery** — some algorithm combinations enable
//!   cross-algorithm attacks when the same key material is used.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. All signatures must use the same algorithm ID.
//! 2. Algorithm ID must be > 0.
//! 3. Signer ID must not be zero.
//! 4. No duplicate signer IDs.
//! 5. Signature must not be empty (sig_len > 0).
//! 6. Batch size <= `IKSC_MAX_SIGNATURES`.
//!
//! Tests **IKSC-01..10**. Error enum [`SignatureConsistencyError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * SIG-CONSISTENT`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum signatures per batch.
pub const IKSC_MAX_SIGNATURES: usize = 256;

/// Signer ID length.
pub const IKSC_SIGNER_ID_LEN: usize = 32;

/// Maximum signature length.
pub const IKSC_MAX_SIG_LEN: usize = 256;

/// A signature record.
#[derive(Debug, Clone)]
pub struct SignatureRecord {
    /// Signer identifier.
    pub signer_id: [u8; IKSC_SIGNER_ID_LEN],
    /// Algorithm identifier.
    pub algo_id: u32,
    /// Signature bytes.
    pub signature: Vec<u8>,
}

/// All ways signature consistency validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SignatureConsistencyError {
    /// Mixed algorithms.
    MixedAlgorithms {
        /// First algorithm.
        expected: u32,
        /// Conflicting algorithm.
        found: u32,
        /// Index.
        idx: usize,
    },
    /// Zero algorithm ID.
    ZeroAlgo(usize),
    /// Zero signer ID.
    ZeroSignerId(usize),
    /// Duplicate signer ID.
    DuplicateSignerId {
        /// Index.
        idx: usize,
    },
    /// Empty signature.
    EmptySignature(usize),
    /// Too many signatures.
    TooMany {
        got: usize,
        max: usize,
    },
}

/// `[VERIFIED]` Validate signature algorithm consistency.
pub fn validate_signature_consistency(
    sigs: &[SignatureRecord],
) -> Result<(), SignatureConsistencyError> {
    if sigs.len() > IKSC_MAX_SIGNATURES {
        return Err(SignatureConsistencyError::TooMany {
            got: sigs.len(),
            max: IKSC_MAX_SIGNATURES,
        });
    }
    let mut seen: BTreeSet<[u8; IKSC_SIGNER_ID_LEN]> = BTreeSet::new();
    let mut first_algo: Option<u32> = None;
    for (i, s) in sigs.iter().enumerate() {
        if s.signer_id == [0u8; IKSC_SIGNER_ID_LEN] {
            return Err(SignatureConsistencyError::ZeroSignerId(i));
        }
        if !seen.insert(s.signer_id) {
            return Err(SignatureConsistencyError::DuplicateSignerId { idx: i });
        }
        if s.algo_id == 0 {
            return Err(SignatureConsistencyError::ZeroAlgo(i));
        }
        if s.signature.is_empty() {
            return Err(SignatureConsistencyError::EmptySignature(i));
        }
        match first_algo {
            None => first_algo = Some(s.algo_id),
            Some(expected) if s.algo_id != expected => {
                return Err(SignatureConsistencyError::MixedAlgorithms {
                    expected,
                    found: s.algo_id,
                    idx: i,
                });
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sid(byte: u8) -> [u8; IKSC_SIGNER_ID_LEN] {
        [byte; IKSC_SIGNER_ID_LEN]
    }

    fn sig(id: u8, algo: u32, sig_bytes: &[u8]) -> SignatureRecord {
        SignatureRecord { signer_id: sid(id), algo_id: algo, signature: sig_bytes.to_vec() }
    }

    fn valid_sigs() -> Vec<SignatureRecord> {
        vec![
            sig(0x01, 1, &[0xAA; 64]),
            sig(0x02, 1, &[0xBB; 64]),
            sig(0x03, 1, &[0xCC; 64]),
        ]
    }

    /// **IKSC-01** — mixed algorithms rejected.
    #[test]
    fn iksc_01_mixed_rejected() {
        let ss = vec![
            sig(0x01, 1, &[0xAA; 64]),
            sig(0x02, 2, &[0xBB; 64]),
        ];
        assert_eq!(
            validate_signature_consistency(&ss),
            Err(SignatureConsistencyError::MixedAlgorithms { expected: 1, found: 2, idx: 1 })
        );
    }

    /// **IKSC-02** — zero algo rejected.
    #[test]
    fn iksc_02_zero_algo_rejected() {
        let s = SignatureRecord { signer_id: sid(0x01), algo_id: 0, signature: vec![0xAA; 64] };
        assert_eq!(
            validate_signature_consistency(&[s]),
            Err(SignatureConsistencyError::ZeroAlgo(0))
        );
    }

    /// **IKSC-03** — zero signer ID rejected.
    #[test]
    fn iksc_03_zero_signer_rejected() {
        let s = SignatureRecord { signer_id: [0u8; IKSC_SIGNER_ID_LEN], algo_id: 1, signature: vec![0xAA; 64] };
        assert_eq!(
            validate_signature_consistency(&[s]),
            Err(SignatureConsistencyError::ZeroSignerId(0))
        );
    }

    /// **IKSC-04** — duplicate signer rejected.
    #[test]
    fn iksc_04_duplicate_rejected() {
        let ss = vec![
            sig(0x01, 1, &[0xAA; 64]),
            sig(0x01, 1, &[0xBB; 64]),
        ];
        assert_eq!(
            validate_signature_consistency(&ss),
            Err(SignatureConsistencyError::DuplicateSignerId { idx: 1 })
        );
    }

    /// **IKSC-05** — empty signature rejected.
    #[test]
    fn iksc_05_empty_sig_rejected() {
        let s = SignatureRecord { signer_id: sid(0x01), algo_id: 1, signature: vec![] };
        assert_eq!(
            validate_signature_consistency(&[s]),
            Err(SignatureConsistencyError::EmptySignature(0))
        );
    }

    /// **IKSC-06** — too many rejected.
    #[test]
    fn iksc_06_too_many_rejected() {
        let ss: Vec<SignatureRecord> = (0..=IKSC_MAX_SIGNATURES)
            .map(|i| {
                let mut s = [0u8; IKSC_SIGNER_ID_LEN];
                let val = (i as u64) + 1;
                s[0..8].copy_from_slice(&val.to_be_bytes());
                SignatureRecord { signer_id: s, algo_id: 1, signature: vec![0xAA; 64] }
            })
            .collect();
        assert_eq!(
            validate_signature_consistency(&ss),
            Err(SignatureConsistencyError::TooMany {
                got: IKSC_MAX_SIGNATURES + 1,
                max: IKSC_MAX_SIGNATURES,
            })
        );
    }

    /// **IKSC-07** — valid accepted.
    #[test]
    fn iksc_07_valid_accepted() {
        assert_eq!(validate_signature_consistency(&valid_sigs()), Ok(()));
    }

    /// **IKSC-08** — empty accepted.
    #[test]
    fn iksc_08_empty_accepted() {
        assert_eq!(validate_signature_consistency(&[]), Ok(()));
    }

    /// **IKSC-09** — single sig accepted.
    #[test]
    fn iksc_09_single_accepted() {
        assert_eq!(validate_signature_consistency(&[sig(0x01, 42, &[0xFF; 128])]), Ok(()));
    }

    /// **IKSC-10** — many same-algo accepted.
    #[test]
    fn iksc_10_many_same_algo_accepted() {
        let ss: Vec<SignatureRecord> = (0..50u8)
            .map(|i| sig(i + 1, 7, &[i; 64]))
            .collect();
        assert_eq!(validate_signature_consistency(&ss), Ok(()));
    }
}
