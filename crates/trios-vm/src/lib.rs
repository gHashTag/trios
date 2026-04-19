//! TRIOS VM — Ternary Virtual Machine
//!
//! FFI bindings to Zig-based VM implementation.

use std::ffi::c_void;

/// VM handle (opaque pointer)
pub type VM = *mut c_void;

/// Create a new VM instance
#[no_mangle]
pub extern "C" fn vm_create() -> VM {
    std::ptr::null_mut()
}

/// Destroy a VM instance
#[no_mangle]
pub unsafe extern "C" fn vm_destroy(vm: VM) {
    // TODO: Free Zig-allocated memory
}

/// Execute a single instruction
#[no_mangle]
pub unsafe extern "C" fn vm_step(vm: VM) -> i32 {
    0
}
