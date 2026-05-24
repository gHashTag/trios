//! # CR-CHAT-02 — Chain key derivation domain separation guard (Wave-124 Lane A)
//!
//! RATCHET — chain keys derived for different purposes must use
//! domain-separated labels; mixing domains enables cross-chain
//! key recovery.
//!
//! The Double Ratchet derives chain keys for sending and receiving.
//! If the KDF inputs are not domain-separated:
//!
//! * **Cross-chain recovery** — knowing a sending chain key could
//!   let the attacker derive the receiving chain key, or vice versa.
//! * **Key reuse** — the same chain key material used for both
//!   directions breaks confidentiality of one direction.
//! * **KDF collision** — without domain separation, different
//!   derivations may produce the same output for different inputs.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Domain label must be in `CKDS_APPROVED_DOMAINS`.
//! 2. Chain ID must not be zero.
//! 3. Key hash must not be zero.
//! 4. No duplicate chain IDs across different domains.
//! 5. No same chain ID used with multiple domains.
//! 6. Total derivations <= `CKDS_MAX_DERIVATIONS`.
//!
//! Tests **CKDS-01..10**. Error enum [`DomainSepError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * DOMAIN-SEPARATED`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Approved domain labels.
pub const CKDS_APPROVED_DOMAINS: &[&str] = &["sending", "receiving", "skipped", "header"];

/// Maximum derivations per batch.
pub const CKDS_MAX_DERIVATIONS: usize = 1024;

/// Chain ID length.
pub const CKDS_CHAIN_ID_LEN: usize = 32;

/// Key hash length.
pub const CKDS_HASH_LEN: usize = 32;

/// A chain key derivation record.
#[derive(Debug, Clone)]
pub struct DerivationRecord {
    /// Domain label (e.g., "sending", "receiving").
    pub domain: String,
    /// Chain identifier.
    pub chain_id: [u8; CKDS_CHAIN_ID_LEN],
    /// Hash of the derived key.
    pub key_hash: [u8; CKDS_HASH_LEN],
}

/// All ways domain separation validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DomainSepError {
    /// Domain not approved.
    UnapprovedDomain { idx: usize, domain: String },
    /// Zero chain ID.
    ZeroChainId(usize),
    /// Zero key hash.
    ZeroKeyHash(usize),
    /// Chain ID reused across different domains.
    CrossDomainReuse { idx: usize, chain_id: [u8; CKDS_CHAIN_ID_LEN], first_domain: String },
    /// Duplicate chain ID within same domain.
    DuplicateChainId { idx: usize, chain_id: [u8; CKDS_CHAIN_ID_LEN] },
    /// Too many derivations.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate chain key derivation domain separation.
pub fn validate_domain_separation(
    records: &[DerivationRecord],
) -> Result<(), DomainSepError> {
    if records.len() > CKDS_MAX_DERIVATIONS {
        return Err(DomainSepError::TooMany {
            got: records.len(),
            max: CKDS_MAX_DERIVATIONS,
        });
    }
    let mut chain_to_domain: std::collections::BTreeMap<[u8; CKDS_CHAIN_ID_LEN], String> =
        std::collections::BTreeMap::new();
    let mut seen: BTreeSet<([u8; CKDS_CHAIN_ID_LEN], String)> = BTreeSet::new();
    for (i, r) in records.iter().enumerate() {
        if !CKDS_APPROVED_DOMAINS.contains(&r.domain.as_str()) {
            return Err(DomainSepError::UnapprovedDomain {
                idx: i,
                domain: r.domain.clone(),
            });
        }
        if r.chain_id == [0u8; CKDS_CHAIN_ID_LEN] {
            return Err(DomainSepError::ZeroChainId(i));
        }
        if r.key_hash == [0u8; CKDS_HASH_LEN] {
            return Err(DomainSepError::ZeroKeyHash(i));
        }
        if let Some(first) = chain_to_domain.get(&r.chain_id) {
            if first != &r.domain {
                return Err(DomainSepError::CrossDomainReuse {
                    idx: i,
                    chain_id: r.chain_id,
                    first_domain: first.clone(),
                });
            }
        } else {
            chain_to_domain.insert(r.chain_id, r.domain.clone());
        }
        let key = (r.chain_id, r.domain.clone());
        if !seen.insert(key) {
            return Err(DomainSepError::DuplicateChainId {
                idx: i,
                chain_id: r.chain_id,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cid(byte: u8) -> [u8; CKDS_CHAIN_ID_LEN] {
        [byte; CKDS_CHAIN_ID_LEN]
    }

    fn khash(byte: u8) -> [u8; CKDS_HASH_LEN] {
        [byte; CKDS_HASH_LEN]
    }

    fn derivation(chain: u8, domain: &str, key: u8) -> DerivationRecord {
        DerivationRecord { domain: domain.to_string(), chain_id: cid(chain), key_hash: khash(key) }
    }

    fn valid_batch() -> Vec<DerivationRecord> {
        vec![
            derivation(0x01, "sending", 0xA1),
            derivation(0x02, "receiving", 0xA2),
            derivation(0x03, "skipped", 0xA3),
        ]
    }

    /// **CKDS-01** — unapproved domain rejected.
    #[test]
    fn ckds_01_unapproved_domain_rejected() {
        let rs = vec![derivation(0x01, "unknown", 0xAA)];
        assert_eq!(
            validate_domain_separation(&rs),
            Err(DomainSepError::UnapprovedDomain { idx: 0, domain: "unknown".to_string() })
        );
    }

    /// **CKDS-02** — zero chain ID rejected.
    #[test]
    fn ckds_02_zero_chain_rejected() {
        let r = DerivationRecord { domain: "sending".to_string(), chain_id: [0u8; CKDS_CHAIN_ID_LEN], key_hash: khash(0xAA) };
        assert_eq!(
            validate_domain_separation(&[r]),
            Err(DomainSepError::ZeroChainId(0))
        );
    }

    /// **CKDS-03** — zero key hash rejected.
    #[test]
    fn ckds_03_zero_key_rejected() {
        let r = DerivationRecord { domain: "sending".to_string(), chain_id: cid(0x01), key_hash: [0u8; CKDS_HASH_LEN] };
        assert_eq!(
            validate_domain_separation(&[r]),
            Err(DomainSepError::ZeroKeyHash(0))
        );
    }

    /// **CKDS-04** — cross-domain reuse rejected.
    #[test]
    fn ckds_04_cross_domain_rejected() {
        let rs = vec![
            derivation(0x01, "sending", 0xA1),
            derivation(0x01, "receiving", 0xA2),
        ];
        assert_eq!(
            validate_domain_separation(&rs),
            Err(DomainSepError::CrossDomainReuse {
                idx: 1,
                chain_id: cid(0x01),
                first_domain: "sending".to_string(),
            })
        );
    }

    /// **CKDS-05** — duplicate chain within domain rejected.
    #[test]
    fn ckds_05_duplicate_chain_rejected() {
        let rs = vec![
            derivation(0x01, "sending", 0xA1),
            derivation(0x01, "sending", 0xA2),
        ];
        assert_eq!(
            validate_domain_separation(&rs),
            Err(DomainSepError::DuplicateChainId { idx: 1, chain_id: cid(0x01) })
        );
    }

    /// **CKDS-06** — too many rejected.
    #[test]
    fn ckds_06_too_many_rejected() {
        let rs: Vec<DerivationRecord> = (0..=CKDS_MAX_DERIVATIONS)
            .map(|i| {
                let mut c = [0u8; CKDS_CHAIN_ID_LEN];
                let val = (i as u64) + 1;
                c[0..8].copy_from_slice(&val.to_be_bytes());
                let mut k = [0u8; CKDS_HASH_LEN];
                k[0..8].copy_from_slice(&(val + 50000).to_be_bytes());
                DerivationRecord { domain: "sending".to_string(), chain_id: c, key_hash: k }
            })
            .collect();
        assert_eq!(
            validate_domain_separation(&rs),
            Err(DomainSepError::TooMany {
                got: CKDS_MAX_DERIVATIONS + 1,
                max: CKDS_MAX_DERIVATIONS,
            })
        );
    }

    /// **CKDS-07** — valid accepted.
    #[test]
    fn ckds_07_valid_accepted() {
        assert_eq!(validate_domain_separation(&valid_batch()), Ok(()));
    }

    /// **CKDS-08** — empty accepted.
    #[test]
    fn ckds_08_empty_accepted() {
        assert_eq!(validate_domain_separation(&[]), Ok(()));
    }

    /// **CKDS-09** — all domains accepted.
    #[test]
    fn ckds_09_all_domains_accepted() {
        let rs = vec![
            derivation(0x01, "sending", 0xA1),
            derivation(0x02, "receiving", 0xA2),
            derivation(0x03, "skipped", 0xA3),
            derivation(0x04, "header", 0xA4),
        ];
        assert_eq!(validate_domain_separation(&rs), Ok(()));
    }

    /// **CKDS-10** — same chain different keys same domain accepted.
    #[test]
    fn ckds_10_same_chain_different_key_rejected() {
        let rs = vec![
            derivation(0x01, "sending", 0xA1),
            derivation(0x01, "sending", 0xA2),
        ];
        assert_eq!(
            validate_domain_separation(&rs),
            Err(DomainSepError::DuplicateChainId { idx: 1, chain_id: cid(0x01) })
        );
    }
}
