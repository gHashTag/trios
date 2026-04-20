//! # trios-hdc
//!
//! Safe Rust wrapper around [zig-hdc](https://github.com/gHashTag/zig-hdc),
//! providing Hyperdimensional Computing (HDC) / Vector Symbolic Architecture (VSA)
//! operations for AI agents.
//!
//! When the Zig vendor library is not available, all FFI-dependent types
//! and functions are replaced with stubs that return errors.

mod ffi;

pub const DEFAULT_DIMENSIONS: usize = 10_000;

#[cfg(has_zig_lib)]
mod zig_impl {
    use super::ffi;
    use std::marker::PhantomData;

    pub struct HdcSpace {
        pub ptr: *mut ffi::HdcSpace,
        pub dimensions: usize,
    }

    unsafe impl Send for HdcSpace {}
    unsafe impl Sync for HdcSpace {}

    impl HdcSpace {
        pub fn new(dimensions: usize) -> Self {
            let ptr = unsafe { ffi::hdc_space_create(dimensions) };
            assert!(!ptr.is_null(), "failed to create HDC space");
            HdcSpace { ptr, dimensions }
        }

        pub fn dimensions(&self) -> usize {
            self.dimensions
        }

        pub fn random_vector(&self) -> Hypervector {
            let mut data = vec![0u32; self.dimensions];
            let rc = unsafe { ffi::hdc_random_vector(self.ptr, data.as_mut_ptr()) };
            assert_eq!(rc, 0, "hdc_random_vector failed");
            Hypervector {
                data,
                _marker: PhantomData,
            }
        }

        pub fn bind(&self, a: &Hypervector, b: &Hypervector) -> Hypervector {
            let mut out = vec![0u32; self.dimensions];
            let rc = unsafe {
                ffi::hdc_bind(self.ptr, a.data.as_ptr(), b.data.as_ptr(), out.as_mut_ptr())
            };
            assert_eq!(rc, 0, "hdc_bind failed");
            Hypervector {
                data: out,
                _marker: PhantomData,
            }
        }

        pub fn bundle(&self, a: &Hypervector, b: &Hypervector) -> Hypervector {
            let mut out = vec![0u32; self.dimensions];
            let rc = unsafe {
                ffi::hdc_bundle(self.ptr, a.data.as_ptr(), b.data.as_ptr(), out.as_mut_ptr())
            };
            assert_eq!(rc, 0, "hdc_bundle failed");
            Hypervector {
                data: out,
                _marker: PhantomData,
            }
        }

        pub fn similarity(&self, a: &Hypervector, b: &Hypervector) -> f64 {
            unsafe { ffi::hdc_similarity(self.ptr, a.data.as_ptr(), b.data.as_ptr()) }
        }

        pub fn permute(&self, v: &Hypervector, shift: usize) -> Hypervector {
            let mut out = vec![0u32; self.dimensions];
            let rc =
                unsafe { ffi::hdc_permute(self.ptr, v.data.as_ptr(), shift, out.as_mut_ptr()) };
            assert_eq!(rc, 0, "hdc_permute failed");
            Hypervector {
                data: out,
                _marker: PhantomData,
            }
        }

        pub fn encode_level(&self, value: f64, min_val: f64, max_val: f64) -> Hypervector {
            let mut out = vec![0u32; self.dimensions];
            let rc = unsafe {
                ffi::hdc_encode_level(self.ptr, value, min_val, max_val, out.as_mut_ptr())
            };
            assert_eq!(rc, 0, "hdc_encode_level failed");
            Hypervector {
                data: out,
                _marker: PhantomData,
            }
        }

        pub fn encode_record(&self, values: &[f64], min_val: f64, max_val: f64) -> Hypervector {
            let mut out = vec![0u32; self.dimensions];
            let rc = unsafe {
                ffi::hdc_encode_record(
                    self.ptr,
                    values.as_ptr(),
                    values.len(),
                    min_val,
                    max_val,
                    out.as_mut_ptr(),
                )
            };
            assert_eq!(rc, 0, "hdc_encode_record failed");
            Hypervector {
                data: out,
                _marker: PhantomData,
            }
        }
    }

    impl Drop for HdcSpace {
        fn drop(&mut self) {
            unsafe { ffi::hdc_space_destroy(self.ptr) };
        }
    }

    pub struct Hypervector {
        pub data: Vec<u32>,
        pub _marker: PhantomData<*const ()>,
    }

    impl Hypervector {
        pub fn as_slice(&self) -> &[u32] {
            &self.data
        }
        pub fn len(&self) -> usize {
            self.data.len()
        }
        pub fn is_empty(&self) -> bool {
            self.data.is_empty()
        }
    }
}

#[cfg(has_zig_lib)]
pub use zig_impl::{HdcSpace, Hypervector};

#[cfg(not(has_zig_lib))]
pub struct HdcSpace {
    _priv: (),
}

#[cfg(not(has_zig_lib))]
impl HdcSpace {
    pub fn new(_dimensions: usize) -> Self {
        HdcSpace { _priv: () }
    }
    pub fn dimensions(&self) -> usize {
        0
    }
}

#[cfg(not(has_zig_lib))]
pub struct Hypervector {
    _priv: (),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hdc_space_create_stub() {
        let space = HdcSpace::new(DEFAULT_DIMENSIONS);
        assert_eq!(
            space.dimensions(),
            if cfg!(has_zig_lib) {
                DEFAULT_DIMENSIONS
            } else {
                0
            }
        );
    }
}
