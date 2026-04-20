//! # trios-training-ffi
//!
//! Rust FFI bindings for zig-training HSLM kernels.
//!
//! Provides interfaces to:
//! - HSLM (Hierarchical Softmax-Latent Model) inference
//! - Forward pass kernels
//! - Training loop orchestration
//!
//! ## Features
//!
//! - **ffi** (default: disabled): Links against zig-training vendor/ library
//!
//! ## Example
//!
//! ```ignore
//! use trios_training_ffi::{hslm_inference, forward_kernel};
//!
//! let logits = hslm_inference(&input, Some(&hidden_state), 1024)?;
//! let grads = forward_kernel(&logits, &hidden_state);
//! ```

#[cfg(feature = "ffi")]
use libc::c_void;
use libc::{c_float, c_int};

/// HSLM configuration parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct HslmConfig {
    /// Embedding dimension.
    pub embedding_dim: usize,
    /// Hidden dimension.
    pub hidden_dim: usize,
    /// Number of attention heads.
    pub num_heads: usize,
    /// Temperature for sampling (0.0 = deterministic).
    pub temperature: c_float,
    /// Top-k sampling parameter.
    pub top_k: usize,
}

/// Forward pass kernel configuration.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ForwardKernelConfig {
    /// Input dimension.
    pub input_dim: usize,
    /// Output dimension.
    pub output_dim: usize,
    /// Activation function (0=relu, 1=gelu, 2=swish).
    pub activation: c_int,
}

#[cfg(feature = "ffi")]
extern "C" {
    pub fn hslm_inference(
        input: *const c_float,
        input_len: usize,
        hidden_state: *const c_float,
        batch_size: usize,
        vocab_size: usize,
        out_logits: *mut c_float,
    ) -> c_int;

    pub fn forward_kernel(
        logits: *const c_float,
        hidden: *const c_float,
        config: *const ForwardKernelConfig,
        out_gradients: *mut c_float,
    ) -> c_int;

    pub fn training_step(
        input: *const c_float,
        target: *const c_float,
        config: *const HslmConfig,
        out_loss: *mut c_float,
    ) -> c_int;

    pub fn hslm_free_memory(ptr: *mut c_void);
}

#[cfg(not(feature = "ffi"))]
pub fn hslm_inference(
    _input: &[f32],
    _hidden_state: Option<&[f32]>,
    _batch_size: usize,
    _vocab_size: usize,
) -> Result<Vec<f32>, String> {
    Err("Zig training FFI not available. Enable with --features ffi and ensure zig-training vendor is present.".to_string())
}

#[cfg(not(feature = "ffi"))]
pub fn forward_kernel(
    _logits: &[f32],
    _hidden: &[f32],
    _config: &ForwardKernelConfig,
) -> Result<Vec<f32>, String> {
    Err("Zig training FFI not available. Enable with --features ffi and ensure zig-training vendor is present.".to_string())
}

#[cfg(not(feature = "ffi"))]
pub fn training_step(_input: &[f32], _target: &[f32], _config: &HslmConfig) -> Result<f32, String> {
    Err("Zig training FFI not available. Enable with --features ffi and ensure zig-training vendor is present.".to_string())
}

#[cfg(test)]
mod tests {
    #[cfg(not(feature = "ffi"))]
    use super::*;

    #[test]
    #[cfg(not(feature = "ffi"))]
    fn stub_returns_error_in_stub_mode() {
        let result = hslm_inference(&[], None, 1, 10);
        assert!(result.is_err());
    }
}
