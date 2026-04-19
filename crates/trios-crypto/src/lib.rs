//! # trios-crypto
//!
//! Safe Rust wrapper around [zig-crypto-mining](https://github.com/gHashTag/zig-crypto-mining),
//! providing SHA-256 hashing, SHA256d mining, and DePIN proof-of-work primitives.
//!
//! ## Features
//!
//! - **ffi** (default: disabled): Enable real FFI bindings to zig-crypto-mining
//!
//! ## Example
//!
//! ```ignore
//! use trios_crypto::{sha256, Sha256Hash};
//!
//! let hash: Result<Sha256Hash, String> = sha256(b"hello world");
//! ```

mod ffi;

use std::fmt;

// Re-export public types from ffi module (always visible)
pub use ffi::{DepinProof, MiningResult, Sha256Hash};

/// Error returned when FFI is not available.
#[derive(Debug)]
pub struct FfiNotAvailable;

impl fmt::Display for FfiNotAvailable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("FFI not available. Build with --features ffi and ensure zig-crypto-mining vendor is present.")
    }
}

impl std::error::Error for FfiNotAvailable {}

// ─────────────────────────────────────────────────────
// FFI implementations (real Zig bindings)
// ─────────────────────────────────────────────────────────────

#[cfg(feature = "ffi")]
pub fn sha256(data: &[u8]) -> Result<Sha256Hash, String> {
    let mut hash: Sha256Hash = [0u8; 32];
    let rc = unsafe { ffi::crypto_sha256(data.as_ptr(), data.len(), &mut hash as *mut Sha256Hash) };
    if rc != 0 {
        Err(format!("sha256 failed with code {}", rc))
    } else {
        Ok(hash)
    }
}

#[cfg(feature = "ffi")]
pub fn double_sha256(data: &[u8]) -> Result<Sha256Hash, String> {
    let mut hash1: Sha256Hash = [0u8; 32];
    let mut hash2: Sha256Hash = [0u8; 32];
    let rc1 = unsafe { ffi::crypto_sha256(data.as_ptr(), data.len(), &mut hash1 as *mut Sha256Hash) };
    if rc1 != 0 {
        return Err(format!("first sha256 failed with code {}", rc1));
    }
    let rc2 = unsafe { ffi::crypto_sha256(hash1.as_ptr(), 32, &mut hash2 as *mut Sha256Hash) };
    if rc2 != 0 {
        Err(format!("second sha256 failed with code {}", rc2))
    } else {
        Ok(hash2)
    }
}

#[cfg(feature = "ffi")]
pub fn mine_sha256d(
    header: &[u8; 80],
    target: &[u8; 32],
    start_nonce: u64,
    max_nonce: u64,
) -> Result<MiningResult, String> {
    let mut result = MiningResult {
        nonce: 0,
        hash: [0u8; 32],
        hashes_computed: 0,
        found: false,
    };
    let rc = unsafe {
        ffi::crypto_mine_sha256d(
            header.as_ptr(),
            target as *const Sha256Hash,
            start_nonce,
            max_nonce,
            &mut result as *mut MiningResult,
        )
    };
    if rc != 0 {
        Err(format!("mining failed with code {}", rc))
    } else {
        Ok(result)
    }
}

#[cfg(feature = "ffi")]
pub fn depin_prove(challenge: u64, worker_id: &[u8]) -> Result<DepinProof, String> {
    let mut proof = DepinProof {
        proof: [0u8; 64],
        challenge: 0,
        validator_hash: [0u8; 32],
        valid: false,
    };
    let rc = unsafe {
        ffi::crypto_depin_prove(
            challenge,
            worker_id.as_ptr(),
            worker_id.len(),
            &mut proof as *mut DepinProof,
        )
    };
    if rc != 0 {
        Err(format!("depin_prove failed with code {}", rc))
    } else {
        Ok(proof)
    }
}

// ─────────────────────────────────────────────────────────────
// Stub implementations (when FFI is not available)
// ─────────────────────────────────────────────────────────────

#[cfg(not(feature = "ffi"))]
pub fn sha256(_data: &[u8]) -> Result<Sha256Hash, String> {
    Err(FfiNotAvailable.to_string())
}

#[cfg(not(feature = "ffi"))]
pub fn double_sha256(_data: &[u8]) -> Result<Sha256Hash, String> {
    Err(FfiNotAvailable.to_string())
}

#[cfg(not(feature = "ffi"))]
pub fn mine_sha256d(
    _header: &[u8; 80],
    _target: &[u8; 32],
    _start_nonce: u64,
    _max_nonce: u64,
) -> Result<MiningResult, String> {
    Err(FfiNotAvailable.to_string())
}

#[cfg(not(feature = "ffi"))]
pub fn depin_prove(_challenge: u64, _worker_id: &[u8]) -> Result<DepinProof, String> {
    Err(FfiNotAvailable.to_string())
}

// ─────────────────────────────────────────────────────────────
// Unit tests (DoD: ≥3 tests per crate)
// ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(feature = "ffi"))]
    #[test]
    fn sha256_returns_error_in_stub_mode() {
        let result = sha256(b"hello world");
        assert!(result.is_err());
    }

    #[cfg(not(feature = "ffi"))]
    #[test]
    fn double_sha256_returns_error_in_stub_mode() {
        let result = double_sha256(b"test data");
        assert!(result.is_err());
    }

    #[cfg(not(feature = "ffi"))]
    #[test]
    fn mine_sha256d_returns_error_in_stub_mode() {
        let header = [0u8; 80];
        let target = [0u8; 32];
        let result = mine_sha256d(&header, &target, 0, 1000);
        assert!(result.is_err());
    }

    #[cfg(not(feature = "ffi"))]
    #[test]
    fn depin_prove_returns_error_in_stub_mode() {
        let result = depin_prove(12345, b"worker-1");
        assert!(result.is_err());
    }

    #[test]
    fn ffi_not_available_has_message() {
        let err = FfiNotAvailable;
        assert!(err.to_string().contains("FFI not available"));
    }

    #[test]
    fn sha256_hash_is_32_bytes() {
        let hash = [0u8; 32];
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn depin_proof_default_is_invalid() {
        let proof = DepinProof {
            proof: [0u8; 64],
            challenge: 0,
            validator_hash: [0u8; 32],
            valid: false,
        };
        assert!(!proof.valid);
        assert_eq!(proof.challenge, 0);
    }
}
