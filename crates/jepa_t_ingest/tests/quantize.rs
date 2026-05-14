//! Boundary tests for `quantize_phi_prior` — Wave-9b RTL byte-for-byte parity.
//!
//! Threshold: φ⁻² in Q1.15 = 12533 (0x30F4)
//!
//! These tests assert that the Rust implementation matches the Wave-9b
//! `phi_prior_quantizer.v` Verilog module for all documented boundary cases
//! and the full ±0x7FFF (i16::MAX / i16::MIN) range extremes.
//!
//! Apache-2.0 — Author: Dmitrii Vasilev <admin@t27.ai>

use jepa_t_ingest::quantize_phi_prior;

// ── Boundary: just below threshold ──────────────────────────────────────────

#[test]
fn boundary_plus_12532_is_zero() {
    // +12532 is strictly below threshold → ternary 0
    assert_eq!(
        quantize_phi_prior(12532),
        0,
        "+12532 must map to 0 (below threshold 12533)"
    );
}

#[test]
fn boundary_minus_12532_is_zero() {
    // -12532 is strictly above -threshold → ternary 0
    assert_eq!(
        quantize_phi_prior(-12532),
        0,
        "-12532 must map to 0 (above -threshold -12533)"
    );
}

// ── Boundary: at threshold ───────────────────────────────────────────────────

#[test]
fn boundary_plus_12533_is_positive_one() {
    // +12533 == threshold → ternary +1
    assert_eq!(
        quantize_phi_prior(12533),
        1,
        "+12533 (φ⁻² Q1.15) must map to +1"
    );
}

#[test]
fn boundary_minus_12533_is_negative_one() {
    // -12533 == -threshold → ternary -1
    assert_eq!(
        quantize_phi_prior(-12533),
        -1,
        "-12533 (−φ⁻² Q1.15) must map to -1"
    );
}

// ── Zero ─────────────────────────────────────────────────────────────────────

#[test]
fn zero_is_zero() {
    assert_eq!(quantize_phi_prior(0), 0, "0 must map to ternary 0");
}

// ── Extremes: ±0x7FFF (i16::MAX / i16::MIN) ──────────────────────────────────

#[test]
fn max_i16_is_positive_one() {
    // i16::MAX = 32767 = 0x7FFF >> threshold → +1
    assert_eq!(
        quantize_phi_prior(i16::MAX),
        1,
        "i16::MAX (0x7FFF = 32767) must map to +1"
    );
}

#[test]
fn min_i16_is_negative_one() {
    // i16::MIN = -32768 = -0x8000 < -threshold → -1
    assert_eq!(
        quantize_phi_prior(i16::MIN),
        -1,
        "i16::MIN (-0x8000 = -32768) must map to -1"
    );
}

// ── Additional parity checks near threshold ──────────────────────────────────

#[test]
fn one_above_threshold_is_positive_one() {
    assert_eq!(quantize_phi_prior(12534), 1);
}

#[test]
fn one_below_negative_threshold_is_negative_one() {
    assert_eq!(quantize_phi_prior(-12534), -1);
}

#[test]
fn mid_range_positive_is_zero() {
    // 6266 is well inside the dead zone
    assert_eq!(quantize_phi_prior(6266), 0);
}

#[test]
fn mid_range_negative_is_zero() {
    assert_eq!(quantize_phi_prior(-6266), 0);
}

// ── Output domain: only ternary values ───────────────────────────────────────

#[test]
fn output_always_ternary_for_all_i16() {
    // Exhaustive check of all 65536 possible i16 inputs.
    for raw in i16::MIN..=i16::MAX {
        let out = quantize_phi_prior(raw);
        assert!(
            out == -1 || out == 0 || out == 1,
            "quantize_phi_prior({}) = {} — not ternary!",
            raw,
            out
        );
    }
}
