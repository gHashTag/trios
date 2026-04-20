#[cfg(has_zig_lib)]
use libc::{c_int, size_t};

#[cfg(has_zig_lib)]
pub type HdcSpace = c_int;

#[cfg(has_zig_lib)]
extern "C" {
    pub fn hdc_space_create(dimensions: size_t) -> *mut HdcSpace;
    pub fn hdc_space_destroy(space: *mut HdcSpace);
    pub fn hdc_random_vector(space: *mut HdcSpace, out: *mut u32) -> c_int;
    pub fn hdc_bind(space: *mut HdcSpace, a: *const u32, b: *const u32, out: *mut u32) -> c_int;
    pub fn hdc_bundle(space: *mut HdcSpace, a: *const u32, b: *const u32, out: *mut u32) -> c_int;
    pub fn hdc_similarity(space: *mut HdcSpace, a: *const u32, b: *const u32) -> f64;
    pub fn hdc_permute(
        space: *mut HdcSpace,
        vec: *const u32,
        shift: size_t,
        out: *mut u32,
    ) -> c_int;
    pub fn hdc_encode_level(
        space: *mut HdcSpace,
        value: f64,
        min_val: f64,
        max_val: f64,
        out: *mut u32,
    ) -> c_int;
    pub fn hdc_encode_record(
        space: *mut HdcSpace,
        values: *const f64,
        len: size_t,
        min_val: f64,
        max_val: f64,
        out: *mut u32,
    ) -> c_int;
}
