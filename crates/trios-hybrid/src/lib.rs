//! TRIOS Hybrid — HybridBigInt
//!
//! FFI bindings to Zig-based HybridBigInt implementation.

use trios_tri::Ternary;

use std::ffi::c_void;

pub type Hybrid = *mut c_void;

#[no_mangle]
pub extern "C" fn hybrid_create() -> Hybrid {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn hybrid_destroy(h: Hybrid) {
    // TODO
}

#[no_mangle]
pub unsafe extern "C" fn hybrid_add(_a: Hybrid, _b: Hybrid, _out: Hybrid) -> i32 {
    0
}
