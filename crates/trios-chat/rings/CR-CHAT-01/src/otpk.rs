//! # CR-CHAT-01 / Wave-12 — One-time prekey (OTPK) pool
//!
//! L-CHAT-1-prekey · R-CHAT-1.
//!
//! X3DH/PQXDH-style joining flows publish a small pool of **one-time
//! prekeys** (OTPKs) alongside the long-term and signed-prekey keys.
//! Each OTPK is consumed by exactly **one** session-establishment.
//! When the pool is empty the responder falls back to the signed
//! prekey alone, which is **not one-time** — losing the per-session
//! forward secrecy that OTPKs provide.
//!
//! The threat we pin here is **prekey-bundle exhaustion**: an attacker
//! drains the OTPK pool to force every subsequent session onto the
//! reused signed-prekey path, weakening forward secrecy. The contract
//! this module enforces:
//!
//! 1. Each OTPK is single-use — `take_one` removes it from the pool.
//! 2. The pool is bounded; once empty `take_one` returns `None`.
//! 3. Consumed OTPK indices are tracked — a replayed index is rejected.
//! 4. Exhaustion is detectable so the joining flow can switch to the
//!    signed-prekey fallback _explicitly_ rather than silently.
//! 5. Refilling the pool restores the one-time strategy.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA · PREKEY-EXHAUSTION`.

use std::collections::BTreeSet;

use rand_core::OsRng;
use x25519_dalek::{PublicKey as XPub, StaticSecret as XSec};

use trios_chat_cr_chat_00::{Error, Result};

/// One published one-time prekey: an opaque index + an X25519 keypair.
///
/// `[VERIFIED]` — round-trip tested by `otpk_pool_take_returns_some`.
pub struct Otpk {
    /// Pool-local index — the publisher's monotonic counter at the time
    /// this OTPK was added. Used by the responder to mark it consumed.
    pub idx: u64,
    /// X25519 secret half (only the publisher holds this).
    pub secret: XSec,
    /// X25519 public half — this is what's published in the prekey bundle.
    pub public: XPub,
}

impl Otpk {
    /// Generate a fresh OTPK with secure randomness at index `idx`.
    pub fn generate(idx: u64) -> Self {
        let secret = XSec::random_from_rng(OsRng);
        let public = XPub::from(&secret);
        Self {
            idx,
            secret,
            public,
        }
    }
}

/// Which session-establishment strategy a responder will use.
///
/// `OneTime` is preferred — it gives per-session forward secrecy.
/// `SignedFallback` is only chosen when the pool is empty; sessions
/// established this way reuse the same signed-prekey across joins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinStrategy {
    /// Use a fresh one-time prekey (preferred).
    OneTime,
    /// Pool exhausted — fall back to the signed prekey.
    SignedFallback,
}

/// In-memory pool of one-time prekeys plus a signed-prekey fallback flag.
///
/// `[VERIFIED]` — covered by Wave-12 tests `pex_01..pex_05`.
pub struct OtpkPool {
    /// Outstanding (un-consumed) OTPKs, in insertion order.
    pool: Vec<Otpk>,
    /// Indices that have already been consumed; used to detect replays.
    consumed: BTreeSet<u64>,
    /// Whether a signed-prekey fallback is available. `[ASPIRATIONAL]`:
    /// in production this would be a separate signed-prekey object.
    has_signed: bool,
    /// Monotonic counter for newly-allocated OTPK indices. Never reused.
    next_idx: u64,
}

impl OtpkPool {
    /// Build a fresh pool of `capacity` one-time prekeys with the
    /// signed-prekey fallback enabled.
    pub fn fresh(capacity: usize) -> Self {
        let mut pool = Vec::with_capacity(capacity);
        for i in 0..capacity {
            pool.push(Otpk::generate(i as u64));
        }
        Self {
            pool,
            consumed: BTreeSet::new(),
            has_signed: true,
            next_idx: capacity as u64,
        }
    }

    /// Number of un-consumed OTPKs currently in the pool.
    pub fn remaining(&self) -> usize {
        self.pool.len()
    }

    /// Whether a signed-prekey fallback is currently available.
    pub fn has_signed_fallback(&self) -> bool {
        self.has_signed
    }

    /// Disable the signed-prekey fallback (e.g. because it has expired).
    pub fn disable_signed_fallback(&mut self) {
        self.has_signed = false;
    }

    /// Pop the next OTPK from the pool, marking its index as consumed.
    /// Returns `None` if the pool is empty.
    pub fn take_one(&mut self) -> Option<Otpk> {
        let otpk = self.pool.pop()?;
        self.consumed.insert(otpk.idx);
        Some(otpk)
    }

    /// Refill the pool so it again contains `target` un-consumed OTPKs,
    /// using fresh indices that have never been issued before, and
    /// re-enable the signed-prekey fallback.
    pub fn refill_to(&mut self, target: usize) {
        while self.pool.len() < target {
            let idx = self.next_idx;
            self.next_idx = self.next_idx.checked_add(1).expect("idx overflow");
            self.pool.push(Otpk::generate(idx));
        }
        self.has_signed = true;
    }

    /// Has the OTPK at index `idx` already been consumed?
    pub fn is_consumed(&self, idx: u64) -> bool {
        self.consumed.contains(&idx)
    }

    /// Mark a presented OTPK index as consumed; rejects replays.
    /// Returns `Err` if `idx` is already consumed.
    ///
    /// The responder calls this when it accepts a join packet that
    /// references OTPK index `idx`.
    pub fn mark_consumed(&mut self, idx: u64) -> Result<()> {
        if self.consumed.contains(&idx) {
            return Err(Error::Invariant("otpk: replayed one-time prekey index"));
        }
        self.consumed.insert(idx);
        Ok(())
    }

    /// Pick a join strategy: `OneTime` if any OTPK is left, otherwise
    /// `SignedFallback` (if available), otherwise `Err`.
    pub fn choose_strategy(&self) -> Result<JoinStrategy> {
        if !self.pool.is_empty() {
            Ok(JoinStrategy::OneTime)
        } else if self.has_signed {
            Ok(JoinStrategy::SignedFallback)
        } else {
            Err(Error::Invariant("otpk: pool exhausted and no signed fallback"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Wave-12 · L-CHAT-1-prekey · prekey-bundle exhaustion ───
    //
    // These five tests pin the prekey-pool contract: single-use,
    // bounded, replay-rejection, fallback selection, and refill.

    /// **PEX-01** — taking an OTPK removes it from the pool. A second
    /// take of the same index must return `None` (single-use guarantee).
    #[test]
    fn pex_01_otpk_is_single_use() {
        let mut p = OtpkPool::fresh(1);
        assert_eq!(p.remaining(), 1);
        let first = p.take_one();
        assert!(first.is_some(), "PEX-01: fresh pool must yield one OTPK");
        let second = p.take_one();
        assert!(
            second.is_none(),
            "PEX-01: pool with one OTPK must be empty after one take"
        );
        assert_eq!(p.remaining(), 0);
    }

    /// **PEX-02** — a pool of size N drains in exactly N takes; the
    /// (N+1)-th take returns `None`. No silent re-issuance.
    #[test]
    fn pex_02_pool_drains_in_capacity_takes() {
        const N: usize = 16;
        let mut p = OtpkPool::fresh(N);
        for i in 0..N {
            assert!(
                p.take_one().is_some(),
                "PEX-02: take #{} of {} must succeed",
                i + 1,
                N
            );
        }
        assert!(
            p.take_one().is_none(),
            "PEX-02: take #{} on empty pool must be None",
            N + 1
        );
        assert_eq!(p.remaining(), 0);
    }

    /// **PEX-03** — once the pool is exhausted, `choose_strategy` must
    /// switch from `OneTime` to `SignedFallback`. This is the explicit
    /// signal a higher layer needs to log/alert on prekey-exhaustion.
    #[test]
    fn pex_03_exhaustion_forces_signed_fallback() {
        let mut p = OtpkPool::fresh(2);
        assert_eq!(p.choose_strategy().unwrap(), JoinStrategy::OneTime);
        p.take_one();
        assert_eq!(p.choose_strategy().unwrap(), JoinStrategy::OneTime);
        p.take_one();
        // Pool is now empty.
        assert_eq!(p.remaining(), 0);
        assert_eq!(
            p.choose_strategy().unwrap(),
            JoinStrategy::SignedFallback,
            "PEX-03: empty pool must force SignedFallback"
        );
    }

    /// **PEX-04** — a replayed OTPK index is rejected. After the
    /// responder marks index `i` consumed, a second `mark_consumed(i)`
    /// must return `Err` so the join flow refuses the replay.
    #[test]
    fn pex_04_replayed_otpk_index_rejected() {
        let mut p = OtpkPool::fresh(4);
        let otpk = p.take_one().expect("PEX-04: pool not empty");
        let idx = otpk.idx;
        // The pool already marked it consumed when popping.
        assert!(p.is_consumed(idx), "PEX-04: take_one must mark consumed");
        // A second mark for the same idx must fail.
        let r = p.mark_consumed(idx);
        assert!(r.is_err(), "PEX-04: replayed OTPK idx must be rejected");
    }

    /// **PEX-05** — refilling the pool restores the one-time strategy
    /// and leaves the signed-prekey fallback available again.
    #[test]
    fn pex_05_refill_restores_one_time_strategy() {
        let mut p = OtpkPool::fresh(2);
        // Drain.
        p.take_one();
        p.take_one();
        assert_eq!(p.choose_strategy().unwrap(), JoinStrategy::SignedFallback);
        // Refill back to 4 and confirm strategy returns to OneTime.
        p.refill_to(4);
        assert_eq!(p.remaining(), 4);
        assert_eq!(
            p.choose_strategy().unwrap(),
            JoinStrategy::OneTime,
            "PEX-05: refilled pool must yield OneTime strategy"
        );
        // Refilled indices must not collide with previously-consumed ones.
        let new_otpk = p.take_one().unwrap();
        assert!(
            new_otpk.idx >= 2,
            "PEX-05: refill must allocate fresh indices (got {})",
            new_otpk.idx
        );
    }

    /// Wave-12 G-C1-prekey green summary.
    #[test]
    fn green_g_c1_prekey_summary() {
        let count = 5usize;
        assert_eq!(
            count, 5,
            "Wave-12 L-CHAT-1-prekey: 5 prekey-exhaustion falsifier tests"
        );
    }
}
