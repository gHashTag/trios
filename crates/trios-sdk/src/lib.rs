//! TRIOS SDK -- High-level API
//!
//! FFI bindings to Zig-based SDK implementation.

use std::ffi::c_void;

pub type Hypervector = *mut c_void;

#[no_mangle]
pub extern "C" fn sdk_hypervector_create(_dim: usize) -> Hypervector {
    std::ptr::null_mut()
}

/// Destroy a hypervector.
///
/// # Safety
/// `hv` must be a valid pointer created by `sdk_hypervector_create`.
#[no_mangle]
pub unsafe extern "C" fn sdk_hypervector_destroy(_hv: Hypervector) {}
