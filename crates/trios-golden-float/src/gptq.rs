//! GPTQ Hessian-correction loop with GF16 as the inner quantiser Q.
//!
//! Implements the algorithm from parameter-golf#2135:
//!   - Compute H = 2·X·X^T + λI (Hessian approximation)
//!   - Cholesky-decompose H
//!   - Iterate columns left-to-right: quantise with Q, scatter residual error via H^{-1} row
//!
//! Anchor: phi^2 + phi^-2 = 3  · DOI 10.5281/zenodo.19227877

#[cfg(has_zig_lib)]
use crate::quantize_matrix;

// ---------------------------------------------------------------------------
// Software GF16 emulator (fallback when zig vendor is absent).
// Uses IEEE f16 bit layout: sign(1) + exponent(5) + mantissa(10).
// This is close enough for reconstruction-MSE invariant testing.
// ---------------------------------------------------------------------------

/// Round an f32 to the nearest f16-representable value (software emulation).
/// Uses the standard IEEE 754 half-precision grid.
pub fn f32_to_f16_bits(x: f32) -> u16 {
    // Handle special cases
    if x.is_nan() {
        return 0x7e00u16; // NaN
    }
    let bits = x.to_bits();
    let sign = ((bits >> 31) & 1) as u16;
    let exp_f32 = ((bits >> 23) & 0xff) as i32;
    let mant_f32 = bits & 0x7fffff;

    if exp_f32 == 0xff {
        // Inf or NaN
        return (sign << 15) | 0x7c00;
    }

    let exp_f16 = exp_f32 - 127 + 15;

    if exp_f16 >= 31 {
        // Overflow → infinity
        return (sign << 15) | 0x7c00;
    }

    if exp_f16 <= 0 {
        // Denormal or underflow
        if exp_f16 < -10 {
            return sign << 15;
        }
        let mant = (mant_f32 | 0x800000) >> (14 - exp_f16);
        let mant16 = ((mant + 0x1000) >> 13) as u16;
        return (sign << 15) | mant16;
    }

    let mant16 = ((mant_f32 + 0x1000) >> 13) as u16;
    if mant16 == 0x400 {
        // Mantissa rounded up, increment exponent
        let e = exp_f16 as u16 + 1;
        return (sign << 15) | (e << 10);
    }
    (sign << 15) | ((exp_f16 as u16) << 10) | (mant16 & 0x3ff)
}

/// Reconstruct f32 from f16 bits.
pub fn f16_bits_to_f32(bits: u16) -> f32 {
    let sign = (bits >> 15) & 1;
    let exp16 = (bits >> 10) & 0x1f;
    let mant16 = bits & 0x3ff;

    let f32_bits: u32 = if exp16 == 0 {
        if mant16 == 0 {
            (sign as u32) << 31
        } else {
            // Denormal
            let mut m = mant16 as u32;
            let mut e = 0u32;
            while (m & 0x400) == 0 {
                m <<= 1;
                e += 1;
            }
            m &= 0x3ff;
            let exp_f32 = (127 - 15 + 1 - e) as u32;
            ((sign as u32) << 31) | (exp_f32 << 23) | (m << 13)
        }
    } else if exp16 == 31 {
        // Inf or NaN
        ((sign as u32) << 31) | 0x7f800000 | ((mant16 as u32) << 13)
    } else {
        let exp_f32 = ((exp16 as i32) - 15 + 127) as u32;
        ((sign as u32) << 31) | (exp_f32 << 23) | ((mant16 as u32) << 13)
    };
    f32::from_bits(f32_bits)
}

// ---------------------------------------------------------------------------
// Quantise a single column of floats to u16 bits (either GF16 or f16-emulator).
// Returns (quantised_bits, dequantised_f32_values).
// ---------------------------------------------------------------------------

/// Quantise a column vector to GF16 bits and return dequantised values.
///
/// When `has_zig_lib`: delegates to `quantize_matrix` (1 col) then reconstructs
/// via f16 round-trip using our software emulator (since no dequantize FFI exists).
/// When NOT `has_zig_lib`: uses software f16 emulator throughout.
fn quantize_column(col: &[f32]) -> (Vec<u16>, Vec<f32>) {
    let n = col.len();

    // Compute max-abs scale (mirrors existing quantize_matrix logic)
    let max_abs = col.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
    let _scale = if max_abs > 0.0 { max_abs } else { 1.0 };

    #[cfg(has_zig_lib)]
    {
        // Use the real GF16 FFI path
        let bits = quantize_matrix(col, 1, n, scale);
        // Dequantise: GF16 encodes as scaled f16-like values.
        // Since there is no gf16_dequantize_matrix FFI, we reconstruct via
        // f16_bits_to_f32 with the same scale used during quantisation.
        let dequant: Vec<f32> = bits.iter().map(|&b| f16_bits_to_f32(b) * scale).collect();
        return (bits, dequant);
    }

    #[cfg(not(has_zig_lib))]
    {
        // Software path: round each value to nearest f16 grid point.
        let mut bits = Vec::with_capacity(n);
        let mut dequant = Vec::with_capacity(n);
        for &v in col {
            let b = f32_to_f16_bits(v);
            bits.push(b);
            dequant.push(f16_bits_to_f32(b));
        }
        (bits, dequant)
    }
}

// ---------------------------------------------------------------------------
// Cholesky decomposition (simple Cholesky-Banachiewicz for symmetric PSD).
// Returns lower-triangular L such that A = L·L^T.
// Input: flat row-major n×n matrix.
// ---------------------------------------------------------------------------

fn cholesky(a: &[f32], n: usize) -> Vec<f32> {
    let mut l = vec![0.0f32; n * n];
    for i in 0..n {
        for j in 0..=i {
            let mut sum = 0.0f32;
            for k in 0..j {
                sum += l[i * n + k] * l[j * n + k];
            }
            if i == j {
                let diag = a[i * n + i] - sum;
                l[i * n + j] = if diag > 0.0 { diag.sqrt() } else { 1e-8f32 };
            } else {
                let lj = l[j * n + j];
                l[i * n + j] = if lj.abs() > 1e-12 {
                    (a[i * n + j] - sum) / lj
                } else {
                    0.0
                };
            }
        }
    }
    l
}

/// Solve L · x = b for x (forward substitution, L lower-triangular).
fn forward_sub(l: &[f32], b: &[f32], n: usize) -> Vec<f32> {
    let mut x = vec![0.0f32; n];
    for i in 0..n {
        let mut s = b[i];
        for j in 0..i {
            s -= l[i * n + j] * x[j];
        }
        let lii = l[i * n + i];
        x[i] = if lii.abs() > 1e-12 { s / lii } else { 0.0 };
    }
    x
}

/// Solve L^T · x = b for x (back substitution, L lower-triangular).
fn back_sub(l: &[f32], b: &[f32], n: usize) -> Vec<f32> {
    let mut x = vec![0.0f32; n];
    for i in (0..n).rev() {
        let mut s = b[i];
        for j in i + 1..n {
            s -= l[j * n + i] * x[j]; // L^T[i,j] = L[j,i]
        }
        let lii = l[i * n + i];
        x[i] = if lii.abs() > 1e-12 { s / lii } else { 0.0 };
    }
    x
}

/// Compute H^{-1} · e_j (j-th column of H inverse) via Cholesky solve.
fn hinv_column(l: &[f32], j: usize, n: usize) -> Vec<f32> {
    let mut e = vec![0.0f32; n];
    e[j] = 1.0;
    let y = forward_sub(l, &e, n);
    back_sub(l, &y, n)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Dequantise a GF16 matrix (u16 bits → f32 values).
///
/// This is a software round-trip using the f16 bit layout. The same layout is
/// used by `gf16_quantize_matrix_gptq` when zig is absent.
pub fn gf16_dequantize_matrix(bits: &[u16], _rows: usize, _cols: usize) -> Vec<f32> {
    bits.iter().map(|&b| f16_bits_to_f32(b)).collect()
}

/// GPTQ Hessian-correction quantisation with GF16 as the inner quantiser Q.
///
/// # Arguments
/// * `w_in`             – row-major weight matrix, shape `rows × cols`
/// * `rows`, `cols`     – matrix dimensions
/// * `x_calib`          – calibration activations, row-major `cols × n_samples`
/// * `n_samples`        – number of calibration samples (0 → naive quantisation)
/// * `dampening_lambda` – Hessian dampening; if 0.0, uses 1e-2 × trace(H)/cols
///
/// # Returns
/// Flat `Vec<u16>` of GF16 bits, row-major, length `rows × cols`.
///
/// # Invariant (calibration_n = 0)
/// When `n_samples == 0` or `x_calib.is_empty()`, the output is byte-identical
/// to calling `quantize_matrix(w_in, rows, cols, scale)` with the per-column
/// max-abs scale — i.e. purely naive column-wise GF16 quantisation.
pub fn gf16_quantize_matrix_gptq(
    w_in: &[f32],
    rows: usize,
    cols: usize,
    x_calib: &[f32],
    n_samples: usize,
    dampening_lambda: f32,
) -> Vec<u16> {
    assert_eq!(w_in.len(), rows * cols, "w_in length mismatch");

    // Fast path: no calibration data → byte-identical to naive quantise_matrix.
    if n_samples == 0 || x_calib.is_empty() {
        return naive_quantize_columns(w_in, rows, cols);
    }

    assert_eq!(
        x_calib.len(),
        cols * n_samples,
        "x_calib length mismatch: expected {}×{}={}, got {}",
        cols,
        n_samples,
        cols * n_samples,
        x_calib.len()
    );

    // Build H = 2·X·X^T + λ·I  (X is cols×n_samples, H is cols×cols)
    let mut h = vec![0.0f32; cols * cols];
    for s in 0..n_samples {
        for i in 0..cols {
            for j in 0..=i {
                let xi = x_calib[i * n_samples + s];
                let xj = x_calib[j * n_samples + s];
                h[i * cols + j] += 2.0 * xi * xj;
                if i != j {
                    h[j * cols + i] += 2.0 * xi * xj;
                }
            }
        }
    }

    // Compute dampening λ
    let lambda = if dampening_lambda > 0.0 {
        dampening_lambda
    } else {
        let trace: f32 = (0..cols).map(|i| h[i * cols + i]).sum();
        1e-2 * trace / cols as f32
    };

    for i in 0..cols {
        h[i * cols + i] += lambda;
    }

    // Cholesky decomposition of H
    let l = cholesky(&h, cols);

    // Work on a mutable copy of W (column-major view for error scatter)
    // We keep it row-major: W[row][col] = w_mut[row * cols + col]
    let mut w_mut: Vec<f32> = w_in.to_vec();
    let mut q_out = vec![0u16; rows * cols];

    for j in 0..cols {
        // Extract column j from all rows
        let col_j: Vec<f32> = (0..rows).map(|r| w_mut[r * cols + j]).collect();

        // Quantise column j → GF16 bits + dequantised values
        let (bits_j, dequant_j) = quantize_column(&col_j);

        // Store bits in output (row-major)
        for r in 0..rows {
            q_out[r * cols + j] = bits_j[r];
        }

        // Compute H^{-1}[j, :] (j-th row of H inverse)
        let hinv_j = hinv_column(&l, j, cols);
        let hinv_jj = hinv_j[j];

        if hinv_jj.abs() < 1e-12 {
            continue;
        }

        // Scatter error: for each remaining column k > j, subtract correction
        for r in 0..rows {
            let err = (col_j[r] - dequant_j[r]) / hinv_jj;
            for k in j + 1..cols {
                w_mut[r * cols + k] -= err * hinv_j[k];
            }
        }
    }

    q_out
}

/// Naive per-column GF16 quantisation (no Hessian correction).
/// This is the baseline that `gf16_quantize_matrix_gptq` with n_samples=0 must match.
pub fn naive_quantize_columns(w_in: &[f32], rows: usize, cols: usize) -> Vec<u16> {
    let mut out = vec![0u16; rows * cols];
    for j in 0..cols {
        let col: Vec<f32> = (0..rows).map(|r| w_in[r * cols + j]).collect();
        let (bits, _) = quantize_column(&col);
        for r in 0..rows {
            out[r * cols + j] = bits[r];
        }
    }
    out
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn f16_roundtrip_one() {
        let x = 1.0f32;
        let b = f32_to_f16_bits(x);
        let y = f16_bits_to_f32(b);
        assert!((y - x).abs() < 0.01, "1.0 roundtrip: got {}", y);
    }

    #[test]
    fn f16_roundtrip_zero() {
        assert_eq!(f32_to_f16_bits(0.0), 0u16);
        assert_eq!(f16_bits_to_f32(0u16), 0.0f32);
    }

    #[test]
    fn cholesky_identity() {
        // H = I → L = I
        let id = vec![1.0f32, 0.0, 0.0, 1.0];
        let l = cholesky(&id, 2);
        assert!((l[0] - 1.0).abs() < 1e-6);
        assert!((l[3] - 1.0).abs() < 1e-6);
        assert!(l[1].abs() < 1e-6);
        assert!(l[2].abs() < 1e-6);
    }
}
