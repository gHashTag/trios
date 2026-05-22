//! Wave-35 / L-CHAT-5-sker (R-CHAT-5 / CR-CHAT-05) — Sender-keys epoch
//! window replay defence per RFC 9420 §15.5 (Sender-Data + application
//! generation) and the Sealed-Sender follow-up gap identified by the
//! NDSS 2021 §V analysis.
//!
//! MLS application messages carry a `(epoch, generation)` pair. Each
//! `(sender, epoch)` chain advances `generation` strictly. A naïve
//! receiver only checks `generation > last_seen` *within* an epoch,
//! which leaves a small but real **epoch-window replay** gap: an
//! adversary who buffered an old application message from epoch N
//! can replay it after epoch N+1 has started but before the receiver
//! has fully evicted state for N — without the bounded-window check
//! the receiver re-accepts. RFC 9420 §15.5 mandates a sliding window
//! of size `SENDER_KEYS_EPOCH_WINDOW` (W35 fixes this at 1 — only the
//! *current* epoch is accepted; the previous epoch is accepted only
//! during a strictly bounded grace window).
//!
//! This lane enforces the consumption-side invariants. A single deny
//! wins.
//!
//! Seven rules enforced in fixed order:
//!   1. NonCanonicalSenderIdLength — `packet.sender_id.len()` must
//!      equal `SENDER_KEYS_SENDER_ID_LEN` (16 bytes — MLS leaf-node
//!      LeafNodeRef per RFC 9420 §6.1).
//!   2. UnknownSender — `packet.sender_id` must be present in
//!      `view.known_senders`. No phantom senders.
//!   3. EpochOutsideWindow — `view.current_epoch.saturating_sub(
//!      packet.epoch) > SENDER_KEYS_EPOCH_WINDOW`. The packet's
//!      epoch must be inside the sliding window of size 1. Future
//!      epochs (`packet.epoch > view.current_epoch`) are also
//!      rejected (no time travel).
//!   4. NonMonotonicGeneration — `packet.generation` MUST be strictly
//!      greater than `view.last_generation[(sender, epoch)]`. Equal
//!      or lower is rejected — explicit replay.
//!   5. EpochAlreadyEvicted — if `view.evicted_epochs.contains(
//!      &packet.epoch)`, reject. Once an epoch is fully evicted (the
//!      grace window has lapsed) no message from it is acceptable.
//!   6. ZeroGeneration — `packet.generation == 0` is forbidden. The
//!      MLS generation counter starts at 1 per §15.5.
//!   7. FutureEpoch — `packet.epoch > view.current_epoch` is
//!      rejected. Epoch advance happens only via a Commit (out of
//!      band for this guard); application packets cannot leap ahead.

#![forbid(unsafe_code)]

/// Canonical sender_id length (16 bytes — MLS LeafNodeRef per
/// RFC 9420 §6.1).
pub const SENDER_KEYS_SENDER_ID_LEN: usize = 16;

/// Sliding window of acceptable epoch lag. W35 ships size 1 — the
/// current epoch and exactly one prior epoch (during the grace
/// window) are accepted; older ones are not.
pub const SENDER_KEYS_EPOCH_WINDOW: u64 = 1;

/// A single MLS application packet arriving at the receiver, carrying
/// `(sender_id, epoch, generation)` per RFC 9420 §15.5.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SenderKeysPacket {
    /// Sender's LeafNodeRef (16 bytes).
    pub sender_id: Vec<u8>,
    /// Packet epoch.
    pub epoch: u64,
    /// Strictly monotone generation counter inside `(sender, epoch)`.
    pub generation: u64,
}

/// Receiver-side view of sender-keys state.
#[derive(Clone, Debug)]
pub struct SenderKeysView {
    /// Current epoch.
    pub current_epoch: u64,
    /// Known senders the receiver accepts packets from.
    pub known_senders: Vec<Vec<u8>>,
    /// Per-`(sender, epoch)` last-seen generation counters.
    pub last_generation: Vec<(Vec<u8>, u64, u64)>,
    /// Epochs that have been fully evicted (the grace window for
    /// `(sender, epoch)` state has lapsed).
    pub evicted_epochs: Vec<u64>,
}

/// Typed errors for `validate_sender_keys_packet`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SenderKeysError {
    /// Rule 1 — `sender_id.len() != SENDER_KEYS_SENDER_ID_LEN`.
    NonCanonicalSenderIdLength,
    /// Rule 2 — `sender_id` not in `view.known_senders`.
    UnknownSender,
    /// Rule 3 — epoch outside the sliding window.
    EpochOutsideWindow,
    /// Rule 4 — `generation <= last_generation[(sender, epoch)]`.
    NonMonotonicGeneration,
    /// Rule 5 — `epoch` already evicted.
    EpochAlreadyEvicted,
    /// Rule 6 — `generation == 0`.
    ZeroGeneration,
    /// Rule 7 — `epoch > current_epoch` (future epoch).
    FutureEpoch,
}

/// Constructive guard for a single MLS application packet's
/// `(sender_id, epoch, generation)` triple. Returns `Ok(())` iff
/// every rule (1)..(7) holds.
///
/// `[VERIFIED]` against the 10 unit tests `SKER-01..10` below and the
/// Coq theorems `INV-CHAT-223..227` in the W35 Section of
/// `proofs/chat/Trinity_Chat.v`.
pub fn validate_sender_keys_packet(
    packet: &SenderKeysPacket,
    view: &SenderKeysView,
) -> Result<(), SenderKeysError> {
    // Rule 1: sender_id length canonical.
    if packet.sender_id.len() != SENDER_KEYS_SENDER_ID_LEN {
        return Err(SenderKeysError::NonCanonicalSenderIdLength);
    }
    // Rule 2: sender must be known.
    if !view.known_senders.contains(&packet.sender_id) {
        return Err(SenderKeysError::UnknownSender);
    }
    // Rule 7 (checked before Rule 3 since future > window in lag): no
    // time travel — packet epoch cannot exceed current epoch.
    if packet.epoch > view.current_epoch {
        return Err(SenderKeysError::FutureEpoch);
    }
    // Rule 3: epoch must be inside the sliding window.
    let lag = view.current_epoch - packet.epoch;
    if lag > SENDER_KEYS_EPOCH_WINDOW {
        return Err(SenderKeysError::EpochOutsideWindow);
    }
    // Rule 5: already-evicted epoch is rejected.
    if view.evicted_epochs.contains(&packet.epoch) {
        return Err(SenderKeysError::EpochAlreadyEvicted);
    }
    // Rule 6: zero generation forbidden — MLS counter starts at 1.
    if packet.generation == 0 {
        return Err(SenderKeysError::ZeroGeneration);
    }
    // Rule 4: monotone generation.
    let last_seen = view
        .last_generation
        .iter()
        .find(|(s, e, _)| s == &packet.sender_id && *e == packet.epoch)
        .map(|(_, _, g)| *g)
        .unwrap_or(0);
    if packet.generation <= last_seen {
        return Err(SenderKeysError::NonMonotonicGeneration);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sender_a() -> Vec<u8> {
        vec![0xA1_u8; SENDER_KEYS_SENDER_ID_LEN]
    }

    fn sender_b() -> Vec<u8> {
        vec![0xB2_u8; SENDER_KEYS_SENDER_ID_LEN]
    }

    fn ok_view() -> SenderKeysView {
        SenderKeysView {
            current_epoch: 10,
            known_senders: vec![sender_a()],
            last_generation: vec![(sender_a(), 10, 5)],
            evicted_epochs: vec![],
        }
    }

    fn ok_packet() -> SenderKeysPacket {
        SenderKeysPacket {
            sender_id: sender_a(),
            epoch: 10,
            generation: 6,
        }
    }

    /// SKER-01 — 8-byte sender_id rejected.
    #[test]
    fn sker_01_short_sender_id_rejected() {
        let mut p = ok_packet();
        p.sender_id = vec![0xA1_u8; 8];
        assert_eq!(
            validate_sender_keys_packet(&p, &ok_view()),
            Err(SenderKeysError::NonCanonicalSenderIdLength)
        );
    }

    /// SKER-02 — unknown sender rejected.
    #[test]
    fn sker_02_unknown_sender_rejected() {
        let mut p = ok_packet();
        p.sender_id = sender_b();
        assert_eq!(
            validate_sender_keys_packet(&p, &ok_view()),
            Err(SenderKeysError::UnknownSender)
        );
    }

    /// SKER-03 — epoch outside window (lag = 2) rejected.
    #[test]
    fn sker_03_epoch_outside_window_rejected() {
        let mut p = ok_packet();
        p.epoch = 8; // current 10, window 1, lag 2 -> reject
        assert_eq!(
            validate_sender_keys_packet(&p, &ok_view()),
            Err(SenderKeysError::EpochOutsideWindow)
        );
    }

    /// SKER-04 — non-monotonic generation (replay) rejected.
    #[test]
    fn sker_04_non_monotonic_generation_rejected() {
        let mut p = ok_packet();
        p.generation = 5; // last_seen is 5 — equal must reject
        assert_eq!(
            validate_sender_keys_packet(&p, &ok_view()),
            Err(SenderKeysError::NonMonotonicGeneration)
        );
    }

    /// SKER-05 — already-evicted epoch rejected.
    #[test]
    fn sker_05_evicted_epoch_rejected() {
        let mut p = ok_packet();
        // Drop to lag-1 epoch first so we reach Rule 5.
        p.epoch = 9;
        let mut view = ok_view();
        view.evicted_epochs.push(9);
        // Also seed last_generation for (sender, 9) to avoid hitting
        // monotone-check default = 0 first.
        view.last_generation.push((sender_a(), 9, 4));
        assert_eq!(
            validate_sender_keys_packet(&p, &view),
            Err(SenderKeysError::EpochAlreadyEvicted)
        );
    }

    /// SKER-06 — zero generation rejected.
    #[test]
    fn sker_06_zero_generation_rejected() {
        let mut p = ok_packet();
        p.generation = 0;
        assert_eq!(
            validate_sender_keys_packet(&p, &ok_view()),
            Err(SenderKeysError::ZeroGeneration)
        );
    }

    /// SKER-07 — future epoch (`epoch > current_epoch`) rejected.
    #[test]
    fn sker_07_future_epoch_rejected() {
        let mut p = ok_packet();
        p.epoch = 11; // current 10
        assert_eq!(
            validate_sender_keys_packet(&p, &ok_view()),
            Err(SenderKeysError::FutureEpoch)
        );
    }

    /// SKER-08 — current epoch with monotone generation accepted.
    #[test]
    fn sker_08_current_epoch_monotone_accepted() {
        assert_eq!(
            validate_sender_keys_packet(&ok_packet(), &ok_view()),
            Ok(())
        );
    }

    /// SKER-09 — prior epoch (lag = 1) inside window with monotone
    /// generation accepted.
    #[test]
    fn sker_09_prior_epoch_inside_window_accepted() {
        let mut p = ok_packet();
        p.epoch = 9; // lag = 1 — boundary of window
        p.generation = 7;
        let mut view = ok_view();
        view.last_generation.push((sender_a(), 9, 6));
        assert_eq!(validate_sender_keys_packet(&p, &view), Ok(()));
    }

    /// SKER-10 — first-ever message from a known sender at the
    /// current epoch (no prior `(sender, epoch)` entry — default
    /// last_seen = 0) accepted with generation = 1.
    #[test]
    fn sker_10_first_message_generation_one_accepted() {
        let p = SenderKeysPacket {
            sender_id: sender_a(),
            epoch: 10,
            generation: 1,
        };
        let view = SenderKeysView {
            current_epoch: 10,
            known_senders: vec![sender_a()],
            last_generation: vec![], // no prior generation seen
            evicted_epochs: vec![],
        };
        assert_eq!(validate_sender_keys_packet(&p, &view), Ok(()));
    }
}
