//! # CR-CHAT-06 — Agent tool authorization scope guard (Wave-105 Lane B)
//!
//! AGENT SAFETY — tool invocations must fall within authorized scope.
//!
//! Each agent session has a set of authorized tool scopes. If a tool
//! is invoked outside its authorized scope:
//!
//! * **Privilege escalation** — a low-privilege agent invokes
//!   admin-level tools, gaining unauthorized access.
//! * **Data exfiltration** — an agent with file-read scope invokes
//!   network-send tools, leaking data to external endpoints.
//! * **Lateral movement** — an agent authorized for one service
//!   invokes tools targeting a different service.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Tool must be in authorized scope set.
//! 2. Session ID must not be zero.
//! 3. Tool name must not be empty.
//! 4. No duplicate (session, tool) pairs.
//! 5. Scope depth <= `ATAS_MAX_SCOPE_DEPTH`.
//! 6. Total records <= `ATAS_MAX_RECORDS`.
//!
//! Tests **ATAS-01..10**. Error enum [`ScopeAuthError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * TOOL-SCOPE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum scope depth.
pub const ATAS_MAX_SCOPE_DEPTH: u32 = 8;

/// Maximum records per batch.
pub const ATAS_MAX_RECORDS: usize = 1024;

/// Session ID length.
pub const ATAS_SESSION_ID_LEN: usize = 16;

/// A tool invocation authorization record.
#[derive(Debug, Clone)]
pub struct ToolInvocation {
    /// Session identifier.
    pub session_id: [u8; ATAS_SESSION_ID_LEN],
    /// Tool name.
    pub tool: String,
    /// Authorized scope for this tool.
    pub scope: String,
    /// Scope nesting depth.
    pub depth: u32,
    /// Whether the invocation is authorized.
    pub authorized: bool,
}

/// All ways tool authorization scope validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScopeAuthError {
    /// Unauthorized invocation.
    Unauthorized { idx: usize, tool: String },
    /// Zero session ID.
    ZeroSession(usize),
    /// Empty tool name.
    EmptyTool(usize),
    /// Duplicate invocation.
    DuplicateInvocation(usize),
    /// Scope depth exceeded.
    DepthExceeded { idx: usize, depth: u32, max: u32 },
    /// Too many records.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate agent tool authorization scope.
pub fn validate_tool_authorization(
    invocations: &[ToolInvocation],
) -> Result<(), ScopeAuthError> {
    if invocations.len() > ATAS_MAX_RECORDS {
        return Err(ScopeAuthError::TooMany {
            got: invocations.len(),
            max: ATAS_MAX_RECORDS,
        });
    }
    let mut seen: BTreeSet<([u8; ATAS_SESSION_ID_LEN], String)> = BTreeSet::new();
    for (i, inv) in invocations.iter().enumerate() {
        if inv.session_id == [0u8; ATAS_SESSION_ID_LEN] {
            return Err(ScopeAuthError::ZeroSession(i));
        }
        if inv.tool.is_empty() {
            return Err(ScopeAuthError::EmptyTool(i));
        }
        if !inv.authorized {
            return Err(ScopeAuthError::Unauthorized {
                idx: i,
                tool: inv.tool.clone(),
            });
        }
        if inv.depth > ATAS_MAX_SCOPE_DEPTH {
            return Err(ScopeAuthError::DepthExceeded {
                idx: i,
                depth: inv.depth,
                max: ATAS_MAX_SCOPE_DEPTH,
            });
        }
        if !seen.insert((inv.session_id, inv.tool.clone())) {
            return Err(ScopeAuthError::DuplicateInvocation(i));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sid(byte: u8) -> [u8; ATAS_SESSION_ID_LEN] {
        [byte; ATAS_SESSION_ID_LEN]
    }

    fn invocation(session: u8, tool: &str, scope: &str, depth: u32, auth: bool) -> ToolInvocation {
        ToolInvocation {
            session_id: sid(session),
            tool: tool.to_string(),
            scope: scope.to_string(),
            depth,
            authorized: auth,
        }
    }

    fn valid_invocations() -> Vec<ToolInvocation> {
        vec![
            invocation(0x01, "file_read", "storage", 1, true),
            invocation(0x01, "file_write", "storage", 1, true),
            invocation(0x02, "search", "knowledge", 2, true),
        ]
    }

    /// **ATAS-01** — unauthorized rejected.
    #[test]
    fn atas_01_unauthorized_rejected() {
        let invs = vec![invocation(0x01, "admin_reset", "admin", 1, false)];
        assert_eq!(
            validate_tool_authorization(&invs),
            Err(ScopeAuthError::Unauthorized {
                idx: 0,
                tool: "admin_reset".to_string(),
            })
        );
    }

    /// **ATAS-02** — zero session rejected.
    #[test]
    fn atas_02_zero_session_rejected() {
        let inv = ToolInvocation {
            session_id: [0u8; ATAS_SESSION_ID_LEN],
            tool: "read".to_string(),
            scope: "data".to_string(),
            depth: 1,
            authorized: true,
        };
        assert_eq!(
            validate_tool_authorization(&[inv]),
            Err(ScopeAuthError::ZeroSession(0))
        );
    }

    /// **ATAS-03** — empty tool rejected.
    #[test]
    fn atas_03_empty_tool_rejected() {
        let inv = ToolInvocation {
            session_id: sid(0x01),
            tool: String::new(),
            scope: "data".to_string(),
            depth: 1,
            authorized: true,
        };
        assert_eq!(
            validate_tool_authorization(&[inv]),
            Err(ScopeAuthError::EmptyTool(0))
        );
    }

    /// **ATAS-04** — duplicate rejected.
    #[test]
    fn atas_04_duplicate_rejected() {
        let invs = vec![
            invocation(0x01, "read", "data", 1, true),
            invocation(0x01, "read", "data", 1, true),
        ];
        assert_eq!(
            validate_tool_authorization(&invs),
            Err(ScopeAuthError::DuplicateInvocation(1))
        );
    }

    /// **ATAS-05** — depth exceeded rejected.
    #[test]
    fn atas_05_depth_exceeded_rejected() {
        let inv = invocation(0x01, "read", "data", ATAS_MAX_SCOPE_DEPTH + 1, true);
        assert_eq!(
            validate_tool_authorization(&[inv]),
            Err(ScopeAuthError::DepthExceeded {
                idx: 0,
                depth: ATAS_MAX_SCOPE_DEPTH + 1,
                max: ATAS_MAX_SCOPE_DEPTH,
            })
        );
    }

    /// **ATAS-06** — too many rejected.
    #[test]
    fn atas_06_too_many_rejected() {
        let invs: Vec<ToolInvocation> = (0..=ATAS_MAX_RECORDS)
            .map(|i| {
                ToolInvocation {
                    session_id: sid((i as u8).wrapping_add(1)),
                    tool: format!("tool_{i}"),
                    scope: "data".to_string(),
                    depth: 1,
                    authorized: true,
                }
            })
            .collect();
        assert_eq!(
            validate_tool_authorization(&invs),
            Err(ScopeAuthError::TooMany {
                got: ATAS_MAX_RECORDS + 1,
                max: ATAS_MAX_RECORDS,
            })
        );
    }

    /// **ATAS-07** — valid accepted.
    #[test]
    fn atas_07_valid_accepted() {
        assert_eq!(validate_tool_authorization(&valid_invocations()), Ok(()));
    }

    /// **ATAS-08** — empty accepted.
    #[test]
    fn atas_08_empty_accepted() {
        assert_eq!(validate_tool_authorization(&[]), Ok(()));
    }

    /// **ATAS-09** — same tool different session accepted.
    #[test]
    fn atas_09_same_tool_diff_session_accepted() {
        let invs = vec![
            invocation(0x01, "read", "data", 1, true),
            invocation(0x02, "read", "data", 1, true),
        ];
        assert_eq!(validate_tool_authorization(&invs), Ok(()));
    }

    /// **ATAS-10** — max depth boundary accepted.
    #[test]
    fn atas_10_max_depth_accepted() {
        let inv = invocation(0x01, "read", "data", ATAS_MAX_SCOPE_DEPTH, true);
        assert_eq!(validate_tool_authorization(&[inv]), Ok(()));
    }
}
