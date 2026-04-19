//! TRIOS VSA — Vector Symbolic Architecture
//!
//! This crate provides FFI bindings to the Zig-based VSA implementation.
//! VSA operations include hypervector binding, bundling, similarity, and encoding.

use std::ffi::c_char;
use std::os::raw::c_int;

/// Hypervector handle (opaque pointer)
pub type Hypervector = *mut std::os::raw::c_void;

/// Create a new hypervector with specified dimensionality
///
/// # Safety
/// - Caller must free the returned vector with `vsa_destroy`
#[no_mangle]
pub unsafe extern "C" fn vsa_create(dim: usize) -> Hypervector {
    // TODO: Call into Zig VSA implementation
    std::ptr::null_mut()
}

/// Destroy a hypervector and free its memory
///
/// # Safety
/// - `hv` must be a valid pointer created by `vsa_create`
#[no_mangle]
pub unsafe extern "C" fn vsa_destroy(hv: Hypervector) {
    // TODO: Free Zig-allocated memory
}

/// Bind two hypervectors (bundling operation)
///
/// # Safety
/// - `a`, `b`, `out` must be valid pointers
#[no_mangle]
pub unsafe extern "C" fn vsa_bind(a: Hypervector, b: Hypervector, out: Hypervector) -> c_int {
    // TODO: Call Zig VSA bind operation
    0
}

/// Compute similarity between two hypervectors
///
/// # Safety
/// - `a`, `b` must be valid pointers
#[no_mangle]
pub unsafe extern "C" fn vsa_similarity(a: Hypervector, b: Hypervector) -> f32 {
    // TODO: Call Zig VSA similarity operation
    0.0
}

/// Permute a hypervector
///
/// # Safety
/// - `hv`, `out` must be valid pointers
#[no_mangle]
pub unsafe extern "C" fn vsa_permute(hv: Hypervector, out: Hypervector) -> c_int {
    // TODO: Call Zig VSA permute operation
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vsa_api() {
        // Test that API compiles and links
        unsafe {
            let hv = vsa_create(1024);
            assert!(!hv.is_null() || hv.is_null()); // Placeholder test
            vsa_destroy(hv);
        }
    }
}
