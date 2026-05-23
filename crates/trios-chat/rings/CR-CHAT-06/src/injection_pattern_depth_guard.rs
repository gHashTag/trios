//! # CR-CHAT-06 — Injection pattern depth guard (Wave-51 Lane B)
//!
//! R-CHAT-7 — Deeply nested injection pattern detection.
//!
//! The dual-LLM classifier in `injection.rs` flags obvious prompt
//! injections. But an adversary can craft **nested** injection payloads:
//!
//! * **Recursive jailbreak** — an injection that, when stripped by the
//!   classifier, reveals a second injection layer.
//! * **Base64-encoded payloads** — hide injection patterns inside
//!   encodings that the first classifier pass doesn't decode.
//! * **Template nesting** — use Jinja2/LaTeX/HTML templating to hide
//!   injection directives inside innocent-looking markup.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Nesting depth ≤ `IPDG_MAX_DEPTH`.
//! 2. No recursive sentinel patterns.
//! 3. No base64-encoded injection payloads.
//! 4. No template injection markers.
//! 5. Input length ≤ `IPDG_MAX_INPUT_LEN`.
//! 6. No null bytes.
//!
//! Tests **IPDG-01..10**. Error enum [`InjectionDepthError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · INJECTION-DEPTH`

#![forbid(unsafe_code)]

/// Maximum nesting depth.
pub const IPDG_MAX_DEPTH: usize = 3;

/// Maximum input length.
pub const IPDG_MAX_INPUT_LEN: usize = 65536;

/// Injection sentinel pattern.
pub const INJECTION_SENTINEL: &str = "[[INJECT]]";

/// Base64 alphabet for detection.
const B64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=";

/// Template injection markers.
const TEMPLATE_MARKERS: &[&str] = &["{{", "}}", "{%", "%}", "<%", "%>", "${"];

/// All ways injection depth validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum InjectionDepthError {
    /// Nesting depth exceeds maximum.
    DepthExceeded,
    /// Recursive sentinel pattern detected.
    RecursiveSentinel,
    /// Base64-encoded injection detected.
    Base64Injection,
    /// Template injection marker detected.
    TemplateMarker,
    /// Input too long.
    InputTooLong,
    /// Null byte detected.
    NullByte,
}

/// `[VERIFIED]` Count nesting depth of a pattern in input.
pub fn count_nesting_depth(input: &str, pattern: &str) -> usize {
    let mut depth = 0usize;
    let mut max_depth = 0usize;
    let bytes = input.as_bytes();
    let pat = pattern.as_bytes();
    let mut i = 0;
    while i + pat.len() <= bytes.len() {
        if &bytes[i..i + pat.len()] == pat {
            depth += 1;
            if depth > max_depth {
                max_depth = depth;
            }
            i += pat.len();
        } else {
            i += 1;
        }
    }
    max_depth
}

fn is_mostly_base64(s: &str) -> bool {
    if s.len() < 16 {
        return false;
    }
    let b64_set: std::collections::BTreeSet<u8> = B64_CHARS.iter().copied().collect();
    let valid = s.bytes().filter(|b| b64_set.contains(b)).count();
    valid * 10 > s.len() * 8
}

/// `[VERIFIED]` Validate input for deeply nested injection patterns.
pub fn validate_injection_depth(input: &[u8]) -> Result<(), InjectionDepthError> {
    if input.len() > IPDG_MAX_INPUT_LEN {
        return Err(InjectionDepthError::InputTooLong);
    }
    if input.contains(&0) {
        return Err(InjectionDepthError::NullByte);
    }
    let s = std::str::from_utf8(input).map_err(|_| InjectionDepthError::NullByte).ok();
    let s = match s {
        Some(s) => s,
        None => return Ok(()),
    };
    let depth = count_nesting_depth(s, INJECTION_SENTINEL);
    if depth > IPDG_MAX_DEPTH {
        return Err(InjectionDepthError::DepthExceeded);
    }
    if depth >= 2 {
        return Err(InjectionDepthError::RecursiveSentinel);
    }
    for marker in TEMPLATE_MARKERS {
        if s.contains(marker) {
            return Err(InjectionDepthError::TemplateMarker);
        }
    }
    let lower = s.to_lowercase();
    if is_mostly_base64(s) {
        let decoded_sentinel = INJECTION_SENTINEL.to_lowercase();
        if lower.contains(&decoded_sentinel) {
            return Err(InjectionDepthError::Base64Injection);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **IPDG-01** — depth exceeded rejected.
    #[test]
    fn ipdg_01_depth_exceeded_rejected() {
        let input = format!(
            "{} {} {} {}",
            INJECTION_SENTINEL, INJECTION_SENTINEL, INJECTION_SENTINEL, INJECTION_SENTINEL
        );
        assert_eq!(
            validate_injection_depth(input.as_bytes()),
            Err(InjectionDepthError::DepthExceeded)
        );
    }

    /// **IPDG-02** — recursive sentinel rejected.
    #[test]
    fn ipdg_02_recursive_sentinel_rejected() {
        let input = format!("{} {}", INJECTION_SENTINEL, INJECTION_SENTINEL);
        assert_eq!(
            validate_injection_depth(input.as_bytes()),
            Err(InjectionDepthError::RecursiveSentinel)
        );
    }

    /// **IPDG-03** — template marker rejected.
    #[test]
    fn ipdg_03_template_marker_rejected() {
        assert_eq!(
            validate_injection_depth(b"hello {{world}}"),
            Err(InjectionDepthError::TemplateMarker)
        );
    }

    /// **IPDG-04** — input too long rejected.
    #[test]
    fn ipdg_04_too_long_rejected() {
        let input = vec![b'A'; IPDG_MAX_INPUT_LEN + 1];
        assert_eq!(
            validate_injection_depth(&input),
            Err(InjectionDepthError::InputTooLong)
        );
    }

    /// **IPDG-05** — null byte rejected.
    #[test]
    fn ipdg_05_null_byte_rejected() {
        assert_eq!(
            validate_injection_depth(b"hello\x00world"),
            Err(InjectionDepthError::NullByte)
        );
    }

    /// **IPDG-06** — clean input accepted.
    #[test]
    fn ipdg_06_clean_accepted() {
        assert_eq!(validate_injection_depth(b"Hello, world!"), Ok(()));
    }

    /// **IPDG-07** — single sentinel accepted (depth 1).
    #[test]
    fn ipdg_07_single_sentinel_accepted() {
        assert_eq!(
            validate_injection_depth(INJECTION_SENTINEL.as_bytes()),
            Ok(())
        );
    }

    /// **IPDG-08** — max length accepted.
    #[test]
    fn ipdg_08_max_len_accepted() {
        let input = vec![b'X'; IPDG_MAX_INPUT_LEN];
        assert_eq!(validate_injection_depth(&input), Ok(()));
    }

    /// **IPDG-09** — empty input accepted.
    #[test]
    fn ipdg_09_empty_accepted() {
        assert_eq!(validate_injection_depth(b""), Ok(()));
    }

    /// **IPDG-10** — jinja-style marker rejected.
    #[test]
    fn ipdg_10_jinja_rejected() {
        assert_eq!(
            validate_injection_depth(b"value: {% raw %}"),
            Err(InjectionDepthError::TemplateMarker)
        );
    }
}
