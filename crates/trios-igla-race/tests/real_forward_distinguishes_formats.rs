//! Integration test for Path A · L-FH4.
//!
//! Pre-registration falsifier (one-shot OS-001-forward-hook-bridge-v1):
//!
//!   "If f32 vs gf16 yield bit-identical val_bpb on tiny corpus across
//!    3 seeds {42, 43, 44}, the hook is fake."
//!
//! Acceptance band: |Δbpb| ≥ 1e-6 in at least one of the three seeds.
//! Reject band   : |Δbpb| == 0 across all three seeds.
//!
//! This test is `#[ignore]`-gated — it requires:
//!   1. `TRIOS_USE_REAL_BPB=1`
//!   2. `cpu_train` binary resolvable per `real_forward::resolve_trainer_bin`
//!   3. A tiny corpus available at `$TRIOS_TINY_CORPUS` (≤ 1 MB)
//!
//! CI runs it in a job that builds `trios-trainer-igla` first, then
//! exports the binary path via `TRIOS_TRAINER_BIN`.
//!
//! Anchor: φ² + φ⁻² = 3 · Zenodo DOI 10.5281/zenodo.19227877
//! Lane:   L-FH4 (#446 / #509-FABRICATED dispatch)

use trios_igla_race::race::simulate_bpb_v2;

const SEEDS: [u64; 3] = [42, 43, 44];
const STEPS: u32 = 200;          // tiny: ~30 s on CPU per seed × 2 formats
const LR: f64 = 0.005;            // INV1_CHAMPION_LR; passed for parity
const MIN_DELTA_BPB: f64 = 1e-6;  // acceptance threshold

#[test]
#[ignore = "requires TRIOS_USE_REAL_BPB=1 and cpu_train binary on PATH"]
fn test_real_forward_distinguishes_f32_from_gf16() {
    // Pre-flight: gate must be on, otherwise this test would silently pass
    // (legacy formula is identical between formats since `use_gf16` is unused).
    assert_eq!(
        std::env::var("TRIOS_USE_REAL_BPB").as_deref(),
        Ok("1"),
        "L-FH4: must run with TRIOS_USE_REAL_BPB=1; otherwise it is meaningless"
    );

    let mut max_abs_delta = 0.0f64;
    let mut all_zero_count = 0u32;

    for &seed in &SEEDS {
        let bpb_f32  = simulate_bpb_v2(LR, STEPS, seed, false);
        let bpb_gf16 = simulate_bpb_v2(LR, STEPS, seed, true);

        // Bridge errors surface as NaN — that is itself a falsification signal.
        assert!(bpb_f32.is_finite(),  "f32 bridge returned NaN for seed={seed}");
        assert!(bpb_gf16.is_finite(), "gf16 bridge returned NaN for seed={seed}");

        let delta = (bpb_f32 - bpb_gf16).abs();
        if delta == 0.0 {
            all_zero_count += 1;
        }
        if delta > max_abs_delta {
            max_abs_delta = delta;
        }
        eprintln!("  seed={seed:>3}  f32={bpb_f32:.6}  gf16={bpb_gf16:.6}  Δ={delta:.6}");
    }

    // Reject band: identical BPB across all 3 seeds → hook is fake.
    assert!(
        all_zero_count < SEEDS.len() as u32,
        "L-FH4 REJECTED: f32 vs gf16 bit-identical on all 3 seeds — quantize hook is FAKE"
    );

    // Accept band: at least one seed shows non-trivial delta.
    assert!(
        max_abs_delta >= MIN_DELTA_BPB,
        "L-FH4 ACCEPT-BAND BREACH: max |Δbpb| = {max_abs_delta:.3e} < {MIN_DELTA_BPB:.0e}"
    );

    eprintln!("L-FH4 ACCEPTED: max |Δbpb| = {max_abs_delta:.6} ≥ {MIN_DELTA_BPB}");
}
