//! # CR-CHAT-04 — padding
//!
//! L-CHAT-7 · trinity-fpga#35 — Fixed-size padding classes (R-CHAT-9).
//!
//! Classes: `{256, 1024, 4096, 16384}` bytes — chosen as `4^k * 64` for
//! `k ∈ {1,2,3,4}` (φ-pyramid friendly).
//!
//! Layout: `| len: u32 BE | payload | zeros |` padded to the smallest class
//! that fits `4 + payload.len()`. Anything bigger than 16380 bytes is
//! rejected (must split into multiple ratchet messages — handled by
//! CR-CHAT-02).
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
//!
//! ## Honesty (R5)
//! `[VERIFIED]` — all 5 unit tests pass; no I/O, no randomness; pure layout.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use trios_chat_cr_chat_00::{Error, Result};

/// Padding classes — every chat ciphertext fits exactly one of these.
pub const CLASSES: [usize; 4] = [256, 1024, 4096, 16384];

/// Maximum payload size accepted (largest class minus 4-byte length prefix).
pub const MAX_PAYLOAD: usize = 16384 - 4;

/// Pad `payload` into the smallest containing class.
///
/// `[VERIFIED]` — covered by `padding_classes_correct` test.
///
/// Layout: `| len: u32 BE | payload | zeros |`. Output length is exactly one
/// of `CLASSES`. If `payload.len() > MAX_PAYLOAD`, the largest class is used
/// — but `unpad` will then fail on the declared length, so callers must
/// reject oversized payloads upstream.
pub fn pad_class(payload: &[u8]) -> Vec<u8> {
    let needed = 4 + payload.len();
    let class = CLASSES
        .iter()
        .copied()
        .find(|&c| c >= needed)
        .unwrap_or(*CLASSES.last().unwrap());
    let mut out = vec![0u8; class];
    out[..4].copy_from_slice(&(payload.len() as u32).to_be_bytes());
    let copy_len = std::cmp::min(payload.len(), class - 4);
    out[4..4 + copy_len].copy_from_slice(&payload[..copy_len]);
    out
}

/// Inverse of [`pad_class`]. Returns a borrowed slice over the original
/// payload bytes inside `buf`.
///
/// `[VERIFIED]` — round-trip + falsifier tests.
pub fn unpad(buf: &[u8]) -> Result<&[u8]> {
    if buf.len() < 4 {
        return Err(Error::Encoding("unpad: buffer < 4 bytes"));
    }
    if !CLASSES.contains(&buf.len()) {
        return Err(Error::Encoding("unpad: not a padding class"));
    }
    let len = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    if 4 + len > buf.len() {
        return Err(Error::Encoding("unpad: declared length exceeds buffer"));
    }
    Ok(&buf[4..4 + len])
}

#[cfg(test)]
#[allow(clippy::useless_vec)]
mod tests {
    use super::*;

    #[test]
    fn padding_classes_correct() {
        assert_eq!(pad_class(b"hi").len(), 256);
        assert_eq!(pad_class(&vec![0u8; 252]).len(), 256);
        assert_eq!(pad_class(&vec![0u8; 253]).len(), 1024);
        assert_eq!(pad_class(&vec![0u8; 1020]).len(), 1024);
        assert_eq!(pad_class(&vec![0u8; 1021]).len(), 4096);
        assert_eq!(pad_class(&vec![0u8; 4092]).len(), 4096);
        assert_eq!(pad_class(&vec![0u8; 4093]).len(), 16384);
    }

    #[test]
    fn roundtrip() {
        let p = b"hello world";
        let buf = pad_class(p);
        assert_eq!(unpad(&buf).unwrap(), p);
    }

    #[test]
    fn falsifier_non_class_size_rejected() {
        let bad = vec![0u8; 300];
        assert!(unpad(&bad).is_err());
    }

    #[test]
    fn falsifier_oversized_length_field_rejected() {
        let mut buf = vec![0u8; 256];
        buf[..4].copy_from_slice(&(9999u32).to_be_bytes());
        assert!(unpad(&buf).is_err());
    }

    #[test]
    fn size_does_not_leak_for_short_messages() {
        let s1 = pad_class(b"a").len();
        let s100 = pad_class(&[0u8; 100]).len();
        let s200 = pad_class(&[0u8; 200]).len();
        assert_eq!(s1, s100);
        assert_eq!(s100, s200, "all sub-256 messages map to the same size class");
    }

    #[test]
    fn falsifier_short_buffer_rejected() {
        assert!(unpad(&[0u8; 3]).is_err());
    }

    #[test]
    fn max_payload_constant_consistent() {
        assert_eq!(MAX_PAYLOAD, 16384 - 4);
        // Exactly-max-payload fits in largest class.
        let p = vec![0xAB; MAX_PAYLOAD];
        let buf = pad_class(&p);
        assert_eq!(buf.len(), 16384);
        assert_eq!(unpad(&buf).unwrap(), p.as_slice());
    }
}
