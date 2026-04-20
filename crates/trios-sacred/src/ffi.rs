use libc::{c_int, size_t};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BealCandidate {
    pub a: u64,
    pub b: u64,
    pub c: u64,
    pub m: u32,
    pub n: u32,
    pub r: u32,
    pub valid: bool,
}

#[cfg(has_zig_lib)]
extern "C" {
    pub fn sacred_phi_attention(
        queries: *const f64,
        keys: *const f64,
        seq_len: size_t,
        dim: size_t,
        phi_factor: f64,
        out_weights: *mut f64,
    ) -> c_int;

    pub fn sacred_fibonacci_spiral(t: f64, out_x: *mut f64, out_y: *mut f64);
    pub fn sacred_golden_sequence(n: size_t, out: *mut f64) -> c_int;
    pub fn sacred_beal_search(
        min_base: u64,
        max_base: u64,
        max_exp: u32,
        candidates: *mut BealCandidate,
        max_results: size_t,
    ) -> size_t;
    pub fn sacred_phi_bottleneck(model_dim: size_t) -> size_t;
    pub fn sacred_head_spacing(n_heads: size_t, out_spacing: *mut f64) -> c_int;

    #[doc(hidden)]
    pub fn _sacred_phi_attention(
        queries: *const f64,
        keys: *const f64,
        seq_len: size_t,
        dim: size_t,
        phi_factor: f64,
        out_weights: *mut f64,
    ) -> c_int;
    #[doc(hidden)]
    pub fn _sacred_fibonacci_spiral(t: f64, out_x: *mut f64, out_y: *mut f64);
    #[doc(hidden)]
    pub fn _sacred_golden_sequence(n: size_t, out: *mut f64) -> c_int;
    #[doc(hidden)]
    pub fn _sacred_beal_search(
        min_base: u64,
        max_base: u64,
        max_exp: u32,
        candidates: *mut BealCandidate,
        max_results: size_t,
    ) -> size_t;
    #[doc(hidden)]
    pub fn _sacred_phi_bottleneck(model_dim: size_t) -> size_t;
    #[doc(hidden)]
    pub fn _sacred_head_spacing(n_heads: size_t, out_spacing: *mut f64) -> c_int;
}
