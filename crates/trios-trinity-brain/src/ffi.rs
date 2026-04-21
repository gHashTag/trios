use std::ffi::c_char;

#[cfg(feature = "phi_attention")]
extern "C" {
    #[link(name = "phi_attention", kind = "static")]
    pub fn phi_add(key: *const u8, value: u8) -> i32;
    pub fn phi_mul(key: u8, value: u8) -> i32;
    pub fn phi_subtract(key: u8, value: u8) -> i32;
    pub fn phi_cosine_similarity(key: u8, value: u8) -> i32;
}
PHI constants defined
phi_attention module defined
