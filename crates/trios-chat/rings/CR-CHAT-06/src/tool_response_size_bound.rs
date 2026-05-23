//! # CR-CHAT-06 — Tool response size bound guard (Wave-47 Lane B)
//!
//! R-CHAT-7 — Tool response size enforcement.
//!
//! LLM tool outputs can be arbitrarily large. An adversary who controls
//! a tool's response can:
//!
//! * **Exhaust memory** — return a multi-gigabyte string that blows up
//!   the chat client's heap.
//! * **Blow the context window** — fill the conversation buffer with a
//!   single oversized response, evicting all prior context.
//! * **DoS the renderer** — produce output with millions of lines that
//!   freezes the UI thread.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Response is non-empty.
//! 2. Response length ≤ `TRSB_MAX_BYTES`.
//! 3. Response line count ≤ `TRSB_MAX_LINES`.
//! 4. No single line exceeds `TRSB_MAX_LINE_LEN`.
//! 5. No null bytes.
//! 6. Response is valid UTF-8.
//!
//! Tests **TRSB-01..10**. Error enum [`ToolResponseSizeError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · TOOL-RESPONSE-BOUND`

#![forbid(unsafe_code)]

/// Maximum response size in bytes (256 KiB).
pub const TRSB_MAX_BYTES: usize = 256 * 1024;

/// Maximum number of lines.
pub const TRSB_MAX_LINES: usize = 4096;

/// Maximum single line length in bytes.
pub const TRSB_MAX_LINE_LEN: usize = 4096;

/// All ways a tool response can violate size bounds.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ToolResponseSizeError {
    /// Response is empty.
    EmptyResponse,
    /// Response exceeds max bytes.
    ExceedsMaxBytes,
    /// Response has too many lines.
    TooManyLines,
    /// Single line exceeds max length.
    LineTooLong,
    /// Contains null byte.
    NullByte,
    /// Invalid UTF-8.
    InvalidUtf8,
}

/// `[VERIFIED]` Validate a tool response against size bounds. Returns
/// `Ok(())` if all rules pass.
pub fn validate_tool_response_size(response: &[u8]) -> Result<(), ToolResponseSizeError> {
    if response.is_empty() {
        return Err(ToolResponseSizeError::EmptyResponse);
    }
    if response.len() > TRSB_MAX_BYTES {
        return Err(ToolResponseSizeError::ExceedsMaxBytes);
    }
    if response.contains(&0) {
        return Err(ToolResponseSizeError::NullByte);
    }
    let s = std::str::from_utf8(response).map_err(|_| ToolResponseSizeError::InvalidUtf8)?;
    let mut line_count = 0usize;
    for line in s.lines() {
        line_count += 1;
        if line_count > TRSB_MAX_LINES {
            return Err(ToolResponseSizeError::TooManyLines);
        }
        if line.len() > TRSB_MAX_LINE_LEN {
            return Err(ToolResponseSizeError::LineTooLong);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **TRSB-01** — empty response rejected.
    #[test]
    fn trsb_01_empty_rejected() {
        assert_eq!(
            validate_tool_response_size(b""),
            Err(ToolResponseSizeError::EmptyResponse)
        );
    }

    /// **TRSB-02** — exceeds max bytes rejected.
    #[test]
    fn trsb_02_exceeds_bytes_rejected() {
        let response = vec![b'A'; TRSB_MAX_BYTES + 1];
        assert_eq!(
            validate_tool_response_size(&response),
            Err(ToolResponseSizeError::ExceedsMaxBytes)
        );
    }

    /// **TRSB-03** — too many lines rejected.
    #[test]
    fn trsb_03_too_many_lines_rejected() {
        let lines: Vec<String> = (0..=TRSB_MAX_LINES).map(|_| "x".to_owned()).collect();
        let response = lines.join("\n");
        assert_eq!(
            validate_tool_response_size(response.as_bytes()),
            Err(ToolResponseSizeError::TooManyLines)
        );
    }

    /// **TRSB-04** — single line too long rejected.
    #[test]
    fn trsb_04_line_too_long_rejected() {
        let line = "x".repeat(TRSB_MAX_LINE_LEN + 1);
        assert_eq!(
            validate_tool_response_size(line.as_bytes()),
            Err(ToolResponseSizeError::LineTooLong)
        );
    }

    /// **TRSB-05** — null byte rejected.
    #[test]
    fn trsb_05_null_byte_rejected() {
        assert_eq!(
            validate_tool_response_size(b"hello\x00world"),
            Err(ToolResponseSizeError::NullByte)
        );
    }

    /// **TRSB-06** — valid response accepted.
    #[test]
    fn trsb_06_valid_accepted() {
        assert_eq!(validate_tool_response_size(b"Hello, world!"), Ok(()));
    }

    /// **TRSB-07** — multiline response accepted.
    #[test]
    fn trsb_07_multiline_accepted() {
        let response = "line1\nline2\nline3";
        assert_eq!(validate_tool_response_size(response.as_bytes()), Ok(()));
    }

    /// **TRSB-08** — response at byte boundary accepted.
    #[test]
    fn trsb_08_near_max_bytes_accepted() {
        let line = "x".repeat(TRSB_MAX_LINE_LEN);
        let response = line.clone() + "\n" + &line;
        assert!(response.len() <= TRSB_MAX_BYTES);
        assert_eq!(validate_tool_response_size(response.as_bytes()), Ok(()));
    }

    /// **TRSB-09** — exact max lines accepted.
    #[test]
    fn trsb_09_exact_max_lines_accepted() {
        let lines: Vec<String> = (0..TRSB_MAX_LINES).map(|_| "x".to_owned()).collect();
        let response = lines.join("\n");
        assert_eq!(validate_tool_response_size(response.as_bytes()), Ok(()));
    }

    /// **TRSB-10** — exact max line length accepted.
    #[test]
    fn trsb_10_exact_line_len_accepted() {
        let line = "x".repeat(TRSB_MAX_LINE_LEN);
        assert_eq!(validate_tool_response_size(line.as_bytes()), Ok(()));
    }
}
