//! Integration tests for GPTQ Hessian-correction with GF16 quantiser.
//!
//! Tests:
//!   1. gptq_n0_byte_identical_to_naive   — calibration_n=0 must match naive quantise
//!   2. gptq_reconstruction_mse_bounded   — GPTQ MSE ≤ naive MSE + epsilon
//!   3. gptq_seed_determinism             — same inputs → same output bytes
//!
//! Anchor: phi^2 + phi^-2 = 3

use trios_golden_float::gptq::{
    f16_bits_to_f32, gf16_quantize_matrix_gptq, naive_quantize_columns,
};

// ---------------------------------------------------------------------------
// Tiny deterministic RNG (LCG — no external crate needed in integration tests)
// ---------------------------------------------------------------------------

struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Lcg(seed ^ 0x5555_5555_5555_5555)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0
    }
    #[allow(dead_code)]
    fn next_f32(&mut self) -> f32 {
        // uniform in (-1, 1)
        let u = self.next_u64();
        let f = (u >> 11) as f32 / (1u64 << 53) as f32; // [0,1)
        f * 2.0 - 1.0
    }
    fn next_gaussian(&mut self) -> f32 {
        // Box-Muller
        let u1 = (self.next_u64() >> 11) as f32 / (1u64 << 53) as f32 + 1e-10;
        let u2 = (self.next_u64() >> 11) as f32 / (1u64 << 53) as f32;
        let r = (-2.0 * u1.ln()).sqrt();
        let theta = 2.0 * std::f32::consts::PI * u2;
        r * theta.cos()
    }
}

fn random_matrix(rng: &mut Lcg, rows: usize, cols: usize) -> Vec<f32> {
    (0..rows * cols).map(|_| rng.next_gaussian()).collect()
}

fn matmul(a: &[f32], a_rows: usize, a_cols: usize, b: &[f32], b_cols: usize) -> Vec<f32> {
    // a: a_rows × a_cols, b: a_cols × b_cols → out: a_rows × b_cols (row-major)
    let mut out = vec![0.0f32; a_rows * b_cols];
    for i in 0..a_rows {
        for k in 0..a_cols {
            for j in 0..b_cols {
                out[i * b_cols + j] += a[i * a_cols + k] * b[k * b_cols + j];
            }
        }
    }
    out
}

fn mse(a: &[f32], b: &[f32]) -> f64 {
    assert_eq!(a.len(), b.len());
    let n = a.len() as f64;
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| ((x - y) as f64).powi(2))
        .sum::<f64>()
        / n
}

fn dequant_bits(bits: &[u16]) -> Vec<f32> {
    bits.iter().map(|&b| f16_bits_to_f32(b)).collect()
}

// ---------------------------------------------------------------------------
// Test 1: calibration_n=0 → byte-identical to naive_quantize_columns
// ---------------------------------------------------------------------------

#[test]
fn gptq_n0_byte_identical_to_naive() {
    let mut rng = Lcg::new(42);
    let rows = 32;
    let cols = 32;
    let w = random_matrix(&mut rng, rows, cols);

    let gptq_out = gf16_quantize_matrix_gptq(&w, rows, cols, &[], 0, 0.0);
    let naive_out = naive_quantize_columns(&w, rows, cols);

    assert_eq!(
        gptq_out.len(),
        naive_out.len(),
        "output length mismatch: gptq={} naive={}",
        gptq_out.len(),
        naive_out.len()
    );
    for (i, (g, n)) in gptq_out.iter().zip(naive_out.iter()).enumerate() {
        assert_eq!(
            g,
            n,
            "byte mismatch at index {}: gptq=0x{:04x} naive=0x{:04x}",
            i,
            g,
            n
        );
    }
}

// ---------------------------------------------------------------------------
// Test 2: GPTQ MSE ≤ naive MSE + epsilon (Hessian correction does not hurt)
// ---------------------------------------------------------------------------

#[test]
fn gptq_reconstruction_mse_bounded() {
    let mut rng = Lcg::new(1234);
    let rows = 32;
    let cols = 32;
    let n_samples = 64;

    let w = random_matrix(&mut rng, rows, cols);
    let x_calib = random_matrix(&mut rng, cols, n_samples); // cols × n_samples

    // Separate RNG seed for validation
    let mut rng_val = Lcg::new(9999);
    let x_val = random_matrix(&mut rng_val, cols, 128); // cols × 128

    // Naive quantisation
    let naive_bits = naive_quantize_columns(&w, rows, cols);
    let naive_deq = dequant_bits(&naive_bits);

    // GPTQ quantisation
    let gptq_bits = gf16_quantize_matrix_gptq(&w, rows, cols, &x_calib, n_samples, 0.0);
    let gptq_deq = dequant_bits(&gptq_bits);

    // Compute reconstruction MSE on x_val: ‖W·X - Q(W)·X‖²
    let wx_true = matmul(&w, rows, cols, &x_val, 128);
    let wx_naive = matmul(&naive_deq, rows, cols, &x_val, 128);
    let wx_gptq = matmul(&gptq_deq, rows, cols, &x_val, 128);

    let mse_naive = mse(&wx_true, &wx_naive);
    let mse_gptq = mse(&wx_true, &wx_gptq);

    // Allow a small tolerance: GPTQ MSE must not exceed naive MSE + 1e-3 * baseline
    let epsilon = (mse_naive * 1e-3).max(1e-6);

    assert!(
        mse_gptq <= mse_naive + epsilon,
        "GPTQ MSE ({:.6e}) exceeds naive MSE ({:.6e}) + epsilon ({:.6e})",
        mse_gptq,
        mse_naive,
        epsilon
    );

    // Also emit the values for debugging
    println!(
        "MSE: naive={:.6e}, gptq={:.6e}, ratio={:.4}",
        mse_naive,
        mse_gptq,
        mse_gptq / mse_naive.max(1e-30)
    );
}

// ---------------------------------------------------------------------------
// Test 3: determinism — same inputs → same output bytes
// ---------------------------------------------------------------------------

#[test]
fn gptq_seed_determinism() {
    let mut rng = Lcg::new(777);
    let rows = 32;
    let cols = 32;
    let n_samples = 32;

    let w = random_matrix(&mut rng, rows, cols);
    let x_calib = random_matrix(&mut rng, cols, n_samples);

    let out1 = gf16_quantize_matrix_gptq(&w, rows, cols, &x_calib, n_samples, 1e-4);
    let out2 = gf16_quantize_matrix_gptq(&w, rows, cols, &x_calib, n_samples, 1e-4);

    assert_eq!(out1, out2, "GPTQ output is non-deterministic");
}
