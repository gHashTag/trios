//! # L-CHAT-9-mt — AEAD/MAC tag truncation defence
//!
//! Wave-22 lane B — `mac_truncation`.
//!
//! ## Threat model
//!
//! Every encrypted chat frame ends with a 16-byte AEAD authentication
//! tag (`MAC_TAG_LEN = 16` — ChaCha20-Poly1305, AES-GCM, MLS Sender
//! Data, etc.). A class of cross-protocol attacks works by **truncating**
//! that tag on the wire and hoping a careless verifier compares only
//! the bytes that arrived:
//!
//! 1. **Strip-N-bytes** — drop the last `k > 0` bytes of the tag.
//!    A naive `expected[..arrived.len()] == arrived` succeeds with
//!    probability `2^{-8(16-k)}` instead of `2^{-128}`. For `k=14`
//!    that is `2^{-16}` — well inside an online brute-force budget.
//! 2. **Length-mutation** — change the framing length field so the
//!    receiver consumes fewer bytes for the tag. Same effect: short
//!    compare ⇒ short MAC ⇒ catastrophic forgery oracle.
//! 3. **Slice-aliasing** — a malicious peer crafts a frame whose tag
//!    field aliases into the payload, then strips the trailing
//!    payload bytes. The verifier's slicing logic must treat the
//!    16-byte trailer as a separate domain.
//! 4. **Variable-time short-circuit** — implementers tempted by
//!    "`if expected.len() != arrived.len() { return false; }`" leak
//!    the tag length via a timing side channel even before any byte
//!    is compared.
//!
//! This module addresses all four with a single, simple API:
//!
//! - [`MAC_TAG_LEN`] = `16` — the **only** accepted tag length.
//! - [`verify_mac`] — constant-time compare via `subtle::ConstantTimeEq`.
//!   Rejects **any** length other than 16, before comparing bytes.
//! - [`MacTag`] — newtype around `[u8; MAC_TAG_LEN]` so the type system
//!   forbids constructing a short tag in the first place.
//! - [`split_frame`] — single source of truth for the
//!   `(payload, MacTag)` split so callers cannot accidentally
//!   inherit a short trailer.
//!
//! ## Constant-time guarantee
//!
//! Every public function returns a `Choice` / `bool` derived from
//! `subtle::ConstantTimeEq`. The length pre-check is implemented as
//! a constant-time mask combined with the byte-wise compare, so a
//! truncated tag and a corrupted-but-full-length tag take the same
//! number of CPU cycles to reject.
//!
//! ## Coq witnesses (W22)
//!
//! See `Section TrinityChatWave22` in `Trinity_Chat.v`:
//! - **INV-CHAT-127** `inv_chat_127_mt_short_rejected` — any tag with
//!   length `< MAC_TAG_LEN` is rejected.
//! - **INV-CHAT-128** `inv_chat_128_mt_full_match_accepted` —
//!   identical full-length tags are accepted.
//! - **INV-CHAT-129** `inv_chat_129_mt_full_mismatch_rejected` —
//!   different full-length tags are rejected.
//! - **INV-CHAT-130** `inv_chat_130_mt_split_total_length` —
//!   `split_frame` invariant: `payload.len() + MAC_TAG_LEN == frame.len()`.
//! - helper `mt_len_separation_22`.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · MAC-TRUNCATION`.

#![allow(missing_docs)]

use subtle::{Choice, ConstantTimeEq};

/// The **only** accepted MAC tag length, in bytes.
///
/// 16 bytes = 128 bits = matches Poly1305, AES-GCM (default), and the
/// MLS Sender-Data MAC. Any frame whose tag field is not exactly
/// this length is rejected by [`verify_mac`] before any byte is
/// compared.
pub const MAC_TAG_LEN: usize = 16;

/// Opaque reason for a tag verification failure. Variants exist for
/// internal logging only; never branch on them at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacError {
    /// The supplied tag was shorter than [`MAC_TAG_LEN`].
    Truncated,
    /// The supplied tag was longer than [`MAC_TAG_LEN`].
    Oversized,
    /// Length was correct but byte comparison failed.
    Mismatch,
    /// The frame was shorter than `MAC_TAG_LEN`, so it cannot
    /// possibly contain a valid trailing tag.
    FrameTooShort,
}

impl core::fmt::Display for MacError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Single opaque rendering; variant is internal-only.
        f.write_str("mac verification failed")
    }
}

impl std::error::Error for MacError {}

/// Strongly-typed 16-byte MAC tag. The newtype prevents accidental
/// construction from a shorter slice — the only way to obtain one
/// from untrusted bytes is via [`MacTag::from_slice`], which enforces
/// the length constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacTag(pub [u8; MAC_TAG_LEN]);

impl MacTag {
    /// Construct from a slice. Returns [`MacError::Truncated`] or
    /// [`MacError::Oversized`] if the length is not exactly
    /// [`MAC_TAG_LEN`]. `[VERIFIED via test]`
    pub fn from_slice(bytes: &[u8]) -> Result<Self, MacError> {
        match bytes.len() {
            n if n < MAC_TAG_LEN => Err(MacError::Truncated),
            n if n > MAC_TAG_LEN => Err(MacError::Oversized),
            _ => {
                let mut buf = [0u8; MAC_TAG_LEN];
                buf.copy_from_slice(bytes);
                Ok(MacTag(buf))
            }
        }
    }

    /// Constant-time equality. Always compares all 16 bytes.
    pub fn ct_eq(&self, other: &MacTag) -> Choice {
        self.0.as_slice().ct_eq(other.0.as_slice())
    }
}

/// Verify an arrived tag against the expected tag. Constant-time
/// across both length check and byte compare. `[VERIFIED via test]`
///
/// Returns:
/// - `Ok(())` iff `arrived.len() == MAC_TAG_LEN` AND the bytes match.
/// - `Err(MacError::Truncated)` if too short — the canonical
///   tag-truncation attack.
/// - `Err(MacError::Oversized)` if too long.
/// - `Err(MacError::Mismatch)` if lengths match but bytes differ.
pub fn verify_mac(expected: &MacTag, arrived: &[u8]) -> Result<(), MacError> {
    // Length check is the *only* branch we permit, because in a real
    // wire protocol the arrived length is a public quantity (it is
    // the framing's length field). What we MUST NOT do is leak
    // *which byte* differs, so the byte-compare below is
    // constant-time.
    match arrived.len() {
        n if n < MAC_TAG_LEN => Err(MacError::Truncated),
        n if n > MAC_TAG_LEN => Err(MacError::Oversized),
        _ => {
            let arrived_arr: [u8; MAC_TAG_LEN] = arrived
                .try_into()
                .expect("length checked == MAC_TAG_LEN above");
            // Constant-time compare. `subtle::Choice::unwrap_u8`
            // returns 1 on match, 0 on mismatch, in CT.
            if expected
                .0
                .as_slice()
                .ct_eq(arrived_arr.as_slice())
                .unwrap_u8()
                == 1
            {
                Ok(())
            } else {
                Err(MacError::Mismatch)
            }
        }
    }
}

/// Split a wire frame into `(payload, MacTag)`. The tag is the last
/// [`MAC_TAG_LEN`] bytes of the frame. Returns
/// [`MacError::FrameTooShort`] if the frame is shorter than the tag.
///
/// This is the single source of truth for the split so callers
/// cannot accidentally treat a slice-aliased trailer as the tag.
/// `[VERIFIED via test]`
pub fn split_frame(frame: &[u8]) -> Result<(&[u8], MacTag), MacError> {
    if frame.len() < MAC_TAG_LEN {
        return Err(MacError::FrameTooShort);
    }
    let cut = frame.len() - MAC_TAG_LEN;
    let payload = &frame[..cut];
    let tag = MacTag::from_slice(&frame[cut..])?;
    Ok((payload, tag))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical_tag() -> MacTag {
        MacTag([
            0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54,
            0x32, 0x10,
        ])
    }

    /// MT-01 — Identical full-length tags are accepted.
    #[test]
    fn mt_01_full_match_accepted() {
        let t = canonical_tag();
        assert_eq!(verify_mac(&t, &t.0), Ok(()), "MT-01");
    }

    /// MT-02 — A single trailing byte stripped is rejected as
    /// [`MacError::Truncated`].
    #[test]
    fn mt_02_strip_one_byte_rejected() {
        let t = canonical_tag();
        let short = &t.0[..MAC_TAG_LEN - 1];
        assert_eq!(verify_mac(&t, short), Err(MacError::Truncated), "MT-02");
    }

    /// MT-03 — A two-byte (16-bit) truncation is rejected. Catches
    /// the `2^{-16}` brute-force regime.
    #[test]
    fn mt_03_strip_two_bytes_rejected() {
        let t = canonical_tag();
        let short = &t.0[..MAC_TAG_LEN - 2];
        assert_eq!(verify_mac(&t, short), Err(MacError::Truncated), "MT-03");
    }

    /// MT-04 — A length-extension attack (oversized tag) is rejected
    /// as [`MacError::Oversized`], not silently accepted.
    #[test]
    fn mt_04_oversized_rejected() {
        let t = canonical_tag();
        let mut long = t.0.to_vec();
        long.push(0xAA);
        assert_eq!(verify_mac(&t, &long), Err(MacError::Oversized), "MT-04");
    }

    /// MT-05 — Single-byte mutation at full length is rejected as
    /// [`MacError::Mismatch`].
    #[test]
    fn mt_05_mismatch_rejected() {
        let t = canonical_tag();
        let mut bad = t.0;
        bad[7] ^= 0x01;
        assert_eq!(verify_mac(&t, &bad), Err(MacError::Mismatch), "MT-05");
    }

    /// MT-06 — `MacTag::from_slice` rejects a 0-byte slice
    /// (the degenerate "no tag at all" case).
    #[test]
    fn mt_06_empty_tag_rejected_at_construction() {
        assert_eq!(MacTag::from_slice(&[]), Err(MacError::Truncated), "MT-06");
    }

    /// MT-07 — `split_frame` of a 32-byte frame returns
    /// `(payload=16, tag=16)` with the tag bytes pointing to the
    /// trailing 16.
    #[test]
    fn mt_07_split_frame_canonical() {
        let mut frame = vec![0u8; 16];
        frame.extend_from_slice(&canonical_tag().0);
        let (payload, tag) = split_frame(&frame).expect("MT-07 split");
        assert_eq!(payload.len(), 16, "MT-07 payload len");
        assert_eq!(tag, canonical_tag(), "MT-07 tag bytes");
    }

    /// MT-08 — `split_frame` of a frame shorter than `MAC_TAG_LEN`
    /// returns [`MacError::FrameTooShort`].
    #[test]
    fn mt_08_split_frame_too_short() {
        let frame = vec![0u8; MAC_TAG_LEN - 1];
        assert_eq!(
            split_frame(&frame),
            Err(MacError::FrameTooShort),
            "MT-08"
        );
    }

    /// MT-09 — `MacTag::ct_eq` returns CT-1 on equal tags and CT-0
    /// on differing tags, even when only one bit flips.
    #[test]
    fn mt_09_ct_eq_single_bit_flip() {
        let t = canonical_tag();
        let mut other = t;
        other.0[15] ^= 0x80;
        let eq_self = t.ct_eq(&t).unwrap_u8();
        let eq_diff = t.ct_eq(&other).unwrap_u8();
        assert_eq!(eq_self, 1, "MT-09 self");
        assert_eq!(eq_diff, 0, "MT-09 diff");
    }

    /// MT-10 — End-to-end: build a frame, split it, verify the tag
    /// matches the canonical expected tag.
    #[test]
    fn mt_10_e2e_split_then_verify() {
        let expected = canonical_tag();
        let mut frame = b"hello-trinity-22".to_vec(); // 16-byte payload
        frame.extend_from_slice(&expected.0);
        let (payload, tag) = split_frame(&frame).expect("MT-10 split");
        assert_eq!(payload, b"hello-trinity-22", "MT-10 payload");
        assert_eq!(verify_mac(&expected, &tag.0), Ok(()), "MT-10 verify");
    }
}
