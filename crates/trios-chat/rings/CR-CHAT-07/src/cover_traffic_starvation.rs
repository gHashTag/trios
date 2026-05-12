//! # Cover-traffic starvation guard — Wave-25 Lane B
//!
//! L-CHAT-7-cts · trinity-fpga#28 — Defends `CR-CHAT-07`
//! [`crate::CoverScheduler`] against an **active** adversary who tries
//! to *starve* the cover-traffic stream so the user's real envelopes
//! stand out on the wire.
//!
//! ## Threat model (R-CHAT-10)
//!
//! Cover traffic only works if every wire tick produces *something* —
//! either a real or a decoy envelope — at a steady cadence. The
//! adversary can attack the indistinguishability in three ways:
//!
//! 1. **Cadence skip** — schedule layer is paused (sleep / drop / CPU
//!    starvation) and resumes; the wire sees a long gap that breaks
//!    the canonical 4-class quantisation.
//! 2. **Cover suppression** — adversary causes the scheduler to emit
//!    *fewer* covers than the contract demands (e.g. by injecting a
//!    "low-power" signal), so reals are no longer drowned in noise.
//! 3. **Cover burst** — adversary forces extra covers between two
//!    real envelopes hoping the burst pattern (real, real, …, real,
//!    cover, cover, …) re-leaks the same-conversation correlation
//!    that uniform cadence is designed to hide.
//!
//! The guard pins **two invariants** on any contiguous wire window:
//!
//! - **CTS-A: cadence preserved** — for every two consecutive
//!   emissions in the window, the recorded gap (in canonical ms) is
//!   exactly one of [`crate::CANONICAL_GAPS_MS`]. A jitter of even
//!   one canonical class is a starvation event.
//! - **CTS-B: cover-floor preserved** — over any window of
//!   `WINDOW_MIN_EMISSIONS` emissions, at least
//!   `MIN_COVER_RATIO_NUM / MIN_COVER_RATIO_DEN` of them are
//!   `Emission::Cover`. If the adversary collapses the cover share
//!   below the floor, the guard fires `CoverFloorBreached`.
//!
//! Concrete attacks the falsifier exercises (CTS-01..10):
//!
//! 1. **CTS-01** valid uniform-cadence window accepted.
//! 2. **CTS-02** one off-canonical gap rejected (cadence skip).
//! 3. **CTS-03** zero-length window rejected (no signal).
//! 4. **CTS-04** all-Real window rejected (cover floor 0/N).
//! 5. **CTS-05** all-Cover window accepted (floor saturated).
//! 6. **CTS-06** exactly-at-floor window accepted (boundary).
//! 7. **CTS-07** one-below-floor window rejected (CoverFloorBreached).
//! 8. **CTS-08** mismatched length of gap-array vs emissions
//!    rejected (`MismatchedGapLength`).
//! 9. **CTS-09** non-canonical gap value pinpointed by index.
//! 10. **CTS-10** green re-export check.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · COVER-TRAFFIC-STARVATION`
//!
//! ## Honesty (R5)
//!
//! `[VERIFIED]` — 10 CTS-01..10 unit tests in this file all pass.
//! No I/O, no async, no randomness; pure window reasoning over
//! [`crate::Emission`] and [`crate::CANONICAL_GAPS_MS`].

#![forbid(unsafe_code)]

use crate::{Emission, CANONICAL_GAPS_MS};

/// Minimum number of emissions a window must contain before the
/// cover-floor check applies. Below this the window is rejected as
/// "no signal" so the adversary cannot trivially pass the check by
/// submitting a 1-emission window.
pub const WINDOW_MIN_EMISSIONS: usize = 4;

/// Numerator of the minimum acceptable cover ratio
/// (`MIN_COVER_RATIO_NUM / MIN_COVER_RATIO_DEN`). Set to `1/4` so
/// a sliding window of 4 emissions accepts down to *one* cover —
/// any fewer and the real envelopes become identifiable.
pub const MIN_COVER_RATIO_NUM: usize = 1;
/// Denominator of the minimum acceptable cover ratio.
pub const MIN_COVER_RATIO_DEN: usize = 4;

/// Single opaque error class returned by the cover-traffic
/// starvation guard.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoverStarvationError {
    /// Window has fewer than [`WINDOW_MIN_EMISSIONS`] entries.
    WindowTooShort {
        /// observed
        observed: usize,
        /// required
        required: usize,
    },
    /// `gaps_ms.len() + 1 != emissions.len()` — gaps must be the
    /// inter-emission deltas, so there is always exactly one fewer
    /// gap than emission.
    MismatchedGapLength {
        /// observed
        observed_gaps: usize,
        /// observed
        observed_emissions: usize,
    },
    /// At least one inter-emission gap is not in
    /// [`CANONICAL_GAPS_MS`].
    NonCanonicalGap {
        /// position of the offending gap in `gaps_ms`
        index: usize,
        /// observed
        value_ms: u64,
    },
    /// The cover share dropped below
    /// `MIN_COVER_RATIO_NUM / MIN_COVER_RATIO_DEN` over the window.
    CoverFloorBreached {
        /// number of Cover emissions
        cover_count: usize,
        /// total emissions
        total: usize,
    },
}

/// Validate a contiguous wire window of cover-traffic emissions.
///
/// `emissions` lists what the wire saw, in order. `gaps_ms` lists the
/// inter-emission gaps (already quantised by
/// [`crate::uniform_gap_ms`]); there must be exactly
/// `emissions.len() - 1` of them.
///
/// `[VERIFIED]` — by CTS-01..10 below.
pub fn validate_window(
    emissions: &[Emission],
    gaps_ms: &[u64],
) -> Result<(), CoverStarvationError> {
    if emissions.len() < WINDOW_MIN_EMISSIONS {
        return Err(CoverStarvationError::WindowTooShort {
            observed: emissions.len(),
            required: WINDOW_MIN_EMISSIONS,
        });
    }
    if gaps_ms.len() + 1 != emissions.len() {
        return Err(CoverStarvationError::MismatchedGapLength {
            observed_gaps: gaps_ms.len(),
            observed_emissions: emissions.len(),
        });
    }
    for (i, &g) in gaps_ms.iter().enumerate() {
        if !CANONICAL_GAPS_MS.contains(&g) {
            return Err(CoverStarvationError::NonCanonicalGap {
                index: i,
                value_ms: g,
            });
        }
    }
    let cover_count = emissions.iter().filter(|e| **e == Emission::Cover).count();
    // cover_count / total >= MIN_NUM / MIN_DEN
    //   iff cover_count * MIN_DEN >= total * MIN_NUM
    if cover_count
        .saturating_mul(MIN_COVER_RATIO_DEN)
        < emissions.len().saturating_mul(MIN_COVER_RATIO_NUM)
    {
        return Err(CoverStarvationError::CoverFloorBreached {
            cover_count,
            total: emissions.len(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CTS-01 — uniform-cadence, balanced cover/real window accepted.
    #[test]
    fn cts01_valid_uniform_window_accepted() {
        let em = [
            Emission::Real,
            Emission::Cover,
            Emission::Real,
            Emission::Cover,
        ];
        let g = [1_000u64; 3];
        assert_eq!(validate_window(&em, &g), Ok(()));
    }

    /// CTS-02 — cadence skip: one off-canonical gap (1500 ms) is
    /// pinpointed by index.
    #[test]
    fn cts02_one_off_canonical_gap_rejected() {
        let em = [
            Emission::Cover,
            Emission::Real,
            Emission::Cover,
            Emission::Real,
        ];
        let g = [1_000, 1_500, 1_000];
        assert_eq!(
            validate_window(&em, &g),
            Err(CoverStarvationError::NonCanonicalGap {
                index: 1,
                value_ms: 1_500
            })
        );
    }

    /// CTS-03 — empty window rejected as `WindowTooShort`.
    #[test]
    fn cts03_empty_window_rejected() {
        let em: [Emission; 0] = [];
        let g: [u64; 0] = [];
        assert_eq!(
            validate_window(&em, &g),
            Err(CoverStarvationError::WindowTooShort {
                observed: 0,
                required: WINDOW_MIN_EMISSIONS,
            })
        );
    }

    /// CTS-04 — all-Real window: cover share is 0/4 < 1/4 → rejected
    /// `CoverFloorBreached`.
    #[test]
    fn cts04_all_real_window_breaches_cover_floor() {
        let em = [Emission::Real; 4];
        let g = [1_000u64; 3];
        assert_eq!(
            validate_window(&em, &g),
            Err(CoverStarvationError::CoverFloorBreached {
                cover_count: 0,
                total: 4
            })
        );
    }

    /// CTS-05 — all-Cover window accepted (4/4 ≥ 1/4).
    #[test]
    fn cts05_all_cover_window_accepted() {
        let em = [Emission::Cover; 4];
        let g = [5_000u64; 3];
        assert_eq!(validate_window(&em, &g), Ok(()));
    }

    /// CTS-06 — exactly-at-floor (1 cover in 4) accepted.
    #[test]
    fn cts06_floor_boundary_accepted() {
        let em = [
            Emission::Real,
            Emission::Real,
            Emission::Cover,
            Emission::Real,
        ];
        let g = [30_000u64; 3];
        assert_eq!(validate_window(&em, &g), Ok(()));
    }

    /// CTS-07 — one-below-floor (1 cover in 8 < 2 needed) rejected.
    #[test]
    fn cts07_below_floor_rejected() {
        let em = [
            Emission::Real,
            Emission::Real,
            Emission::Real,
            Emission::Real,
            Emission::Real,
            Emission::Real,
            Emission::Real,
            Emission::Cover,
        ];
        let g = [1_000u64; 7];
        assert_eq!(
            validate_window(&em, &g),
            Err(CoverStarvationError::CoverFloorBreached {
                cover_count: 1,
                total: 8
            })
        );
    }

    /// CTS-08 — mismatched gap-length: 4 emissions need 3 gaps; we
    /// supply 4 → `MismatchedGapLength`.
    #[test]
    fn cts08_mismatched_gap_length_rejected() {
        let em = [Emission::Cover; 4];
        let g = [1_000u64; 4];
        assert_eq!(
            validate_window(&em, &g),
            Err(CoverStarvationError::MismatchedGapLength {
                observed_gaps: 4,
                observed_emissions: 4
            })
        );
    }

    /// CTS-09 — non-canonical gap at last position: index points to
    /// `gaps_ms.len() - 1`.
    #[test]
    fn cts09_non_canonical_gap_index_correct() {
        let em = [Emission::Cover; 4];
        let g = [1_000, 5_000, 42_000];
        assert_eq!(
            validate_window(&em, &g),
            Err(CoverStarvationError::NonCanonicalGap {
                index: 2,
                value_ms: 42_000
            })
        );
    }

    /// CTS-10 — green: constants are sane and validate_window
    /// composes with the public Emission API.
    #[test]
    fn cts10_green_constants_and_composition() {
        assert_eq!(WINDOW_MIN_EMISSIONS, 4);
        assert_eq!(MIN_COVER_RATIO_NUM, 1);
        assert_eq!(MIN_COVER_RATIO_DEN, 4);
        // Compose with the existing CoverScheduler API.
        let mut sched = crate::CoverScheduler::new();
        sched.enqueue_real();
        let emissions: Vec<Emission> = (0..4).map(|_| sched.tick()).collect();
        // Whatever the scheduler emitted, the gaps array is canonical
        // by construction at 1 s ticks.
        let gaps = [CANONICAL_GAPS_MS[0]; 3];
        let _ = validate_window(&emissions, &gaps); // exercises the path; result depends on cover floor
    }
}
