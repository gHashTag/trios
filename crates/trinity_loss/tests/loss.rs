// SPDX-License-Identifier: Apache-2.0
// Trinity Loss — integration tests
// Author: Dmitrii Vasilev <admin@t27.ai>
//
// 10 deterministic hand-computed triplets (±1e-4 tolerance).
// 50 LFSR-random stability triplets (self-consistency, no Python dependency).

use trinity_loss::{dot_ternary, phi_prior_term, sim, trinity_loss, zero_count, DEFAULT_LAMBDA, DEFAULT_MARGIN};

const TOL: f32 = 1e-4;

// ─── helpers ────────────────────────────────────────────────────────────────

fn assert_close(actual: f32, expected: f32, label: &str) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= TOL,
        "{label}: got {actual:.7}, expected {expected:.7}, diff {diff:.2e}"
    );
}

/// Build a [i8; 64] from a repeated pattern slice (cycled to fill 64 slots).
fn from_pattern(pat: &[i8]) -> [i8; 64] {
    let mut out = [0i8; 64];
    for i in 0..64 {
        out[i] = pat[i % pat.len()];
    }
    out
}

// ─── Hand-computed triplets ──────────────────────────────────────────────────
//
// Reference computations (all verified against python_ref/trinity_loss_ref.py):
//   PHI_INV_SQ = 0.382
//   sim(a,b)   = dot_ternary(a,b) / 64
//   L_trip     = max(0, margin + sim(a,n) - sim(a,p))
//   L_phi      = 0.382 * (zeros_a + zeros_p + zeros_n) / 192
//   L_total    = L_trip + 0.1 * L_phi

/// T1: all-ones anchor, all-ones positive, all-(-1) negative.
///   sim(a,p)=1, sim(a,n)=-1 → L_trip=0, zeros=0 → L_total=0.0
#[test]
fn t01_perfect_triplet() {
    let a = [1i8; 64];
    let p = [1i8; 64];
    let n = [-1i8; 64];
    assert_close(trinity_loss(&a, &p, &n, DEFAULT_MARGIN, DEFAULT_LAMBDA), 0.0, "T01");
}

/// T2: all-zeros anchor, positive, negative.
///   sim(a,p)=0, sim(a,n)=0 → L_trip=max(0,0.5)=0.5
///   zeros=64+64+64=192 → L_phi=0.382
///   L_total=0.5+0.1*0.382=0.5382
#[test]
fn t02_all_zeros() {
    let a = [0i8; 64];
    let p = [0i8; 64];
    let n = [0i8; 64];
    assert_close(trinity_loss(&a, &p, &n, DEFAULT_MARGIN, DEFAULT_LAMBDA), 0.5382, "T02");
}

/// T3: a = [1,0,-1]*21+[1], p = a (identical), n = -a.
///   dot(a,a)=43 → sim(a,p)=43/64≈0.671875, sim(a,n)=-0.671875
///   L_trip=max(0,0.5-1.34375)=0
///   zeros_a=zeros_p=zeros_n=21, total=63 → L_phi=0.382*63/192≈0.125344
///   L_total=0+0.1*0.125344≈0.012534
#[test]
fn t03_mixed_pattern() {
    let pat: [i8; 3] = [1, 0, -1];
    let a = from_pattern(&pat);
    let p = a;
    let mut n = a;
    for x in n.iter_mut() { *x = x.saturating_neg(); }
    let expected = 0.0f32 + 0.1 * (0.382 * 63.0 / 192.0);
    assert_close(trinity_loss(&a, &p, &n, DEFAULT_MARGIN, DEFAULT_LAMBDA), expected, "T03");
}

/// T4: a=[1]*32+[-1]*32, p=[-1]*64, n=[1]*64.
///   sim(a,p)=0, sim(a,n)=0 → L_trip=0.5
///   zeros=0 → L_phi=0
///   L_total=0.5
#[test]
fn t04_orthogonal_both() {
    let mut a = [1i8; 64];
    for i in 32..64 { a[i] = -1; }
    let p = [-1i8; 64];
    let n = [1i8; 64];
    assert_close(trinity_loss(&a, &p, &n, DEFAULT_MARGIN, DEFAULT_LAMBDA), 0.5, "T04");
}

/// T5: a=[0]*32+[1]*32, p=[1]*32+[0]*32, n=[-1]*64.
///   dot(a,p)=0 → sim(a,p)=0
///   dot(a,n)=-32 → sim(a,n)=-0.5
///   L_trip=max(0,0.5-0.5-0)=0
///   zeros_a=32, zeros_p=32, zeros_n=0, total=64 → L_phi=0.382*64/192≈0.127333
///   L_total=0+0.1*0.127333≈0.012733
#[test]
fn t05_half_zeros_neg_close() {
    let mut a = [0i8; 64];
    for i in 32..64 { a[i] = 1; }
    let mut p = [1i8; 64];
    for i in 32..64 { p[i] = 0; }
    let n = [-1i8; 64];
    let expected = 0.1 * (0.382 * 64.0 / 192.0);
    assert_close(trinity_loss(&a, &p, &n, DEFAULT_MARGIN, DEFAULT_LAMBDA), expected, "T05");
}

/// T6: a=[1,-1]*32, p=a (identical), n=[-1,1]*32.
///   sim(a,p)=1, sim(a,n)=-1 → L_trip=0
///   zeros=0 → L_total=0
#[test]
fn t06_alternating_perfect() {
    let a = from_pattern(&[1i8, -1]);
    let p = a;
    let n = from_pattern(&[-1i8, 1]);
    assert_close(trinity_loss(&a, &p, &n, DEFAULT_MARGIN, DEFAULT_LAMBDA), 0.0, "T06");
}

/// T7: a=[0]*64, p=[1]*32+[-1]*32, n=[1]*32+[-1]*32 (p==n).
///   sim(a,p)=0, sim(a,n)=0 → L_trip=0.5
///   zeros_a=64, zeros_p=0, zeros_n=0, total=64 → L_phi=0.382*64/192≈0.127333
///   L_total=0.5+0.1*0.127333≈0.512733
#[test]
fn t07_zero_anchor_equal_pn() {
    let a = [0i8; 64];
    let mut p = [1i8; 64];
    for i in 32..64 { p[i] = -1; }
    let n = p;
    let expected = 0.5 + 0.1 * (0.382 * 64.0 / 192.0);
    assert_close(trinity_loss(&a, &p, &n, DEFAULT_MARGIN, DEFAULT_LAMBDA), expected, "T07");
}

/// T8: a=[1]*16+[0]*16+[-1]*16+[0]*16, p=a, n=[-1]*16+[0]*16+[1]*16+[0]*16.
///   dot(a,p)=32, sim(a,p)=0.5
///   dot(a,n)=-32, sim(a,n)=-0.5
///   L_trip=max(0,0.5-0.5-0.5)=0
///   zeros_a=zeros_p=zeros_n=32, total=96 → L_phi=0.382*96/192=0.191
///   L_total=0+0.1*0.191=0.0191
#[test]
fn t08_half_zeros_flipped_neg() {
    let mut a = [0i8; 64];
    for i in 0..16  { a[i] = 1;  }
    for i in 32..48 { a[i] = -1; }
    let p = a;
    let mut n = [0i8; 64];
    for i in 0..16  { n[i] = -1; }
    for i in 32..48 { n[i] = 1;  }
    let expected = 0.1 * (0.382 * 96.0 / 192.0);
    assert_close(trinity_loss(&a, &p, &n, DEFAULT_MARGIN, DEFAULT_LAMBDA), expected, "T08");
}

/// T9: a=[1]*64, p=[-1]*32+[0]*32, n=[1]*32+[0]*32.
///   sim(a,p)=-0.5, sim(a,n)=0.5
///   L_trip=max(0,0.5+0.5+0.5)=1.5
///   zeros_a=0, zeros_p=32, zeros_n=32, total=64 → L_phi=0.382*64/192≈0.127333
///   L_total=1.5+0.1*0.127333≈1.512733
#[test]
fn t09_worst_case_loss() {
    let a = [1i8; 64];
    let mut p = [-1i8; 64];
    for i in 32..64 { p[i] = 0; }
    let mut n = [1i8; 64];
    for i in 32..64 { n[i] = 0; }
    let expected = 1.5 + 0.1 * (0.382 * 64.0 / 192.0);
    assert_close(trinity_loss(&a, &p, &n, DEFAULT_MARGIN, DEFAULT_LAMBDA), expected, "T09");
}

/// T10: a=[0]*48+[1]*16, p=a, n=[0]*48+[-1]*16.
///   dot(a,p)=16, sim(a,p)=0.25
///   dot(a,n)=-16, sim(a,n)=-0.25
///   L_trip=max(0,0.5-0.25-0.25)=max(0,0)=0
///   zeros_a=zeros_p=zeros_n=48, total=144 → L_phi=0.382*144/192=0.2865
///   L_total=0+0.1*0.2865=0.028650
#[test]
fn t10_sparse_anchor() {
    let mut a = [0i8; 64];
    for i in 48..64 { a[i] = 1; }
    let p = a;
    let mut n = [0i8; 64];
    for i in 48..64 { n[i] = -1; }
    let expected = 0.1 * (0.382 * 144.0 / 192.0);
    assert_close(trinity_loss(&a, &p, &n, DEFAULT_MARGIN, DEFAULT_LAMBDA), expected, "T10");
}

// ─── LFSR-random stability tests ────────────────────────────────────────────
//
// 50 triplets generated by a 32-bit Galois LFSR (polynomial 0xB400_0000).
// Elements are mapped: 0→-1, 1→0, 2→1 (3-valued from 2 bits of LFSR output).
// Each test asserts that trinity_loss is non-negative and that calling it twice
// returns the same value (determinism check).

fn lfsr_next(state: &mut u32) -> u32 {
    let lsb = *state & 1;
    *state >>= 1;
    if lsb != 0 {
        *state ^= 0xB400_0000;
    }
    *state
}

fn lfsr_ternary_vec(state: &mut u32) -> [i8; 64] {
    let mut v = [0i8; 64];
    for slot in v.iter_mut() {
        let bits = (lfsr_next(state) & 3) % 3; // 0,1,2
        *slot = (bits as i8) - 1;               // -1,0,1
    }
    v
}

#[test]
fn lfsr_stability_50() {
    let mut state: u32 = 0xDEAD_BEEF;
    for trial in 0..50u32 {
        let a = lfsr_ternary_vec(&mut state);
        let p = lfsr_ternary_vec(&mut state);
        let n = lfsr_ternary_vec(&mut state);

        let loss1 = trinity_loss(&a, &p, &n, DEFAULT_MARGIN, DEFAULT_LAMBDA);
        let loss2 = trinity_loss(&a, &p, &n, DEFAULT_MARGIN, DEFAULT_LAMBDA);

        assert_eq!(
            loss1, loss2,
            "LFSR trial {trial}: trinity_loss is not deterministic"
        );
        assert!(
            loss1 >= 0.0,
            "LFSR trial {trial}: loss is negative ({loss1})"
        );
        assert!(
            loss1.is_finite(),
            "LFSR trial {trial}: loss is not finite ({loss1})"
        );
    }
}

// ─── Sub-function unit sanity checks ────────────────────────────────────────

#[test]
fn sub_dot_ternary_cross() {
    let a = [1i8; 64];
    let b = [-1i8; 64];
    assert_eq!(dot_ternary(&a, &b), -64);
}

#[test]
fn sub_sim_orthogonal() {
    let mut a = [1i8; 64];
    let mut b = [0i8; 64];
    // a = [1,-1]*32, b = [0,1]*32 → dot=0
    for i in (0..64).step_by(2) { a[i] = 1; a[i+1] = -1; b[i] = 0; b[i+1] = 1; }
    // dot = sum over 32 pairs of (1*0 + (-1)*1) = -1 per pair = -32, not 0
    // Use truly orthogonal: a=[1]*32+[0]*32, b=[0]*32+[1]*32
    let mut a2 = [0i8; 64];
    let mut b2 = [0i8; 64];
    for i in 0..32 { a2[i] = 1; }
    for i in 32..64 { b2[i] = 1; }
    assert!((sim(&a2, &b2) - 0.0).abs() < 1e-6);
}

#[test]
fn sub_zero_count_mixed() {
    let mut a = [0i8; 64];
    for i in 0..32 { a[i] = 1; }
    assert_eq!(zero_count(&a), 32);
}

#[test]
fn sub_phi_prior_all_nonzero() {
    let a = [1i8; 64];
    let p = [1i8; 64];
    let n = [1i8; 64];
    assert!((phi_prior_term(&a, &p, &n) - 0.0).abs() < 1e-6);
}

#[test]
fn sub_phi_prior_all_zero() {
    let a = [0i8; 64];
    let p = [0i8; 64];
    let n = [0i8; 64];
    // 0.382 * 192 / 192 = 0.382
    assert_close(phi_prior_term(&a, &p, &n), 0.382, "phi_prior_all_zero");
}
