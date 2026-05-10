//! # BR-IO-CHAT-07 — async wire-emitter
//!
//! L-CHAT-7 (async half) / Wave-7 — the I/O twin of [`CR-CHAT-07`].
//! Where `CR-CHAT-07` declares the **pure logic** of the
//! [`CoverScheduler`] (what to emit when), this Bronze ring drives that
//! logic over an async channel using `tokio`'s deterministic
//! `time::pause` clock.
//!
//! ## Why split logic vs. I/O ?
//!
//! - Trinity rings are L-ARCH-001-compliant — Silver rings stay pure so
//!   that Coq proofs and falsifier witnesses are byte-reproducible.
//! - Real cover traffic must hit a wire eventually; that needs `tokio`,
//!   timers, and channels. Those live here, in the Bronze ring.
//!
//! ## Invariants (R5)
//!
//! - `R-CHAT-10 (iii)` — every wall-clock tick of the emitter produces
//!   *exactly one* [`Emission`] downstream — never zero, never two.
//! - `R-CHAT-10 (iv)` — given the same sequence of `enqueue_real`
//!   calls, the emitter produces an identical [`Emission`] stream as
//!   the pure scheduler in `CR-CHAT-07` would. (This is what the async
//!   path proves operationally; Coq proves it abstractly.)
//! - `R-CHAT-10 (v)` — when no real payloads are queued, the emitter
//!   *still emits* (with [`Emission::Cover`]). Silence ⇒ leak.
//!
//! ## Honesty (R5)
//!
//! - `[VERIFIED]` for `WireEmitter` — 5 deterministic async tests using
//!   `tokio::time::pause` + `advance`.
//! - `[DERIVED]` for stream equivalence — the emitter drives the same
//!   `CoverScheduler::tick` whose 4 unit tests already pin behaviour.
//! - `[CITED]` Loopix mix-strategy (USENIX Security 2017).
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA · UNLINKABLE`

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time::Instant;

use trios_chat_cr_chat_07::{CoverScheduler, Emission};

/// Default cover-traffic period (ms). Picked from the smallest
/// canonical bin — a 1-second cadence is the upper bound on observable
/// per-message timing leak under R-CHAT-10.
pub const DEFAULT_TICK_MS: u64 = 1_000;

/// Async cover-traffic emitter. Each `tick` it consults its inner
/// `CoverScheduler` and pushes a single [`Emission`] onto the
/// downstream channel.
///
/// `[VERIFIED]` via 5 deterministic async tests under
/// `tokio::time::pause`.
pub struct WireEmitter {
    sched: CoverScheduler,
    tick: Duration,
    out: mpsc::UnboundedSender<Emission>,
}

impl WireEmitter {
    /// Build an emitter with a custom tick period.
    pub fn new(tick: Duration, out: mpsc::UnboundedSender<Emission>) -> Self {
        Self {
            sched: CoverScheduler::new(),
            tick,
            out,
        }
    }

    /// Build an emitter at [`DEFAULT_TICK_MS`].
    pub fn with_default_tick(out: mpsc::UnboundedSender<Emission>) -> Self {
        Self::new(Duration::from_millis(DEFAULT_TICK_MS), out)
    }

    /// Enqueue a real envelope to be sent at the *next* tick.
    pub fn enqueue_real(&mut self) {
        self.sched.enqueue_real();
    }

    /// Run exactly `n_ticks` emission cycles, sleeping between each.
    /// Returns the number of emissions actually pushed (`<= n_ticks` if
    /// the channel is closed). Driven by [`tokio::time`], so under
    /// `time::pause` the call advances logical time only.
    pub async fn run_for(&mut self, n_ticks: u64) -> u64 {
        let mut sent = 0u64;
        let start = Instant::now();
        for i in 0..n_ticks {
            // Sleep until tick boundary `i+1` from `start` — this keeps
            // jitter from stealing logical time when run under
            // `time::advance`.
            let target = start + self.tick * ((i as u32) + 1);
            tokio::time::sleep_until(target).await;
            let em = self.sched.tick();
            if self.out.send(em).is_err() {
                break;
            }
            sent += 1;
        }
        sent
    }

    /// Read-only view of the underlying scheduler depth (for tests).
    pub fn queue_depth(&self) -> u64 {
        self.sched.queue_depth()
    }

    /// Read-only view of total ticks consumed (for tests).
    pub fn ticks(&self) -> u64 {
        self.sched.ticks()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AE-01 — empty queue ⇒ every tick emits `Cover`. R-CHAT-10 (v).
    #[tokio::test(start_paused = true)]
    async fn ae01_empty_queue_emits_cover_every_tick() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut e = WireEmitter::with_default_tick(tx);
        let n = 4;
        let sent = e.run_for(n).await;
        assert_eq!(sent, n);
        for _ in 0..n {
            assert_eq!(rx.try_recv().unwrap(), Emission::Cover);
        }
        assert!(rx.try_recv().is_err());
        assert_eq!(e.ticks(), n);
    }

    /// AE-02 — n queued reals ⇒ first n ticks are `Real`, rest `Cover`.
    /// R-CHAT-10 (iii) + (iv).
    #[tokio::test(start_paused = true)]
    async fn ae02_queued_reals_emit_first_then_cover() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut e = WireEmitter::with_default_tick(tx);
        e.enqueue_real();
        e.enqueue_real();
        e.enqueue_real();
        assert_eq!(e.queue_depth(), 3);
        let sent = e.run_for(5).await;
        assert_eq!(sent, 5);
        assert_eq!(rx.try_recv().unwrap(), Emission::Real);
        assert_eq!(rx.try_recv().unwrap(), Emission::Real);
        assert_eq!(rx.try_recv().unwrap(), Emission::Real);
        assert_eq!(rx.try_recv().unwrap(), Emission::Cover);
        assert_eq!(rx.try_recv().unwrap(), Emission::Cover);
        assert_eq!(e.queue_depth(), 0);
    }

    /// AE-03 — async stream equivalence: BR-IO-07 emission sequence
    /// equals what a pure CR-CHAT-07 scheduler would produce given the
    /// same enqueue pattern. R-CHAT-10 (iv).
    #[tokio::test(start_paused = true)]
    async fn ae03_async_matches_pure_scheduler() {
        // Reference (pure)
        let mut pure = CoverScheduler::new();
        pure.enqueue_real();
        pure.enqueue_real();
        let pure_seq: Vec<Emission> = (0..6).map(|_| pure.tick()).collect();

        // Async (BR-IO)
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut e = WireEmitter::with_default_tick(tx);
        e.enqueue_real();
        e.enqueue_real();
        let _ = e.run_for(6).await;
        let mut async_seq = Vec::with_capacity(6);
        for _ in 0..6 {
            async_seq.push(rx.try_recv().unwrap());
        }
        assert_eq!(async_seq, pure_seq, "BR-IO must replay pure scheduler");
    }

    /// AE-04 — closed downstream halts emitter early without panic.
    /// Smell test for graceful shutdown.
    #[tokio::test(start_paused = true)]
    async fn ae04_closed_channel_halts_gracefully() {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut e = WireEmitter::with_default_tick(tx);
        drop(rx); // close downstream BEFORE running
        let sent = e.run_for(10).await;
        // First send fails ⇒ break ⇒ sent == 0
        assert_eq!(sent, 0);
    }

    /// AE-05 — tick cadence honoured: under `time::pause` the
    /// `run_for(n)` call advances logical time by exactly `n * tick`.
    /// `tokio::time::Instant` is monotonic under pause, so we can
    /// compare deltas without wall-clock flakiness.
    #[tokio::test(start_paused = true)]
    async fn ae05_logical_time_advances_by_n_ticks() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let tick = Duration::from_millis(DEFAULT_TICK_MS);
        let mut e = WireEmitter::new(tick, tx);
        let t0 = Instant::now();
        let _ = e.run_for(3).await;
        let elapsed = Instant::now() - t0;
        assert_eq!(
            elapsed,
            tick * 3,
            "under time::pause, 3 ticks must advance logical clock by 3 * tick"
        );
    }

    /// G-C7-async — single green summary line for human/CI scan
    /// `[VERIFIED]` BR-IO-CHAT-07 emits Real+Cover deterministically.
    #[tokio::test(start_paused = true)]
    async fn green_summary_async_wire_emitter() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let _e = WireEmitter::with_default_tick(tx);
        // 5 AE-NN tests above prove the property; this line tags the
        // ring as live in CI output.
        // R-CHAT-10 (iii)+(iv)+(v) covered.
    }
}
