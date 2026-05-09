//! GPTQ Calibration Ablation — Wave 26 L-GPTQ-ON-GF16
//!
//! Runs a 3-seed × N∈{0,16,32} grid ablation to test whether Hessian-correction
//! calibration improves GF16 reconstruction quality.
//!
//! Falsifier H0: GPTQ-correction with N∈{16,32} calibration batches gives
//! no significant BPB improvement over naive GF16 quantisation
//! (paired-t one-tail p ≥ 0.25 across seeds {47, 89, 144}).
//!
//! NOTE: "bpb_proxy" is a synthetic proxy (log2(MSE+1)) computed on held-out
//! validation data, NOT real language-model BPB. It is clearly labelled as such.
//!
//! Anchor: phi^2 + phi^-2 = 3  · DOI 10.5281/zenodo.19227877

use std::fs::OpenOptions;
use std::io::Write;
use std::process::Command;
use std::time::Instant;

use trios_golden_float::gptq::{
    f16_bits_to_f32, gf16_quantize_matrix_gptq,
};

// ---------------------------------------------------------------------------
// Ablation configuration
// ---------------------------------------------------------------------------

const SEEDS: [u64; 3] = [47, 89, 144];
const CALIB_NS: [usize; 3] = [0, 16, 32];

// Matrix dimensions (synthetic)
const ROWS: usize = 128;
const COLS: usize = 128;
const BATCH_SIZE: usize = 32; // samples per calibration batch
const VAL_SAMPLES: usize = 256;

// ---------------------------------------------------------------------------
// Minimal deterministic RNG (LCG, no external crate)
// ---------------------------------------------------------------------------

struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Lcg(seed ^ 0xdeadbeef_cafebabe)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }
    fn next_gaussian(&mut self) -> f32 {
        let u1 = (self.next_u64() >> 11) as f32 / (1u64 << 53) as f32 + 1e-10;
        let u2 = (self.next_u64() >> 11) as f32 / (1u64 << 53) as f32;
        let r = (-2.0 * u1.ln()).sqrt();
        let theta = 2.0 * std::f32::consts::PI * u2;
        r * theta.cos()
    }
}

fn rand_matrix(rng: &mut Lcg, rows: usize, cols: usize) -> Vec<f32> {
    (0..rows * cols).map(|_| rng.next_gaussian()).collect()
}

// ---------------------------------------------------------------------------
// Matrix multiplication: A(m×k) · B(k×n) → C(m×n), row-major
// ---------------------------------------------------------------------------

fn matmul(a: &[f32], m: usize, k: usize, b: &[f32], n: usize) -> Vec<f32> {
    let mut c = vec![0.0f32; m * n];
    for i in 0..m {
        for p in 0..k {
            let aip = a[i * k + p];
            for j in 0..n {
                c[i * n + j] += aip * b[p * n + j];
            }
        }
    }
    c
}

fn mse(a: &[f32], b: &[f32]) -> f64 {
    let n = a.len() as f64;
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| ((x - y) as f64).powi(2))
        .sum::<f64>()
        / n
}

fn dequant(bits: &[u16]) -> Vec<f32> {
    bits.iter().map(|&b| f16_bits_to_f32(b)).collect()
}

// ---------------------------------------------------------------------------
// Statistics: one-tailed paired-t test (df = n-1)
// ---------------------------------------------------------------------------

/// One-tailed paired-t test: H1 is delta_mean < 0 (improvement).
/// Returns (t_stat, p_value).
fn paired_t_one_tail(deltas: &[f64]) -> (f64, f64) {
    let n = deltas.len() as f64;
    let mean = deltas.iter().sum::<f64>() / n;
    let var = deltas.iter().map(|&d| (d - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let se = (var / n).sqrt();
    let t = if se > 1e-15 { mean / se } else { 0.0 };

    // One-tailed p-value for df=2 via t-distribution approximation
    // For df=2, the CDF of t-distribution is analytically tractable.
    // P(T ≤ t) = 0.5 + t/(2*sqrt(2)) * (1 + t²/4)^(-3/2) * (3/4)^(1/2) ... 
    // We use the simple approximation for df=2:
    // p-value = one-tail = P(T ≤ t_stat) where H1: mean < 0, so p = P(T ≤ t)
    let p = t_cdf_df2(t);
    (t, p)
}

/// Approximate CDF of t-distribution with df=2.
/// P(T ≤ x) for T ~ t(2).
fn t_cdf_df2(x: f64) -> f64 {
    // t(2) CDF: F(x) = 0.5 + x / (2 * sqrt(x^2 + 2)) * (1 + x^2/2)^(-1/2) * ...
    // Exact formula for df=2: F(x) = 0.5 * (1 + x / sqrt(x^2 + 2))
    0.5 * (1.0 + x / (x * x + 2.0).sqrt())
}

// ---------------------------------------------------------------------------
// Get git SHA (best effort)
// ---------------------------------------------------------------------------

fn git_sha() -> String {
    Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

// ---------------------------------------------------------------------------
// Main ablation
// ---------------------------------------------------------------------------

fn main() {
    // Parse --out flag
    let args: Vec<String> = std::env::args().collect();
    let out_path = args
        .iter()
        .position(|a| a == "--out")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
        .unwrap_or("assertions/calibration_ablation.jsonl");

    // Ensure output directory exists
    if let Some(parent) = std::path::Path::new(out_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let sha = git_sha();

    // Collect per-seed results indexed by [seed_idx][n_idx]
    let mut mse_vals: Vec<Vec<f64>> = vec![vec![0.0; 3]; 3]; // [seed][n_idx]
    let mut all_rows: Vec<String> = Vec::new();

    println!(
        "Wave 26 GPTQ-ON-GF16 calibration ablation — 3 seeds × N∈{{0,16,32}}"
    );
    println!(
        "NOTE: bpb_proxy = log2(reconstruction_mse_val + 1) — synthetic proxy, NOT real model BPB."
    );
    println!("{}", "=".repeat(78));

    for (si, &seed) in SEEDS.iter().enumerate() {
        for (ni, &calib_n) in CALIB_NS.iter().enumerate() {
            let t0 = Instant::now();

            // Calibration RNG (seed-derived, training shards only — never validation)
            let mut rng_calib = Lcg::new(seed.wrapping_mul(31337).wrapping_add(1));

            // Build weight matrix W (rows × cols)
            let w = rand_matrix(&mut rng_calib, ROWS, COLS);

            // Build calibration X (cols × n_samples)
            let n_samples = calib_n * BATCH_SIZE;
            let x_calib = if n_samples > 0 {
                rand_matrix(&mut rng_calib, COLS, n_samples)
            } else {
                vec![]
            };

            // Build held-out validation X_val (cols × VAL_SAMPLES)
            // SEPARATE seed — never overlaps with calibration
            let mut rng_val = Lcg::new(seed.wrapping_mul(99991).wrapping_add(7));
            let x_val = rand_matrix(&mut rng_val, COLS, VAL_SAMPLES);

            // Quantise with GPTQ (n=0 → naive, byte-identical to baseline)
            let bits = gf16_quantize_matrix_gptq(&w, ROWS, COLS, &x_calib, n_samples, 0.0);
            let w_deq = dequant(&bits);

            // Compute calibration MSE (on x_calib if available, else on w directly)
            let reconstruction_mse_calib = if n_samples > 0 {
                let wx_c = matmul(&w, ROWS, COLS, &x_calib, n_samples);
                let wq_c = matmul(&w_deq, ROWS, COLS, &x_calib, n_samples);
                mse(&wx_c, &wq_c)
            } else {
                // n=0: no calibration data — report weight-space MSE
                mse(&w, &w_deq)
            };

            // Compute validation MSE
            let wx_v = matmul(&w, ROWS, COLS, &x_val, VAL_SAMPLES);
            let wq_v = matmul(&w_deq, ROWS, COLS, &x_val, VAL_SAMPLES);
            let reconstruction_mse_val = mse(&wx_v, &wq_v);

            // BPB proxy (clearly labelled as synthetic)
            let bpb_proxy = (reconstruction_mse_val + 1.0).log2();

            let wallclock_ms = t0.elapsed().as_millis() as u64;

            mse_vals[si][ni] = reconstruction_mse_val;

            // Emit JSON row
            let row = format!(
                r#"{{"type":"cell","seed":{seed},"calibration_n":{calib_n},"reconstruction_mse_calib":{reconstruction_mse_calib:.8e},"reconstruction_mse_val":{reconstruction_mse_val:.8e},"bpb_proxy":{bpb_proxy:.8e},"wallclock_ms":{wallclock_ms},"git_sha":"{sha}","note":"bpb_proxy is log2(mse_val+1) — synthetic proxy only NOT real model BPB"}}"#
            );

            println!("seed={seed:3} N={calib_n:2} mse_val={reconstruction_mse_val:.4e} bpb_proxy={bpb_proxy:.6} [{wallclock_ms}ms]");
            all_rows.push(row);
        }
    }

    println!("{}", "=".repeat(78));

    // ---------------------------------------------------------------------------
    // Paired-t analysis: (N=0 vs N=16) and (N=16 vs N=32) across seeds
    // Positive delta means the higher-N result is WORSE (we want negative delta → lift).
    // ---------------------------------------------------------------------------

    // delta[seed] = mse(N=higher) - mse(N=lower)  — negative means improvement
    let deltas_0_16: Vec<f64> = (0..3).map(|si| mse_vals[si][1] - mse_vals[si][0]).collect();
    let deltas_16_32: Vec<f64> = (0..3).map(|si| mse_vals[si][2] - mse_vals[si][1]).collect();

    let (t_0_16, p_0_16) = paired_t_one_tail(&deltas_0_16);
    let (t_16_32, p_16_32) = paired_t_one_tail(&deltas_16_32);

    let verdict_0_16 = if p_0_16 < 0.25 { "PASS" } else { "FAIL" };
    let verdict_16_32 = if p_16_32 < 0.25 { "PASS" } else { "FAIL" };

    println!(
        "paired_t(0→16):  t={t_0_16:.4} p={p_0_16:.4} verdict={verdict_0_16}  [deltas: {deltas_0_16:?}]"
    );
    println!(
        "paired_t(16→32): t={t_16_32:.4} p={p_16_32:.4} verdict={verdict_16_32}  [deltas: {deltas_16_32:?}]"
    );

    // Emit verdict row
    let verdict_row = format!(
        r#"{{"type":"verdict","paired_t_0_16":{{"t":{t_0_16:.6},"p":{p_0_16:.6},"verdict":"{verdict_0_16}","deltas":{deltas_0_16:?}}},"paired_t_16_32":{{"t":{t_16_32:.6},"p":{p_16_32:.6},"verdict":"{verdict_16_32}","deltas":{deltas_16_32:?}}},"git_sha":"{sha}","h0":"GPTQ N-batch calibration gives no significant reconstruction improvement","h0_result_0_16":"{verdict_0_16}","h0_result_16_32":"{verdict_16_32}"}}"#
    );
    all_rows.push(verdict_row);

    // Write JSONL
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(out_path)
        .expect("Failed to open output file");

    for row in &all_rows {
        writeln!(file, "{row}").expect("Failed to write row");
    }

    println!("{}", "=".repeat(78));
    println!("Wrote {} rows to {out_path}", all_rows.len());
    println!(
        "Summary: paired_t(0→16): t={t_0_16:.4} p={p_0_16:.4} verdict={verdict_0_16}"
    );
    println!(
        "         paired_t(16→32): t={t_16_32:.4} p={p_16_32:.4} verdict={verdict_16_32}"
    );

    if verdict_0_16 == "PASS" && verdict_16_32 == "PASS" {
        println!("CONCLUSION: H0 REJECTED — calibration lever lifts GF16 reconstruction (p<0.25).");
    } else {
        println!(
            "CONCLUSION: H0 NOT REJECTED — calibration lever shows no significant lift on GF16 in this synthetic setting."
        );
        println!("(This is a valid result per R5: naive GF16 may already sit on Hessian-floor of its representational grid.)");
    }
}
