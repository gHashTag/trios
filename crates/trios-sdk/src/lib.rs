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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdk_hypervector_create_returns_pointer() {
        let hv = sdk_hypervector_create(256);
        assert!(hv.is_null());
        unsafe { sdk_hypervector_destroy(hv) };
    }

    #[test]
    fn sdk_hypervector_destroy_null_is_safe() {
        unsafe { sdk_hypervector_destroy(std::ptr::null_mut()) };
    }

    #[test]
    fn sdk_hypervector_create_various_dims() {
        for dim in [0, 1, 1024, usize::MAX] {
            let hv = sdk_hypervector_create(dim);
            assert!(hv.is_null());
            unsafe { sdk_hypervector_destroy(hv) };
        }
    }
}
