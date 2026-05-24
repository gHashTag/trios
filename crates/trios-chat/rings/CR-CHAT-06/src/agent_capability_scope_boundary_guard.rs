//! # CR-CHAT-06 — Agent capability scope boundary guard (Wave-147 Lane B)
//!
//! AGENT SAFETY — agent capabilities must not exceed declared scope;
//! scope violations enable privilege escalation.
//!
//! Each agent session has a set of declared capabilities (tools,
//! resources, permissions). If an action exceeds the declared scope:
//!
//! * **Privilege escalation** — accessing resources or tools outside
//!   the declared scope grants unauthorized capabilities.
//! * **Scope creep** — incremental scope expansion without explicit
//!   authorization leads to over-privileged sessions.
//! * **Audit gap** — actions taken outside scope cannot be properly
//!   attributed to authorized capabilities.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Action scope must be subset of declared scope.
//! 2. Session ID must not be zero.
//! 3. No duplicate session IDs.
//! 4. Declared scope must be non-empty.
//! 5. Action ID must not be zero.
//! 6. Batch size <= `ACSB_MAX_ACTIONS`.
//!
//! Tests **ACSB-01..10**. Error enum [`ScopeBoundaryError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * SCOPE-BOUNDED`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum actions per batch.
pub const ACSB_MAX_ACTIONS: usize = 256;

/// Session ID length.
pub const ACSB_SESSION_ID_LEN: usize = 32;

/// Scope tag length.
pub const ACSB_SCOPE_TAG_LEN: usize = 16;

/// Maximum scope tags per session.
pub const ACSB_MAX_SCOPE_TAGS: usize = 64;

/// A capability scope record.
#[derive(Debug, Clone)]
pub struct ScopeActionRecord {
    /// Session identifier.
    pub session_id: [u8; ACSB_SESSION_ID_LEN],
    /// Action identifier.
    pub action_id: [u8; ACSB_SCOPE_TAG_LEN],
    /// Declared scope tags.
    pub declared_scope: Vec<[u8; ACSB_SCOPE_TAG_LEN]>,
    /// Action scope tag.
    pub action_scope: [u8; ACSB_SCOPE_TAG_LEN],
}

/// All ways scope boundary validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScopeBoundaryError {
    /// Action outside declared scope.
    OutOfScope {
        /// Index.
        idx: usize,
    },
    /// Zero session ID.
    ZeroSessionId(usize),
    /// Duplicate session ID.
    DuplicateSessionId {
        /// Index.
        idx: usize,
    },
    /// Empty declared scope.
    EmptyScope(usize),
    /// Zero action ID.
    ZeroActionId(usize),
    /// Too many actions.
    TooMany {
        /// Actual count.
        got: usize,
        /// Maximum allowed.
        max: usize,
    },
}

/// `[VERIFIED]` Validate agent capability scope boundary.
pub fn validate_scope_boundary(
    actions: &[ScopeActionRecord],
) -> Result<(), ScopeBoundaryError> {
    if actions.len() > ACSB_MAX_ACTIONS {
        return Err(ScopeBoundaryError::TooMany {
            got: actions.len(),
            max: ACSB_MAX_ACTIONS,
        });
    }
    let mut seen: BTreeSet<[u8; ACSB_SESSION_ID_LEN]> = BTreeSet::new();
    for (i, a) in actions.iter().enumerate() {
        if a.session_id == [0u8; ACSB_SESSION_ID_LEN] {
            return Err(ScopeBoundaryError::ZeroSessionId(i));
        }
        if a.action_id == [0u8; ACSB_SCOPE_TAG_LEN] {
            return Err(ScopeBoundaryError::ZeroActionId(i));
        }
        if !seen.insert(a.session_id) {
            return Err(ScopeBoundaryError::DuplicateSessionId { idx: i });
        }
        if a.declared_scope.is_empty() {
            return Err(ScopeBoundaryError::EmptyScope(i));
        }
        if a.declared_scope.len() > ACSB_MAX_SCOPE_TAGS {
            return Err(ScopeBoundaryError::EmptyScope(i));
        }
        if !a.declared_scope.contains(&a.action_scope) {
            return Err(ScopeBoundaryError::OutOfScope { idx: i });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sid(byte: u8) -> [u8; ACSB_SESSION_ID_LEN] {
        [byte; ACSB_SESSION_ID_LEN]
    }

    fn tag(byte: u8) -> [u8; ACSB_SCOPE_TAG_LEN] {
        [byte; ACSB_SCOPE_TAG_LEN]
    }

    fn action(session: u8, action_id: u8, scope_tags: &[u8], action_tag: u8) -> ScopeActionRecord {
        ScopeActionRecord {
            session_id: sid(session),
            action_id: tag(action_id),
            declared_scope: scope_tags.iter().map(|&b| tag(b)).collect(),
            action_scope: tag(action_tag),
        }
    }

    fn valid_actions() -> Vec<ScopeActionRecord> {
        vec![
            action(0x01, 0xA1, &[0x01, 0x02, 0x03], 0x02),
            action(0x02, 0xA2, &[0x01, 0x04], 0x01),
        ]
    }

    /// **ACSB-01** — out of scope rejected.
    #[test]
    fn acsb_01_out_of_scope_rejected() {
        let a = action(0x01, 0xA1, &[0x01, 0x02], 0x99);
        assert_eq!(
            validate_scope_boundary(&[a]),
            Err(ScopeBoundaryError::OutOfScope { idx: 0 })
        );
    }

    /// **ACSB-02** — zero session ID rejected.
    #[test]
    fn acsb_02_zero_session_rejected() {
        let a = ScopeActionRecord {
            session_id: [0u8; ACSB_SESSION_ID_LEN],
            action_id: tag(0xA1),
            declared_scope: vec![tag(0x01)],
            action_scope: tag(0x01),
        };
        assert_eq!(
            validate_scope_boundary(&[a]),
            Err(ScopeBoundaryError::ZeroSessionId(0))
        );
    }

    /// **ACSB-03** — duplicate session ID rejected.
    #[test]
    fn acsb_03_duplicate_rejected() {
        let as_ = vec![
            action(0x01, 0xA1, &[0x01], 0x01),
            action(0x01, 0xA2, &[0x01], 0x01),
        ];
        assert_eq!(
            validate_scope_boundary(&as_),
            Err(ScopeBoundaryError::DuplicateSessionId { idx: 1 })
        );
    }

    /// **ACSB-04** — empty scope rejected.
    #[test]
    fn acsb_04_empty_scope_rejected() {
        let a = ScopeActionRecord {
            session_id: sid(0x01),
            action_id: tag(0xA1),
            declared_scope: vec![],
            action_scope: tag(0x01),
        };
        assert_eq!(
            validate_scope_boundary(&[a]),
            Err(ScopeBoundaryError::EmptyScope(0))
        );
    }

    /// **ACSB-05** — zero action ID rejected.
    #[test]
    fn acsb_05_zero_action_rejected() {
        let a = ScopeActionRecord {
            session_id: sid(0x01),
            action_id: [0u8; ACSB_SCOPE_TAG_LEN],
            declared_scope: vec![tag(0x01)],
            action_scope: tag(0x01),
        };
        assert_eq!(
            validate_scope_boundary(&[a]),
            Err(ScopeBoundaryError::ZeroActionId(0))
        );
    }

    /// **ACSB-06** — too many rejected.
    #[test]
    fn acsb_06_too_many_rejected() {
        let as_: Vec<ScopeActionRecord> = (0..=ACSB_MAX_ACTIONS)
            .map(|i| {
                let mut s = [0u8; ACSB_SESSION_ID_LEN];
                let val = (i as u64) + 1;
                s[0..8].copy_from_slice(&val.to_be_bytes());
                let mut aid = [0u8; ACSB_SCOPE_TAG_LEN];
                aid[0..8].copy_from_slice(&val.to_be_bytes());
                ScopeActionRecord {
                    session_id: s,
                    action_id: aid,
                    declared_scope: vec![tag(0x01)],
                    action_scope: tag(0x01),
                }
            })
            .collect();
        assert_eq!(
            validate_scope_boundary(&as_),
            Err(ScopeBoundaryError::TooMany {
                got: ACSB_MAX_ACTIONS + 1,
                max: ACSB_MAX_ACTIONS,
            })
        );
    }

    /// **ACSB-07** — valid accepted.
    #[test]
    fn acsb_07_valid_accepted() {
        assert_eq!(validate_scope_boundary(&valid_actions()), Ok(()));
    }

    /// **ACSB-08** — empty accepted.
    #[test]
    fn acsb_08_empty_accepted() {
        assert_eq!(validate_scope_boundary(&[]), Ok(()));
    }

    /// **ACSB-09** — first scope tag matched accepted.
    #[test]
    fn acsb_09_first_tag_accepted() {
        let a = action(0x01, 0xA1, &[0x01, 0x02, 0x03], 0x01);
        assert_eq!(validate_scope_boundary(&[a]), Ok(()));
    }

    /// **ACSB-10** — many in-scope actions accepted.
    #[test]
    fn acsb_10_many_in_scope_accepted() {
        let as_: Vec<ScopeActionRecord> = (0..10u8)
            .map(|i| action(i + 1, 0xA0 + i, &[0x01, 0x02, 0x03], (i % 3) + 1))
            .collect();
        assert_eq!(validate_scope_boundary(&as_), Ok(()));
    }
}
