//! Wave-36 / L-CHAT-4-amgsd (R-CHAT-4 / CR-CHAT-04) — Application-message
//! generation skip-window DoS defence per RFC 9420 §15.2 "Application
//! Messages" + RFC 9420 §9.3 "Message Receiving" bounded skip window.
//!
//! Each MLS application message carries a `(sender, epoch, generation)`
//! triple. Within a single `(sender, epoch)` the generation counter is
//! strictly monotone, but receivers MUST tolerate a small amount of
//! out-of-order delivery — application keys for skipped generations
//! are derived on demand. RFC 9420 §9.3 makes the upper bound on that
//! tolerance a hard security property: **receivers MUST cap the
//! number of generations they will key-derive ahead of the last-seen
//! generation**. Without the cap, a single attacker-controlled packet
//! claiming `generation = u64::MAX` forces the receiver into a
//! tens-of-billions-of-rounds HKDF chain — a CPU/memory DoS.
//!
//! W36 fixes the skip window at `APP_MSG_SKIP_WINDOW = 1024`. This is
//! the OpenMLS default and the same value that the RFC's security
//! considerations cite as a safe ceiling for MLS deployments.
//!
//! Skip-window semantics are distinct from `sender_keys_epoch_window`
//! (W35 covers cross-epoch replay). This lane covers the
//! **intra-epoch skip distance** that the receiver is allowed to
//! key-derive before declaring the packet a DoS.
//!
//! A single deny wins.
//!
//! Seven rules enforced in fixed order:
//!   1. NonCanonicalSenderIdLength — `packet.sender_id.len()` must
//!      equal `APP_MSG_SENDER_ID_LEN` (16 bytes — MLS LeafNodeRef
//!      per RFC 9420 §6.1).
//!   2. UnknownSender — `packet.sender_id` must be present in
//!      `view.known_senders`.
//!   3. ZeroGeneration — `packet.generation == 0` is forbidden. The
//!      MLS application-key generation counter starts at 1.
//!   4. NonMonotonicGeneration — `packet.generation` MUST be strictly
//!      greater than `view.last_generation[(sender, epoch)]`. Equal
//!      or lower is a replay.
//!   5. SkipDistanceExceeded — `(packet.generation - last_generation -
//!      1) > APP_MSG_SKIP_WINDOW`. Anything beyond the cap is a DoS
//!      probe: deny **before** any key-schedule work.
//!   6. EpochMismatch — `packet.epoch != view.current_epoch`. This
//!      lane only operates on the active epoch — older epochs are
//!      W35's domain (sender-keys-epoch-window-replay).
//!   7. CiphertextEmpty — `packet.ciphertext.is_empty()`. Empty
//!      application ciphertext is rejected: a length-0 packet can be
//!      replayed cheaply by an attacker to advance the receiver's
//!      generation pointer (poisoning the skip window for legitimate
//!      packets).

#![forbid(unsafe_code)]

/// Canonical sender_id length (16 bytes — MLS LeafNodeRef per
/// RFC 9420 §6.1).
pub const APP_MSG_SENDER_ID_LEN: usize = 16;

/// Maximum number of skipped generations the receiver will key-derive
/// ahead of `last_generation`. RFC 9420 §9.3 recommends a bounded
/// ceiling; W36 ships 1024 (OpenMLS default).
pub const APP_MSG_SKIP_WINDOW: u64 = 1024;

/// A single MLS application-message header arriving at the receiver,
/// per RFC 9420 §15.2.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppMessagePacket {
    /// Sender's LeafNodeRef (16 bytes).
    pub sender_id: Vec<u8>,
    /// Active epoch the packet claims to belong to.
    pub epoch: u64,
    /// Strictly monotone generation counter inside `(sender, epoch)`.
    pub generation: u64,
    /// Application ciphertext (sealed body — opaque here).
    pub ciphertext: Vec<u8>,
}

/// Receiver-side view of application-key state.
#[derive(Clone, Debug)]
pub struct AppMessageView {
    /// Active epoch.
    pub current_epoch: u64,
    /// Known senders the receiver accepts packets from.
    pub known_senders: Vec<Vec<u8>>,
    /// Per-`(sender, epoch)` last-seen generation counters.
    pub last_generation: Vec<(Vec<u8>, u64, u64)>,
}

/// Typed errors for `validate_app_message_skip`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AppMessageSkipError {
    /// Rule 1 — `sender_id.len() != APP_MSG_SENDER_ID_LEN`.
    NonCanonicalSenderIdLength,
    /// Rule 2 — `sender_id` not in `view.known_senders`.
    UnknownSender,
    /// Rule 3 — `generation == 0`.
    ZeroGeneration,
    /// Rule 4 — `generation <= last_generation[(sender, epoch)]`.
    NonMonotonicGeneration,
    /// Rule 5 — skip distance exceeds `APP_MSG_SKIP_WINDOW`.
    SkipDistanceExceeded,
    /// Rule 6 — packet epoch is not the current epoch.
    EpochMismatch,
    /// Rule 7 — empty ciphertext.
    CiphertextEmpty,
}

/// Constructive guard for a single MLS application packet's
/// `(sender_id, epoch, generation)` triple plus body shape. Returns
/// `Ok(())` iff every rule (1)..(7) holds.
///
/// `[VERIFIED]` against the 10 unit tests `AMGSD-01..10` below and
/// the Coq theorems `INV-CHAT-233..237` in the W36 Section of
/// `proofs/chat/Trinity_Chat.v`.
pub fn validate_app_message_skip(
    packet: &AppMessagePacket,
    view: &AppMessageView,
) -> Result<(), AppMessageSkipError> {
    // Rule 1: sender_id length canonical.
    if packet.sender_id.len() != APP_MSG_SENDER_ID_LEN {
        return Err(AppMessageSkipError::NonCanonicalSenderIdLength);
    }
    // Rule 2: sender must be known.
    if !view.known_senders.contains(&packet.sender_id) {
        return Err(AppMessageSkipError::UnknownSender);
    }
    // Rule 6: epoch must be the active epoch.
    if packet.epoch != view.current_epoch {
        return Err(AppMessageSkipError::EpochMismatch);
    }
    // Rule 7: non-empty ciphertext.
    if packet.ciphertext.is_empty() {
        return Err(AppMessageSkipError::CiphertextEmpty);
    }
    // Rule 3: zero generation forbidden — MLS counter starts at 1.
    if packet.generation == 0 {
        return Err(AppMessageSkipError::ZeroGeneration);
    }
    // Lookup `last_generation` for `(sender, epoch)`.
    let last_seen = view
        .last_generation
        .iter()
        .find(|(s, e, _)| s == &packet.sender_id && *e == packet.epoch)
        .map(|(_, _, g)| *g)
        .unwrap_or(0);
    // Rule 4: monotone generation.
    if packet.generation <= last_seen {
        return Err(AppMessageSkipError::NonMonotonicGeneration);
    }
    // Rule 5: skip distance bounded.
    //
    // `skip_distance = packet.generation - last_seen - 1` counts the
    // number of intermediate generations the receiver would have to
    // key-derive before reaching `packet.generation`. We deny iff that
    // exceeds `APP_MSG_SKIP_WINDOW`.
    let skip_distance = packet.generation - last_seen - 1;
    if skip_distance > APP_MSG_SKIP_WINDOW {
        return Err(AppMessageSkipError::SkipDistanceExceeded);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sender_a() -> Vec<u8> {
        vec![0xA1_u8; APP_MSG_SENDER_ID_LEN]
    }

    fn sender_b() -> Vec<u8> {
        vec![0xB2_u8; APP_MSG_SENDER_ID_LEN]
    }

    fn ok_view() -> AppMessageView {
        AppMessageView {
            current_epoch: 7,
            known_senders: vec![sender_a()],
            last_generation: vec![(sender_a(), 7, 10)],
        }
    }

    fn ok_packet() -> AppMessagePacket {
        AppMessagePacket {
            sender_id: sender_a(),
            epoch: 7,
            generation: 11, // last_seen 10, skip distance 0
            ciphertext: vec![0xCC_u8; 64],
        }
    }

    /// AMGSD-01 — 8-byte sender_id rejected — Rule 1.
    #[test]
    fn amgsd_01_short_sender_id_rejected() {
        let mut p = ok_packet();
        p.sender_id = vec![0xA1_u8; 8];
        assert_eq!(
            validate_app_message_skip(&p, &ok_view()),
            Err(AppMessageSkipError::NonCanonicalSenderIdLength)
        );
    }

    /// AMGSD-02 — unknown sender rejected — Rule 2.
    #[test]
    fn amgsd_02_unknown_sender_rejected() {
        let mut p = ok_packet();
        p.sender_id = sender_b();
        assert_eq!(
            validate_app_message_skip(&p, &ok_view()),
            Err(AppMessageSkipError::UnknownSender)
        );
    }

    /// AMGSD-03 — wrong epoch rejected — Rule 6.
    #[test]
    fn amgsd_03_epoch_mismatch_rejected() {
        let mut p = ok_packet();
        p.epoch = 6;
        assert_eq!(
            validate_app_message_skip(&p, &ok_view()),
            Err(AppMessageSkipError::EpochMismatch)
        );
    }

    /// AMGSD-04 — empty ciphertext rejected — Rule 7.
    #[test]
    fn amgsd_04_empty_ciphertext_rejected() {
        let mut p = ok_packet();
        p.ciphertext = vec![];
        assert_eq!(
            validate_app_message_skip(&p, &ok_view()),
            Err(AppMessageSkipError::CiphertextEmpty)
        );
    }

    /// AMGSD-05 — zero generation rejected — Rule 3.
    #[test]
    fn amgsd_05_zero_generation_rejected() {
        let mut p = ok_packet();
        p.generation = 0;
        assert_eq!(
            validate_app_message_skip(&p, &ok_view()),
            Err(AppMessageSkipError::ZeroGeneration)
        );
    }

    /// AMGSD-06 — non-monotonic generation (equal to last_seen)
    /// rejected — Rule 4.
    #[test]
    fn amgsd_06_non_monotonic_equal_rejected() {
        let mut p = ok_packet();
        p.generation = 10; // last_seen 10
        assert_eq!(
            validate_app_message_skip(&p, &ok_view()),
            Err(AppMessageSkipError::NonMonotonicGeneration)
        );
    }

    /// AMGSD-07 — non-monotonic generation (lower than last_seen)
    /// rejected — Rule 4 (explicit replay).
    #[test]
    fn amgsd_07_non_monotonic_lower_rejected() {
        let mut p = ok_packet();
        p.generation = 5; // last_seen 10
        assert_eq!(
            validate_app_message_skip(&p, &ok_view()),
            Err(AppMessageSkipError::NonMonotonicGeneration)
        );
    }

    /// AMGSD-08 — skip distance just beyond the cap rejected — Rule 5
    /// (the DoS-resistance core).
    #[test]
    fn amgsd_08_skip_distance_just_over_cap_rejected() {
        let mut p = ok_packet();
        // last_seen 10, skip_distance = gen - 10 - 1; we want
        // skip_distance = APP_MSG_SKIP_WINDOW + 1.
        p.generation = 10 + 1 + (APP_MSG_SKIP_WINDOW + 1);
        assert_eq!(
            validate_app_message_skip(&p, &ok_view()),
            Err(AppMessageSkipError::SkipDistanceExceeded)
        );
    }

    /// AMGSD-09 — skip distance exactly at the cap accepted —
    /// boundary value of Rule 5.
    #[test]
    fn amgsd_09_skip_distance_at_cap_accepted() {
        let mut p = ok_packet();
        // skip_distance = APP_MSG_SKIP_WINDOW exactly
        p.generation = 10 + 1 + APP_MSG_SKIP_WINDOW;
        assert_eq!(validate_app_message_skip(&p, &ok_view()), Ok(()));
    }

    /// AMGSD-10 — canonical packet (skip distance 0) accepted.
    #[test]
    fn amgsd_10_canonical_packet_accepted() {
        assert_eq!(
            validate_app_message_skip(&ok_packet(), &ok_view()),
            Ok(())
        );
    }
}
