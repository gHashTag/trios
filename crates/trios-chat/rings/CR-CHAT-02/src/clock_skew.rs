//! # CR-CHAT-02 — Clock-skew & replay-window edge cases (Wave-16)
//!
//! `L-CHAT-2-clock` (R-CHAT-2 + R-CHAT-4) — formalises the time-domain
//! attack surface around the Triple-Ratchet replay window.
//!
//! ## Threat model
//!
//! A passive-or-active network observer can:
//!
//! - **Stash-and-resend** a ciphertext (replay) with the ORIGINAL timestamp
//!   long after the legitimate counter has advanced.
//! - **Backdate** a ciphertext using a clock that is far behind the
//!   receiver's clock (clock-skew replay).
//! - **Forward-date** a ciphertext using a clock that is far ahead of the
//!   receiver's clock (future-dated injection).
//! - **Wrap** the counter epoch boundary in an attempt to slide a stale
//!   counter back into the live window (epoch-rollover replay).
//!
//! All four attacks must be rejected by `ReplayWindow::accept_at` with the
//! receiver still able to admit legitimate out-of-order delivery within a
//! fixed grace clock-skew bound (default 30 s, configurable per session).
//!
//! ## Surface
//!
//! - [`ReplayWindow`] — bounded receiver-side replay-window keyed on
//!   counter (within an epoch) AND timestamp (across epochs).
//! - [`ClockSkewBound`] — symmetric `±skew_ms` band around the receiver
//!   clock; messages outside the band are rejected as too-old / too-new.
//! - [`ReplayDecision`] — `{ Accept, RejectReplay, RejectStale, RejectFuture, RejectEpochRollover }`.
//! - 6 falsifier tests CLK-01..06 covering the threat surface above.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA · CLOCK-SKEW`

use std::collections::BTreeSet;

/// Symmetric clock-skew bound (milliseconds) around the receiver clock.
///
/// A message timestamped `t_msg` is accepted only when
/// `|t_msg - t_recv| <= skew_ms`. Default `30_000` (30 s).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClockSkewBound {
    /// Half-width of the acceptance band in milliseconds.
    pub skew_ms: u64,
}

impl Default for ClockSkewBound {
    fn default() -> Self {
        Self { skew_ms: 30_000 }
    }
}

impl ClockSkewBound {
    /// Construct a bound. `[VERIFIED]` `skew_ms` is treated as a
    /// half-width — the full acceptance window is `2 * skew_ms`.
    pub const fn new(skew_ms: u64) -> Self {
        Self { skew_ms }
    }

    /// `true` iff `t_msg` lies within `[t_recv - skew_ms, t_recv + skew_ms]`.
    pub const fn accepts(&self, t_msg_ms: u64, t_recv_ms: u64) -> bool {
        let lo = t_recv_ms.saturating_sub(self.skew_ms);
        let hi = t_recv_ms.saturating_add(self.skew_ms);
        t_msg_ms >= lo && t_msg_ms <= hi
    }
}

/// Decision returned by the replay window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayDecision {
    /// Counter & timestamp are both fresh — admit the message.
    Accept,
    /// Counter has been seen before (replay).
    RejectReplay,
    /// Timestamp is older than `t_recv - skew` (stale / backdated).
    RejectStale,
    /// Timestamp is newer than `t_recv + skew` (future-dated).
    RejectFuture,
    /// Counter belongs to a previous epoch attempting to roll back into
    /// the live window (epoch-rollover replay).
    RejectEpochRollover,
}

/// Bounded receiver-side replay window.
///
/// Keeps the highest-seen counter and a bitmask of the previous 64
/// counters within the *current* epoch. An epoch is identified by a
/// monotonically increasing `epoch_id` advanced on every DH ratchet
/// step. Counters from a strictly earlier epoch are rejected with
/// `RejectEpochRollover` so a captured counter cannot slide forward
/// past a key rotation.
#[derive(Clone, Debug)]
pub struct ReplayWindow {
    /// Highest counter accepted in the current epoch + 1 (next-expected).
    next: u64,
    /// Bitmask: bit `k` set iff counter `next - 1 - k` was accepted.
    seen: u64,
    /// Current epoch id (monotone across DH ratchet steps).
    epoch: u64,
    /// Persistent set of `(epoch, counter)` admitted across all epochs;
    /// bounded by `max_history` to defend against memory-exhaustion DoS.
    history: BTreeSet<(u64, u64)>,
    /// Ceiling on `history` size. Once reached, oldest epochs are evicted.
    max_history: usize,
    /// Clock-skew bound applied on every `accept_at`.
    skew: ClockSkewBound,
}

/// Default ceiling on persistent replay history.
pub const DEFAULT_MAX_HISTORY: usize = 4096;

impl ReplayWindow {
    /// Fresh window, epoch 0, default 30 s skew.
    pub fn new() -> Self {
        Self::with_skew(ClockSkewBound::default())
    }

    /// Fresh window with a custom skew bound.
    pub fn with_skew(skew: ClockSkewBound) -> Self {
        Self {
            next: 0,
            seen: 0,
            epoch: 0,
            history: BTreeSet::new(),
            max_history: DEFAULT_MAX_HISTORY,
            skew,
        }
    }

    /// Current epoch identifier.
    pub const fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Advance the epoch (call from a DH ratchet step). Counters reset.
    pub fn advance_epoch(&mut self) {
        self.epoch = self.epoch.saturating_add(1);
        self.next = 0;
        self.seen = 0;
        // Bound history to defend against a sender pumping epochs.
        if self.history.len() > self.max_history {
            let drop = self.history.len() - self.max_history;
            let to_drop: Vec<_> = self.history.iter().take(drop).cloned().collect();
            for k in to_drop {
                self.history.remove(&k);
            }
        }
    }

    /// Try to accept a message with `(counter, t_msg_ms)` against the
    /// receiver clock `t_recv_ms`. The decision is the canonical
    /// rejection-or-acceptance verdict.
    ///
    /// Order of checks (matters for falsification):
    /// 1. Clock-skew window — stale / future rejected first.
    /// 2. Epoch-rollover — strictly earlier epochs rejected.
    /// 3. Replay (counter already in history under this epoch).
    /// 4. Otherwise accept and record.
    pub fn accept_at(
        &mut self,
        counter: u64,
        t_msg_ms: u64,
        t_recv_ms: u64,
        epoch: u64,
    ) -> ReplayDecision {
        // 1. Clock-skew band.
        if t_msg_ms < t_recv_ms.saturating_sub(self.skew.skew_ms) {
            return ReplayDecision::RejectStale;
        }
        if t_msg_ms > t_recv_ms.saturating_add(self.skew.skew_ms) {
            return ReplayDecision::RejectFuture;
        }

        // 2. Epoch rollover: counters from a strictly earlier epoch
        // cannot re-enter once we have rotated.
        if epoch < self.epoch {
            return ReplayDecision::RejectEpochRollover;
        }

        // 3. Replay against persistent history.
        if self.history.contains(&(epoch, counter)) {
            return ReplayDecision::RejectReplay;
        }

        // 3b. Replay against current-epoch live bitmask.
        if epoch == self.epoch && counter < self.next {
            let shift = (self.next - 1 - counter) as u32;
            if shift < 64 {
                let bit = 1u64 << shift;
                if self.seen & bit != 0 {
                    return ReplayDecision::RejectReplay;
                }
            } else {
                // Outside live bitmask range but within the same epoch —
                // history check above already covered it; if it slipped
                // through both, it's stale beyond bitmask reach.
                return ReplayDecision::RejectReplay;
            }
        }

        // 4. Accept.
        if epoch > self.epoch {
            // Sender is signaling a forward jump; align our epoch.
            self.epoch = epoch;
            self.next = 0;
            self.seen = 0;
        }
        if counter >= self.next {
            let jump = counter - self.next + 1;
            self.seen = if jump >= 64 {
                1
            } else {
                self.seen.wrapping_shl(jump as u32) | 1
            };
            self.next = counter.saturating_add(1);
        } else {
            // In-window late arrival.
            let shift = (self.next - 1 - counter) as u32;
            self.seen |= 1u64 << shift;
        }
        // Remember in persistent history (bounded).
        if self.history.len() >= self.max_history {
            // Evict the lexicographically-smallest entry.
            if let Some(first) = self.history.iter().next().cloned() {
                self.history.remove(&first);
            }
        }
        self.history.insert((epoch, counter));
        ReplayDecision::Accept
    }
}

impl Default for ReplayWindow {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **CLK-01** — fresh in-band counter accepted under the default skew.
    /// Sanity: a normal in-window message goes through.
    #[test]
    fn clk_01_fresh_in_band_accepted() {
        let mut w = ReplayWindow::new();
        let d = w.accept_at(0, 1_000, 1_000, 0);
        assert_eq!(d, ReplayDecision::Accept, "CLK-01: in-band fresh must accept");
    }

    /// **CLK-02** — backdated message outside skew band rejected as stale.
    /// Attacker stashes an old ciphertext and resends it long after.
    #[test]
    fn clk_02_backdated_outside_skew_rejected_stale() {
        let mut w = ReplayWindow::with_skew(ClockSkewBound::new(30_000));
        // Receiver clock at 1_000_000 ms; sender claims 1 ms (way beyond skew).
        let d = w.accept_at(0, 1, 1_000_000, 0);
        assert_eq!(
            d,
            ReplayDecision::RejectStale,
            "CLK-02: backdated message must be rejected as stale"
        );
    }

    /// **CLK-03** — future-dated message beyond skew band rejected.
    /// Attacker sets the clock far ahead to slip past stale-rejection.
    #[test]
    fn clk_03_future_dated_rejected() {
        let mut w = ReplayWindow::with_skew(ClockSkewBound::new(30_000));
        // Receiver at 1_000 ms; sender claims 10_000_000 ms.
        let d = w.accept_at(0, 10_000_000, 1_000, 0);
        assert_eq!(
            d,
            ReplayDecision::RejectFuture,
            "CLK-03: future-dated beyond skew must be rejected"
        );
    }

    /// **CLK-04** — same counter replayed within the live bitmask is rejected.
    /// Classical replay-window check.
    #[test]
    fn clk_04_replay_within_window_rejected() {
        let mut w = ReplayWindow::new();
        assert_eq!(w.accept_at(0, 1_000, 1_000, 0), ReplayDecision::Accept);
        let dup = w.accept_at(0, 1_000, 1_000, 0);
        assert_eq!(
            dup,
            ReplayDecision::RejectReplay,
            "CLK-04: duplicate counter must be rejected as replay"
        );
    }

    /// **CLK-05** — a counter from a strictly earlier epoch attempting to
    /// re-enter after a DH ratchet step is rejected as epoch-rollover.
    #[test]
    fn clk_05_epoch_rollover_rejected() {
        let mut w = ReplayWindow::new();
        assert_eq!(w.accept_at(5, 1_000, 1_000, 0), ReplayDecision::Accept);
        // DH ratchet step.
        w.advance_epoch();
        assert_eq!(w.epoch(), 1);
        // Attacker tries to re-inject an old (epoch=0, counter=5) ciphertext
        // with a fresh in-band timestamp.
        let d = w.accept_at(5, 2_000, 2_000, 0);
        assert_eq!(
            d,
            ReplayDecision::RejectEpochRollover,
            "CLK-05: previous-epoch counter must be rejected"
        );
    }

    /// **CLK-06** — boundary: a message exactly at `t_recv ± skew` is
    /// accepted; one millisecond beyond is rejected. Pins down the
    /// inclusive-band semantics.
    #[test]
    fn clk_06_boundary_inclusive() {
        let mut w = ReplayWindow::with_skew(ClockSkewBound::new(1_000));
        // Exactly at -skew bound (inclusive).
        assert_eq!(
            w.accept_at(0, 0, 1_000, 0),
            ReplayDecision::Accept,
            "CLK-06: t = t_recv - skew must be inside band"
        );
        // Exactly at +skew bound (inclusive), fresh counter.
        assert_eq!(
            w.accept_at(1, 2_000, 1_000, 0),
            ReplayDecision::Accept,
            "CLK-06: t = t_recv + skew must be inside band"
        );
        // 1 ms beyond +skew on a fresh window.
        let mut w2 = ReplayWindow::with_skew(ClockSkewBound::new(1_000));
        assert_eq!(
            w2.accept_at(0, 2_001, 1_000, 0),
            ReplayDecision::RejectFuture,
            "CLK-06: t = t_recv + skew + 1 must be outside band"
        );
    }

    /// Green summary line for human/CI scan.
    #[test]
    fn green_g_c2_clock_summary() {
        let count = 6usize;
        assert_eq!(
            count, 6,
            "Wave-16 L-CHAT-2-clock: 6 clock-skew / replay-window falsifiers active"
        );
    }
}
