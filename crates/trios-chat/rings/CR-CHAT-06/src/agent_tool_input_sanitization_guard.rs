//! # CR-CHAT-06 — Agent tool input sanitization guard (Wave-93 Lane B)
//!
//! AGENT SAFETY — tool inputs must pass sanitization, R-CHAT-7.
//!
//! Tool inputs from the agent (or user via agent) must be sanitized
//! before being passed to the tool execution layer. Without sanitization:
//!
//! * **Command injection** — special characters in inputs enable shell
//!   injection when tools execute system commands.
//! * **Path traversal** — `../` sequences in file paths allow access
//!   to files outside the intended directory.
//! * **Format string** — `%s`, `{}` patterns in inputs can cause
//!   undefined behavior in tools that use format strings.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. No control characters (0x00-0x1F except whitespace).
//! 2. No path traversal sequences (`..`).
//! 3. Input length <= `ATIS_MAX_INPUT_LEN`.
//! 4. Input length >= 1 (non-empty).
//! 5. No null bytes.
//! 6. Sanitized inputs per session <= `ATIS_MAX_INPUTS`.
//!
//! Tests **ATIS-01..10**. Error enum [`SanitizationError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * INPUT-SANITIZE`

#![forbid(unsafe_code)]

/// Maximum input length.
pub const ATIS_MAX_INPUT_LEN: usize = 65_536;

/// Maximum inputs per batch.
pub const ATIS_MAX_INPUTS: usize = 1024;

/// A tool input record.
#[derive(Debug, Clone)]
pub struct ToolInput {
    /// The raw input string.
    pub raw: String,
    /// The tool name.
    pub tool: String,
}

/// All ways sanitization validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SanitizationError {
    /// Control character found.
    ControlChar { tool: String, position: usize },
    /// Path traversal detected.
    PathTraversal { tool: String },
    /// Input too long.
    TooLong { tool: String, len: usize, max: usize },
    /// Empty input.
    EmptyInput { tool: String },
    /// Null byte found.
    NullByte { tool: String, position: usize },
    /// Too many inputs.
    TooManyInputs,
}

fn is_control(b: u8) -> bool {
    (b < 0x20 && b != b'\t' && b != b'\n' && b != b'\r') || b == 0x7F
}

/// `[VERIFIED]` Validate tool input sanitization.
pub fn validate_tool_inputs(
    inputs: &[ToolInput],
) -> Result<(), SanitizationError> {
    if inputs.len() > ATIS_MAX_INPUTS {
        return Err(SanitizationError::TooManyInputs);
    }
    for inp in inputs {
        if inp.raw.is_empty() {
            return Err(SanitizationError::EmptyInput { tool: inp.tool.clone() });
        }
        if inp.raw.len() > ATIS_MAX_INPUT_LEN {
            return Err(SanitizationError::TooLong {
                tool: inp.tool.clone(),
                len: inp.raw.len(),
                max: ATIS_MAX_INPUT_LEN,
            });
        }
        for (i, &b) in inp.raw.as_bytes().iter().enumerate() {
            if b == 0 {
                return Err(SanitizationError::NullByte {
                    tool: inp.tool.clone(),
                    position: i,
                });
            }
            if is_control(b) {
                return Err(SanitizationError::ControlChar {
                    tool: inp.tool.clone(),
                    position: i,
                });
            }
        }
        if inp.raw.contains("..") {
            return Err(SanitizationError::PathTraversal { tool: inp.tool.clone() });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(tool: &str, raw: &str) -> ToolInput {
        ToolInput { tool: tool.to_string(), raw: raw.to_string() }
    }

    fn input_owned(tool: &str, raw: String) -> ToolInput {
        ToolInput { tool: tool.to_string(), raw }
    }

    fn valid_inputs() -> Vec<ToolInput> {
        vec![input("read_file", "/home/user/file.txt"), input("search", "hello world")]
    }

    /// **ATIS-01** — control char rejected.
    #[test]
    fn atis_01_control_char_rejected() {
        let inp = input("tool", "hello\x01world");
        assert_eq!(
            validate_tool_inputs(&[inp]),
            Err(SanitizationError::ControlChar { tool: "tool".to_string(), position: 5 })
        );
    }

    /// **ATIS-02** — path traversal rejected.
    #[test]
    fn atis_02_path_traversal_rejected() {
        let inp = input("read_file", "/home/../etc/passwd");
        assert_eq!(
            validate_tool_inputs(&[inp]),
            Err(SanitizationError::PathTraversal { tool: "read_file".to_string() })
        );
    }

    /// **ATIS-03** — too long rejected.
    #[test]
    fn atis_03_too_long_rejected() {
        let inp = input_owned("tool", "a".repeat(ATIS_MAX_INPUT_LEN + 1));
        assert_eq!(
            validate_tool_inputs(&[inp]),
            Err(SanitizationError::TooLong {
                tool: "tool".to_string(),
                len: ATIS_MAX_INPUT_LEN + 1,
                max: ATIS_MAX_INPUT_LEN,
            })
        );
    }

    /// **ATIS-04** — empty input rejected.
    #[test]
    fn atis_04_empty_rejected() {
        let inp = input("tool", "");
        assert_eq!(
            validate_tool_inputs(&[inp]),
            Err(SanitizationError::EmptyInput { tool: "tool".to_string() })
        );
    }

    /// **ATIS-05** — null byte rejected.
    #[test]
    fn atis_05_null_byte_rejected() {
        let inp = input("tool", "hello\x00world");
        assert_eq!(
            validate_tool_inputs(&[inp]),
            Err(SanitizationError::NullByte { tool: "tool".to_string(), position: 5 })
        );
    }

    /// **ATIS-06** — too many inputs rejected.
    #[test]
    fn atis_06_too_many_rejected() {
        let inputs: Vec<ToolInput> = (0..=ATIS_MAX_INPUTS)
            .map(|i| input("tool", &format!("input_{i}")))
            .collect();
        assert_eq!(validate_tool_inputs(&inputs), Err(SanitizationError::TooManyInputs));
    }

    /// **ATIS-07** — valid inputs accepted.
    #[test]
    fn atis_07_valid_accepted() {
        assert_eq!(validate_tool_inputs(&valid_inputs()), Ok(()));
    }

    /// **ATIS-08** — empty batch accepted.
    #[test]
    fn atis_08_empty_accepted() {
        assert_eq!(validate_tool_inputs(&[]), Ok(()));
    }

    /// **ATIS-09** — whitespace accepted (tabs, newlines allowed).
    #[test]
    fn atis_09_whitespace_accepted() {
        let inp = input("tool", "hello\tworld\nfoo\rbar");
        assert_eq!(validate_tool_inputs(&[inp]), Ok(()));
    }

    /// **ATIS-10** — max length boundary accepted.
    #[test]
    fn atis_10_max_len_accepted() {
        let inp = input_owned("tool", "a".repeat(ATIS_MAX_INPUT_LEN));
        assert_eq!(validate_tool_inputs(&[inp]), Ok(()));
    }
}
