//! # CR-CHAT-06 — Agent tool response content type guard (Wave-153 Lane B)
//!
//! AGENT SAFETY — tool responses must declare valid content types;
//! undeclared types enable content sniffing attacks.
//!
//! When agent tools return responses, they must declare a content type.
//! If content types are missing or invalid:
//!
//! * **Content sniffing** — clients may guess the content type,
//!   leading to XSS or other injection attacks.
//! * **Type confusion** — mismatched content types cause incorrect
//!   parsing and potential security vulnerabilities.
//! * **MIME confusion** — an attacker can craft responses that are
//!   interpreted differently by different parsers.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Content type must be in the allow-list.
//! 2. Tool ID must not be zero.
//! 3. No duplicate tool IDs.
//! 4. Response payload must not be empty.
//! 5. Content type string must not be empty.
//! 6. Batch size <= `CTCT_MAX_RESPONSES`.
//!
//! Tests **CTCT-01..10**. Error enum [`ContentTypeError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * CONTENT-TYPED`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum responses per batch.
pub const CTCT_MAX_RESPONSES: usize = 256;

/// Tool ID length.
pub const CTCT_TOOL_ID_LEN: usize = 16;

/// Maximum content type string length.
pub const CTCT_MAX_TYPE_LEN: usize = 128;

/// Allowed content types.
pub const CTCT_ALLOWED_TYPES: &[&str] = &[
    "text/plain",
    "text/markdown",
    "application/json",
    "application/octet-stream",
    "image/png",
    "image/jpeg",
];

/// A tool response content type record.
#[derive(Debug, Clone)]
pub struct ContentTypeRecord {
    /// Tool identifier.
    pub tool_id: [u8; CTCT_TOOL_ID_LEN],
    /// Content type string.
    pub content_type: String,
    /// Response payload length.
    pub payload_len: usize,
}

/// All ways content type validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContentTypeError {
    /// Content type not in allow-list.
    DisallowedType {
        idx: usize,
        got: String,
    },
    /// Zero tool ID.
    ZeroToolId(usize),
    /// Duplicate tool ID.
    DuplicateToolId {
        idx: usize,
    },
    /// Empty payload.
    EmptyPayload(usize),
    /// Empty content type.
    EmptyContentType(usize),
    /// Too many responses.
    TooMany {
        got: usize,
        max: usize,
    },
}

/// `[VERIFIED]` Validate tool response content type.
pub fn validate_content_types(
    responses: &[ContentTypeRecord],
) -> Result<(), ContentTypeError> {
    if responses.len() > CTCT_MAX_RESPONSES {
        return Err(ContentTypeError::TooMany {
            got: responses.len(),
            max: CTCT_MAX_RESPONSES,
        });
    }
    let mut seen: BTreeSet<[u8; CTCT_TOOL_ID_LEN]> = BTreeSet::new();
    for (i, r) in responses.iter().enumerate() {
        if r.tool_id == [0u8; CTCT_TOOL_ID_LEN] {
            return Err(ContentTypeError::ZeroToolId(i));
        }
        if !seen.insert(r.tool_id) {
            return Err(ContentTypeError::DuplicateToolId { idx: i });
        }
        if r.content_type.is_empty() {
            return Err(ContentTypeError::EmptyContentType(i));
        }
        if r.payload_len == 0 {
            return Err(ContentTypeError::EmptyPayload(i));
        }
        if !CTCT_ALLOWED_TYPES.contains(&r.content_type.as_str()) {
            return Err(ContentTypeError::DisallowedType {
                idx: i,
                got: r.content_type.clone(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tid(byte: u8) -> [u8; CTCT_TOOL_ID_LEN] {
        [byte; CTCT_TOOL_ID_LEN]
    }

    fn resp(id: u8, ct: &str, len: usize) -> ContentTypeRecord {
        ContentTypeRecord { tool_id: tid(id), content_type: ct.to_string(), payload_len: len }
    }

    fn valid_responses() -> Vec<ContentTypeRecord> {
        vec![
            resp(0x01, "text/plain", 100),
            resp(0x02, "application/json", 256),
            resp(0x03, "text/markdown", 512),
        ]
    }

    /// **CTCT-01** — disallowed type rejected.
    #[test]
    fn ctct_01_disallowed_rejected() {
        let r = resp(0x01, "text/html", 100);
        assert_eq!(
            validate_content_types(&[r]),
            Err(ContentTypeError::DisallowedType { idx: 0, got: "text/html".to_string() })
        );
    }

    /// **CTCT-02** — zero tool ID rejected.
    #[test]
    fn ctct_02_zero_tool_rejected() {
        let r = ContentTypeRecord { tool_id: [0u8; CTCT_TOOL_ID_LEN], content_type: "text/plain".to_string(), payload_len: 100 };
        assert_eq!(
            validate_content_types(&[r]),
            Err(ContentTypeError::ZeroToolId(0))
        );
    }

    /// **CTCT-03** — duplicate tool ID rejected.
    #[test]
    fn ctct_03_duplicate_rejected() {
        let rs = vec![
            resp(0x01, "text/plain", 100),
            resp(0x01, "application/json", 200),
        ];
        assert_eq!(
            validate_content_types(&rs),
            Err(ContentTypeError::DuplicateToolId { idx: 1 })
        );
    }

    /// **CTCT-04** — empty payload rejected.
    #[test]
    fn ctct_04_empty_payload_rejected() {
        let r = ContentTypeRecord { tool_id: tid(0x01), content_type: "text/plain".to_string(), payload_len: 0 };
        assert_eq!(
            validate_content_types(&[r]),
            Err(ContentTypeError::EmptyPayload(0))
        );
    }

    /// **CTCT-05** — empty content type rejected.
    #[test]
    fn ctct_05_empty_type_rejected() {
        let r = ContentTypeRecord { tool_id: tid(0x01), content_type: String::new(), payload_len: 100 };
        assert_eq!(
            validate_content_types(&[r]),
            Err(ContentTypeError::EmptyContentType(0))
        );
    }

    /// **CTCT-06** — too many rejected.
    #[test]
    fn ctct_06_too_many_rejected() {
        let rs: Vec<ContentTypeRecord> = (0..=CTCT_MAX_RESPONSES)
            .map(|i| {
                let mut id = [0u8; CTCT_TOOL_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                ContentTypeRecord { tool_id: id, content_type: "text/plain".to_string(), payload_len: 100 }
            })
            .collect();
        assert_eq!(
            validate_content_types(&rs),
            Err(ContentTypeError::TooMany {
                got: CTCT_MAX_RESPONSES + 1,
                max: CTCT_MAX_RESPONSES,
            })
        );
    }

    /// **CTCT-07** — valid accepted.
    #[test]
    fn ctct_07_valid_accepted() {
        assert_eq!(validate_content_types(&valid_responses()), Ok(()));
    }

    /// **CTCT-08** — empty accepted.
    #[test]
    fn ctct_08_empty_accepted() {
        assert_eq!(validate_content_types(&[]), Ok(()));
    }

    /// **CTCT-09** — each allowed type accepted.
    #[test]
    fn ctct_09_each_allowed_type_accepted() {
        for (i, ct) in CTCT_ALLOWED_TYPES.iter().enumerate() {
            let mut id = [0u8; CTCT_TOOL_ID_LEN];
            id[0] = (i as u8) + 1;
            let r = ContentTypeRecord { tool_id: id, content_type: ct.to_string(), payload_len: 50 };
            assert_eq!(validate_content_types(&[r]), Ok(()));
        }
    }

    /// **CTCT-10** — many valid accepted.
    #[test]
    fn ctct_10_many_valid_accepted() {
        let rs: Vec<ContentTypeRecord> = (0..20u8)
            .map(|i| resp(i + 1, "application/json", (i as usize + 1) * 64))
            .collect();
        assert_eq!(validate_content_types(&rs), Ok(()));
    }
}
