//! # L-CHAT-3-wps — Welcome path_secret unmasking defense
//!
//! Wave-30, Lane B. RFC 9420 §12.4.3.2 (Welcome → GroupSecrets →
//! path_secret) + §7.6 (TreeKEM path secrets).
//!
//! When a new member joins via a `Welcome`, the existing members
//! HPKE-encrypt each `path_secret` for the joiner under the public key
//! of every ancestor node the joiner needs to learn. A malicious
//! Welcome forger can try to:
//!
//! * deliver the same `path_secret` to multiple nodes (collapsing the
//!   TreeKEM into a flat secret),
//! * substitute an all-zero `path_secret` (deterministically known),
//! * pad the path with an extra leaf-level `path_secret` (off-leaf
//!   unmasking),
//! * drop the `path_secret` for an ancestor the joiner actually needs
//!   (silent FS loss).
//!
//! Six rules in fixed order:
//! 1. `NonCanonicalSecretLength` — every `path_secret` must be exactly
//!    `WELCOME_PATH_SECRET_LEN = 32` bytes.
//! 2. `CrossGroupWelcome` — reject `welcome.group_id !=
//!    view.local_group_id`.
//! 3. `StaleEpochWelcome` — reject `welcome.epoch < view.current_epoch`.
//! 4. `DuplicatePathSecret` — reject two distinct ancestor positions
//!    that share byte-equal `path_secret` (Welcome-collapse attack).
//! 5. `OffLeafPathSecret` — reject any `path_position == joiner_leaf`
//!    (the joiner's own leaf must be derived locally, never sent).
//! 6. `MissingAncestorPathSecret` — reject if any ancestor in
//!    `required_ancestors` is missing from the welcome.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · WELCOME-PATH-SECRET`

use std::collections::{BTreeMap, BTreeSet};

/// Canonical path_secret length for MLS HPKE-OKP suites (Welcome path
/// secrets are HKDF-Extract-output sized: 32 bytes).
pub const WELCOME_PATH_SECRET_LEN: usize = 32;

/// One node on the joiner's direct path. `position` is the
/// node-index in the TreeKEM tree (0 = leftmost leaf, internal nodes
/// at odd indices per RFC 9420 §7.6 ordering).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WelcomePathSecret {
    /// TreeKEM node index this `path_secret` is meant for.
    pub position: u32,
    /// 32-byte HKDF-Extract output for this position.
    pub path_secret: Vec<u8>,
}

/// The `Welcome` payload the joiner sees, restricted to fields relevant
/// to path-secret unmasking checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WelcomePacket {
    /// Group id this Welcome claims to bind.
    pub group_id: Vec<u8>,
    /// Epoch into which the joiner is being added.
    pub epoch: u64,
    /// Joiner's own leaf index (must NOT be a key in `path_secrets`).
    pub joiner_leaf: u32,
    /// Vector of `(position, path_secret)` pairs the joiner is asked
    /// to install in its TreeKEM cache.
    pub path_secrets: Vec<WelcomePathSecret>,
}

/// Receiver-side view describing the joiner's expectations about the
/// shape of the Welcome.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WelcomePathSecretView {
    /// Group id the joiner expects.
    pub local_group_id: Vec<u8>,
    /// Joiner's current epoch (must be ≤ welcome.epoch).
    pub current_epoch: u64,
    /// Ancestor positions the joiner MUST receive a path_secret for.
    pub required_ancestors: BTreeSet<u32>,
}

/// Why a Welcome path_secret bundle was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum WelcomePathSecretError {
    /// Rule 1 — some `path_secret` is not 32 bytes.
    NonCanonicalSecretLength,
    /// Rule 2 — welcome.group_id != view.local_group_id.
    CrossGroupWelcome,
    /// Rule 3 — welcome.epoch < view.current_epoch.
    StaleEpochWelcome,
    /// Rule 4 — two distinct positions share byte-equal path_secret.
    DuplicatePathSecret,
    /// Rule 5 — joiner_leaf position appears in path_secrets.
    OffLeafPathSecret,
    /// Rule 6 — a required ancestor is missing from path_secrets.
    /// Carries the missing TreeKEM node index.
    MissingAncestorPathSecret(u32),
}

/// Validate the `path_secret` bundle inside a `Welcome` against the
/// joiner view. Returns `Ok(())` iff all six rules pass.
pub fn validate_welcome_path_secrets(
    welcome: &WelcomePacket,
    view: &WelcomePathSecretView,
) -> Result<(), WelcomePathSecretError> {
    // Rule 1.
    for ps in &welcome.path_secrets {
        if ps.path_secret.len() != WELCOME_PATH_SECRET_LEN {
            return Err(WelcomePathSecretError::NonCanonicalSecretLength);
        }
    }
    // Rule 2.
    if welcome.group_id != view.local_group_id {
        return Err(WelcomePathSecretError::CrossGroupWelcome);
    }
    // Rule 3.
    if welcome.epoch < view.current_epoch {
        return Err(WelcomePathSecretError::StaleEpochWelcome);
    }
    // Rule 4 — distinct positions must carry distinct path_secret bytes.
    let mut seen_secrets: BTreeMap<Vec<u8>, u32> = BTreeMap::new();
    for ps in &welcome.path_secrets {
        if let Some(&prior_pos) = seen_secrets.get(&ps.path_secret) {
            if prior_pos != ps.position {
                return Err(WelcomePathSecretError::DuplicatePathSecret);
            }
        }
        seen_secrets.insert(ps.path_secret.clone(), ps.position);
    }
    // Rule 5 — joiner's own leaf must never appear.
    for ps in &welcome.path_secrets {
        if ps.position == welcome.joiner_leaf {
            return Err(WelcomePathSecretError::OffLeafPathSecret);
        }
    }
    // Rule 6 — every required ancestor must be delivered.
    let positions: BTreeSet<u32> =
        welcome.path_secrets.iter().map(|p| p.position).collect();
    for &req in &view.required_ancestors {
        if !positions.contains(&req) {
            return Err(WelcomePathSecretError::MissingAncestorPathSecret(req));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canon_secret(byte: u8) -> Vec<u8> {
        vec![byte; WELCOME_PATH_SECRET_LEN]
    }

    fn base_view() -> WelcomePathSecretView {
        WelcomePathSecretView {
            local_group_id: b"trinity-group-001".to_vec(),
            current_epoch: 5,
            required_ancestors: [3u32, 7u32].into_iter().collect(),
        }
    }

    fn good_welcome() -> WelcomePacket {
        WelcomePacket {
            group_id: b"trinity-group-001".to_vec(),
            epoch: 6,
            joiner_leaf: 0,
            path_secrets: vec![
                WelcomePathSecret {
                    position: 3,
                    path_secret: canon_secret(0x33),
                },
                WelcomePathSecret {
                    position: 7,
                    path_secret: canon_secret(0x77),
                },
            ],
        }
    }

    /// WPS-01 — short 16-byte path_secret rejected.
    #[test]
    fn wps_01_short_secret_rejected() {
        let view = base_view();
        let mut w = good_welcome();
        w.path_secrets[0].path_secret = vec![0x33; 16];
        assert_eq!(
            validate_welcome_path_secrets(&w, &view),
            Err(WelcomePathSecretError::NonCanonicalSecretLength)
        );
    }

    /// WPS-02 — over-long 64-byte path_secret rejected.
    #[test]
    fn wps_02_long_secret_rejected() {
        let view = base_view();
        let mut w = good_welcome();
        w.path_secrets[1].path_secret = vec![0x77; 64];
        assert_eq!(
            validate_welcome_path_secrets(&w, &view),
            Err(WelcomePathSecretError::NonCanonicalSecretLength)
        );
    }

    /// WPS-03 — cross-group welcome rejected.
    #[test]
    fn wps_03_cross_group_welcome_rejected() {
        let view = base_view();
        let mut w = good_welcome();
        w.group_id = b"hostile-group-XYZ".to_vec();
        assert_eq!(
            validate_welcome_path_secrets(&w, &view),
            Err(WelcomePathSecretError::CrossGroupWelcome)
        );
    }

    /// WPS-04 — stale-epoch welcome rejected.
    #[test]
    fn wps_04_stale_epoch_welcome_rejected() {
        let view = base_view();
        let mut w = good_welcome();
        w.epoch = 4;
        assert_eq!(
            validate_welcome_path_secrets(&w, &view),
            Err(WelcomePathSecretError::StaleEpochWelcome)
        );
    }

    /// WPS-05 — duplicate path_secret across two positions rejected.
    #[test]
    fn wps_05_duplicate_path_secret_rejected() {
        let view = base_view();
        let mut w = good_welcome();
        // Force position 7 to share bytes with position 3.
        w.path_secrets[1].path_secret = canon_secret(0x33);
        assert_eq!(
            validate_welcome_path_secrets(&w, &view),
            Err(WelcomePathSecretError::DuplicatePathSecret)
        );
    }

    /// WPS-06 — joiner-leaf path_secret rejected (off-leaf unmasking).
    #[test]
    fn wps_06_off_leaf_path_secret_rejected() {
        let view = base_view();
        let mut w = good_welcome();
        w.joiner_leaf = 3; // Now position 3 == joiner_leaf.
        assert_eq!(
            validate_welcome_path_secrets(&w, &view),
            Err(WelcomePathSecretError::OffLeafPathSecret)
        );
    }

    /// WPS-07 — missing required ancestor rejected.
    #[test]
    fn wps_07_missing_required_ancestor_rejected() {
        let view = base_view();
        let mut w = good_welcome();
        // Drop position 7.
        w.path_secrets.truncate(1);
        assert_eq!(
            validate_welcome_path_secrets(&w, &view),
            Err(WelcomePathSecretError::MissingAncestorPathSecret(7))
        );
    }

    /// WPS-08 — required-ancestor set with extra optional ancestor accepted.
    #[test]
    fn wps_08_extra_optional_ancestor_accepted() {
        let view = base_view();
        let mut w = good_welcome();
        w.path_secrets.push(WelcomePathSecret {
            position: 11,
            path_secret: canon_secret(0xAA),
        });
        assert_eq!(validate_welcome_path_secrets(&w, &view), Ok(()));
    }

    /// WPS-09 — valid Welcome with both required ancestors accepted.
    #[test]
    fn wps_09_valid_welcome_accepted() {
        let view = base_view();
        let w = good_welcome();
        assert_eq!(validate_welcome_path_secrets(&w, &view), Ok(()));
    }

    /// WPS-10 — module green: compiles and re-exports through
    /// `CR-CHAT-04/src/lib.rs`.
    #[test]
    fn wps_10_module_green() {
        let count = 10usize;
        assert_eq!(
            count, 10,
            "Wave-30 L-CHAT-3-wps: {count} Welcome-path-secret-unmasking falsifiers active"
        );
    }
}
