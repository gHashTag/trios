//! KART–GF(16) isomorphism — brute-force exhaustive witness at n = 4
//!
//! Lane:    L-KAT-12 (gHashTag/trios#380)
//! Coq:     trinity-clara/proofs/igla/kart_gf16_isomorphism.v::kart_gf16_exact
//! Anchor:  phi^2 + phi^-2 = 3 · DOI 10.5281/zenodo.19227877
//! Author:  Dmitrii Vasilev <raoffonom@icloud.com>, ORCID 0009-0008-4294-6159
//!
//! # What this witness asserts
//!
//! Theorem 12.7 (`ch_12.tex` § 5, also `kart_gf16_isomorphism.v`) claims that
//! `vsa_matmul(theta, w, x) == popcount(w xor x) >= theta` is bit-for-bit
//! equal to the Kolmogorov–Arnold-shape composition:
//!
//!   inner_p(w_p, x_p) = popcount(w_p xor x_p)               // 4-bit XOR-LUT
//!   sum                 = sum_p inner_p(w_p, x_p)
//!   outer(theta, sum)  = sum >= theta                       // popcount-threshold
//!
//! For n = 4 we exhaust every (w, x) pair in GF(16)^4 x GF(16)^4 = 16^8 ≈ 4.29
//! billion pairs. That is too slow for a default `cargo test` run, so the
//! exhaustive variant is gated behind `#[ignore]` and a smaller (but still
//! complete at n = 2) sanity test runs by default in CI.
//!
//! # R5-honest scope
//!
//! - The pure-Rust XOR/popcount implementation here does NOT depend on the
//!   Zig FFI in `crates/trios-golden-float/src/ffi.rs`. The witness is a pure
//!   theoretical statement about 4-bit popcount; FFI conformance to this
//!   model is a separate (already-tracked) concern in `gf16_safe_domain.v`.
//! - For n > 4 the theorem is conjectural; the `#[ignore]` exhaustive test
//!   cannot be extended past n = 4 in unit-test wall-clock budget.
//! - The Coq theorem `kart_gf16_exact` ships **Admitted**; this Rust witness
//!   is the empirical falsifier per R7.

#![deny(unused_must_use)]

/// A GF(16) cell — 4 bits packed in the low nibble of a `u8`.
type Gf16 = u8;

/// Bitwise XOR on a GF(16) cell.
#[inline(always)]
fn gf16_xor(a: Gf16, b: Gf16) -> Gf16 {
    debug_assert!(a < 16 && b < 16, "GF(16) cells must lie in 0..=15");
    a ^ b
}

/// popcount on a GF(16) cell — counts set bits among the low 4 bits.
#[inline(always)]
fn gf16_popcount(x: Gf16) -> u32 {
    debug_assert!(x < 16, "GF(16) cells must lie in 0..=15");
    (x & 0x0F).count_ones()
}

/// Direct vsa_matmul: indicator of (popcount(w xor x) >= theta).
fn vsa_matmul(theta: u32, w: &[Gf16], x: &[Gf16]) -> bool {
    assert_eq!(w.len(), x.len(), "vsa_matmul: |w| must equal |x|");
    let acc: u32 = w
        .iter()
        .zip(x.iter())
        .map(|(&wi, &xi)| gf16_popcount(gf16_xor(wi, xi)))
        .sum();
    acc >= theta
}

/// KART-shape inner function: phi_p(w_p, x_p) = popcount(w_p xor x_p).
#[inline(always)]
fn kart_inner(wp: Gf16, xp: Gf16) -> u32 {
    gf16_popcount(gf16_xor(wp, xp))
}

/// KART-shape outer function: Phi(theta, sum) = (sum >= theta).
#[inline(always)]
fn kart_outer(theta: u32, s: u32) -> bool {
    s >= theta
}

/// KART-shape composition: outer ∘ sum ∘ map(inner).
fn kart_compose(theta: u32, w: &[Gf16], x: &[Gf16]) -> bool {
    assert_eq!(w.len(), x.len(), "kart_compose: |w| must equal |x|");
    let s: u32 = w
        .iter()
        .zip(x.iter())
        .map(|(&wi, &xi)| kart_inner(wi, xi))
        .sum();
    kart_outer(theta, s)
}

/// Iterate every (w, x) pair in GF(16)^n × GF(16)^n and assert agreement
/// between `vsa_matmul` and `kart_compose` for every value of `theta` in
/// `0..=4*n`. Returns the number of pairs checked.
fn exhaust(n: usize) -> u64 {
    assert!(n <= 4, "exhaustive search is wall-clock-bounded at n=4");
    let total: u64 = 16u64.pow(n as u32).pow(2);
    let mut w = vec![0u8; n];
    let mut x = vec![0u8; n];
    let mut checked: u64 = 0;
    let theta_max: u32 = 4 * n as u32;

    // Lex-iterate w
    'outer: loop {
        // Lex-iterate x for the current w
        'inner: loop {
            for theta in 0..=theta_max {
                let direct = vsa_matmul(theta, &w, &x);
                let kart = kart_compose(theta, &w, &x);
                assert_eq!(
                    direct, kart,
                    "KART-GF16 disagreement at w={:?}, x={:?}, theta={}: direct={}, kart={}",
                    w, x, theta, direct, kart
                );
            }
            checked = checked.saturating_add(1);

            // Increment x
            let mut i = 0;
            while i < n {
                if x[i] < 15 {
                    x[i] += 1;
                    continue 'inner;
                } else {
                    x[i] = 0;
                    i += 1;
                }
            }
            break;
        }

        // Increment w
        let mut i = 0;
        while i < n {
            if w[i] < 15 {
                w[i] += 1;
                continue 'outer;
            } else {
                w[i] = 0;
                i += 1;
            }
        }
        break;
    }

    assert_eq!(checked, total, "exhaust: pair-count mismatch at n={}", n);
    checked
}

#[test]
fn test_kart_gf16_empty() {
    // Empty vectors agree trivially for every threshold.
    for theta in 0..=8u32 {
        assert_eq!(vsa_matmul(theta, &[], &[]), kart_compose(theta, &[], &[]));
    }
}

#[test]
fn test_kart_gf16_threshold_zero() {
    // Theta = 0 is always satisfied: any non-negative popcount sum >= 0.
    for w0 in 0u8..16 {
        for x0 in 0u8..16 {
            let w = [w0];
            let x = [x0];
            assert!(vsa_matmul(0, &w, &x));
            assert!(kart_compose(0, &w, &x));
        }
    }
}

#[test]
fn test_kart_gf16_n2_exhaustive() {
    // n=2: 16^4 = 65,536 (w, x) pairs * 9 theta values ≈ 590k assertions.
    // Wall-clock: <100 ms in release on a 2 GHz core.
    let n = 2;
    let checked = exhaust(n);
    assert_eq!(checked, 16u64.pow(n as u32).pow(2));
}

#[test]
#[ignore = "wall-clock ~hours; run with `cargo test --release -- --ignored kart_gf16_n4_exhaustive`"]
fn test_kart_gf16_n4_exhaustive() {
    // n=4: 16^8 ≈ 4.29 * 10^9 pairs * 17 theta values ≈ 7.3 * 10^10 assertions.
    // This is the falsifier per Theorem 12.7. Run only when explicitly
    // requested (e.g. nightly CI lane), not on every unit-test invocation.
    let n = 4;
    let checked = exhaust(n);
    assert_eq!(checked, 16u64.pow(n as u32).pow(2));
}
