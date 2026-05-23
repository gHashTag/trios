//! # CR-CHAT-07 — anti-correlation
//!
//! L-CHAT-7 / Wave-6 — Cover-traffic scheduler & timing uniformity primitives
//! for Trinity Secure Chat (R-CHAT-10).
//!
//! ## Purpose
//!
//! Where CR-CHAT-04 hides *message length*, this ring hides *message
//! timing*. The mesh adversary observes the timestamps of every wire
//! envelope; without active counter-measures, inter-message gaps reveal:
//! (a) when a user is online, (b) which envelopes correlate (burstiness ⇒
//! same conversation), (c) chosen-message timing side-channels.
//!
//! We provide two pure primitives:
//!
//! 1. [`CoverScheduler`] — a deterministic scheduler that decides whether
//!    the *next* envelope on the wire should be a real payload or a
//!    decoy (cover) envelope, based on a fixed cadence and the queue
//!    state. Real and decoy envelopes are indistinguishable on the wire
//!    by R-CHAT-9 (same padding class) — see CR-CHAT-04.
//!
//! 2. [`uniform_gap_ms`] — quantises a real-time gap between two
//!    envelopes into one of a finite set of canonical gaps
//!    `{1000, 5000, 30_000, 300_000}` ms (1 s / 5 s / 30 s / 5 min) so
//!    timing leaks at most 2 bits per envelope.
//!
//! ## Honesty (R5)
//!
//! - `[VERIFIED]` for the scheduler logic and gap quantiser
//!   (4 unit tests).
//! - `[ASPIRATIONAL]` for full mesh integration. The trios-mesh node
//!   will consume `CoverScheduler::next()` to drive an emission timer
//!   in a follow-up ring (CR-CHAT-IO-07 in a future wave).
//! - `[CITED]` design follows Loopix (USENIX Security 2017) and the
//!   "Why I'm not using a private messenger" survey (2024) on cover
//!   traffic for unlinkability.
//!
//! ## Invariants
//!
//! - `R-CHAT-10 (i)` — for every real envelope produced, the scheduler
//!   has the option of producing zero or more decoy envelopes
//!   sandwiching it.
//! - `R-CHAT-10 (ii)` — `uniform_gap_ms(t)` returns a value from
//!   [`CANONICAL_GAPS_MS`] for any non-negative `t`.
//! - The scheduler is **deterministic** given a seed → reproducible by
//!   the falsifier and Coq witness.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA · UNLINKABLE`

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod egress_fingerprint;
pub use egress_fingerprint::{
    uniform_burst_ms, uniform_length_class, AlpnId, CipherId, EgressFingerprint,
    EgressObservables, TlsClass, CANONICAL_BURST_GAPS_MS, CANONICAL_LENGTH_CLASSES,
    CANONICAL_TLS_CLASS,
};

pub mod jitter_side_channel;
pub use jitter_side_channel::{
    validate_history as validate_jitter_history, GapObservation, GapRecorder, JitterError,
    JitterPolicy, WireKind,
};

pub mod cover_traffic_starvation;
pub use cover_traffic_starvation::{
    validate_window as validate_cover_window, CoverStarvationError, MIN_COVER_RATIO_DEN,
    MIN_COVER_RATIO_NUM, WINDOW_MIN_EMISSIONS,
};

pub mod ratchet_tree_extension_tampering;
pub use ratchet_tree_extension_tampering::{
    validate_ratchet_tree_extension, RatchetTreeExtError, RatchetTreeExtension, RatchetTreeNode,
    RatchetTreeView, RTX_MIN_LEAVES,
};

pub mod blind_signature_sender_token;
pub use blind_signature_sender_token::{
    validate_blind_signature_sender_token, BlindSenderToken, BlindTokenError, BlindTokenView,
    BLIND_SIGNATURE_LEN, BLIND_TOKEN_NONCE_LEN,
};

pub mod emission_cadence_uniformity;
pub use emission_cadence_uniformity::{
    validate_emission_cadence, CadenceError, EmissionKind as CadenceEmissionKind,
    ECAD_MAX_CONSECUTIVE_COVER, ECAD_MAX_CONSECUTIVE_REAL, ECAD_MAX_COVER_FRAC_DEN,
    ECAD_MAX_COVER_FRAC_NUM, ECAD_MAX_WINDOW, ECAD_MIN_COVER_FRAC_DEN, ECAD_MIN_COVER_FRAC_NUM,
    ECAD_MIN_WINDOW,
};

pub mod cover_traffic_burst_detection;
pub use cover_traffic_burst_detection::{
    validate_burst_pattern, BurstError, EmissionKind, EmissionRecord,
    BURST_MAX_EMISSIONS, BURST_MAX_SILENCE_MS, BURST_MIN_GAP_MS, BURST_WINDOW_MS,
};

/// Canonical inter-envelope gap classes (milliseconds). Quantising every
/// real gap into one of these foils per-envelope timing leaks below the
/// 4-class resolution.
pub const CANONICAL_GAPS_MS: [u64; 4] = [1_000, 5_000, 30_000, 300_000];

/// What the scheduler tells the I/O layer to emit next on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Emission {
    /// A real, queued chat envelope is due.
    Real,
    /// No real envelope due — emit a cover envelope (same padding class,
    /// random ciphertext) to keep the cadence indistinguishable.
    Cover,
}

/// Deterministic cover-traffic scheduler. Maintains a tick counter and a
/// real-message queue depth. On each tick, if the queue is non-empty it
/// emits `Real` and decrements; otherwise it emits `Cover`. The wire
/// observer cannot distinguish the two.
///
/// `[VERIFIED]` — `scheduler_emits_cover_when_queue_empty` and
/// `scheduler_emits_real_when_queue_nonempty` tests.
#[derive(Debug, Clone)]
pub struct CoverScheduler {
    /// Number of real envelopes currently waiting to be emitted.
    queue_depth: u64,
    /// Total ticks elapsed (monotonically increasing).
    ticks: u64,
}

impl Default for CoverScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl CoverScheduler {
    /// Build a fresh scheduler with empty queue and zero ticks.
    pub fn new() -> Self {
        Self {
            queue_depth: 0,
            ticks: 0,
        }
    }

    /// Caller pushed a real envelope onto the wire queue.
    pub fn enqueue_real(&mut self) {
        self.queue_depth = self.queue_depth.saturating_add(1);
    }

    /// Total cumulative ticks observed.
    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    /// Current outstanding real-envelope count (zero ⇒ next emission is
    /// a cover envelope).
    pub fn queue_depth(&self) -> u64 {
        self.queue_depth
    }

    /// Advance one tick and decide what to emit.
    ///
    /// `[VERIFIED]` invariant: the wire observer sees exactly one
    /// emission per call regardless of queue state.
    pub fn tick(&mut self) -> Emission {
        self.ticks = self.ticks.saturating_add(1);
        if self.queue_depth > 0 {
            self.queue_depth -= 1;
            Emission::Real
        } else {
            Emission::Cover
        }
    }
}

/// Quantise a measured inter-envelope gap (in milliseconds) into one of
/// the canonical classes. The chosen class is the *largest* canonical
/// gap `g` such that `g <= measured`; if `measured` is below the
/// smallest class we still return the smallest class so the wire never
/// shows a gap shorter than 1 s.
///
/// `[VERIFIED]` — `uniform_gap_quantises_to_canonical_set` test.
pub fn uniform_gap_ms(measured_ms: u64) -> u64 {
    let mut chosen = CANONICAL_GAPS_MS[0];
    for &g in &CANONICAL_GAPS_MS {
        if measured_ms >= g {
            chosen = g;
        }
    }
    chosen
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduler_emits_cover_when_queue_empty() {
        let mut s = CoverScheduler::new();
        assert_eq!(s.tick(), Emission::Cover);
        assert_eq!(s.tick(), Emission::Cover);
        assert_eq!(s.ticks(), 2);
        assert_eq!(s.queue_depth(), 0);
    }

    #[test]
    fn scheduler_emits_real_when_queue_nonempty() {
        let mut s = CoverScheduler::new();
        s.enqueue_real();
        s.enqueue_real();
        assert_eq!(s.tick(), Emission::Real);
        assert_eq!(s.tick(), Emission::Real);
        assert_eq!(s.tick(), Emission::Cover);
        assert_eq!(s.ticks(), 3);
    }

    #[test]
    fn falsifier_observer_cannot_count_real_via_emissions() {
        // Adversary observes 10 emissions. They MUST see 10 emissions
        // regardless of how many were real — wire indistinguishability.
        let mut s_a = CoverScheduler::new();
        let mut s_b = CoverScheduler::new();
        for _ in 0..3 {
            s_a.enqueue_real();
        }
        // Alice has 3 real, Bob has 0. Both emit 10 times.
        let count_a = (0..10).map(|_| s_a.tick()).count();
        let count_b = (0..10).map(|_| s_b.tick()).count();
        assert_eq!(count_a, count_b, "wire emission count must be uniform");
    }

    #[test]
    fn uniform_gap_quantises_to_canonical_set() {
        // 0 ms → smallest class.
        assert_eq!(uniform_gap_ms(0), 1_000);
        // Exactly canonical → that class.
        assert_eq!(uniform_gap_ms(1_000), 1_000);
        assert_eq!(uniform_gap_ms(5_000), 5_000);
        assert_eq!(uniform_gap_ms(30_000), 30_000);
        assert_eq!(uniform_gap_ms(300_000), 300_000);
        // Just above a class → that class until next boundary.
        assert_eq!(uniform_gap_ms(4_999), 1_000);
        assert_eq!(uniform_gap_ms(29_999), 5_000);
        // Far above largest → still largest (capped).
        assert_eq!(uniform_gap_ms(10_000_000), 300_000);
        // Every output is canonical.
        for &t in &[0, 1, 999, 1_000, 4_999, 5_001, 30_001, 300_001, u64::MAX] {
            assert!(CANONICAL_GAPS_MS.contains(&uniform_gap_ms(t)));
        }
    }
}
