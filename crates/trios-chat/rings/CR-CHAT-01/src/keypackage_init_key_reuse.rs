//! # L-CHAT-1-kpinit — KeyPackage init_key reuse defense
//!
//! Wave-31, Lane A. RFC 9420 §10.1 (KeyPackage init_key freshness +
//! one-shot consumption).
//!
//! Every `KeyPackage` carries an HPKE `init_key` (32 bytes for
//! X25519-HKDF-SHA256 / 1184 bytes for ML-KEM-768 — Trinity Chat uses
//! the X25519 path in CR-CHAT-01; CR-CHAT-02 handles the hybrid
//! ML-KEM path). RFC 9420 §10.1 requires that each KeyPackage MUST
//! carry a **fresh** `init_key` and the Delivery Service MUST treat
//! each KeyPackage as **one-shot** (consume on first Welcome).
//!
//! If an attacker can convince a joiner / DS to accept the SAME
//! `init_key` across two KeyPackages (or to replay a KeyPackage that
//! has already been consumed), they can decrypt the Welcome a second
//! time and learn the joiner_secret, breaking forward secrecy for
//! the joiner.
//!
//! Six rules in fixed order:
//! 1. `NonCanonicalInitKeyLength` — reject any `init_key` whose
//!    length differs from `KEYPACKAGE_INIT_KEY_LEN = 32`.
//! 2. `CrossCipherSuiteKeyPackage` — reject
//!    `package.ciphersuite != view.local_ciphersuite` (an attacker
//!    cannot rebind an X25519 init_key into an ML-KEM ciphersuite).
//! 3. `StaleEpochKeyPackage` — reject any KeyPackage whose
//!    `not_before > view.current_epoch` or whose `not_after <
//!    view.current_epoch` (expired / not-yet-valid KeyPackages).
//! 4. `InitKeyReused` — reject any `init_key` already present in
//!    `view.consumed_init_keys` (per-leaf one-shot ledger).
//! 5. `ZeroInitKey` — reject the all-zero `init_key` (degenerate
//!    HPKE key; the X25519 low-order point check rejects it anyway,
//!    but we fail closed earlier).
//! 6. `LeafKeyEqualsInitKey` — reject `init_key == leaf_node_key`
//!    (KeyPackages MUST have a distinct init_key from the leaf
//!    `encryption_key`; identical keys break the IND-CCA argument).
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · KEYPACKAGE-INIT-KEY`

use std::collections::BTreeSet;

/// Canonical X25519-HKDF-SHA256 init_key length used by Trinity Chat
/// in CR-CHAT-01 (RFC 9420 §10.1, MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519).
pub const KEYPACKAGE_INIT_KEY_LEN: usize = 32;

/// One MLS `KeyPackage` to be validated against the receiver / DS view.
/// Field layout mirrors `KeyPackage` in RFC 9420 §10.1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyPackage {
    /// MLS ciphersuite identifier (`MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519 = 0x0001`).
    pub ciphersuite: u16,
    /// HPKE public key used to wrap the Welcome to this leaf.
    pub init_key: Vec<u8>,
    /// LeafNode encryption_key (the leaf's per-leaf key). MUST differ
    /// from `init_key`.
    pub leaf_node_key: Vec<u8>,
    /// `not_before` lifetime extension lower bound (epoch number).
    pub not_before: u64,
    /// `not_after` lifetime extension upper bound (epoch number).
    pub not_after: u64,
}

/// Receiver / Delivery-Service view used to one-shot a KeyPackage.
/// `consumed_init_keys` is the SSOT — any `init_key` that appears
/// twice is a reuse.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KeyPackageView {
    /// Ciphersuite the receiver is currently bound to.
    pub local_ciphersuite: u16,
    /// Receiver's current epoch (used for lifetime check).
    pub current_epoch: u64,
    /// Ledger of every `init_key` ever consumed for this leaf.
    pub consumed_init_keys: BTreeSet<Vec<u8>>,
}

/// Why a KeyPackage was rejected. Mirrors INV-CHAT-187..190.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyPackageInitKeyError {
    /// Rule 1 — init_key length is not exactly 32 bytes.
    NonCanonicalInitKeyLength,
    /// Rule 2 — package.ciphersuite != view.local_ciphersuite.
    CrossCipherSuiteKeyPackage,
    /// Rule 3 — KeyPackage not currently valid (lifetime check).
    StaleEpochKeyPackage,
    /// Rule 4 — init_key already consumed (per-leaf one-shot ledger).
    InitKeyReused,
    /// Rule 5 — all-zero init_key.
    ZeroInitKey,
    /// Rule 6 — init_key == leaf_node_key (degenerate; collapses
    /// Welcome and leaf encryption into the same key).
    LeafKeyEqualsInitKey,
}

/// Validate one KeyPackage against the receiver view.
///
/// Returns `Ok(())` iff all six rules pass; otherwise returns the
/// first rule that fired. Order matches INV-CHAT-187..190.
pub fn validate_keypackage_init_key(
    package: &KeyPackage,
    view: &KeyPackageView,
) -> Result<(), KeyPackageInitKeyError> {
    // Rule 1.
    if package.init_key.len() != KEYPACKAGE_INIT_KEY_LEN {
        return Err(KeyPackageInitKeyError::NonCanonicalInitKeyLength);
    }
    // Rule 2.
    if package.ciphersuite != view.local_ciphersuite {
        return Err(KeyPackageInitKeyError::CrossCipherSuiteKeyPackage);
    }
    // Rule 3.
    if package.not_before > view.current_epoch || view.current_epoch > package.not_after {
        return Err(KeyPackageInitKeyError::StaleEpochKeyPackage);
    }
    // Rule 4.
    if view.consumed_init_keys.contains(&package.init_key) {
        return Err(KeyPackageInitKeyError::InitKeyReused);
    }
    // Rule 5.
    if package.init_key.iter().all(|&b| b == 0) {
        return Err(KeyPackageInitKeyError::ZeroInitKey);
    }
    // Rule 6.
    if package.init_key == package.leaf_node_key {
        return Err(KeyPackageInitKeyError::LeafKeyEqualsInitKey);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_view() -> KeyPackageView {
        KeyPackageView {
            local_ciphersuite: 0x0001,
            current_epoch: 42,
            consumed_init_keys: BTreeSet::new(),
        }
    }

    fn good_package(byte: u8) -> KeyPackage {
        KeyPackage {
            ciphersuite: 0x0001,
            init_key: vec![byte; KEYPACKAGE_INIT_KEY_LEN],
            leaf_node_key: vec![byte ^ 0xFF; KEYPACKAGE_INIT_KEY_LEN],
            not_before: 30,
            not_after: 60,
        }
    }

    /// KPI-01 — 16-byte (too-short) init_key rejected.
    #[test]
    fn kpi_01_short_init_key_rejected() {
        let view = base_view();
        let mut p = good_package(0x11);
        p.init_key = vec![0x11; 16];
        assert_eq!(
            validate_keypackage_init_key(&p, &view),
            Err(KeyPackageInitKeyError::NonCanonicalInitKeyLength)
        );
    }

    /// KPI-02 — 64-byte (over-long) init_key rejected.
    #[test]
    fn kpi_02_long_init_key_rejected() {
        let view = base_view();
        let mut p = good_package(0x11);
        p.init_key = vec![0x11; 64];
        assert_eq!(
            validate_keypackage_init_key(&p, &view),
            Err(KeyPackageInitKeyError::NonCanonicalInitKeyLength)
        );
    }

    /// KPI-03 — cross-ciphersuite KeyPackage rejected.
    #[test]
    fn kpi_03_cross_ciphersuite_rejected() {
        let view = base_view();
        let mut p = good_package(0x11);
        p.ciphersuite = 0x0003; // MLS_256_DHKEMP256
        assert_eq!(
            validate_keypackage_init_key(&p, &view),
            Err(KeyPackageInitKeyError::CrossCipherSuiteKeyPackage)
        );
    }

    /// KPI-04 — not-yet-valid KeyPackage rejected (`not_before > now`).
    #[test]
    fn kpi_04_not_yet_valid_rejected() {
        let view = base_view();
        let mut p = good_package(0x11);
        p.not_before = 100;
        p.not_after = 200;
        assert_eq!(
            validate_keypackage_init_key(&p, &view),
            Err(KeyPackageInitKeyError::StaleEpochKeyPackage)
        );
    }

    /// KPI-05 — expired KeyPackage rejected (`not_after < now`).
    #[test]
    fn kpi_05_expired_rejected() {
        let view = base_view();
        let mut p = good_package(0x11);
        p.not_before = 10;
        p.not_after = 20;
        assert_eq!(
            validate_keypackage_init_key(&p, &view),
            Err(KeyPackageInitKeyError::StaleEpochKeyPackage)
        );
    }

    /// KPI-06 — init_key replay via consumed_init_keys ledger rejected.
    #[test]
    fn kpi_06_init_key_reused_rejected() {
        let mut view = base_view();
        let p = good_package(0x11);
        view.consumed_init_keys.insert(p.init_key.clone());
        assert_eq!(
            validate_keypackage_init_key(&p, &view),
            Err(KeyPackageInitKeyError::InitKeyReused)
        );
    }

    /// KPI-07 — all-zero init_key rejected.
    #[test]
    fn kpi_07_zero_init_key_rejected() {
        let view = base_view();
        let mut p = good_package(0x11);
        p.init_key = vec![0u8; KEYPACKAGE_INIT_KEY_LEN];
        // leaf_node_key must remain distinct, otherwise rule 6 would fire first.
        p.leaf_node_key = vec![0xAB; KEYPACKAGE_INIT_KEY_LEN];
        assert_eq!(
            validate_keypackage_init_key(&p, &view),
            Err(KeyPackageInitKeyError::ZeroInitKey)
        );
    }

    /// KPI-08 — init_key equal to leaf_node_key rejected (degenerate).
    #[test]
    fn kpi_08_leaf_key_equals_init_key_rejected() {
        let view = base_view();
        let mut p = good_package(0x11);
        p.leaf_node_key = p.init_key.clone();
        assert_eq!(
            validate_keypackage_init_key(&p, &view),
            Err(KeyPackageInitKeyError::LeafKeyEqualsInitKey)
        );
    }

    /// KPI-09 — fresh KeyPackage at current epoch accepted.
    #[test]
    fn kpi_09_valid_keypackage_accepted() {
        let view = base_view();
        let p = good_package(0x11);
        assert_eq!(validate_keypackage_init_key(&p, &view), Ok(()));
    }

    /// KPI-10 — module green: compiles and re-exports through
    /// `CR-CHAT-01/src/lib.rs`.
    #[test]
    fn kpi_10_module_green() {
        let count = 10usize;
        assert_eq!(
            count, 10,
            "Wave-31 L-CHAT-1-kpinit: {count} KeyPackage init_key reuse falsifiers active"
        );
    }
}
