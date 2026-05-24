//! # CR-CHAT-06 — Agent output content type validation guard (Wave-86 Lane B)
//!
//! AGENT SAFETY — agent outputs must match the tool's expected content
//! type, R-CHAT-7.
//!
//! Tools declare expected response types (JSON, plaintext, binary, etc.).
//! If content types are not validated:
//!
//! * **Content smuggling** — a compromised agent returns HTML where JSON
//!   was expected, injecting scripts into a downstream renderer.
//! * **Type confusion** — binary data interpreted as text enables
//!   encoding-based injection attacks on downstream consumers.
//! * **MIME confusion** — mismatched MIME types allow an attacker to
//!   bypass content-type-based security policies.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Declared content type must match tool's expected type.
//! 2. Content type must be recognized.
//! 3. Payload size <= `OCTV_MAX_PAYLOAD`.
//! 4. Payload size >= `OCTV_MIN_PAYLOAD`.
//! 5. No more than `OCTV_MAX_MISMATCHES` type mismatches per session.
//! 6. Tool name must be non-empty.
//!
//! Tests **OCTV-01..10**. Error enum [`ContentTypeError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * CONTENT-TYPE-VALID`

#![forbid(unsafe_code)]

/// Maximum payload size.
pub const OCTV_MAX_PAYLOAD: usize = 1_048_576;

/// Minimum payload size.
pub const OCTV_MIN_PAYLOAD: usize = 1;

/// Maximum allowed mismatches.
pub const OCTV_MAX_MISMATCHES: usize = 0;

/// Known content types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    /// JSON content.
    Json,
    /// Plain text.
    PlainText,
    /// Binary data.
    Binary,
    /// HTML content.
    Html,
}

impl ContentType {
    fn name(&self) -> &'static str {
        match self {
            ContentType::Json => "application/json",
            ContentType::PlainText => "text/plain",
            ContentType::Binary => "application/octet-stream",
            ContentType::Html => "text/html",
        }
    }
}

/// A tool output with declared content type.
#[derive(Debug, Clone)]
pub struct ToolOutput {
    /// Tool name.
    pub tool: String,
    /// Expected content type from the tool manifest.
    pub expected_type: ContentType,
    /// Declared content type in the output.
    pub declared_type: ContentType,
    /// Payload size.
    pub payload_len: usize,
}

/// All ways content type validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContentTypeError {
    /// Content type mismatch.
    TypeMismatch { tool: String, expected: String, got: String },
    /// Unrecognized content type.
    UnrecognizedType(String),
    /// Payload too large.
    PayloadTooLarge { tool: String, len: usize, max: usize },
    /// Payload too small.
    PayloadTooSmall { tool: String, len: usize, min: usize },
    /// Too many mismatches.
    TooManyMismatches { count: usize, max: usize },
    /// Empty tool name.
    EmptyToolName,
}

/// `[VERIFIED]` Validate agent output content types.
pub fn validate_content_types(
    outputs: &[ToolOutput],
) -> Result<(), ContentTypeError> {
    let mut mismatches = 0usize;
    for o in outputs {
        if o.tool.is_empty() {
            return Err(ContentTypeError::EmptyToolName);
        }
        if o.payload_len < OCTV_MIN_PAYLOAD {
            return Err(ContentTypeError::PayloadTooSmall {
                tool: o.tool.clone(),
                len: o.payload_len,
                min: OCTV_MIN_PAYLOAD,
            });
        }
        if o.payload_len > OCTV_MAX_PAYLOAD {
            return Err(ContentTypeError::PayloadTooLarge {
                tool: o.tool.clone(),
                len: o.payload_len,
                max: OCTV_MAX_PAYLOAD,
            });
        }
        if o.declared_type != o.expected_type {
            mismatches += 1;
            if mismatches > OCTV_MAX_MISMATCHES {
                return Err(ContentTypeError::TooManyMismatches {
                    count: mismatches,
                    max: OCTV_MAX_MISMATCHES,
                });
            }
        }
    }
    if mismatches > OCTV_MAX_MISMATCHES {
        return Err(ContentTypeError::TooManyMismatches {
            count: mismatches,
            max: OCTV_MAX_MISMATCHES,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output(tool: &str, expected: ContentType, declared: ContentType, len: usize) -> ToolOutput {
        ToolOutput { tool: tool.to_string(), expected_type: expected, declared_type: declared, payload_len: len }
    }

    fn json_out(tool: &str, len: usize) -> ToolOutput {
        output(tool, ContentType::Json, ContentType::Json, len)
    }

    fn valid_outputs() -> Vec<ToolOutput> {
        vec![json_out("read_file", 100), json_out("search", 200)]
    }

    /// **OCTV-01** — type mismatch rejected.
    #[test]
    fn octv_01_type_mismatch_rejected() {
        let o = output("tool", ContentType::Json, ContentType::Html, 100);
        let err = validate_content_types(&[o]);
        assert!(matches!(err, Err(ContentTypeError::TooManyMismatches { .. })));
    }

    /// **OCTV-02** — recognized types accepted.
    #[test]
    fn octv_02_recognized_accepted() {
        for (expected, declared) in [
            (ContentType::Json, ContentType::Json),
            (ContentType::PlainText, ContentType::PlainText),
            (ContentType::Binary, ContentType::Binary),
            (ContentType::Html, ContentType::Html),
        ] {
            let o = output("tool", expected, declared, 50);
            assert_eq!(validate_content_types(&[o]), Ok(()));
        }
    }

    /// **OCTV-03** — payload too large rejected.
    #[test]
    fn octv_03_payload_too_large_rejected() {
        let o = output("tool", ContentType::Json, ContentType::Json, OCTV_MAX_PAYLOAD + 1);
        assert_eq!(
            validate_content_types(&[o]),
            Err(ContentTypeError::PayloadTooLarge {
                tool: "tool".to_string(),
                len: OCTV_MAX_PAYLOAD + 1,
                max: OCTV_MAX_PAYLOAD,
            })
        );
    }

    /// **OCTV-04** — payload too small rejected.
    #[test]
    fn octv_04_payload_too_small_rejected() {
        let o = output("tool", ContentType::Json, ContentType::Json, 0);
        assert_eq!(
            validate_content_types(&[o]),
            Err(ContentTypeError::PayloadTooSmall {
                tool: "tool".to_string(),
                len: 0,
                min: OCTV_MIN_PAYLOAD,
            })
        );
    }

    /// **OCTV-05** — too many mismatches rejected.
    #[test]
    fn octv_05_too_many_mismatches_rejected() {
        let outputs: Vec<ToolOutput> = (0..=OCTV_MAX_MISMATCHES as u64 + 1)
            .map(|i| output(&format!("tool_{i}"), ContentType::Json, ContentType::Html, 10))
            .collect();
        assert!(matches!(
            validate_content_types(&outputs),
            Err(ContentTypeError::TooManyMismatches { .. })
        ));
    }

    /// **OCTV-06** — empty tool name rejected.
    #[test]
    fn octv_06_empty_tool_rejected() {
        let o = output("", ContentType::Json, ContentType::Json, 50);
        assert_eq!(validate_content_types(&[o]), Err(ContentTypeError::EmptyToolName));
    }

    /// **OCTV-07** — valid outputs accepted.
    #[test]
    fn octv_07_valid_accepted() {
        assert_eq!(validate_content_types(&valid_outputs()), Ok(()));
    }

    /// **OCTV-08** — empty accepted.
    #[test]
    fn octv_08_empty_accepted() {
        assert_eq!(validate_content_types(&[]), Ok(()));
    }

    /// **OCTV-09** — single output accepted.
    #[test]
    fn octv_09_single_accepted() {
        assert_eq!(validate_content_types(&[json_out("tool", 50)]), Ok(()));
    }

    /// **OCTV-10** — max payload boundary accepted.
    #[test]
    fn octv_10_max_payload_accepted() {
        assert_eq!(
            validate_content_types(&[output("tool", ContentType::Binary, ContentType::Binary, OCTV_MAX_PAYLOAD)]),
            Ok(())
        );
    }
}
