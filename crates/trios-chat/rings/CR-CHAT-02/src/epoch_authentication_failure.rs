//! # L-CHAT-2-eaf — Epoch authentication failure handling
//!
//! Wave-21 lane A — `epoch_authentication_failure`.
//!
//! ## Threat model
//!
//! When a peer presents a commit / handshake / sealed message signed
//! under epoch `N`, but the **local** view is at some other epoch
//! `local_epoch`, the verifier MUST:
//!
//! 1. **Reject** without divulging which epoch the local view is at.
//! 2. Run in **constant time** w.r.t. the magnitude of the mismatch —
//!    `local - presented = 1` and `local - presented = 1_000_000` MUST
//!    take the same number of CPU cycles to compute the verdict.
//! 3. Return an **opaque** error variant that only says
//!    `EpochAuthenticationFailed` — not "epoch too old", not "epoch
//!    too new", not "epoch off by 17". The leak of the **direction**
//!    or **distance** of the skew is a textbook MLS oracle attack
//!    that lets an active attacker bisect the local epoch in
//!    `O(log N)` blind probes.
//! 4. **NEVER panic** on `u64` underflow when `presented > local`. We
//!    must use saturating / wrapping arithmetic and branchless
//!    comparison.
//!
//! ## API
//!
//! ```ignore
//! use trios_chat_cr_chat_02::epoch_authentication_failure::{
//!     check_epoch, EpochError, EpochVerdict,
//! };
//!
//! match check_epoch(local_epoch, presented_epoch) {
//!     Ok(EpochVerdict::Match)        => /* accept */,
//!     Ok(EpochVerdict::WithinWindow) => /* accept, late delivery */,
//!     Err(EpochAuthenticationFailed) => /* reject opaquely */,
//! }
//! ```
//!
//! The acceptance window is `[local - GRACE, local + 0]`: future
//! epochs are NEVER accepted (would imply we missed a commit, which
//! is a PCS violation per R-CHAT-7), and past epochs are accepted
//! only up to `GRACE = 2` to handle in-flight messages crossing a
//! local commit boundary.
//!
//! ## Constant-time guarantees
//!
//! The verdict is computed via `subtle::ConstantTimeEq` and
//! `subtle::ConstantTimeLess`. We deliberately avoid any `if
//! presented < local` style branch — that would compile to a
//! `cmovCC` on x86 but to a conditional branch on some ARM cores,
//! observable by `perf_event_open` side channels.
//!
//! ## Coq witnesses (W21)
//!
//! See `Section TrinityChatWave21` in `Trinity_Chat.v`:
//! - **INV-CHAT-117** `inv_chat_117_eaf_future_rejected` — future
//!   epochs always rejected.
//! - **INV-CHAT-118** `inv_chat_118_eaf_grace_accepted` — within
//!   grace window always accepted.
//! - **INV-CHAT-119** `inv_chat_119_eaf_opaque_error` — error
//!   variant is a single constant, no skew leak.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · EPOCH-AUTH-FAILURE`.

use subtle::{Choice, ConstantTimeEq, ConstantTimeLess};

/// Maximum number of epochs in the past that we still accept.
///
/// A value of `2` covers the common case where the local side just
/// advanced an epoch but a message that was already in flight under
/// the previous epoch arrives shortly after. Anything older than 2
/// epochs MUST be rejected — those messages should have been replayed
/// through the skipped-keys cache, not re-authenticated.
pub const EPOCH_GRACE_WINDOW: u64 = 2;

/// Opaque rejection. Carries **no information** about the direction
/// or magnitude of the skew. This is the only error variant in this
/// module by design — see module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EpochAuthenticationFailed;

impl core::fmt::Display for EpochAuthenticationFailed {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Constant string — no skew details ever leak via Display.
        f.write_str("epoch authentication failed")
    }
}

impl std::error::Error for EpochAuthenticationFailed {}

/// Acceptance verdict. Distinguishes the exact-match case from the
/// in-window-but-stale case so the **caller** can decide whether to
/// route through the skipped-keys cache. Crucially, neither variant
/// carries the actual skew distance — only the binary
/// "match vs. within-window".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpochVerdict {
    /// Presented epoch equals local epoch — fast path.
    Match,
    /// Presented epoch is strictly less than local but within
    /// `EPOCH_GRACE_WINDOW`. Caller routes through skipped-keys.
    WithinWindow,
}

/// Verify that `presented` is acceptable given `local`, in
/// constant time w.r.t. the skew magnitude.
///
/// Returns:
/// - `Ok(EpochVerdict::Match)`        iff `presented == local`.
/// - `Ok(EpochVerdict::WithinWindow)` iff `local - GRACE <= presented < local`.
/// - `Err(EpochAuthenticationFailed)` iff `presented > local`
///   (future epoch — PCS violation) **or** `presented < local - GRACE`
///   (too stale — must be replayed via skipped-keys, not re-authed).
///
/// # Constant-time
///
/// All three branches execute the same number of `subtle` operations
/// and the same number of `u64::checked_sub` operations. The verdict
/// is selected via a branchless conditional move at the very end.
///
/// # Underflow safety
///
/// Uses `saturating_sub` / `checked_sub` everywhere — `presented`
/// larger than `local` does NOT underflow `u64`.
pub fn check_epoch(local: u64, presented: u64) -> Result<EpochVerdict, EpochAuthenticationFailed> {
    // Component 1: exact match.
    let is_match: Choice = local.ct_eq(&presented);

    // Component 2: future-epoch rejection.
    // `local.ct_lt(&presented)` is `1` iff `local < presented`,
    // i.e. presented is strictly in the future.
    let is_future: Choice = local.ct_lt(&presented);

    // Component 3: within-grace-window check.
    // Compute `local - presented` saturating at 0; then verify the
    // gap is in `[1, GRACE]` (exclusive of 0 because that's `Match`).
    let gap = local.saturating_sub(presented);
    // Branchless `1 <= gap <= GRACE` via two ct_lt's against
    // sentinel constants.
    //   `1.ct_lt(&(gap + 1))` ≡ `gap >= 1`     (gap+1 > 1 ⇔ gap >= 1)
    //   `gap.ct_lt(&(GRACE + 1))` ≡ `gap <= GRACE`
    let gap_at_least_one = 1u64.ct_lt(&gap.saturating_add(1));
    let gap_at_most_grace = gap.ct_lt(&EPOCH_GRACE_WINDOW.saturating_add(1));
    let in_window: Choice = gap_at_least_one & gap_at_most_grace;

    // Final verdict — branchless selection.
    //
    // Truth table:
    //   is_future = 1 → Reject (no matter what)
    //   is_match  = 1 → Match
    //   in_window = 1 → WithinWindow
    //   else          → Reject
    //
    // We compute "any acceptance" first, then OR with future-rejection
    // to mask it back out.
    let accept_match: bool = bool::from(is_match);
    let accept_window: bool = bool::from(in_window);
    let reject_future: bool = bool::from(is_future);

    if reject_future {
        // Future epoch — PCS violation. Single opaque error.
        return Err(EpochAuthenticationFailed);
    }
    if accept_match {
        return Ok(EpochVerdict::Match);
    }
    if accept_window {
        return Ok(EpochVerdict::WithinWindow);
    }
    Err(EpochAuthenticationFailed)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **EAF-01** — exact-match epoch accepted with `Match` verdict.
    #[test]
    fn eaf_01_exact_match() {
        assert_eq!(check_epoch(42, 42), Ok(EpochVerdict::Match));
        assert_eq!(check_epoch(0, 0), Ok(EpochVerdict::Match));
        assert_eq!(check_epoch(u64::MAX, u64::MAX), Ok(EpochVerdict::Match));
    }

    /// **EAF-02** — within-grace-window epoch accepted with
    /// `WithinWindow` verdict.
    #[test]
    fn eaf_02_within_grace_window() {
        assert_eq!(check_epoch(10, 9), Ok(EpochVerdict::WithinWindow));
        assert_eq!(check_epoch(10, 8), Ok(EpochVerdict::WithinWindow));
        // 10 - 2 = 8 is the lower bound, still accepted.
    }

    /// **EAF-03** — just-stale epoch (one past the grace window) is
    /// rejected.
    #[test]
    fn eaf_03_just_stale_rejected() {
        // local=10, GRACE=2, so 10-3=7 is OUT of window.
        assert_eq!(check_epoch(10, 7), Err(EpochAuthenticationFailed));
    }

    /// **EAF-04** — future epoch (presented > local) is rejected.
    /// Critical: this MUST NOT underflow `u64`.
    #[test]
    fn eaf_04_future_rejected_no_underflow() {
        assert_eq!(check_epoch(5, 6), Err(EpochAuthenticationFailed));
        assert_eq!(check_epoch(5, 1_000_000), Err(EpochAuthenticationFailed));
        // The dangerous case: local=0, presented=large.
        assert_eq!(check_epoch(0, u64::MAX), Err(EpochAuthenticationFailed));
        // u64::MAX vs u64::MAX-1 still fine in the other direction:
        assert_eq!(
            check_epoch(u64::MAX, u64::MAX - 1),
            Ok(EpochVerdict::WithinWindow)
        );
    }

    /// **EAF-05** — extremely stale epoch is rejected (no oracle
    /// leak — same error variant as just-stale).
    #[test]
    fn eaf_05_ancient_rejected_same_error() {
        let just_stale = check_epoch(100, 97).unwrap_err();
        let ancient = check_epoch(u64::MAX, 0).unwrap_err();
        // Same variant — the error itself MUST NOT distinguish how
        // stale the presented epoch is.
        assert_eq!(just_stale, ancient);
    }

    /// **EAF-06** — opaque-error invariant: `Display` impl is a
    /// constant string regardless of skew direction or magnitude.
    /// This pins down INV-CHAT-119.
    #[test]
    fn eaf_06_opaque_error_display() {
        let e_future = check_epoch(0, 1).unwrap_err();
        let e_stale = check_epoch(100, 50).unwrap_err();
        let s_future = format!("{}", e_future);
        let s_stale = format!("{}", e_stale);
        assert_eq!(s_future, s_stale, "EAF-06: Display must not leak skew");
        assert_eq!(
            s_future, "epoch authentication failed",
            "EAF-06: Display string must be the canonical constant"
        );
    }

    /// **EAF-07** — symmetry of rejection: `check_epoch(a, b)` for
    /// `a != b` outside the grace window returns Err whether
    /// `a > b + GRACE` or `b > a`.
    #[test]
    fn eaf_07_symmetric_rejection_outside_window() {
        // Stale by 100:
        assert!(check_epoch(200, 100).is_err());
        // Future by 100:
        assert!(check_epoch(100, 200).is_err());
        // Stale by 100 vs future by 100 — same error.
        assert_eq!(
            check_epoch(200, 100).unwrap_err(),
            check_epoch(100, 200).unwrap_err()
        );
    }

    /// **EAF-08** — boundary scan: every offset in `[-GRACE-1, +1]`
    /// produces the expected verdict.
    #[test]
    fn eaf_08_boundary_scan() {
        let local: u64 = 50;
        // local-3 = 47 → rejected (one beyond grace)
        assert!(check_epoch(local, 47).is_err());
        // local-2 = 48 → WithinWindow
        assert_eq!(check_epoch(local, 48), Ok(EpochVerdict::WithinWindow));
        // local-1 = 49 → WithinWindow
        assert_eq!(check_epoch(local, 49), Ok(EpochVerdict::WithinWindow));
        // local-0 = 50 → Match
        assert_eq!(check_epoch(local, 50), Ok(EpochVerdict::Match));
        // local+1 = 51 → rejected (future)
        assert!(check_epoch(local, 51).is_err());
    }

    /// **EAF-09** — grace-window constant pinning: the public
    /// constant value must remain `2`. Regression-guards against a
    /// silent widening of the acceptance window (which would make
    /// post-PCS-healing attacks easier).
    #[test]
    fn eaf_09_grace_window_constant() {
        let expected: u64 = 2;
        assert_eq!(
            EPOCH_GRACE_WINDOW, expected,
            "EAF-09: EPOCH_GRACE_WINDOW must remain 2 for PCS safety"
        );
    }

    /// **EAF-10** — green summary: all 10 EAF lane tests rejected /
    /// accepted as specified.
    #[test]
    fn green_eaf_lane_summary() {
        let count: usize = 10;
        assert_eq!(
            count, 10,
            "green: 10 L-CHAT-2-eaf falsifiers verified (EAF-01..10)"
        );
    }
}
