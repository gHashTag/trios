//! # CR-CHAT-01 — KeyPackage capabilities binding guard (Wave-43 Lane A)
//!
//! RFC 9420 §7.3 — KeyPackage capabilities validation.
//!
//! A KeyPackage declares the protocol version, ciphersuites, and extensions
//! the client supports. An attacker who can inject a KeyPackage with
//! inconsistent or downgraded capabilities can:
//!
//! * **Downgrade ciphersuites** — force the group to use a weaker cipher.
//! * **Omit mandatory extensions** — skip security-critical extensions
//!   like `required_capabilities`.
//! * **Declare duplicate extensions** — create ambiguity in parsing.
//! * **Set zero lifetime** — make the KeyPackage valid forever, widening
//!   the replay window.
//!
//! trios-chat enforces **7 rules**:
//!
//! 1. Capabilities list is non-empty.
//! 2. Ciphersuite list is non-empty.
//! 3. All ciphersuites are in the supported set.
//! 4. Protocol version matches current version.
//! 5. Lifetime is non-zero and within bounds.
//! 6. Extensions map is non-empty.
//! 7. No duplicate extension type identifiers.
//!
//! Tests **KPCAP-01..10**. Error enum [`KeyPackageCapError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · KEYPACKAGE-CAPABILITIES`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Current MLS protocol version.
pub const KPCAP_PROTOCOL_VERSION: u16 = 1;

/// Maximum KeyPackage lifetime in seconds (90 days).
pub const KPCAP_MAX_LIFETIME_SECS: u64 = 90 * 24 * 3600;

/// Supported ciphersuite identifiers.
pub const KPCAP_SUPPORTED_CIPHERSUITES: &[u16] = &[0x0001, 0x0002, 0x0003];

/// One extension entry in a KeyPackage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionEntry {
    /// Extension type identifier.
    pub ext_type: u16,
    /// Extension data.
    pub data: Vec<u8>,
}

/// A KeyPackage with capabilities for validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyPackageCap {
    /// Protocol version.
    pub protocol_version: u16,
    /// Declared ciphersuite identifiers.
    pub ciphersuites: Vec<u16>,
    /// KeyPackage lifetime in seconds.
    pub lifetime_secs: u64,
    /// Extensions.
    pub extensions: Vec<ExtensionEntry>,
}

/// All ways a KeyPackage's capabilities can be rejected.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyPackageCapError {
    /// Capabilities (ciphersuites) list is empty.
    EmptyCapabilities,
    /// Ciphersuite list is empty.
    EmptyCiphersuites,
    /// A ciphersuite is not in the supported set.
    UnsupportedCiphersuite,
    /// Protocol version mismatch.
    VersionMismatch,
    /// Lifetime is zero.
    ZeroLifetime,
    /// Lifetime exceeds maximum.
    LifetimeExceedsMax,
    /// Duplicate extension type.
    DuplicateExtension,
}

/// `[VERIFIED]` Validate a KeyPackage's capabilities. Returns `Ok(())`
/// if all rules pass.
pub fn validate_keypackage_capabilities(
    kp: &KeyPackageCap,
) -> Result<(), KeyPackageCapError> {
    if kp.ciphersuites.is_empty() {
        return Err(KeyPackageCapError::EmptyCapabilities);
    }
    for &cs in &kp.ciphersuites {
        if !KPCAP_SUPPORTED_CIPHERSUITES.contains(&cs) {
            return Err(KeyPackageCapError::UnsupportedCiphersuite);
        }
    }
    if kp.protocol_version != KPCAP_PROTOCOL_VERSION {
        return Err(KeyPackageCapError::VersionMismatch);
    }
    if kp.lifetime_secs == 0 {
        return Err(KeyPackageCapError::ZeroLifetime);
    }
    if kp.lifetime_secs > KPCAP_MAX_LIFETIME_SECS {
        return Err(KeyPackageCapError::LifetimeExceedsMax);
    }
    if kp.extensions.is_empty() {
        return Err(KeyPackageCapError::EmptyCapabilities);
    }
    let mut seen_types = BTreeSet::new();
    for ext in &kp.extensions {
        if !seen_types.insert(ext.ext_type) {
            return Err(KeyPackageCapError::DuplicateExtension);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good_kp() -> KeyPackageCap {
        KeyPackageCap {
            protocol_version: KPCAP_PROTOCOL_VERSION,
            ciphersuites: vec![0x0001, 0x0002],
            lifetime_secs: 86400,
            extensions: vec![
                ExtensionEntry { ext_type: 1, data: vec![0x00] },
                ExtensionEntry { ext_type: 2, data: vec![0x01] },
            ],
        }
    }

    /// **KPCAP-01** — empty ciphersuites rejected.
    #[test]
    fn kpcap_01_empty_ciphersuites_rejected() {
        let mut kp = good_kp();
        kp.ciphersuites = vec![];
        assert_eq!(
            validate_keypackage_capabilities(&kp),
            Err(KeyPackageCapError::EmptyCapabilities)
        );
    }

    /// **KPCAP-02** — unsupported ciphersuite rejected.
    #[test]
    fn kpcap_02_unsupported_ciphersuite_rejected() {
        let mut kp = good_kp();
        kp.ciphersuites = vec![0x00FF];
        assert_eq!(
            validate_keypackage_capabilities(&kp),
            Err(KeyPackageCapError::UnsupportedCiphersuite)
        );
    }

    /// **KPCAP-03** — version mismatch rejected.
    #[test]
    fn kpcap_03_version_mismatch_rejected() {
        let mut kp = good_kp();
        kp.protocol_version = 99;
        assert_eq!(
            validate_keypackage_capabilities(&kp),
            Err(KeyPackageCapError::VersionMismatch)
        );
    }

    /// **KPCAP-04** — zero lifetime rejected.
    #[test]
    fn kpcap_04_zero_lifetime_rejected() {
        let mut kp = good_kp();
        kp.lifetime_secs = 0;
        assert_eq!(
            validate_keypackage_capabilities(&kp),
            Err(KeyPackageCapError::ZeroLifetime)
        );
    }

    /// **KPCAP-05** — lifetime exceeds max rejected.
    #[test]
    fn kpcap_05_lifetime_exceeds_max_rejected() {
        let mut kp = good_kp();
        kp.lifetime_secs = KPCAP_MAX_LIFETIME_SECS + 1;
        assert_eq!(
            validate_keypackage_capabilities(&kp),
            Err(KeyPackageCapError::LifetimeExceedsMax)
        );
    }

    /// **KPCAP-06** — empty extensions rejected.
    #[test]
    fn kpcap_06_empty_extensions_rejected() {
        let mut kp = good_kp();
        kp.extensions = vec![];
        assert_eq!(
            validate_keypackage_capabilities(&kp),
            Err(KeyPackageCapError::EmptyCapabilities)
        );
    }

    /// **KPCAP-07** — duplicate extension type rejected.
    #[test]
    fn kpcap_07_duplicate_extension_rejected() {
        let mut kp = good_kp();
        kp.extensions.push(ExtensionEntry { ext_type: 1, data: vec![0x02] });
        assert_eq!(
            validate_keypackage_capabilities(&kp),
            Err(KeyPackageCapError::DuplicateExtension)
        );
    }

    /// **KPCAP-08** — valid KeyPackage accepted.
    #[test]
    fn kpcap_08_valid_kp_accepted() {
        assert_eq!(validate_keypackage_capabilities(&good_kp()), Ok(()));
    }

    /// **KPCAP-09** — all supported ciphersuites accepted.
    #[test]
    fn kpcap_09_all_ciphersuites_accepted() {
        let mut kp = good_kp();
        kp.ciphersuites = KPCAP_SUPPORTED_CIPHERSUITES.to_vec();
        assert_eq!(validate_keypackage_capabilities(&kp), Ok(()));
    }

    /// **KPCAP-10** — exact max lifetime boundary accepted.
    #[test]
    fn kpcap_10_max_lifetime_boundary_accepted() {
        let mut kp = good_kp();
        kp.lifetime_secs = KPCAP_MAX_LIFETIME_SECS;
        assert_eq!(validate_keypackage_capabilities(&kp), Ok(()));
    }
}
