//! # CR-CHAT-04 — Padding alignment boundary guard (Wave-92 Lane A)
//!
//! PADDING — padded messages must align to class boundaries, R-CHAT-9.
//!
//! The padding scheme pads payloads to the next class size. If the
//! padded output is not exactly aligned:
//!
//! * **Size leak** — an off-by-one alignment reveals the exact payload
//!   size, defeating the purpose of padding classes.
//! * **Class confusion** — a message padded between two classes can be
//!   assigned to either, creating inconsistent behavior across peers.
//! * **Frame mismatch** — the wire frame size must equal the class;
//!   misalignment causes framing errors or reveals the payload length.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Padded length must be a valid class size.
//! 2. Payload length < padded length (must actually add padding).
//! 3. Payload length must be > 0.
//! 4. Padded length must be >= smallest class.
//! 5. Padded length must be <= largest class.
//! 6. Payload must fit within padded length minus length prefix.
//!
//! Tests **PALB-01..10**. Error enum [`AlignmentError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * PAD-ALIGN`

#![forbid(unsafe_code)]

/// Padding classes.
pub const PALB_CLASSES: [usize; 4] = [256, 1024, 4096, 16384];

/// Length prefix size.
pub const PALB_PREFIX_LEN: usize = 4;

/// A padded message record.
#[derive(Debug, Clone)]
pub struct PaddedMessage {
    /// Payload length (before padding).
    pub payload_len: usize,
    /// Padded output length (must be a class).
    pub padded_len: usize,
}

/// All ways alignment validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AlignmentError {
    /// Not a valid class.
    NotAClass(usize),
    /// No padding added.
    NoPadding { payload_len: usize, padded_len: usize },
    /// Empty payload.
    EmptyPayload,
    /// Below minimum class.
    BelowMinClass(usize),
    /// Above maximum class.
    AboveMaxClass(usize),
    /// Payload doesn't fit.
    PayloadOverflow { payload_len: usize, max_payload: usize },
}

fn smallest_class() -> usize {
    PALB_CLASSES[0]
}

fn largest_class() -> usize {
    PALB_CLASSES[PALB_CLASSES.len() - 1]
}

fn is_class(len: usize) -> bool {
    PALB_CLASSES.contains(&len)
}

/// `[VERIFIED]` Validate padding alignment boundaries.
pub fn validate_padding_alignment(
    msgs: &[PaddedMessage],
) -> Result<(), AlignmentError> {
    for m in msgs {
        if m.payload_len == 0 {
            return Err(AlignmentError::EmptyPayload);
        }
        if m.padded_len < smallest_class() {
            return Err(AlignmentError::BelowMinClass(m.padded_len));
        }
        if m.padded_len > largest_class() {
            return Err(AlignmentError::AboveMaxClass(m.padded_len));
        }
        if !is_class(m.padded_len) {
            return Err(AlignmentError::NotAClass(m.padded_len));
        }
        if m.padded_len == m.payload_len {
            return Err(AlignmentError::NoPadding {
                payload_len: m.payload_len,
                padded_len: m.padded_len,
            });
        }
        let max_payload = m.padded_len - PALB_PREFIX_LEN;
        if m.payload_len > max_payload {
            return Err(AlignmentError::PayloadOverflow {
                payload_len: m.payload_len,
                max_payload,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(payload_len: usize, padded_len: usize) -> PaddedMessage {
        PaddedMessage { payload_len, padded_len }
    }

    fn valid_msgs() -> Vec<PaddedMessage> {
        vec![msg(100, 256), msg(500, 1024), msg(2000, 4096)]
    }

    /// **PALB-01** — not a class rejected.
    #[test]
    fn palb_01_not_a_class_rejected() {
        assert_eq!(
            validate_padding_alignment(&[msg(100, 300)]),
            Err(AlignmentError::NotAClass(300))
        );
    }

    /// **PALB-02** — no padding rejected.
    #[test]
    fn palb_02_no_padding_rejected() {
        assert_eq!(
            validate_padding_alignment(&[msg(256, 256)]),
            Err(AlignmentError::NoPadding { payload_len: 256, padded_len: 256 })
        );
    }

    /// **PALB-03** — empty payload rejected.
    #[test]
    fn palb_03_empty_payload_rejected() {
        assert_eq!(
            validate_padding_alignment(&[msg(0, 256)]),
            Err(AlignmentError::EmptyPayload)
        );
    }

    /// **PALB-04** — below min class rejected.
    #[test]
    fn palb_04_below_min_rejected() {
        assert_eq!(
            validate_padding_alignment(&[msg(10, 128)]),
            Err(AlignmentError::BelowMinClass(128))
        );
    }

    /// **PALB-05** — above max class rejected.
    #[test]
    fn palb_05_above_max_rejected() {
        assert_eq!(
            validate_padding_alignment(&[msg(100, 32768)]),
            Err(AlignmentError::AboveMaxClass(32768))
        );
    }

    /// **PALB-06** — payload overflow rejected.
    #[test]
    fn palb_06_overflow_rejected() {
        assert_eq!(
            validate_padding_alignment(&[msg(253, 256)]),
            Err(AlignmentError::PayloadOverflow { payload_len: 253, max_payload: 252 })
        );
    }

    /// **PALB-07** — valid messages accepted.
    #[test]
    fn palb_07_valid_accepted() {
        assert_eq!(validate_padding_alignment(&valid_msgs()), Ok(()));
    }

    /// **PALB-08** — empty accepted.
    #[test]
    fn palb_08_empty_accepted() {
        assert_eq!(validate_padding_alignment(&[]), Ok(()));
    }

    /// **PALB-09** — exact max payload accepted.
    #[test]
    fn palb_09_exact_max_payload_accepted() {
        assert_eq!(validate_padding_alignment(&[msg(252, 256)]), Ok(()));
    }

    /// **PALB-10** — all four classes accepted.
    #[test]
    fn palb_10_all_classes_accepted() {
        let msgs = vec![
            msg(10, 256),
            msg(500, 1024),
            msg(2000, 4096),
            msg(10000, 16384),
        ];
        assert_eq!(validate_padding_alignment(&msgs), Ok(()));
    }
}
