//! # trios-sacred
//!
//! Safe Rust wrapper around zig-sacred-geometry, providing sacred geometry primitives.
//!
//! When the Zig vendor library is not available, all FFI-dependent functions
//! return errors or stub values.

mod ffi;

pub use ffi::BealCandidate;

#[cfg(has_zig_lib)]
pub fn phi_attention(
    queries: &[f64],
    keys: &[f64],
    seq_len: usize,
    dim: usize,
    phi_factor: f64,
) -> Result<Vec<f64>, String> {
    let expected = seq_len * dim;
    if queries.len() != expected || keys.len() != expected {
        return Err(format!(
            "dimension mismatch: queries={}, keys={}, expected={}",
            queries.len(),
            keys.len(),
            expected
        ));
    }
    let mut out = vec![0.0f64; seq_len * seq_len];
    let rc = unsafe {
        ffi::sacred_phi_attention(
            queries.as_ptr(),
            keys.as_ptr(),
            seq_len,
            dim,
            phi_factor,
            out.as_mut_ptr(),
        )
    };
    if rc == 0 {
        Ok(out)
    } else {
        Err(format!("phi_attention failed with code {rc}"))
    }
}

#[cfg(has_zig_lib)]
pub fn fibonacci_spiral(t: f64) -> (f64, f64) {
    let mut x = 0.0;
    let mut y = 0.0;
    unsafe { ffi::sacred_fibonacci_spiral(t, &mut x, &mut y) }
    (x, y)
}

#[cfg(has_zig_lib)]
pub fn golden_sequence(n: usize) -> Vec<f64> {
    let mut out = vec![0.0f64; n];
    unsafe {
        ffi::sacred_golden_sequence(n, out.as_mut_ptr());
    }
    out
}

#[cfg(has_zig_lib)]
pub fn beal_search(
    min_base: u64,
    max_base: u64,
    max_exp: u32,
    max_results: usize,
) -> Vec<BealCandidate> {
    let mut candidates = vec![
        BealCandidate {
            a: 0,
            b: 0,
            c: 0,
            m: 0,
            n: 0,
            r: 0,
            valid: false
        };
        max_results
    ];
    let found = unsafe {
        ffi::sacred_beal_search(
            min_base,
            max_base,
            max_exp,
            candidates.as_mut_ptr(),
            max_results,
        )
    };
    candidates.truncate(found);
    candidates
}

#[cfg(has_zig_lib)]
pub fn phi_bottleneck(model_dim: usize) -> usize {
    unsafe { ffi::sacred_phi_bottleneck(model_dim) }
}

#[cfg(has_zig_lib)]
pub fn head_spacing(n_heads: usize) -> Vec<f64> {
    let mut out = vec![0.0f64; n_heads];
    unsafe {
        ffi::sacred_head_spacing(n_heads, out.as_mut_ptr());
    }
    out
}

#[cfg(not(has_zig_lib))]
pub fn phi_attention(
    _queries: &[f64],
    _keys: &[f64],
    _seq_len: usize,
    _dim: usize,
    _phi_factor: f64,
) -> Result<Vec<f64>, String> {
    Err("zig-sacred-geometry FFI not available. Build zig vendor first.".into())
}

#[cfg(not(has_zig_lib))]
pub fn fibonacci_spiral(_t: f64) -> (f64, f64) {
    (0.0, 0.0)
}

#[cfg(not(has_zig_lib))]
pub fn golden_sequence(n: usize) -> Vec<f64> {
    vec![0.0; n]
}

#[cfg(not(has_zig_lib))]
pub fn beal_search(
    _min_base: u64,
    _max_base: u64,
    _max_exp: u32,
    _max_results: usize,
) -> Vec<BealCandidate> {
    vec![]
}

#[cfg(not(has_zig_lib))]
pub fn phi_bottleneck(_model_dim: usize) -> usize {
    0
}

#[cfg(not(has_zig_lib))]
pub fn head_spacing(n_heads: usize) -> Vec<f64> {
    vec![0.0; n_heads]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_phi_attention_returns_ok_or_err() {
        let result = phi_attention(&[], &[], 0, 0, 1.618);
        if cfg!(has_zig_lib) {
            assert!(result.is_ok());
        } else {
            assert!(result.is_err());
        }
    }

    #[test]
    fn stub_golden_sequence_length() {
        let seq = golden_sequence(5);
        assert_eq!(seq.len(), 5);
    }

    #[test]
    fn stub_phi_bottleneck() {
        let bn = phi_bottleneck(512);
        if cfg!(has_zig_lib) {
            assert!(bn > 0);
        }
    }
}
