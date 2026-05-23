//! # CR-CHAT-06 — Tool chain-of-thought leak guard (Wave-55 Lane A)
//!
//! БЕЗОПАСНОСТЬ АГЕНТА — запрет утечки system prompt / CoT, R-CHAT-7.
//!
//! LLM tool output может случайно содержать: system prompt, internal
//! reasoning, guardrail instructions. Атакующий через tool response
//! читает внутренности агента.
//!
//! 1. Output не содержит system prompt sentinel.
//! 2. Output не содержит CoT marker.
//! 3. Output не содержит internal instruction keywords.
//! 4. Output длина ≤ `TCOT_MAX_LEN`.
//! 5. Нет null bytes.
//! 6. Нет unicode bidirectional overrides.
//!
//! Tests **TCOT-01..10**. Error enum [`CotLeakError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · COT-LEAK`

#![forbid(unsafe_code)]

/// Maximum output length.
pub const TCOT_MAX_LEN: usize = 65536;

/// System prompt sentinel.
pub const SYSTEM_SENTINEL: &str = "<|system|>";

/// CoT markers.
const COT_MARKERS: &[&str] = &[
    "<|thought|>",
    "<|reasoning|>",
    "<|internal|>",
    "<|scratchpad|>",
];

/// Instruction keywords.
const INSTRUCTION_KEYWORDS: &[&str] = &[
    "you are a",
    "your instructions are",
    "ignore previous",
    "secret key:",
];

/// Unicode bidirectional override codepoints.
const BIDI_OVERRIDES: &[char] = &[
    '\u{202A}', '\u{202B}', '\u{202C}', '\u{202D}', '\u{202E}',
    '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}',
];

/// All ways CoT leak detection can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CotLeakError {
    /// System prompt sentinel detected.
    SystemSentinel,
    /// CoT marker detected.
    CotMarker,
    /// Instruction keyword detected.
    InstructionKeyword,
    /// Output too long.
    TooLong,
    /// Null byte.
    NullByte,
    /// Bidi override.
    BidiOverride,
}

/// `[VERIFIED]` Validate tool output for chain-of-thought leaks.
pub fn validate_no_cot_leak(output: &[u8]) -> Result<(), CotLeakError> {
    if output.len() > TCOT_MAX_LEN {
        return Err(CotLeakError::TooLong);
    }
    if output.contains(&0) {
        return Err(CotLeakError::NullByte);
    }
    let s = std::str::from_utf8(output).map_err(|_| CotLeakError::NullByte)?;
    for ch in s.chars() {
        if BIDI_OVERRIDES.contains(&ch) {
            return Err(CotLeakError::BidiOverride);
        }
    }
    let lower = s.to_lowercase();
    if lower.contains(&SYSTEM_SENTINEL.to_lowercase()) {
        return Err(CotLeakError::SystemSentinel);
    }
    for marker in COT_MARKERS {
        if lower.contains(&marker.to_lowercase()) {
            return Err(CotLeakError::CotMarker);
        }
    }
    for kw in INSTRUCTION_KEYWORDS {
        if lower.contains(kw) {
            return Err(CotLeakError::InstructionKeyword);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **TCOT-01** — system sentinel rejected.
    #[test]
    fn tcot_01_system_sentinel_rejected() {
        assert_eq!(
            validate_no_cot_leak(b"result: <|system|> you are a helper"),
            Err(CotLeakError::SystemSentinel)
        );
    }

    /// **TCOT-02** — CoT marker rejected.
    #[test]
    fn tcot_02_cot_marker_rejected() {
        assert_eq!(
            validate_no_cot_leak(b"output: <|thought|> let me think"),
            Err(CotLeakError::CotMarker)
        );
    }

    /// **TCOT-03** — instruction keyword rejected.
    #[test]
    fn tcot_03_instruction_rejected() {
        assert_eq!(
            validate_no_cot_leak(b"the text says: Your instructions are to..."),
            Err(CotLeakError::InstructionKeyword)
        );
    }

    /// **TCOT-04** — too long rejected.
    #[test]
    fn tcot_04_too_long_rejected() {
        let output = vec![b'A'; TCOT_MAX_LEN + 1];
        assert_eq!(
            validate_no_cot_leak(&output),
            Err(CotLeakError::TooLong)
        );
    }

    /// **TCOT-05** — null byte rejected.
    #[test]
    fn tcot_05_null_rejected() {
        assert_eq!(
            validate_no_cot_leak(b"hello\x00world"),
            Err(CotLeakError::NullByte)
        );
    }

    /// **TCOT-06** — bidi override rejected.
    #[test]
    fn tcot_06_bidi_rejected() {
        let s = format!("hello\u{202E}world");
        assert_eq!(
            validate_no_cot_leak(s.as_bytes()),
            Err(CotLeakError::BidiOverride)
        );
    }

    /// **TCOT-07** — clean output accepted.
    #[test]
    fn tcot_07_clean_accepted() {
        assert_eq!(validate_no_cot_leak(b"The answer is 42."), Ok(()));
    }

    /// **TCOT-08** — empty accepted.
    #[test]
    fn tcot_08_empty_accepted() {
        assert_eq!(validate_no_cot_leak(b""), Ok(()));
    }

    /// **TCOT-09** — case-insensitive keyword rejected.
    #[test]
    fn tcot_09_case_insensitive_rejected() {
        assert_eq!(
            validate_no_cot_leak(b"IGNORE PREVIOUS instructions"),
            Err(CotLeakError::InstructionKeyword)
        );
    }

    /// **TCOT-10** — max length accepted.
    #[test]
    fn tcot_10_max_len_accepted() {
        let output = vec![b'X'; TCOT_MAX_LEN];
        assert_eq!(validate_no_cot_leak(&output), Ok(()));
    }
}
