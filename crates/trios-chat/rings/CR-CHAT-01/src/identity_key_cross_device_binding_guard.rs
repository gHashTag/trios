//! # CR-CHAT-01 — Identity key cross-device binding guard (Wave-107 Lane A)
//!
//! IDENTITY — identity-to-device binding must be consistent.
//!
//! A single identity may be active on multiple devices. Each device
//! has its own prekey bundle, but the binding between the identity key
//! and device-specific keys must be consistent:
//!
//! * **Device impersonation** — if two different identity keys claim
//!   the same device, an attacker can inject their key as a "new
//!   device" for the victim's identity.
//! * **Binding conflict** — if the same device is bound to different
//!   identity keys in different sessions, verification fails
//!   unpredictably.
//! * **Session fragmentation** — inconsistent bindings cause messages
//!   to be routed to the wrong device or dropped entirely.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. No device bound to multiple identities.
//! 2. Device key must not be zero.
//! 3. Identity key must not be zero.
//! 4. No duplicate (identity, device) pairs.
//! 5. Devices per identity <= `ICDB_MAX_DEVICES`.
//! 6. Total bindings <= `ICDB_MAX_BINDINGS`.
//!
//! Tests **ICDB-01..10**. Error enum [`CrossDeviceError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * CROSS-DEVICE-BINDING`

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

/// Maximum devices per identity.
pub const ICDB_MAX_DEVICES: usize = 8;

/// Maximum total bindings.
pub const ICDB_MAX_BINDINGS: usize = 256;

/// Key length.
pub const ICDB_KEY_LEN: usize = 32;

/// A cross-device binding record.
#[derive(Debug, Clone)]
pub struct DeviceBinding {
    /// Identity key.
    pub identity_key: [u8; ICDB_KEY_LEN],
    /// Device-specific key.
    pub device_key: [u8; ICDB_KEY_LEN],
}

/// All ways cross-device binding validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CrossDeviceError {
    /// Device bound to multiple identities.
    ConflictingDevice { idx: usize },
    /// Zero device key.
    ZeroDeviceKey(usize),
    /// Zero identity key.
    ZeroIdentityKey(usize),
    /// Duplicate binding.
    DuplicateBinding(usize),
    /// Too many devices for one identity.
    TooManyDevices { idx: usize, count: usize, max: usize },
    /// Too many bindings.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate identity key cross-device binding consistency.
pub fn validate_cross_device_bindings(
    bindings: &[DeviceBinding],
) -> Result<(), CrossDeviceError> {
    if bindings.len() > ICDB_MAX_BINDINGS {
        return Err(CrossDeviceError::TooMany {
            got: bindings.len(),
            max: ICDB_MAX_BINDINGS,
        });
    }
    let mut device_to_identity: BTreeMap<[u8; ICDB_KEY_LEN], [u8; ICDB_KEY_LEN]> = BTreeMap::new();
    let mut identity_count: BTreeMap<[u8; ICDB_KEY_LEN], usize> = BTreeMap::new();
    let mut seen: BTreeSet<([u8; ICDB_KEY_LEN], [u8; ICDB_KEY_LEN])> = BTreeSet::new();
    for (i, b) in bindings.iter().enumerate() {
        if b.identity_key == [0u8; ICDB_KEY_LEN] {
            return Err(CrossDeviceError::ZeroIdentityKey(i));
        }
        if b.device_key == [0u8; ICDB_KEY_LEN] {
            return Err(CrossDeviceError::ZeroDeviceKey(i));
        }
        if let Some(existing_id) = device_to_identity.get(&b.device_key) {
            if *existing_id != b.identity_key {
                return Err(CrossDeviceError::ConflictingDevice { idx: i });
            }
        }
        if !seen.insert((b.identity_key, b.device_key)) {
            return Err(CrossDeviceError::DuplicateBinding(i));
        }
        device_to_identity.insert(b.device_key, b.identity_key);
        let count = identity_count.entry(b.identity_key).or_insert(0);
        *count += 1;
        if *count > ICDB_MAX_DEVICES {
            return Err(CrossDeviceError::TooManyDevices {
                idx: i,
                count: *count,
                max: ICDB_MAX_DEVICES,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> [u8; ICDB_KEY_LEN] {
        [byte; ICDB_KEY_LEN]
    }

    fn binding(identity: u8, device: u8) -> DeviceBinding {
        DeviceBinding { identity_key: key(identity), device_key: key(device) }
    }

    fn valid_bindings() -> Vec<DeviceBinding> {
        vec![
            binding(0xA0, 0x01),
            binding(0xA0, 0x02),
            binding(0xB0, 0x03),
        ]
    }

    /// **ICDB-01** — conflicting device rejected.
    #[test]
    fn icdb_01_conflicting_device_rejected() {
        let bs = vec![binding(0xA0, 0x01), binding(0xB0, 0x01)];
        assert_eq!(
            validate_cross_device_bindings(&bs),
            Err(CrossDeviceError::ConflictingDevice { idx: 1 })
        );
    }

    /// **ICDB-02** — zero device key rejected.
    #[test]
    fn icdb_02_zero_device_rejected() {
        let b = DeviceBinding { identity_key: key(0xA0), device_key: [0u8; ICDB_KEY_LEN] };
        assert_eq!(
            validate_cross_device_bindings(&[b]),
            Err(CrossDeviceError::ZeroDeviceKey(0))
        );
    }

    /// **ICDB-03** — zero identity key rejected.
    #[test]
    fn icdb_03_zero_identity_rejected() {
        let b = DeviceBinding { identity_key: [0u8; ICDB_KEY_LEN], device_key: key(0x01) };
        assert_eq!(
            validate_cross_device_bindings(&[b]),
            Err(CrossDeviceError::ZeroIdentityKey(0))
        );
    }

    /// **ICDB-04** — duplicate binding rejected.
    #[test]
    fn icdb_04_duplicate_rejected() {
        let bs = vec![binding(0xA0, 0x01), binding(0xA0, 0x01)];
        assert_eq!(
            validate_cross_device_bindings(&bs),
            Err(CrossDeviceError::DuplicateBinding(1))
        );
    }

    /// **ICDB-05** — too many devices rejected.
    #[test]
    fn icdb_05_too_many_devices_rejected() {
        let bs: Vec<DeviceBinding> = (0..=ICDB_MAX_DEVICES)
            .map(|i| DeviceBinding {
                identity_key: key(0xA0),
                device_key: key((i as u8).wrapping_add(1)),
            })
            .collect();
        assert!(matches!(
            validate_cross_device_bindings(&bs),
            Err(CrossDeviceError::TooManyDevices { .. })
        ));
    }

    /// **ICDB-06** — too many bindings rejected.
    #[test]
    fn icdb_06_too_many_rejected() {
        let bs: Vec<DeviceBinding> = (0..=ICDB_MAX_BINDINGS)
            .map(|i| {
                let b = (i as u8).wrapping_add(1);
                DeviceBinding { identity_key: key(b), device_key: key(b) }
            })
            .collect();
        assert_eq!(
            validate_cross_device_bindings(&bs),
            Err(CrossDeviceError::TooMany {
                got: ICDB_MAX_BINDINGS + 1,
                max: ICDB_MAX_BINDINGS,
            })
        );
    }

    /// **ICDB-07** — valid accepted.
    #[test]
    fn icdb_07_valid_accepted() {
        assert_eq!(validate_cross_device_bindings(&valid_bindings()), Ok(()));
    }

    /// **ICDB-08** — empty accepted.
    #[test]
    fn icdb_08_empty_accepted() {
        assert_eq!(validate_cross_device_bindings(&[]), Ok(()));
    }

    /// **ICDB-09** — same device same identity accepted (no duplicate since set prevents it).
    /// Testing max devices boundary accepted.
    #[test]
    fn icdb_09_max_devices_accepted() {
        let bs: Vec<DeviceBinding> = (0..ICDB_MAX_DEVICES)
            .map(|i| DeviceBinding {
                identity_key: key(0xA0),
                device_key: key((i as u8).wrapping_add(1)),
            })
            .collect();
        assert_eq!(validate_cross_device_bindings(&bs), Ok(()));
    }

    /// **ICDB-10** — multiple identities multiple devices accepted.
    #[test]
    fn icdb_10_multi_identity_accepted() {
        let bs = vec![
            binding(0xA0, 0x01),
            binding(0xA0, 0x02),
            binding(0xB0, 0x03),
            binding(0xB0, 0x04),
            binding(0xC0, 0x05),
        ];
        assert_eq!(validate_cross_device_bindings(&bs), Ok(()));
    }
}
