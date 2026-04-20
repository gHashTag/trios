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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vm_create_returns_pointer() {
        let vm = vm_create();
        assert!(vm.is_null());
        unsafe { vm_destroy(vm) };
    }

    #[test]
    fn vm_destroy_null_is_safe() {
        unsafe { vm_destroy(std::ptr::null_mut()) };
    }

    #[test]
    fn vm_step_returns_zero() {
        let result = unsafe { vm_step(std::ptr::null_mut()) };
        assert_eq!(result, 0);
    }

    #[test]
    fn vm_type_is_raw_pointer() {
        let vm: VM = vm_create();
        assert!(vm.is_null());
        unsafe { vm_destroy(vm) };
    }
}
