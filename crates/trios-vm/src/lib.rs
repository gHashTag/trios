//! TRIOS VM -- Ternary Virtual Machine
//!
//! FFI bindings to Zig-based VM implementation.

use std::ffi::c_void;

pub type VM = *mut c_void;

/// Create a new VM instance.
#[no_mangle]
pub extern "C" fn vm_create() -> VM {
    std::ptr::null_mut()
}

/// Destroy a VM instance.
///
/// # Safety
/// `vm` must be a valid pointer created by `vm_create`.
#[no_mangle]
pub unsafe extern "C" fn vm_destroy(_vm: VM) {}

/// Execute a single instruction.
///
/// # Safety
/// `vm` must be a valid pointer created by `vm_create`.
#[no_mangle]
pub unsafe extern "C" fn vm_step(_vm: VM) -> i32 {
    0
}
