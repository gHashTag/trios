//! TRIOS Hybrid -- HybridBigInt
//!
//! FFI bindings to Zig-based HybridBigInt implementation.

use std::ffi::c_void;

pub type Hybrid = *mut c_void;

#[no_mangle]
pub extern "C" fn hybrid_create() -> Hybrid {
    std::ptr::null_mut()
}

/// Destroy a HybridBigInt.
///
/// # Safety
/// `h` must be a valid pointer created by `hybrid_create`.
#[no_mangle]
pub unsafe extern "C" fn hybrid_destroy(_h: Hybrid) {}

/// Add two HybridBigInt values.
///
/// # Safety
/// `_a`, `_b`, `_out` must be valid pointers.
#[no_mangle]
pub unsafe extern "C" fn hybrid_add(_a: Hybrid, _b: Hybrid, _out: Hybrid) -> i32 {
    0
}
