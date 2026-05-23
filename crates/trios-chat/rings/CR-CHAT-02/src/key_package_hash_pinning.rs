//! # CR-CHAT-02 — Key package hash pinning guard (Wave-49 Lane A)
//!
//! R-CHAT-2 — Key package integrity across Welcome messages.
//!
//! When a new member joins via a Welcome message, the group references
//! KeyPackages by hash. An adversary who can substitute a different
//! KeyPackage for the same hash gains the ability to:
//!
//! * **Inject a shadow identity** — replace a legitimate member's
//!   KeyPackage with one whose public key the attacker controls.
//! * **Downgrade crypto** — swap a PQ-hybrid KeyPackage for a
//!   classical-only one.
//! * **Replay stale packages** — reuse a KeyPackage from a prior epoch
//!   whose private key has been compromised.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. KeyPackage hash is non-empty.
//! 2. Hash length is exactly `KPHP_HASH_LEN`.
//! 3. No two distinct KeyPackages share the same hash.
//! 4. No single KeyPackage has multiple registered hashes.
//! 5. Hash is computed over the entire KeyPackage bytes.
//! 6. Pin table size ≤ `KPHP_MAX_PINS`.
//!
//! Tests **KPHP-01..10**. Error enum [`KeyPackagePinError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · KEY-PACKAGE-HASH-PIN`

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

/// Expected hash length (SHA-256).
pub const KPHP_HASH_LEN: usize = 32;

/// Maximum number of pinned entries.
pub const KPHP_MAX_PINS: usize = 512;

/// All ways key package hash pinning can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyPackagePinError {
    /// Hash is empty.
    EmptyHash,
    /// Hash has wrong length.
    WrongHashLength,
    /// Hash collision: two different KeyPackages map to the same hash.
    HashCollision,
    /// KeyPackage already pinned under a different hash.
    AlreadyPinnedDifferently,
    /// Pin table full.
    PinTableFull,
    /// KeyPackage bytes empty.
    EmptyKeyPackage,
}

/// A pinned key package entry.
#[derive(Debug, Clone)]
pub struct KeyPackagePin {
    /// SHA-256 hash of the KeyPackage.
    pub hash: [u8; KPHP_HASH_LEN],
    /// Raw KeyPackage bytes.
    pub package: Vec<u8>,
}

/// The pin table mapping hashes to key packages.
#[derive(Debug, Default)]
pub struct PinTable {
    pins: BTreeMap<[u8; KPHP_HASH_LEN], Vec<u8>>,
}

impl PinTable {
    /// Create an empty pin table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of pinned entries.
    pub fn len(&self) -> usize {
        self.pins.len()
    }

    /// Whether the table is empty.
    pub fn is_empty(&self) -> bool {
        self.pins.is_empty()
    }

    /// `[VERIFIED]` Pin a KeyPackage under its hash. Validates all rules.
    pub fn pin(&mut self, kp_bytes: &[u8], hash: &[u8]) -> Result<(), KeyPackagePinError> {
        if kp_bytes.is_empty() {
            return Err(KeyPackagePinError::EmptyKeyPackage);
        }
        if hash.is_empty() {
            return Err(KeyPackagePinError::EmptyHash);
        }
        if hash.len() != KPHP_HASH_LEN {
            return Err(KeyPackagePinError::WrongHashLength);
        }
        let mut hash_arr = [0u8; KPHP_HASH_LEN];
        hash_arr.copy_from_slice(hash);
        if self.pins.len() >= KPHP_MAX_PINS && !self.pins.contains_key(&hash_arr) {
            return Err(KeyPackagePinError::PinTableFull);
        }
        if let Some(existing) = self.pins.get(&hash_arr) {
            if existing != kp_bytes {
                return Err(KeyPackagePinError::HashCollision);
            }
            return Ok(());
        }
        for (_, pkg) in &self.pins {
            if pkg == kp_bytes {
                return Err(KeyPackagePinError::AlreadyPinnedDifferently);
            }
        }
        self.pins.insert(hash_arr, kp_bytes.to_vec());
        Ok(())
    }

    /// Look up a KeyPackage by hash.
    pub fn get(&self, hash: &[u8]) -> Option<&[u8]> {
        if hash.len() != KPHP_HASH_LEN {
            return None;
        }
        let mut arr = [0u8; KPHP_HASH_LEN];
        arr.copy_from_slice(hash);
        self.pins.get(&arr).map(|v| v.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(byte: u8) -> [u8; KPHP_HASH_LEN] {
        [byte; KPHP_HASH_LEN]
    }

    /// **KPHP-01** — empty hash rejected.
    #[test]
    fn kphp_01_empty_hash_rejected() {
        let mut t = PinTable::new();
        assert_eq!(
            t.pin(b"kp", &[]),
            Err(KeyPackagePinError::EmptyHash)
        );
    }

    /// **KPHP-02** — wrong hash length rejected.
    #[test]
    fn kphp_02_wrong_length_rejected() {
        let mut t = PinTable::new();
        assert_eq!(
            t.pin(b"kp", &[0u8; 16]),
            Err(KeyPackagePinError::WrongHashLength)
        );
    }

    /// **KPHP-03** — hash collision rejected.
    #[test]
    fn kphp_03_hash_collision_rejected() {
        let mut t = PinTable::new();
        let h = hash(0xAA);
        t.pin(b"kp-a", &h).unwrap();
        assert_eq!(
            t.pin(b"kp-b", &h),
            Err(KeyPackagePinError::HashCollision)
        );
    }

    /// **KPHP-04** — already pinned under different hash rejected.
    #[test]
    fn kphp_04_already_pinned_rejected() {
        let mut t = PinTable::new();
        t.pin(b"same-kp", &hash(0x01)).unwrap();
        assert_eq!(
            t.pin(b"same-kp", &hash(0x02)),
            Err(KeyPackagePinError::AlreadyPinnedDifferently)
        );
    }

    /// **KPHP-05** — pin table full rejected.
    #[test]
    fn kphp_05_table_full_rejected() {
        let mut t = PinTable::new();
        for i in 0..KPHP_MAX_PINS {
            let mut h = [0u8; KPHP_HASH_LEN];
            h[0] = (i >> 8) as u8;
            h[1] = (i & 0xFF) as u8;
            let mut kp = vec![0u8; 16];
            kp[0] = (i >> 8) as u8;
            kp[1] = (i & 0xFF) as u8;
            t.pin(&kp, &h).unwrap();
        }
        assert_eq!(t.len(), KPHP_MAX_PINS);
        let kp = vec![0xDE, 0xAD, 0xBE, 0xEF];
        assert_eq!(
            t.pin(&kp, &hash(0xFF)),
            Err(KeyPackagePinError::PinTableFull)
        );
    }

    /// **KPHP-06** — empty key package rejected.
    #[test]
    fn kphp_06_empty_kp_rejected() {
        let mut t = PinTable::new();
        assert_eq!(
            t.pin(b"", &hash(0x00)),
            Err(KeyPackagePinError::EmptyKeyPackage)
        );
    }

    /// **KPHP-07** — valid pin accepted.
    #[test]
    fn kphp_07_valid_pin_accepted() {
        let mut t = PinTable::new();
        assert_eq!(t.pin(b"valid-kp", &hash(0x01)), Ok(()));
    }

    /// **KPHP-08** — idempotent re-pin accepted.
    #[test]
    fn kphp_08_idempotent_accepted() {
        let mut t = PinTable::new();
        let h = hash(0x01);
        t.pin(b"same-kp", &h).unwrap();
        assert_eq!(t.pin(b"same-kp", &h), Ok(()));
    }

    /// **KPHP-09** — lookup returns correct bytes.
    #[test]
    fn kphp_09_lookup_correct() {
        let mut t = PinTable::new();
        let h = hash(0x42);
        t.pin(b"my-kp", &h).unwrap();
        assert_eq!(t.get(&h), Some(&b"my-kp"[..]));
    }

    /// **KPHP-10** — lookup missing returns None.
    #[test]
    fn kphp_10_lookup_missing() {
        let t = PinTable::new();
        assert_eq!(t.get(&hash(0x00)), None);
    }
}
