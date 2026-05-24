//! # CR-CHAT-01 — Credential chain path length guard (Wave-68 Lane A)
//!
//! IDENTITY — credential chain depth must be bounded, R-CHAT-1.
//!
//! An X.509-style credential chain allows delegation. Without depth
//! limits an attacker can:
//!
//! * **DoS via deep chain** — force the verifier to walk thousands of
//!   certs, exhausting CPU.
//! * **Hide rogue cert** — bury a compromised intermediate in a long
//!   chain, hoping the verifier gives up.
//! * **Cycle injection** — create a circular chain that loops forever.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Chain depth <= `CCPL_MAX_DEPTH`.
//! 2. No cycles (each cert ID appears once).
//! 3. Root cert is self-signed (issuer == subject).
//! 4. Each intermediate's issuer matches the next cert's subject.
//! 5. No duplicate cert IDs in the chain.
//! 6. Chain length >= 1 (at least a root).
//!
//! Tests **CCPL-01..10**. Error enum [`CredentialChainError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * CREDENTIAL-CHAIN-PATH`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum chain depth.
pub const CCPL_MAX_DEPTH: usize = 8;

/// A credential in the chain.
#[derive(Debug, Clone)]
pub struct Credential {
    /// Subject identifier.
    pub subject: Vec<u8>,
    /// Issuer identifier.
    pub issuer: Vec<u8>,
}

/// All ways credential chain validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CredentialChainError {
    /// Chain too deep.
    ChainTooDeep,
    /// Cycle detected.
    CycleDetected,
    /// Root not self-signed.
    NotSelfSignedRoot,
    /// Issuer-subject mismatch at position.
    IssuerMismatch(usize),
    /// Duplicate certificate.
    DuplicateCert,
    /// Empty chain.
    EmptyChain,
}

/// `[VERIFIED]` Validate credential chain depth, continuity, and root.
pub fn validate_credential_chain(
    chain: &[Credential],
) -> Result<(), CredentialChainError> {
    if chain.is_empty() {
        return Err(CredentialChainError::EmptyChain);
    }
    if chain.len() > CCPL_MAX_DEPTH {
        return Err(CredentialChainError::ChainTooDeep);
    }
    let mut seen = BTreeSet::new();
    for cert in chain {
        if !seen.insert(cert.subject.clone()) {
            return Err(CredentialChainError::DuplicateCert);
        }
    }
    let root = &chain[0];
    if root.subject != root.issuer {
        return Err(CredentialChainError::NotSelfSignedRoot);
    }
    for i in 1..chain.len() {
        if chain[i].issuer != chain[i - 1].subject {
            return Err(CredentialChainError::IssuerMismatch(i));
        }
        if chain[i].subject == chain[i - 1].subject {
            return Err(CredentialChainError::CycleDetected);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(byte: u8) -> Vec<u8> {
        vec![byte]
    }

    fn valid_chain_3() -> Vec<Credential> {
        vec![
            Credential { subject: id(0x01), issuer: id(0x01) },
            Credential { subject: id(0x02), issuer: id(0x01) },
            Credential { subject: id(0x03), issuer: id(0x02) },
        ]
    }

    /// **CCPL-01** — chain too deep rejected.
    #[test]
    fn ccpl_01_too_deep_rejected() {
        let chain: Vec<Credential> = (0..=CCPL_MAX_DEPTH)
            .map(|i| Credential {
                subject: id(i as u8),
                issuer: if i == 0 { id(0) } else { id((i - 1) as u8) },
            })
            .collect();
        assert_eq!(
            validate_credential_chain(&chain),
            Err(CredentialChainError::ChainTooDeep)
        );
    }

    /// **CCPL-02** — duplicate subject in chain rejected (cycle proxy).
    #[test]
    fn ccpl_02_cycle_rejected() {
        let chain = vec![
            Credential { subject: id(0x01), issuer: id(0x01) },
            Credential { subject: id(0x02), issuer: id(0x01) },
            Credential { subject: id(0x01), issuer: id(0x02) },
        ];
        assert_eq!(
            validate_credential_chain(&chain),
            Err(CredentialChainError::DuplicateCert)
        );
    }

    /// **CCPL-03** — root not self-signed rejected.
    #[test]
    fn ccpl_03_not_self_signed_rejected() {
        let chain = vec![
            Credential { subject: id(0x01), issuer: id(0x02) },
            Credential { subject: id(0x02), issuer: id(0x01) },
        ];
        assert_eq!(
            validate_credential_chain(&chain),
            Err(CredentialChainError::NotSelfSignedRoot)
        );
    }

    /// **CCPL-04** — issuer mismatch rejected.
    #[test]
    fn ccpl_04_issuer_mismatch_rejected() {
        let chain = vec![
            Credential { subject: id(0x01), issuer: id(0x01) },
            Credential { subject: id(0x02), issuer: id(0xFF) },
        ];
        assert_eq!(
            validate_credential_chain(&chain),
            Err(CredentialChainError::IssuerMismatch(1))
        );
    }

    /// **CCPL-05** — duplicate cert rejected.
    #[test]
    fn ccpl_05_duplicate_rejected() {
        let chain = vec![
            Credential { subject: id(0x01), issuer: id(0x01) },
            Credential { subject: id(0x01), issuer: id(0x01) },
        ];
        assert_eq!(
            validate_credential_chain(&chain),
            Err(CredentialChainError::DuplicateCert)
        );
    }

    /// **CCPL-06** — empty chain rejected.
    #[test]
    fn ccpl_06_empty_rejected() {
        assert_eq!(
            validate_credential_chain(&[]),
            Err(CredentialChainError::EmptyChain)
        );
    }

    /// **CCPL-07** — valid 3-cert chain accepted.
    #[test]
    fn ccpl_07_valid_3_accepted() {
        assert_eq!(validate_credential_chain(&valid_chain_3()), Ok(()));
    }

    /// **CCPL-08** — single self-signed root accepted.
    #[test]
    fn ccpl_08_single_root_accepted() {
        let root = Credential { subject: id(0xAA), issuer: id(0xAA) };
        assert_eq!(validate_credential_chain(&[root]), Ok(()));
    }

    /// **CCPL-09** — max depth chain accepted.
    #[test]
    fn ccpl_09_max_depth_accepted() {
        let n = CCPL_MAX_DEPTH;
        let chain: Vec<Credential> = (0..n)
            .map(|i| Credential {
                subject: id(i as u8),
                issuer: if i == 0 { id(0) } else { id((i - 1) as u8) },
            })
            .collect();
        assert_eq!(validate_credential_chain(&chain), Ok(()));
    }

    /// **CCPL-10** — 2-cert chain accepted.
    #[test]
    fn ccpl_10_two_cert_accepted() {
        let chain = vec![
            Credential { subject: id(0x01), issuer: id(0x01) },
            Credential { subject: id(0x02), issuer: id(0x01) },
        ];
        assert_eq!(validate_credential_chain(&chain), Ok(()));
    }
}
