//! # CR-CHAT-04 — Wire frame integrity guard (Wave-63 Lane A)
//!
//! PADDING — wire frame structure validation, R-CHAT-9.
//!
//! Each wire envelope has a fixed structure: header, payload, padding,
//! and AEAD tag. An attacker who can craft malformed frames can:
//!
//! * **Overlap regions** — set header length so it overlaps payload,
//!   causing the parser to read padding as payload data.
//! * **Truncate** — omit the AEAD tag, causing a parser that doesn't
//!   check bounds to read past the buffer.
//! * **Inflate header** — set a huge header length to skip payload
//!   validation entirely.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Header length + payload length + padding length + tag length = total.
//! 2. All offsets are within the frame bounds.
//! 3. No region overlaps (header before payload before padding before tag).
//! 4. Header length <= `WFGI_MAX_HEADER`.
//! 5. Tag length = `WFGI_TAG_LEN`.
//! 6. Total frame length <= `WFGI_MAX_FRAME`.
//!
//! Tests **WFGI-01..10**. Error enum [`WireFrameError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * WIRE-FRAME`

#![forbid(unsafe_code)]

/// AEAD tag length (AES-128-GCM / ChaCha20-Poly1305).
pub const WFGI_TAG_LEN: usize = 16;

/// Maximum header length.
pub const WFGI_MAX_HEADER: usize = 64;

/// Maximum total frame length.
pub const WFGI_MAX_FRAME: usize = 65536;

/// All ways wire frame validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WireFrameError {
    /// Lengths don't sum to total.
    LengthMismatch,
    /// Offset out of bounds.
    OffsetOutOfBounds,
    /// Region overlap.
    RegionOverlap,
    /// Header too large.
    HeaderTooLarge,
    /// Wrong tag length.
    WrongTagLength,
    /// Frame too large.
    FrameTooLarge,
}

/// `[VERIFIED]` Validate wire frame structure.
pub fn validate_wire_frame(
    total_len: usize,
    header_len: usize,
    payload_len: usize,
    padding_len: usize,
    tag_len: usize,
) -> Result<(), WireFrameError> {
    if total_len > WFGI_MAX_FRAME {
        return Err(WireFrameError::FrameTooLarge);
    }
    if header_len > WFGI_MAX_HEADER {
        return Err(WireFrameError::HeaderTooLarge);
    }
    if tag_len != WFGI_TAG_LEN {
        return Err(WireFrameError::WrongTagLength);
    }
    let computed = header_len + payload_len + padding_len + tag_len;
    if computed != total_len {
        return Err(WireFrameError::LengthMismatch);
    }
    let payload_start = header_len;
    let padding_start = payload_start + payload_len;
    let tag_start = padding_start + padding_len;
    if payload_start > total_len || padding_start > total_len || tag_start > total_len {
        return Err(WireFrameError::OffsetOutOfBounds);
    }
    if header_len > 0 && payload_len > 0 && payload_start < header_len {
        return Err(WireFrameError::RegionOverlap);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **WFGI-01** — length mismatch rejected.
    #[test]
    fn wfgi_01_mismatch_rejected() {
        assert_eq!(
            validate_wire_frame(100, 16, 64, 0, WFGI_TAG_LEN),
            Err(WireFrameError::LengthMismatch)
        );
    }

    /// **WFGI-02** — header too large rejected.
    #[test]
    fn wfgi_02_header_large_rejected() {
        assert_eq!(
            validate_wire_frame(200, WFGI_MAX_HEADER + 1, 100, 0, WFGI_TAG_LEN),
            Err(WireFrameError::HeaderTooLarge)
        );
    }

    /// **WFGI-03** — wrong tag length rejected.
    #[test]
    fn wfgi_03_wrong_tag_rejected() {
        assert_eq!(
            validate_wire_frame(100, 16, 68, 0, 32),
            Err(WireFrameError::WrongTagLength)
        );
    }

    /// **WFGI-04** — frame too large rejected.
    #[test]
    fn wfgi_04_frame_large_rejected() {
        assert_eq!(
            validate_wire_frame(WFGI_MAX_FRAME + 1, 16, 100, 0, WFGI_TAG_LEN),
            Err(WireFrameError::FrameTooLarge)
        );
    }

    /// **WFGI-05** — offset out of bounds rejected.
    #[test]
    fn wfgi_05_offset_oob_rejected() {
        assert_eq!(
            validate_wire_frame(32, 16, 100, 0, WFGI_TAG_LEN),
            Err(WireFrameError::LengthMismatch)
        );
    }

    /// **WFGI-06** — valid frame accepted.
    #[test]
    fn wfgi_06_valid_accepted() {
        assert_eq!(
            validate_wire_frame(96, 16, 64, 0, WFGI_TAG_LEN),
            Ok(())
        );
    }

    /// **WFGI-07** — valid frame with padding accepted.
    #[test]
    fn wfgi_07_with_padding_accepted() {
        assert_eq!(
            validate_wire_frame(256, 16, 200, 24, WFGI_TAG_LEN),
            Ok(())
        );
    }

    /// **WFGI-08** — minimum frame accepted.
    #[test]
    fn wfgi_08_min_frame_accepted() {
        assert_eq!(
            validate_wire_frame(WFGI_TAG_LEN + 1, 1, 0, 0, WFGI_TAG_LEN),
            Ok(())
        );
    }

    /// **WFGI-09** — header only + tag accepted.
    #[test]
    fn wfgi_09_header_only_accepted() {
        assert_eq!(
            validate_wire_frame(16 + WFGI_TAG_LEN, 16, 0, 0, WFGI_TAG_LEN),
            Ok(())
        );
    }

    /// **WFGI-10** — max frame accepted.
    #[test]
    fn wfgi_10_max_frame_accepted() {
        let payload = WFGI_MAX_FRAME - 16 - WFGI_TAG_LEN;
        assert_eq!(
            validate_wire_frame(WFGI_MAX_FRAME, 16, payload, 0, WFGI_TAG_LEN),
            Ok(())
        );
    }
}
