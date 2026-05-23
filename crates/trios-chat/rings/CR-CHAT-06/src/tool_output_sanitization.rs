//! # CR-CHAT-06 — Tool output sanitization guard (Wave-44 Lane B)
//!
//! R-CHAT-7 — Tool output sanitization for agent safety.
//!
//! LLM-generated tool outputs can contain injection payloads that exploit
//! downstream processing. An adversary who controls tool output can:
//!
//! * **Inject SQL** — craft tool output containing SQL injection patterns
//!   that are executed when stored or displayed.
//! * **Traverse paths** — embed `../` sequences to escape sandboxes.
//! * **Nest tool calls** — include the sentinel string that triggers
//!   recursive tool execution (see CR-CHAT-06 `tag_stripping`).
//! * **Insert control characters** — embed null bytes, escape sequences,
//!   or bidi overrides that corrupt downstream parsers.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Output is non-empty.
//! 2. Output length is within bounds (≤ 65535 bytes).
//! 3. No ASCII control characters (0x00..=0x1F except 0x0A, 0x0D).
//! 4. No nested tool call sentinel.
//! 5. No SQL injection patterns.
//! 6. No path traversal sequences.
//!
//! Tests **TOUT-01..10**. Error enum [`ToolOutputError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · TOOL-OUTPUT-SANITIZE`

#![forbid(unsafe_code)]

/// Maximum tool output length in bytes.
pub const TOUT_MAX_LEN: usize = 65535;

/// Nested tool call sentinel (from CR-CHAT-06 tag_stripping).
pub const TOUT_NESTED_SENTINEL: &str = "<<TOOL_CALL>>";

/// SQL injection patterns (case-insensitive prefixes).
const SQL_PATTERNS: &[&str] = &[
    "DROP TABLE",
    "DELETE FROM",
    "INSERT INTO",
    "UPDATE ",
    "UNION SELECT",
    "OR 1=1",
    "; --",
    "'; --",
];

/// Path traversal patterns.
const PATH_PATTERNS: &[&str] = &["../", "..\\", "/etc/passwd", "\\windows\\"];

/// All ways tool output can be rejected.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ToolOutputError {
    /// Output is empty.
    EmptyOutput,
    /// Output exceeds maximum length.
    OutputTooLong,
    /// Contains ASCII control characters (except newline/CR).
    ControlCharacter,
    /// Contains nested tool call sentinel.
    NestedToolCallSentinel,
    /// Contains SQL injection pattern.
    SqlInjectionPattern,
    /// Contains path traversal sequence.
    PathTraversal,
}

/// `[VERIFIED]` Sanitize tool output against injection patterns. Returns
/// `Ok(())` if all rules pass.
pub fn sanitize_tool_output(output: &[u8]) -> Result<(), ToolOutputError> {
    if output.is_empty() {
        return Err(ToolOutputError::EmptyOutput);
    }
    if output.len() > TOUT_MAX_LEN {
        return Err(ToolOutputError::OutputTooLong);
    }
    for &b in output {
        if b < 0x20 && b != 0x0A && b != 0x0D {
            return Err(ToolOutputError::ControlCharacter);
        }
    }
    let s = String::from_utf8_lossy(output);
    if s.contains(TOUT_NESTED_SENTINEL) {
        return Err(ToolOutputError::NestedToolCallSentinel);
    }
    let upper = s.to_uppercase();
    for pat in SQL_PATTERNS {
        if upper.contains(pat) {
            return Err(ToolOutputError::SqlInjectionPattern);
        }
    }
    for pat in PATH_PATTERNS {
        if s.contains(pat) {
            return Err(ToolOutputError::PathTraversal);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **TOUT-01** — empty output rejected.
    #[test]
    fn tout_01_empty_output_rejected() {
        assert_eq!(
            sanitize_tool_output(b""),
            Err(ToolOutputError::EmptyOutput)
        );
    }

    /// **TOUT-02** — oversized output rejected.
    #[test]
    fn tout_02_oversized_rejected() {
        let output = vec![b'A'; TOUT_MAX_LEN + 1];
        assert_eq!(
            sanitize_tool_output(&output),
            Err(ToolOutputError::OutputTooLong)
        );
    }

    /// **TOUT-03** — control character (null byte) rejected.
    #[test]
    fn tout_03_control_char_rejected() {
        assert_eq!(
            sanitize_tool_output(b"hello\x00world"),
            Err(ToolOutputError::ControlCharacter)
        );
    }

    /// **TOUT-04** — nested tool call sentinel rejected.
    #[test]
    fn tout_04_nested_sentinel_rejected() {
        let output = format!("result: {} done", TOUT_NESTED_SENTINEL);
        assert_eq!(
            sanitize_tool_output(output.as_bytes()),
            Err(ToolOutputError::NestedToolCallSentinel)
        );
    }

    /// **TOUT-05** — SQL injection pattern rejected.
    #[test]
    fn tout_05_sql_injection_rejected() {
        assert_eq!(
            sanitize_tool_output(b"DROP TABLE users; --"),
            Err(ToolOutputError::SqlInjectionPattern)
        );
    }

    /// **TOUT-06** — path traversal rejected.
    #[test]
    fn tout_06_path_traversal_rejected() {
        assert_eq!(
            sanitize_tool_output(b"file: ../../etc/passwd"),
            Err(ToolOutputError::PathTraversal)
        );
    }

    /// **TOUT-07** — clean output accepted.
    #[test]
    fn tout_07_clean_output_accepted() {
        assert_eq!(sanitize_tool_output(b"Hello, world!"), Ok(()));
    }

    /// **TOUT-08** — output with newlines accepted.
    #[test]
    fn tout_08_newlines_accepted() {
        assert_eq!(sanitize_tool_output(b"line1\nline2\r\nline3"), Ok(()));
    }

    /// **TOUT-09** — exact max length accepted.
    #[test]
    fn tout_09_exact_max_len_accepted() {
        let output = vec![b'X'; TOUT_MAX_LEN];
        assert_eq!(sanitize_tool_output(&output), Ok(()));
    }

    /// **TOUT-10** — SQL pattern case-insensitive rejected.
    #[test]
    fn tout_10_sql_case_insensitive_rejected() {
        assert_eq!(
            sanitize_tool_output(b"drop table users"),
            Err(ToolOutputError::SqlInjectionPattern)
        );
    }
}
