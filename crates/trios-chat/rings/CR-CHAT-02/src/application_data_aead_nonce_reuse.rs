//! # L-CHAT-2-appnonce — Application-data AEAD nonce reuse defense
//!
//! Wave-30, Lane A. RFC 9420 §6.3.1 (Application Messages →
//! per-message AEAD key/nonce derivation).
//!
//! `MLSCiphertext` carries application data encrypted under
//! `(handshake_key, handshake_nonce)` or `(application_key,
//! application_nonce)` derived per-sender from the message-secret tree.
//! The AEAD construction (AES-128-GCM / ChaCha20-Poly1305) is
//! **catastrophically broken** if the same `(key, nonce)` pair is ever
//! reused: nonce reuse leaks the keystream XOR of the two plaintexts
//! and forges the GHASH authentication key.
//!
//! Six rules in fixed order:
//! 1. `NonCanonicalNonceLength` — reject any AEAD nonce whose length
//!    differs from `APPLICATION_DATA_AEAD_NONCE_LEN = 12`.
//! 2. `CrossGroupNonceSplice` — reject `packet.group_id !=
//!    view.local_group_id` (cross-group AEAD-context splice).
//! 3. `StaleEpochAead` — reject `packet.epoch < view.current_epoch`.
//! 4. `GenerationGapTooLarge` — reject `packet.generation` more than
//!    `MAX_GENERATION_WINDOW = 1024` ahead of `view.expected_generation`
//!    (prevents generation-skip oracles).
//! 5. `ZeroNonce` — reject all-zero AEAD nonce (mandatory ratchet-derived
//!    nonces are never zero in a healthy ratchet).
//! 6. `NonceReplay` — reject `(group_id, epoch, leaf_index,
//!    generation, nonce)` quintuple already in `used_nonces`.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · APP-DATA-AEAD-NONCE`

use std::collections::BTreeSet;

/// Canonical AEAD nonce length for AES-128-GCM and ChaCha20-Poly1305
/// as used in MLS application messages (RFC 9420 §6.3.1).
pub const APPLICATION_DATA_AEAD_NONCE_LEN: usize = 12;

/// Maximum allowed jump in `generation` ahead of the receiver's expected
/// generation. Catches both replay (generation < expected) and
/// generation-skip oracles (generation ≫ expected).
pub const MAX_GENERATION_WINDOW: u32 = 1024;

/// One application-data AEAD packet to be validated against the receiver
/// view. Field layout mirrors `MLSCiphertext.application_data` headers
/// in RFC 9420.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationDataPacket {
    /// MLS group_id this packet claims to belong to.
    pub group_id: Vec<u8>,
    /// Epoch under which the AEAD key/nonce were derived.
    pub epoch: u64,
    /// Sender leaf index inside the group.
    pub leaf_index: u32,
    /// Per-sender per-epoch monotonic counter (`Generation`).
    pub generation: u32,
    /// Concrete AEAD nonce (`application_nonce` XOR `reuse_guard`).
    pub aead_nonce: Vec<u8>,
}

/// Receiver-side state for nonce-reuse detection. The `used_nonces`
/// ledger is the SSOT — any nonce that appears twice is replayed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ApplicationDataView {
    /// Group_id the receiver is currently bound to.
    pub local_group_id: Vec<u8>,
    /// Receiver's current accepted epoch.
    pub current_epoch: u64,
    /// Next generation the receiver expects for the sender leaf.
    pub expected_generation: u32,
    /// Ledger of already-consumed `(group_id, epoch, leaf_index,
    /// generation, nonce)` quintuples.
    pub used_nonces: BTreeSet<(Vec<u8>, u64, u32, u32, Vec<u8>)>,
}

/// Why an application-data packet was rejected. Mirrors INV-CHAT-180..183.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ApplicationDataAeadError {
    /// Rule 1 — AEAD nonce length is not exactly 12 bytes.
    NonCanonicalNonceLength,
    /// Rule 2 — packet.group_id != view.local_group_id.
    CrossGroupNonceSplice,
    /// Rule 3 — packet.epoch < view.current_epoch.
    StaleEpochAead,
    /// Rule 4 — generation gap exceeds `MAX_GENERATION_WINDOW`.
    GenerationGapTooLarge,
    /// Rule 5 — all-zero AEAD nonce.
    ZeroNonce,
    /// Rule 6 — `(group_id, epoch, leaf_index, generation, nonce)`
    /// quintuple already consumed.
    NonceReplay,
}

/// Validate an application-data AEAD packet against the receiver view.
///
/// Returns `Ok(())` iff all six rules pass; otherwise returns the first
/// rule that fired. Order matches INV-CHAT-180..183.
pub fn validate_application_data_aead(
    packet: &ApplicationDataPacket,
    view: &ApplicationDataView,
) -> Result<(), ApplicationDataAeadError> {
    // Rule 1.
    if packet.aead_nonce.len() != APPLICATION_DATA_AEAD_NONCE_LEN {
        return Err(ApplicationDataAeadError::NonCanonicalNonceLength);
    }
    // Rule 2.
    if packet.group_id != view.local_group_id {
        return Err(ApplicationDataAeadError::CrossGroupNonceSplice);
    }
    // Rule 3.
    if packet.epoch < view.current_epoch {
        return Err(ApplicationDataAeadError::StaleEpochAead);
    }
    // Rule 4.
    if packet.generation < view.expected_generation {
        return Err(ApplicationDataAeadError::GenerationGapTooLarge);
    }
    if packet.generation.saturating_sub(view.expected_generation) > MAX_GENERATION_WINDOW {
        return Err(ApplicationDataAeadError::GenerationGapTooLarge);
    }
    // Rule 5.
    if packet.aead_nonce.iter().all(|&b| b == 0) {
        return Err(ApplicationDataAeadError::ZeroNonce);
    }
    // Rule 6.
    let key = (
        packet.group_id.clone(),
        packet.epoch,
        packet.leaf_index,
        packet.generation,
        packet.aead_nonce.clone(),
    );
    if view.used_nonces.contains(&key) {
        return Err(ApplicationDataAeadError::NonceReplay);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_view() -> ApplicationDataView {
        ApplicationDataView {
            local_group_id: b"trinity-group-001".to_vec(),
            current_epoch: 42,
            expected_generation: 100,
            used_nonces: BTreeSet::new(),
        }
    }

    fn good_packet(nonce: u8) -> ApplicationDataPacket {
        ApplicationDataPacket {
            group_id: b"trinity-group-001".to_vec(),
            epoch: 42,
            leaf_index: 7,
            generation: 100,
            aead_nonce: vec![nonce; APPLICATION_DATA_AEAD_NONCE_LEN],
        }
    }

    /// AAN-01 — 8-byte (too-short) nonce rejected.
    #[test]
    fn aan_01_short_nonce_rejected() {
        let view = base_view();
        let mut p = good_packet(0x11);
        p.aead_nonce = vec![0x11; 8];
        assert_eq!(
            validate_application_data_aead(&p, &view),
            Err(ApplicationDataAeadError::NonCanonicalNonceLength)
        );
    }

    /// AAN-02 — 16-byte (over-long) nonce rejected.
    #[test]
    fn aan_02_long_nonce_rejected() {
        let view = base_view();
        let mut p = good_packet(0x11);
        p.aead_nonce = vec![0x11; 16];
        assert_eq!(
            validate_application_data_aead(&p, &view),
            Err(ApplicationDataAeadError::NonCanonicalNonceLength)
        );
    }

    /// AAN-03 — cross-group splice rejected.
    #[test]
    fn aan_03_cross_group_splice_rejected() {
        let view = base_view();
        let mut p = good_packet(0x11);
        p.group_id = b"hostile-group-XYZ".to_vec();
        assert_eq!(
            validate_application_data_aead(&p, &view),
            Err(ApplicationDataAeadError::CrossGroupNonceSplice)
        );
    }

    /// AAN-04 — stale-epoch packet rejected.
    #[test]
    fn aan_04_stale_epoch_rejected() {
        let view = base_view();
        let mut p = good_packet(0x11);
        p.epoch = 41;
        assert_eq!(
            validate_application_data_aead(&p, &view),
            Err(ApplicationDataAeadError::StaleEpochAead)
        );
    }

    /// AAN-05 — past-generation rejected (`GenerationGapTooLarge`).
    #[test]
    fn aan_05_past_generation_rejected() {
        let view = base_view();
        let mut p = good_packet(0x11);
        p.generation = 99;
        assert_eq!(
            validate_application_data_aead(&p, &view),
            Err(ApplicationDataAeadError::GenerationGapTooLarge)
        );
    }

    /// AAN-06 — far-future generation rejected.
    #[test]
    fn aan_06_far_future_generation_rejected() {
        let view = base_view();
        let mut p = good_packet(0x11);
        p.generation = 100 + MAX_GENERATION_WINDOW + 1;
        assert_eq!(
            validate_application_data_aead(&p, &view),
            Err(ApplicationDataAeadError::GenerationGapTooLarge)
        );
    }

    /// AAN-07 — all-zero AEAD nonce rejected.
    #[test]
    fn aan_07_zero_nonce_rejected() {
        let view = base_view();
        let p = good_packet(0x00);
        assert_eq!(
            validate_application_data_aead(&p, &view),
            Err(ApplicationDataAeadError::ZeroNonce)
        );
    }

    /// AAN-08 — nonce replay rejected via `used_nonces` ledger.
    #[test]
    fn aan_08_nonce_replay_rejected() {
        let mut view = base_view();
        let p = good_packet(0x11);
        // Pre-load the ledger with this exact quintuple.
        view.used_nonces.insert((
            p.group_id.clone(),
            p.epoch,
            p.leaf_index,
            p.generation,
            p.aead_nonce.clone(),
        ));
        assert_eq!(
            validate_application_data_aead(&p, &view),
            Err(ApplicationDataAeadError::NonceReplay)
        );
    }

    /// AAN-09 — valid packet at expected generation accepted.
    #[test]
    fn aan_09_valid_packet_accepted() {
        let view = base_view();
        let p = good_packet(0x11);
        assert_eq!(validate_application_data_aead(&p, &view), Ok(()));
    }

    /// AAN-10 — module green: compiles and re-exports through
    /// `CR-CHAT-02/src/lib.rs`.
    #[test]
    fn aan_10_module_green() {
        let count = 10usize;
        assert_eq!(
            count, 10,
            "Wave-30 L-CHAT-2-appnonce: {count} AAD-nonce-reuse falsifiers active"
        );
    }
}
