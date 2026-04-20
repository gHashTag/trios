#[cfg(has_zig_lib)]
use libc::{c_int, size_t};

#[cfg(has_zig_lib)]
extern "C" {
    pub fn gf16_from_f32(x: f32) -> u16;
    pub fn gf16_to_f32(x: u16) -> f32;
    pub fn gf16_add(a: u16, b: u16) -> u16;
    pub fn gf16_mul(a: u16, b: u16) -> u16;
    pub fn gf16_sub(a: u16, b: u16) -> u16;
    pub fn gf16_div(a: u16, b: u16) -> u16;
    pub fn gf16_compress_weights(weights: *const f32, len: size_t, out: *mut u16) -> size_t;
    pub fn gf16_decompress_weights(compressed: *const u16, len: size_t, out: *mut f32) -> size_t;
    pub fn gf16_dot_product(a: *const u16, b: *const u16, len: size_t) -> u16;
    pub fn gf16_quantize_matrix(
        data: *const f32,
        rows: size_t,
        cols: size_t,
        scale: f32,
        out: *mut u16,
    ) -> c_int;
    #[doc(hidden)]
    pub fn _gf16_compress_weights(weights: *const f32, len: size_t, out: *mut u16) -> size_t;
    #[doc(hidden)]
    pub fn _gf16_decompress_weights(compressed: *const u16, len: size_t, out: *mut f32) -> size_t;
}
