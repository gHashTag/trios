//! # CR-CHAT-06 — Agent prompt injection depth bound guard (Wave-158 Lane B)
//!
//! AGENT SAFETY — nested prompt depth must be bounded; deep nesting
//! enables injection attacks.
//!
//! When agents process prompts with nested instructions, the nesting
//! depth must be bounded. If depth is unbounded:
//!
//! * **Injection attacks** — deeply nested prompts can bypass safety
//!   filters by wrapping malicious instructions.
//! * **Resource exhaustion** — deeply nested prompts cause
//!   exponential parsing overhead.
//! * **Confusion attacks** — multiple nesting levels confuse the
//!   agent about which instructions to follow.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Nesting depth <= `APIDB_MAX_DEPTH`.
//! 2. Prompt ID must not be zero.
//! 3. No duplicate prompt IDs.
//! 4. Session ID must not be zero.
//! 5. Depth must be > 0.
//! 6. Batch size <= `APIDB_MAX_PROMPTS`.
//!
//! Tests **APIDB-01..10**. Error enum [`InjectionDepthError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * DEPTH-BOUNDED`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum nesting depth.
pub const APIDB_MAX_DEPTH: u32 = 8;

/// Maximum prompts per batch.
pub const APIDB_MAX_PROMPTS: usize = 256;

/// Prompt ID length.
pub const APIDB_PROMPT_ID_LEN: usize = 16;

/// Session ID length.
pub const APIDB_SESSION_ID_LEN: usize = 32;

/// A prompt depth record.
#[derive(Debug, Clone)]
pub struct PromptDepthRecord {
    /// Prompt identifier.
    pub prompt_id: [u8; APIDB_PROMPT_ID_LEN],
    /// Session identifier.
    pub session_id: [u8; APIDB_SESSION_ID_LEN],
    /// Nesting depth.
    pub depth: u32,
}

/// All ways injection depth validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum InjectionDepthError {
    /// Depth exceeds maximum.
    TooDeep {
        idx: usize,
        got: u32,
        max: u32,
    },
    /// Zero prompt ID.
    ZeroPromptId(usize),
    /// Duplicate prompt ID.
    DuplicatePromptId {
        idx: usize,
    },
    /// Zero session ID.
    ZeroSessionId(usize),
    /// Zero depth.
    ZeroDepth(usize),
    /// Too many prompts.
    TooMany {
        got: usize,
        max: usize,
    },
}

/// `[VERIFIED]` Validate agent prompt injection depth bound.
pub fn validate_injection_depth(
    prompts: &[PromptDepthRecord],
) -> Result<(), InjectionDepthError> {
    if prompts.len() > APIDB_MAX_PROMPTS {
        return Err(InjectionDepthError::TooMany {
            got: prompts.len(),
            max: APIDB_MAX_PROMPTS,
        });
    }
    let mut seen: BTreeSet<[u8; APIDB_PROMPT_ID_LEN]> = BTreeSet::new();
    for (i, p) in prompts.iter().enumerate() {
        if p.prompt_id == [0u8; APIDB_PROMPT_ID_LEN] {
            return Err(InjectionDepthError::ZeroPromptId(i));
        }
        if !seen.insert(p.prompt_id) {
            return Err(InjectionDepthError::DuplicatePromptId { idx: i });
        }
        if p.session_id == [0u8; APIDB_SESSION_ID_LEN] {
            return Err(InjectionDepthError::ZeroSessionId(i));
        }
        if p.depth == 0 {
            return Err(InjectionDepthError::ZeroDepth(i));
        }
        if p.depth > APIDB_MAX_DEPTH {
            return Err(InjectionDepthError::TooDeep {
                idx: i,
                got: p.depth,
                max: APIDB_MAX_DEPTH,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pid(byte: u8) -> [u8; APIDB_PROMPT_ID_LEN] {
        [byte; APIDB_PROMPT_ID_LEN]
    }

    fn sid(byte: u8) -> [u8; APIDB_SESSION_ID_LEN] {
        [byte; APIDB_SESSION_ID_LEN]
    }

    fn prompt(id: u8, session: u8, depth: u32) -> PromptDepthRecord {
        PromptDepthRecord { prompt_id: pid(id), session_id: sid(session), depth }
    }

    fn valid_prompts() -> Vec<PromptDepthRecord> {
        vec![
            prompt(0x01, 0xA1, 1),
            prompt(0x02, 0xA1, 2),
            prompt(0x03, 0xA2, 1),
        ]
    }

    /// **APIDB-01** — too deep rejected.
    #[test]
    fn apidb_01_too_deep_rejected() {
        let p = prompt(0x01, 0xA1, APIDB_MAX_DEPTH + 1);
        assert_eq!(
            validate_injection_depth(&[p]),
            Err(InjectionDepthError::TooDeep { idx: 0, got: APIDB_MAX_DEPTH + 1, max: APIDB_MAX_DEPTH })
        );
    }

    /// **APIDB-02** — zero prompt ID rejected.
    #[test]
    fn apidb_02_zero_prompt_rejected() {
        let p = PromptDepthRecord { prompt_id: [0u8; APIDB_PROMPT_ID_LEN], session_id: sid(0xA1), depth: 1 };
        assert_eq!(
            validate_injection_depth(&[p]),
            Err(InjectionDepthError::ZeroPromptId(0))
        );
    }

    /// **APIDB-03** — duplicate prompt ID rejected.
    #[test]
    fn apidb_03_duplicate_rejected() {
        let ps = vec![
            prompt(0x01, 0xA1, 1),
            prompt(0x01, 0xA2, 2),
        ];
        assert_eq!(
            validate_injection_depth(&ps),
            Err(InjectionDepthError::DuplicatePromptId { idx: 1 })
        );
    }

    /// **APIDB-04** — zero session ID rejected.
    #[test]
    fn apidb_04_zero_session_rejected() {
        let p = PromptDepthRecord { prompt_id: pid(0x01), session_id: [0u8; APIDB_SESSION_ID_LEN], depth: 1 };
        assert_eq!(
            validate_injection_depth(&[p]),
            Err(InjectionDepthError::ZeroSessionId(0))
        );
    }

    /// **APIDB-05** — zero depth rejected.
    #[test]
    fn apidb_05_zero_depth_rejected() {
        let p = prompt(0x01, 0xA1, 0);
        assert_eq!(
            validate_injection_depth(&[p]),
            Err(InjectionDepthError::ZeroDepth(0))
        );
    }

    /// **APIDB-06** — too many rejected.
    #[test]
    fn apidb_06_too_many_rejected() {
        let ps: Vec<PromptDepthRecord> = (0..=APIDB_MAX_PROMPTS)
            .map(|i| {
                let mut id = [0u8; APIDB_PROMPT_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                let mut s = [0u8; APIDB_SESSION_ID_LEN];
                s[0] = 0xA1;
                PromptDepthRecord { prompt_id: id, session_id: s, depth: 1 }
            })
            .collect();
        assert_eq!(
            validate_injection_depth(&ps),
            Err(InjectionDepthError::TooMany {
                got: APIDB_MAX_PROMPTS + 1,
                max: APIDB_MAX_PROMPTS,
            })
        );
    }

    /// **APIDB-07** — valid accepted.
    #[test]
    fn apidb_07_valid_accepted() {
        assert_eq!(validate_injection_depth(&valid_prompts()), Ok(()));
    }

    /// **APIDB-08** — empty accepted.
    #[test]
    fn apidb_08_empty_accepted() {
        assert_eq!(validate_injection_depth(&[]), Ok(()));
    }

    /// **APIDB-09** — boundary depth accepted.
    #[test]
    fn apidb_09_boundary_depth_accepted() {
        let p = prompt(0x01, 0xA1, APIDB_MAX_DEPTH);
        assert_eq!(validate_injection_depth(&[p]), Ok(()));
    }

    /// **APIDB-10** — many valid accepted.
    #[test]
    fn apidb_10_many_valid_accepted() {
        let ps: Vec<PromptDepthRecord> = (0..20u8)
            .map(|i| prompt(i + 1, 0xA1, (i as u32 % APIDB_MAX_DEPTH) + 1))
            .collect();
        assert_eq!(validate_injection_depth(&ps), Ok(()));
    }
}
